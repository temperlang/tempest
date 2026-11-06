// The playground page: the editor, the output tabs, and the workers.
//
// - Typing schedules a compile (debounced); the compiler's worker gets
//   one request at a time, and only the newest pending one waits. A
//   request that runs long while a newer one waits gets its worker
//   replaced, so a runaway loop can't hold up typing.
// - Each result fills every backend's tab, the editor's diagnostics,
//   and the Run tab's interpreter half; the generated JavaScript then
//   runs in a worker of its own for the other half.

import { autocompletion, closeBrackets, closeBracketsKeymap } from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import {
  HighlightStyle,
  type LanguageSupport,
  StreamLanguage,
  bracketMatching,
  indentOnInput,
  syntaxHighlighting,
} from "@codemirror/language";
import { type Diagnostic as LintDiagnostic, lintGutter, setDiagnostics } from "@codemirror/lint";
import { highlightSelectionMatches, searchKeymap } from "@codemirror/search";
import { Compartment, EditorState, type Extension } from "@codemirror/state";
import {
  EditorView,
  drawSelection,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import { tags } from "@lezer/highlight";
import { cpp, csharp, java } from "@codemirror/legacy-modes/mode/clike";
import { cmake } from "@codemirror/legacy-modes/mode/cmake";
import { javascript, json } from "@codemirror/legacy-modes/mode/javascript";
import { lua } from "@codemirror/legacy-modes/mode/lua";
import { python } from "@codemirror/legacy-modes/mode/python";
import { rust } from "@codemirror/legacy-modes/mode/rust";
import { toml } from "@codemirror/legacy-modes/mode/toml";
import { xml } from "@codemirror/legacy-modes/mode/xml";

import type {
  BackendOutput,
  CompileResult,
  Diagnostic,
  FromCompiler,
  FromJsRunner,
  GeneratedFile,
  RunOutput,
  TestResult,
  ToCompiler,
  ToJsRunner,
} from "./api";
import { examples } from "./examples";
import { decode, encode } from "./share";
import { temper } from "./temper-language";

const DEBOUNCE_MS = 300;
/** How long a compile may run while a newer request waits. */
const STALE_MS = 1500;
/** How long the generated JavaScript may run. */
const JS_RUN_MS = 5000;

/** Backends' names for people, and the files to show first, in tab order. */
const BACKENDS: Record<string, { label: string; main: RegExp }> = {
  cpp: { label: "C++", main: /\/init\.cpp$/ },
  csharp: { label: "C#", main: /Global\.cs$/ },
  java: { label: "Java", main: /Global\.java$/ },
  js: { label: "JavaScript", main: /\.internal\.js$/ },
  lua: { label: "Lua", main: /\/init\.lua$/ },
  py: { label: "Python", main: /^main\/main\/main\.py$/ },
  rust: { label: "Rust", main: /\/mod\.rs$/ },
};
const DEFAULT_TAB = "py";

const $ = <T extends HTMLElement>(selector: string): T => {
  const element = document.querySelector<T>(selector);
  if (!element) throw new Error(`no ${selector}`);
  return element;
};

// Theme: the system's, unless the toggle picks one.

const THEMES = ["system", "light", "dark"] as const;
type Theme = (typeof THEMES)[number];
const themeButton = $<HTMLButtonElement>("#theme");

function storedTheme(): Theme {
  try {
    const theme = localStorage.getItem("theme");
    return THEMES.find((t) => t === theme) ?? "system";
  } catch {
    return "system";
  }
}

function applyTheme(theme: Theme): void {
  if (theme === "system") delete document.documentElement.dataset.theme;
  else document.documentElement.dataset.theme = theme;
  themeButton.textContent = { system: "Theme: system", light: "Theme: light", dark: "Theme: dark" }[theme];
  try {
    localStorage.setItem("theme", theme);
  } catch {
    // Private windows may refuse; the theme still applies.
  }
}

let theme = storedTheme();
applyTheme(theme);
themeButton.addEventListener("click", () => {
  theme = THEMES[(THEMES.indexOf(theme) + 1) % THEMES.length] ?? "system";
  applyTheme(theme);
});

// Highlighting and editor looks, from CSS variables, so one theme
// serves light and dark.

const highlight = HighlightStyle.define([
  { tag: [tags.keyword, tags.controlKeyword, tags.definitionKeyword, tags.moduleKeyword], color: "var(--tok-keyword)" },
  { tag: [tags.string, tags.special(tags.string), tags.regexp], color: "var(--tok-string)" },
  { tag: tags.escape, color: "var(--tok-meta)" },
  { tag: [tags.comment, tags.lineComment, tags.blockComment, tags.docComment], color: "var(--tok-comment)", fontStyle: "italic" },
  { tag: [tags.number, tags.integer, tags.float], color: "var(--tok-number)" },
  { tag: [tags.typeName, tags.className, tags.namespace], color: "var(--tok-type)" },
  { tag: [tags.function(tags.variableName), tags.function(tags.propertyName)], color: "var(--tok-function)" },
  { tag: [tags.definition(tags.variableName), tags.definition(tags.propertyName)], color: "var(--tok-function)" },
  { tag: [tags.atom, tags.bool, tags.null, tags.self], color: "var(--tok-atom)" },
  { tag: [tags.meta, tags.annotation, tags.processingInstruction], color: "var(--tok-meta)" },
  { tag: [tags.tagName, tags.attributeName], color: "var(--tok-keyword)" },
  { tag: tags.invalid, color: "var(--error)" },
]);

const looks = EditorView.theme({
  "&": { height: "100%", backgroundColor: "var(--surface)", color: "var(--text)" },
  ".cm-scroller": { fontFamily: "var(--mono)", lineHeight: "1.55" },
  ".cm-content": { caretColor: "var(--accent)" },
  ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--accent)", borderLeftWidth: "2px" },
  ".cm-gutters": { backgroundColor: "var(--surface)", color: "var(--muted)", border: "none" },
  ".cm-activeLine": { backgroundColor: "var(--active-line)" },
  ".cm-activeLineGutter": { backgroundColor: "var(--active-line)", color: "var(--text)" },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection": {
    backgroundColor: "var(--selection) !important",
  },
  ".cm-matchingBracket": { backgroundColor: "var(--match)", outline: "none" },
  ".cm-tooltip": { backgroundColor: "var(--panel)", color: "var(--text)", border: "1px solid var(--border)" },
  ".cm-panels": { backgroundColor: "var(--panel)", color: "var(--text)" },
});

