// The wasm compiler's JSON (crates/temper-wasm's `compile` and
// `std_library`) and the messages between the page and its workers.

/** A generated file, by its path in the backend's output. */
export interface GeneratedFile {
  path: string;
  text: string;
}

/** What a backend generated, or why it couldn't. */
export type BackendOutput = { files: GeneratedFile[] } | { error: string };

export type Level = "error" | "warn" | "info";

/**
 * A problem the compiler found. `from` and `to` count UTF-16 code units
 * in the main file, as editors do; diagnostics in other files (std's)
 * have no position.
 */
export interface Diagnostic {
  level: Level;
  message: string;
  file: string;
  from?: number;
  to?: number;
  line?: number;
  column?: number;
}

/** A test that ran, with why it failed, if it did. */
export interface TestResult {
  name: string;
  failure: string | null;
}

/** What a run logged, how it failed, and its tests' results. */
export interface RunOutput {
  output: string[];
  failure: string | null;
  tests: TestResult[];
  /** How many tests the library declares. */
  defined: number;
  /** As `tempest test` prints it: `Tests passed: 2 of 2`. */
  summary: string;
}

/** All of one `compile` call. */
export interface CompileResult {
  diagnostics: Diagnostic[];
  run: RunOutput | null;
  backends: Record<string, BackendOutput>;
}

/** The editor's queries, by the wasm build's names for them. */
export type Query = "hover" | "definition" | "completions";

/** The page to the compiler's worker. */
export type ToCompiler =
  | { type: "init"; module: WebAssembly.Module }
  | { type: "compile"; id: number; source: string }
  /** `at` counts UTF-16 code units in the last source compiled. */
  | { type: "query"; qid: number; what: Query; at: number };

/** The compiler's worker to the page. */
export type FromCompiler =
  | { type: "ready"; backends: string[] }
  | { type: "diagnostics"; id: number; ms: number; diagnostics: Diagnostic[] }
  | { type: "result"; id: number; ms: number; result: CompileResult }
  | { type: "answer"; qid: number; value: unknown }
  | { type: "std"; backend: string; output: BackendOutput }
  | { type: "failed"; id: number; message: string };

/** The page to the worker that runs generated JavaScript. */
export interface ToJsRunner {
  /** The library's generated files, laid out under `main/`. */
  files: GeneratedFile[];
  /** std's generated files, laid out under `std/`. */
  std: GeneratedFile[];
}

/** The JavaScript runner's worker to the page. */
export type FromJsRunner =
  | { type: "log"; line: string }
  | { type: "failure"; message: string }
  | { type: "tests"; tests: TestResult[] }
  | { type: "done" };

// Narrowing what the wasm build gives, at the boundary.

type Json = unknown;

function isRecord(value: Json): value is Record<string, Json> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function fail(what: string): never {
  throw new Error(`unexpected compiler output: ${what}`);
}

function str(value: Json, what: string): string {
  return typeof value === "string" ? value : fail(what);
}

function num(value: Json, what: string): number {
  return typeof value === "number" ? value : fail(what);
}

function optNum(value: Json, what: string): number | undefined {
  return value === undefined ? undefined : num(value, what);
}

function strOrNull(value: Json, what: string): string | null {
  return value === null ? null : str(value, what);
}

function list<T>(value: Json, what: string, item: (v: Json) => T): T[] {
  return Array.isArray(value) ? value.map(item) : fail(what);
}

function level(value: Json): Level {
  return value === "error" || value === "warn" || value === "info" ? value : fail("level");
}

export function toFiles(value: Json): GeneratedFile[] {
  return list(value, "files", (f) => {
    if (!isRecord(f)) return fail("file");
    return { path: str(f.path, "path"), text: str(f.text, "text") };
  });
}

export function toBackendOutput(value: Json): BackendOutput {
  if (!isRecord(value)) return fail("backend");
  if (value.error !== undefined) return { error: str(value.error, "error") };
  return { files: toFiles(value.files) };
}

export function toStrings(value: Json): string[] {
  return list(value, "strings", (s) => str(s, "string"));
}

function toRun(value: Json): RunOutput {
  if (!isRecord(value)) return fail("run");
  const tests = value.tests === undefined ? [] : list(value.tests, "tests", (t) => {
    if (!isRecord(t)) return fail("test");
    return { name: str(t.name, "test name"), failure: strOrNull(t.failure, "test failure") };
  });
  return {
    output: toStrings(value.output),
    failure: strOrNull(value.failure, "failure"),
    tests,
    defined: optNum(value.defined, "defined") ?? 0,
    summary: value.summary === undefined ? "" : str(value.summary, "summary"),
  };
}

/** What hovering shows: a name's kind and type, or an expression's type. */
export interface HoverInfo {
  from: number;
  to: number;
  name: string | null;
  /** `let`, `var`, `function`, `parameter`, `method`, and the like. */
  kind: string | null;
  type: string | null;
  doc: string | null;
}

/** A span in the source, as UTF-16 offsets. */
export interface Span {
  from: number;
  to: number;
}

/** A name that may go at a position. */
export interface CompletionItem {
  name: string;
  kind: string;
  type: string | null;
}

export function toHover(value: Json): HoverInfo | null {
  if (value === null) return null;
  if (!isRecord(value)) return fail("hover");
  return {
    from: num(value.from, "from"),
    to: num(value.to, "to"),
    name: strOrNull(value.name, "name"),
    kind: strOrNull(value.kind, "kind"),
    type: strOrNull(value.type, "type"),
    doc: strOrNull(value.doc, "doc"),
  };
}

export function toSpan(value: Json): Span | null {
  if (value === null) return null;
  if (!isRecord(value)) return fail("span");
  return { from: num(value.from, "from"), to: num(value.to, "to") };
}

export function toCompletions(value: Json): CompletionItem[] {
  if (value === null) return [];
  return list(value, "completions", (c) => {
    if (!isRecord(c)) return fail("completion");
    return { name: str(c.name, "name"), kind: str(c.kind, "kind"), type: strOrNull(c.type, "type") };
  });
}

export function toDiagnostics(value: Json): Diagnostic[] {
  if (!isRecord(value)) return fail("analysis");
  return list(value.diagnostics, "diagnostics", (d): Diagnostic => {
    if (!isRecord(d)) return fail("diagnostic");
    return {
      level: level(d.level),
      message: str(d.message, "message"),
      file: str(d.file, "file"),
      from: optNum(d.from, "from"),
      to: optNum(d.to, "to"),
      line: optNum(d.line, "line"),
      column: optNum(d.column, "column"),
    };
  });
}

export function toCompileResult(value: Json): CompileResult {
  if (!isRecord(value)) return fail("result");
  const diagnostics = toDiagnostics(value);
  const backends: Record<string, BackendOutput> = {};
  if (!isRecord(value.backends)) return fail("backends");
  for (const [name, output] of Object.entries(value.backends)) {
    backends[name] = toBackendOutput(output);
  }
  return { diagnostics, run: value.run === undefined ? null : toRun(value.run), backends };
}
