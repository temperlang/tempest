// The REPL tab: a session of chained inputs, run by crates/temper-wasm's
// `repl_*` in a worker of its own (repl-worker.ts), with the editor's
// code as input 0.
//
// - Enter asks the worker whether the cell's text is whole (`tempest
//   repl`'s rule); if it is, each input it splits into runs in turn,
//   else Enter starts a new line, indented by the brackets open.
//   Shift+Enter always starts a new line; Ctrl or Cmd+Enter, and Run,
//   always run. On touch screens, Enter starts a new line. Up and Down
//   on the first and last line go through history.
// - Each input's answer shows as the CLI prints it, rendered as text,
//   never as HTML. Logged lines stream in as they're logged.
// - Each input may take `FUEL` steps; one that runs out offers "Keep
//   going", which runs the same text again, as the next input, with ten
//   times as many.
// - Stop terminates the worker and starts another, which replays the
//   earlier inputs quietly; the page keeps the list. A trap (a panic, or
//   the memory cap) restores the session the same way. So does a changed
//   editor, the next time the REPL is used.

import { defaultKeymap, history, historyKeymap, indentWithTab, insertNewlineAndIndent } from "@codemirror/commands";
import { type HighlightStyle, type Language, bracketMatching } from "@codemirror/language";
import { EditorState, type Extension } from "@codemirror/state";
import { EditorView, drawSelection, keymap, placeholder } from "@codemirror/view";
import { highlightCode } from "@lezer/highlight";

import type { FromRepl, ReplAnswer, ReplCompletion, ReplReplay, ToRepl } from "./api";

/** Steps per input, as for a compile (temper-frontend's `interp::FUEL`). */
export const FUEL = 50_000_000;
/** How many more steps "Keep going" gives, as a factor. */
const MORE = 10;
/** Steps for replaying an input that failed for its steps or was stopped. */
const FAIL_FAST = 1;
const HISTORY_KEY = "repl-history";
const HISTORY_SIZE = 200;

export interface ReplHost {
  /** The editor's code, for input 0. */
  source(): string;
  /** The compiled wasm module, once loaded. */
  module(): WebAssembly.Module | null;
  /** Temper highlighting and the editor's looks, for the cell. */
  extensions: Extension[];
  highlight: HighlightStyle;
  language: Language;
}

/** An input waiting for, or having, its turn in the worker. */
interface Job {
  text: string;
  fuel: number;
  block: HTMLElement;
}

const $ = <T extends HTMLElement>(selector: string): T => {
  const element = document.querySelector<T>(selector);
  if (!element) throw new Error(`no ${selector}`);
  return element;
};

function element(tag: string, className: string, text?: string): HTMLElement {
  const e = document.createElement(tag);
  e.className = className;
  if (text !== undefined) e.textContent = text;
  return e;
}

function plural(n: number, one: string, many = `${one}s`): string {
  return `${n} ${n === 1 ? one : many}`;
}