// The editor.

const compileSoon = debounce(requestCompile, DEBOUNCE_MS);
const shareSoon = debounce(() => void updateHash(), 1000);

const editor = new EditorView({
  parent: $("#editor"),
  state: EditorState.create({
    doc: "",
    extensions: [
      lineNumbers(),
      highlightActiveLineGutter(),
      history(),
      drawSelection(),
      indentOnInput(),
      bracketMatching(),
      closeBrackets(),
      autocompletion(),
      highlightActiveLine(),
      highlightSelectionMatches(),
      lintGutter(),
      keymap.of([...closeBracketsKeymap, ...defaultKeymap, ...searchKeymap, ...historyKeymap, indentWithTab]),
      temper,
      syntaxHighlighting(highlight),
      looks,
      EditorView.lineWrapping,
      EditorView.contentAttributes.of({ "aria-label": "Temper source" }),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          document.body.classList.add("pending");
          compileSoon();
          shareSoon();
        }
      }),
    ],
  }),
});

function source(): string {
  return editor.state.doc.toString();
}

function setSource(text: string): void {
  editor.dispatch({ changes: { from: 0, to: editor.state.doc.length, insert: text } });
}

// Output: one read-only viewer, its language swapped per file.

const outputLanguage = new Compartment();
const viewer = new EditorView({
  parent: $("#code"),
  state: EditorState.create({
    extensions: [
      lineNumbers(),
      EditorState.readOnly.of(true),
      EditorView.editable.of(false),
      EditorView.contentAttributes.of({ "aria-label": "Generated code", tabindex: "0" }),
      outputLanguage.of([]),
      syntaxHighlighting(highlight),
      highlightSelectionMatches(),
      keymap.of(searchKeymap),
      looks,
      EditorView.lineWrapping,
    ],
  }),
});

const MODES: [RegExp, Parameters<typeof StreamLanguage.define>[0]][] = [
  [/\.py$/, python],
  [/\.rs$/, rust],
  [/\.(js|mjs)$/, javascript],
  [/\.json$/, json],
  [/\.java$/, java],
  [/\.cs$/, csharp],
  [/\.(cpp|hpp|h|cc)$/, cpp],
  [/\.(lua|rockspec)$/, lua],
  [/\.toml$/, toml],
  [/\.(xml|csproj|props)$/, xml],
  [/CMakeLists\.txt$/, cmake],
];
const languages = new Map<string, LanguageSupport | Extension>();

function languageFor(path: string): Extension {
  const found = MODES.find(([pattern]) => pattern.test(path));
  if (!found) return [];
  const key = found[0].source;
  let language = languages.get(key);
  if (!language) {
    language = StreamLanguage.define(found[1]);
    languages.set(key, language);
  }
  return language;
}

// Tabs: the backends the wasm build has, by name, then Run and Errors.

const tabs = $("#tabs");
const panes = { code: $("#code-pane"), run: $("#run-pane"), errors: $("#errors-pane") };
const filePicker = $<HTMLSelectElement>("#file");
const backendError = $("#backend-error");
let backends: string[] = [];
let selected = DEFAULT_TAB;
const chosenFile = new Map<string, string>();
let latest: CompileResult | null = null;

function label(backend: string): string {
  return BACKENDS[backend]?.label ?? backend;
}

function buildTabs(): void {
  tabs.replaceChildren();
  // Known backends in the order above (alphabetical, as people say
  // them), then any others the build has.
  const order = Object.keys(BACKENDS);
  const rank = (name: string): number => (order.includes(name) ? order.indexOf(name) : order.length);
  const names = [...backends].sort((a, b) => rank(a) - rank(b) || a.localeCompare(b));
  for (const name of [...names, "run", "errors"]) {
    const tab = document.createElement("button");
    tab.className = "tab";
    tab.role = "tab";
    tab.dataset.tab = name;
    tab.id = `tab-${name}`;
    tab.append(name === "run" ? "Run" : name === "errors" ? "Errors" : label(name));
    if (name === "run" || name === "errors") {
      const badge = document.createElement("span");
      badge.className = "badge";
      badge.hidden = true;
      tab.append(badge);
    }
    tab.addEventListener("click", () => select(name));
    tabs.append(tab);
  }
  tabs.addEventListener("keydown", (event) => {
    if (event.key !== "ArrowRight" && event.key !== "ArrowLeft") return;
    const all = [...tabs.querySelectorAll<HTMLButtonElement>(".tab")];
    const at = all.findIndex((t) => t.dataset.tab === selected);
    const next = all[(at + (event.key === "ArrowRight" ? 1 : all.length - 1)) % all.length];
    if (next?.dataset.tab) {
      select(next.dataset.tab);
      next.focus();
    }
  });
  if (!backends.includes(selected)) selected = backends[0] ?? "run";
  select(selected);
}

function select(name: string): void {
  selected = name;
  for (const tab of tabs.querySelectorAll<HTMLButtonElement>(".tab")) {
    const on = tab.dataset.tab === name;
    tab.setAttribute("aria-selected", String(on));
    tab.tabIndex = on ? 0 : -1;
  }
  panes.code.hidden = name === "run" || name === "errors";
  panes.run.hidden = name !== "run";
  panes.errors.hidden = name !== "errors";
  if (!panes.code.hidden) showBackend();
}

function badge(tab: string, text: string, kind: "good" | "bad" | "plain"): void {
  const element = document.querySelector<HTMLElement>(`#tab-${tab} .badge`);
  if (!element) return;
  element.hidden = text === "";
  element.textContent = text;
  element.dataset.kind = kind;
}

/** The files worth showing: not source maps. */
function shown(output: BackendOutput | undefined): GeneratedFile[] {
  return output && "files" in output ? output.files.filter((f) => !f.path.endsWith(".map")) : [];
}

function showBackend(): void {
  const output = latest?.backends[selected];
  const files = shown(output);
  backendError.hidden = !(output && "error" in output);
  backendError.textContent = output && "error" in output ? output.error : "";
  const wanted = chosenFile.get(selected);
  const main = BACKENDS[selected]?.main;
  const file =
    files.find((f) => f.path === wanted) ?? files.find((f) => main?.test(f.path)) ?? files[0];
  filePicker.replaceChildren(
    ...files.map((f) => {
      const option = document.createElement("option");
      option.value = f.path;
      option.textContent = f.path;
      option.selected = f === file;
      return option;
    }),
  );
  filePicker.disabled = files.length < 2;
  filePicker.hidden = files.length === 0;
  const text = file?.text ?? "";
  viewer.dispatch({
    changes: { from: 0, to: viewer.state.doc.length, insert: text },
    effects: outputLanguage.reconfigure(file ? languageFor(file.path) : []),
  });
}