export function createRepl(host: ReplHost): { show(): void } {
  const log = $("#repl-transcript");
  const runButton = $<HTMLButtonElement>("#repl-run");
  const stopButton = $<HTMLButtonElement>("#repl-stop");
  const restartButton = $<HTMLButtonElement>("#repl-restart");
  const statusLine = $("#repl-status");
  const touch = window.matchMedia("(pointer: coarse)").matches;

  let worker: Worker | null = null;
  let nextId = 1;
  /** The editor's code the session started with, or null for none yet. */
  let sessionSource: string | null = null;
  /** The editor's code whose input 0 the transcript last showed. */
  let shownSource: string | null = null;
  /** Inputs after input 0, in order, for replay. */
  let taken: ReplReplay[] = [];
  /** The number the next input gets. */
  let nextNumber = 0;
  /** Waiting for the session to start (or restart). */
  let starting = 0;
  let running: (Job & { id: number }) | null = null;
  const queue: Job[] = [];
  const completions = new Map<number, (c: ReplCompletion | null) => void>();
  /** Lines streamed for the running input, before its answer. */
  let streamed: HTMLElement | null = null;

  // History: per viewer, in localStorage when it's there.
  let past: string[] = [];
  try {
    const stored: unknown = JSON.parse(localStorage.getItem(HISTORY_KEY) ?? "[]");
    if (Array.isArray(stored)) past = stored.filter((s): s is string => typeof s === "string");
  } catch {
    // No storage: history lasts the visit.
  }
  let at = past.length;
  let draft = "";

  function remember(text: string): void {
    if (past[past.length - 1] !== text) past.push(text);
    past = past.slice(-HISTORY_SIZE);
    at = past.length;
    draft = "";
    try {
      localStorage.setItem(HISTORY_KEY, JSON.stringify(past));
    } catch {
      // Private windows may refuse.
    }
  }

  function walk(view: EditorView, step: -1 | 1): boolean {
    const head = view.state.selection.main.head;
    const line = view.state.doc.lineAt(head).number;
    if (step < 0 ? line !== 1 : line !== view.state.doc.lines) return false;
    const to = at + step;
    if (to < 0 || to > past.length) return true;
    if (at === past.length) draft = view.state.doc.toString();
    at = to;
    const text = at === past.length ? draft : (past[at] ?? "");
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: text },
      selection: { anchor: step < 0 ? Math.min(text.indexOf("\n") >>> 0, text.length) : text.length },
    });
    return true;
  }

  // The input cell.

  const cell = new EditorView({
    parent: $("#repl-cell"),
    state: EditorState.create({
      extensions: [
        history(),
        drawSelection(),
        bracketMatching(),
        keymap.of([
          { key: "Enter", run: (view) => (touch ? insertNewlineAndIndent(view) : (void enter(view, false), true)) },
          { key: "Shift-Enter", run: insertNewlineAndIndent },
          { key: "Mod-Enter", run: (view) => (void enter(view, true), true) },
          { key: "ArrowUp", run: (view) => walk(view, -1) },
          { key: "ArrowDown", run: (view) => walk(view, 1) },
          { key: "Escape", run: () => (running || starting ? (stop(), true) : false) },
        ]),
        keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
        ...host.extensions,
        placeholder(touch ? "Temper, then Run" : "Temper, then Enter"),
        EditorView.lineWrapping,
        EditorView.contentAttributes.of({ "aria-label": "REPL input", "aria-multiline": "true" }),
      ],
    }),
  });

  /** Enter, Ctrl+Enter, or Run: runs the cell's inputs if it's whole. */
  async function enter(view: EditorView, force: boolean): Promise<void> {
    const text = view.state.doc.toString();
    if (!text.trim()) return;
    if (!host.module()) {
      status("Loading the compiler…");
      return;
    }
    const answer = await completion(text);
    // No answer: the session restarted meanwhile, as for Stop.
    if (!answer || view.state.doc.toString() !== text) return;
    let inputs: string[];
    if (answer.complete) inputs = answer.inputs;
    else if (force) inputs = [text];
    else {
      const head = view.state.selection.main.head;
      if (head === view.state.doc.length) {
        const indent = "  ".repeat(answer.brackets);
        view.dispatch({ changes: { from: head, insert: `\n${indent}` }, selection: { anchor: head + 1 + indent.length }, scrollIntoView: true });
      } else {
        insertNewlineAndIndent(view);
      }
      return;
    }
    remember(text);
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: "" } });
    for (const input of inputs) submit(input, FUEL);
  }

  function completion(text: string): Promise<ReplCompletion | null> {
    if (!ensureSession()) return Promise.resolve(null);
    const id = nextId++;
    const message: ToRepl = { type: "complete", id, text };
    const answer = new Promise<ReplCompletion | null>((resolve) => completions.set(id, resolve));
    worker?.postMessage(message);
    return answer;
  }

  runButton.addEventListener("click", () => {
    void enter(cell, true);
    cell.focus();
  });
  stopButton.addEventListener("click", () => {
    stop();
    cell.focus();
  });
  restartButton.addEventListener("click", () => {
    taken = [];
    queue.splice(0);
    log.replaceChildren();
    restart([], "");
    cell.focus();
  });

  // The transcript.

  function scrollDown(): void {
    log.scrollTop = log.scrollHeight;
  }

  function note(text: string, kind = "note"): void {
    log.append(element("p", `repl-banner ${kind}`, text));
    scrollDown();
  }

  /** The echoed input, highlighted, after the CLI's prompt. */
  function echo(text: string): HTMLElement {
    const pre = element("pre", "repl-echo");
    pre.append(element("span", "repl-prompt", "$ "));
    const code = element("code", "");
    const tree = host.language.parser.parse(text);
    highlightCode(
      text,
      tree,
      host.highlight,
      (piece, classes) => code.append(classes ? element("span", classes, piece) : document.createTextNode(piece)),
      () => code.append("\n"),
    );
    pre.append(code);
    return pre;
  }

  function block(text: string): HTMLElement {
    const div = element("div", "repl-block");
    div.append(echo(text));
    log.append(div);
    scrollDown();
    return div;
  }

  function line(target: HTMLElement, kind: string, text: string, level?: string): void {
    target.append(element("pre", `repl-event repl-${kind}${level ? ` ${level}` : ""}`, text));
  }

  function render(job: Job, answer: ReplAnswer): void {
    const { block: div } = job;
    streamed?.remove();
    streamed = null;
    div.dataset.input = String(answer.input);
    for (const event of answer.events) line(div, event.kind, event.text, event.level);
    if (answer.out_of_steps) {
      const more = job.fuel * MORE;
      const button = element("button", "repl-more", `Keep going (${steps(more)} steps)`) as HTMLButtonElement;
      button.type = "button";
      button.addEventListener("click", () => {
        button.disabled = true;
        submit(job.text, more);
      });
      div.append(button);
    }
    scrollDown();
  }

  function steps(n: number): string {
    return n >= 1e9 ? `${n / 1e9} billion` : `${n / 1e6} million`;
  }

  // The worker, one request in it at a time, besides Enter's questions.

  /** Starts a session if there's none, or the editor changed. */
  function ensureSession(): boolean {
    if (!host.module()) {
      status("Loading the compiler…");
      return false;
    }
    if (!worker) restart(taken, "");
    else if (sessionSource !== host.source() && !running && !starting) {
      const n = taken.length;
      restart(taken, n ? `The editor changed: a new session, with ${plural(n, "input")} run again.` : "The editor changed: a new session.");
    }
    return true;
  }

  /** A new worker and session, from the editor's code, then `replay`. */
  function restart(replay: ReplReplay[], why: string): void {
    const module = host.module();
    if (!module) return;
    worker?.terminate();
    for (const resolve of completions.values()) resolve(null);
    completions.clear();
    running = null;
    streamed?.remove();
    streamed = null;
    taken = [...replay];
    const w = new Worker(`repl-worker.js?v=${BUILD_VERSION}`, { type: "module", name: "repl" });
    worker = w;
    w.onmessage = (event: MessageEvent<FromRepl>) => {
      if (worker === w) onMessage(event.data);
    };
    w.onerror = (event) => {
      if (worker !== w) return;
      note(`The REPL stopped: ${event.message}`, "failure");
      worker = null;
      starting = 0;
      busy(false);
    };
    w.postMessage({ type: "init", module } satisfies ToRepl);
    const source = host.source();
    sessionSource = source;
    starting = nextId++;
    if (why) note(why);
    status(taken.length ? `Running ${plural(taken.length, "earlier input")} again…` : "Starting…");
    busy(true);
    const message: ToRepl = { type: "start", id: starting, source, fuel: FUEL, replay: taken };
    w.postMessage(message);
  }

  function submit(text: string, fuel: number): void {
    if (!ensureSession()) return;
    queue.push({ text, fuel, block: block(text) });
    pump();
  }

  function pump(): void {
    if (running || starting || !worker) return;
    const job = queue.shift();
    if (!job) {
      busy(false);
      return;
    }
    const id = nextId++;
    running = { ...job, id };
    busy(true);
    status("Running…");
    worker.postMessage({ type: "input", id, text: job.text, fuel: job.fuel } satisfies ToRepl);
  }

  function onMessage(message: FromRepl): void {
    switch (message.type) {
      case "log":
        if (running?.id !== message.id) return;
        if (!streamed) {
          streamed = element("div", "repl-streamed");
          running.block.append(streamed);
        }
        line(streamed, "log", message.line);
        scrollDown();
        return;
      case "complete": {
        const resolve = completions.get(message.id);
        completions.delete(message.id);
        resolve?.(message.completion);
        return;
      }
      case "started": {
        if (message.id !== starting) return;
        starting = 0;
        nextNumber = message.answer ? message.answer.input + 1 + message.replayed : message.replayed;
        const answer = message.answer;
        const quiet = (kind: string) => kind === "log" || kind === "result" || kind === "truncated";
        const telling = answer && (answer.value === null || answer.events.some((e) => !quiet(e.kind)));
        if (answer && telling && sessionSource !== shownSource) {
          // Input 0, the editor's code, failed, or has more to say than
          // its logs, as its tests' summary: show it, once for the code.
          // What it declared without errors still works.
          shownSource = sessionSource;
          const div = element("div", "repl-block");
          div.append(element("p", "repl-banner note", "The editor's code, input 0:"));
          log.append(div);
          render({ text: sessionSource ?? "", fuel: FUEL, block: div }, answer);
        }
        status("");
        pump();
        return;
      }
      case "answer":
        if (running?.id !== message.id) return;
        render(running, message.answer);
        nextNumber = message.answer.input + 1;
        taken.push({ text: running.text, fuel: message.answer.out_of_steps ? FAIL_FAST : running.fuel });
        running = null;
        status("");
        pump();
        return;
      case "trap":
        onTrap(message.message, message.replaying, message.id);
        return;
    }
  }

  /** The instance trapped: restore the session without what trapped. */
  function onTrap(why: string, replaying: number | null, id: number): void {
    const brief = why.replace(/^RuntimeError: /, "");
    if (replaying !== null && id === starting) {
      // An earlier input trapped again as it replayed: leave it out.
      const dropped = taken[replaying];
      const rest = taken.filter((_, i) => i !== replaying);
      note(`Input ${JSON.stringify(dropped?.text.slice(0, 40) ?? "")} crashed again as it ran again (${brief}), so it's left out; later inputs' numbers move down one.`, "failure");
      restart(rest, "");
      return;
    }
    if (running?.id === id) {
      const job = running;
      line(job.block, "failure", `interactive#${nextNumber}: crashed (${brief})`);
      restart([...taken, { text: job.text, fuel: FAIL_FAST }], "Restoring the session…");
      pump();
      return;
    }
    // Input 0, or Enter's question.
    note(`The REPL crashed (${brief}); a new session.`, "failure");
    restart(id === starting ? [] : taken, "");
  }

  /** Stops the running input, restoring the session without it. */
  function stop(): void {
    if (!worker) return;
    const dropped = queue.splice(0);
    for (const job of dropped) line(job.block, "note", "Not run: stopped before its turn.");
    if (running) {
      const job = running;
      streamed = null;
      line(job.block, "interrupted", `interactive#${nextNumber}: stopped`);
      restart([...taken, { text: job.text, fuel: FAIL_FAST }], "");
    } else if (starting) {
      // Stopped while replaying: start over, with none.
      restart([], `Stopped while restoring; a new session without the earlier ${plural(taken.length, "input")}.`);
    }
    pump();
  }

  function busy(on: boolean): void {
    stopButton.disabled = !on;
    log.setAttribute("aria-busy", String(on));
  }

  function status(text: string): void {
    statusLine.textContent = text;
  }

  let introduced = false;
  return {
    show(): void {
      if (!introduced) {
        introduced = true;
        note(
          `The editor's code is input 0. Enter runs an input once it's whole${touch ? "; on a touch screen, Run does" : ""}. ` +
            "Try help(), or translate(1, \"py\") after an input.",
        );
      }
      ensureSession();
      cell.focus();
    },
  };
}