filePicker.addEventListener("change", () => {
  chosenFile.set(selected, filePicker.value);
  showBackend();
});

// Errors: in the editor, and listed in their tab.

function showDiagnostics(diagnostics: Diagnostic[]): void {
  const length = editor.state.doc.length;
  const inMain = diagnostics.filter((d) => d.from !== undefined && d.to !== undefined);
  const lint: LintDiagnostic[] = inMain.map((d) => {
    const from = Math.min(d.from ?? 0, length);
    return {
      from,
      to: Math.max(from, Math.min(d.to ?? from, length)),
      severity: d.level === "warn" ? "warning" : d.level,
      message: d.message,
    };
  });
  editor.dispatch(setDiagnostics(editor.state, lint));
  const list = $("#errors");
  const sorted = [...diagnostics].sort((a, b) => (a.from ?? 1e9) - (b.from ?? 1e9));
  list.replaceChildren(
    ...sorted.map((d) => {
      const item = document.createElement("li");
      item.className = `diagnostic ${d.level}`;
      const where = document.createElement("button");
      where.className = "where";
      where.textContent = d.line ? `${d.line}:${d.column ?? 1}` : d.file;
      where.disabled = d.from === undefined;
      where.addEventListener("click", () => {
        const at = Math.min(d.from ?? 0, editor.state.doc.length);
        editor.dispatch({ selection: { anchor: at }, scrollIntoView: true });
        editor.focus();
      });
      const message = document.createElement("span");
      message.textContent = `${d.level}: ${d.message}`;
      item.append(where, message);
      return item;
    }),
  );
  $("#no-errors").hidden = diagnostics.length > 0;
  const errors = diagnostics.filter((d) => d.level === "error").length;
  badge("errors", diagnostics.length ? String(diagnostics.length) : "", errors ? "bad" : "plain");
}

// Run: the interpreter's results beside the generated JavaScript's.

function renderRun(target: HTMLElement, run: RunOutput | null, note = ""): void {
  target.replaceChildren();
  if (note) {
    const p = document.createElement("p");
    p.className = "note";
    p.textContent = note;
    target.append(p);
  }
  if (!run) return;
  if (run.tests.length || run.defined) {
    const list = document.createElement("ul");
    list.className = "tests";
    for (const test of run.tests) {
      const item = document.createElement("li");
      item.className = test.failure === null ? "pass" : "fail";
      const mark = document.createElement("span");
      mark.className = "mark";
      mark.textContent = test.failure === null ? "✓" : "✗";
      mark.setAttribute("aria-label", test.failure === null ? "passed" : "failed");
      const name = document.createElement("span");
      name.textContent = test.name;
      item.append(mark, name);
      if (test.failure !== null) {
        const why = document.createElement("div");
        why.className = "why";
        why.textContent = test.failure;
        item.append(why);
      }
      list.append(item);
    }
    const summary = document.createElement("p");
    summary.className = `summary ${allPassed(run) ? "pass" : "fail"}`;
    summary.textContent = run.summary || summarize(run.tests, run.defined);
    target.append(summary, list);
  }
  if (run.output.length || run.failure) {
    const heading = document.createElement("h3");
    heading.textContent = "Output";
    const log = document.createElement("pre");
    log.className = "log";
    log.textContent = run.output.join("\n");
    target.append(heading, log);
  }
  if (run.failure) {
    const failure = document.createElement("pre");
    failure.className = "failure";
    failure.textContent = run.failure;
    target.append(failure);
  }
}

function allPassed(run: RunOutput): boolean {
  return run.tests.every((t) => t.failure === null) && run.tests.length === run.defined && !run.failure;
}

/** `tempest test`'s summary, for the JavaScript run's tests. */
function summarize(tests: TestResult[], defined: number): string {
  const passed = tests.filter((t) => t.failure === null).length;
  const notRun = Math.max(0, defined - tests.length);
  return `Tests passed: ${passed} of ${defined}${notRun ? ` (${notRun} not run)` : ""}`;
}

let interpreterRun: RunOutput | null = null;
let jsRun: RunOutput | null = null;
let jsDone = false;

function compareRuns(): void {
  const same = $("#same");
  if (!interpreterRun || !jsRun || !jsDone) {
    same.hidden = true;
    return;
  }
  const key = (r: RunOutput): string =>
    JSON.stringify([r.output, r.tests.map((t) => [t.name, t.failure === null])]);
  same.hidden = false;
  const agree = key(interpreterRun) === key(jsRun);
  same.textContent = agree ? "Both runs agree" : "The runs differ";
  same.dataset.kind = agree ? "good" : "bad";
}

function showInterpreter(run: RunOutput | null): void {
  interpreterRun = run;
  renderRun($("#interp"), run);
  if (run && (run.tests.length || run.defined)) {
    const passed = run.tests.filter((t) => t.failure === null).length;
    badge("run", `${passed}/${run.defined}`, allPassed(run) ? "good" : "bad");
  } else {
    badge("run", run?.failure ? "!" : "", "bad");
  }
}

// The generated JavaScript's run.

let jsStd: GeneratedFile[] | null = null;
let jsStdError = "";
let jsWorker: Worker | null = null;
let jsTimer = 0;
let pendingJs: GeneratedFile[] | null = null;

function runJs(output: BackendOutput | undefined): void {
  jsWorker?.terminate();
  jsWorker = null;
  clearTimeout(jsTimer);
  jsDone = false;
  compareRuns();
  if (!output || "error" in output) {
    jsRun = null;
    renderRun($("#js"), null, output ? "No JavaScript to run: it didn't generate." : "");
    return;
  }
  if (!jsStd) {
    pendingJs = output.files;
    renderRun($("#js"), null, jsStdError || "Loading std…");
    return;
  }
  const run: RunOutput = { output: [], failure: null, tests: [], defined: interpreterRun?.defined ?? 0, summary: "" };
  jsRun = run;
  renderRun($("#js"), null, "Running…");
  const worker = new Worker(`js-worker.js?v=${BUILD_VERSION}`, { type: "module", name: "javascript" });
  jsWorker = worker;
  const finish = (): void => {
    jsDone = true;
    worker.terminate();
    if (jsWorker === worker) jsWorker = null;
    clearTimeout(jsTimer);
    renderRun($("#js"), run);
    compareRuns();
  };
  worker.onmessage = (event: MessageEvent<FromJsRunner>) => {
    if (jsRun !== run) return;
    const message = event.data;
    if (message.type === "log") run.output.push(message.line);
    else if (message.type === "failure") run.failure = message.message;
    else if (message.type === "tests") run.tests = message.tests;
    else finish();
    // No summary until the tests have run.
    if (message.type !== "done") renderRun($("#js"), { ...run, defined: run.tests.length ? run.defined : 0 });
  };
  worker.onerror = (event) => {
    run.failure = event.message || "the JavaScript run failed";
    finish();
  };
  jsTimer = window.setTimeout(() => {
    run.failure = `Stopped after ${JS_RUN_MS / 1000} s`;
    finish();
  }, JS_RUN_MS);
  const message: ToJsRunner = { files: output.files, std: jsStd };
  worker.postMessage(message);
}

// The compiler's worker.

let wasmModule: WebAssembly.Module | null = null;
let worker: Worker | null = null;
let busy: { id: number; since: number } | null = null;
let waiting: { id: number; source: string } | null = null;
let nextId = 1;
let shownId = 0;
let staleTimer = 0;

function startWorker(module: WebAssembly.Module): void {
  worker?.terminate();
  busy = null;
  const w = new Worker(`compiler-worker.js?v=${BUILD_VERSION}`, { type: "module", name: "compiler" });
  worker = w;
  w.onmessage = (event: MessageEvent<FromCompiler>) => onCompiler(event.data);
  w.onerror = (event) => status(`The compiler stopped: ${event.message}`, true);
  const init: ToCompiler = { type: "init", module };
  w.postMessage(init);
}

function onCompiler(message: FromCompiler): void {
  switch (message.type) {
    case "ready":
      if (backends.join() !== message.backends.join()) {
        backends = message.backends;
        buildTabs();
      }
      sendNext();
      return;
    case "std":
      if ("files" in message.output) jsStd = message.output.files;
      else jsStdError = `std didn't generate: ${message.output.error}`;
      if (pendingJs) {
        const files = pendingJs;
        pendingJs = null;
        runJs({ files });
      }
      return;
    case "result":
      busy = null;
      if (message.id > shownId) {
        shownId = message.id;
        show(message.result, message.ms);
      }
      sendNext();
      return;
    case "failed":
      busy = null;
      if (message.id > shownId) {
        shownId = message.id;
        status(`The compiler failed: ${message.message}`, true);
      }
      sendNext();
      return;
  }
}

function requestCompile(): void {
  waiting = { id: nextId++, source: source() };
  sendNext();
}

function sendNext(): void {
  if (!worker || !waiting) return;
  if (busy) {
    // Replace a worker that's still on an older request after a while.
    clearTimeout(staleTimer);
    const left = STALE_MS - (performance.now() - busy.since);
    staleTimer = window.setTimeout(() => {
      if (busy && waiting && wasmModule) {
        status("Restarting the compiler after a slow run…", false);
        startWorker(wasmModule);
      }
    }, Math.max(0, left));
    return;
  }
  const request = waiting;
  waiting = null;
  busy = { id: request.id, since: performance.now() };
  const message: ToCompiler = { type: "compile", id: request.id, source: request.source };
  worker.postMessage(message);
}

function show(result: CompileResult, ms: number): void {
  latest = result;
  if (!waiting) document.body.classList.remove("pending");
  if (!panes.code.hidden) showBackend();
  showDiagnostics(result.diagnostics);
  showInterpreter(result.run);
  runJs(result.backends.js);
  status(`Compiled in ${ms < 10 ? ms.toFixed(1) : Math.round(ms)} ms`, false);
}

function status(text: string, bad: boolean): void {
  const element = $("#compile-status");
  element.textContent = text;
  element.dataset.kind = bad ? "bad" : "plain";
}

// Examples and links.

const picker = $<HTMLSelectElement>("#example");
picker.replaceChildren(
  ...examples.map((example, i) => {
    const option = document.createElement("option");
    option.value = String(i);
    option.textContent = example.title;
    return option;
  }),
);
picker.addEventListener("change", () => {
  const example = examples[Number(picker.value)];
  if (example) setSource(example.source);
});

async function updateHash(): Promise<void> {
  const text = source();
  const isExample = examples.some((e) => e.source === text);
  const hash = isExample ? "" : `#${await encode(text)}`;
  window.history.replaceState(null, "", `${location.pathname}${location.search}${hash}`);
}

$<HTMLButtonElement>("#share").addEventListener("click", async () => {
  window.history.replaceState(null, "", `${location.pathname}${location.search}#${await encode(source())}`);
  const button = $<HTMLButtonElement>("#share");
  try {
    await navigator.clipboard.writeText(location.href);
    button.textContent = "Link copied";
  } catch {
    button.textContent = "Link in address bar";
  }
  setTimeout(() => (button.textContent = "Share"), 2000);
});

function debounce(f: () => void, ms: number): () => void {
  let timer = 0;
  return () => {
    clearTimeout(timer);
    timer = window.setTimeout(f, ms);
  };
}

function size(bytes: number): string {
  return bytes >= 1 << 20 ? `${(bytes / (1 << 20)).toFixed(1)} MiB` : `${Math.round(bytes / 1024)} KiB`;
}

// Start: the source from the link or the first example, then the wasm.

async function start(): Promise<void> {
  $("#wasm-size").textContent = `wasm ${size(BUILD.wasmBytes)} (${size(BUILD.wasmGzipBytes)} gzipped)`;
  $("#commit").textContent = BUILD.commit ? `tempest ${BUILD.commit.slice(0, 7)}` : "tempest";
  const linked = await decode(location.hash);
  const first = examples[0];
  setSource(linked ?? first?.source ?? "");
  if (linked !== null) picker.value = String(examples.findIndex((e) => e.source === linked));
  status("Loading the compiler…", false);
  try {
    const response = fetch(`temper_wasm.wasm?v=${BUILD_VERSION}`);
    wasmModule = await WebAssembly.compileStreaming(response).catch(async () =>
      WebAssembly.compile(await (await fetch(`temper_wasm.wasm?v=${BUILD_VERSION}`)).arrayBuffer()),
    );
  } catch (error) {
    status(`Couldn't load the compiler: ${String(error)}`, true);
    return;
  }
  startWorker(wasmModule);
  requestCompile();
}

void start();
