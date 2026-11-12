// The REPL's worker: one wasm instance holding one REPL session
// (crates/temper-wasm's `repl_*`), apart from the compiler's worker,
// whose instance gets replaced as you type. The page sends the compiled
// module once, then requests, one at a time, each answered with its id.
// What an input logs is posted line by line as it's logged
// (`env.host_log`), so a long run's output shows while it runs. A run
// can't be interrupted from here: the page stops one by terminating this
// worker and starting another, which replays the earlier inputs quietly.
// A trap (a panic, or the memory cap) leaves the instance unusable, so
// it's reported and the page does the same. The cell's completion,
// hover, and signature help are `repl_query`'s, which reads the cell's
// text against the session without settling or running it.

import {
  type FromRepl,
  type ReplQuery,
  type ReplReplay,
  type ToRepl,
  toReplAnswer,
  toReplCompletion,
} from "./api";

/** The REPL's C-ABI exports of crates/temper-wasm. */
interface Exports {
  memory: WebAssembly.Memory;
  alloc(len: number): number;
  repl_start(ptr: number, len: number, fuel: number): void;
  repl_input(ptr: number, len: number, fuel: number): void;
  repl_complete(ptr: number, len: number): number;
  repl_query(kind: number, at: number, ptr: number, len: number): void;
  output_ptr(): number;
  output_len(): number;
}

const post = (message: FromRepl): void => self.postMessage(message);

let module: WebAssembly.Module | null = null;
let exports: Exports | null = null;
/** The request whose input is running, for its streamed lines. */
let running = 0;
/** While replaying, logged lines aren't posted. */
let quiet = false;

function instantiate(m: WebAssembly.Module): Exports {
  let memory: WebAssembly.Memory | null = null;
  const imports = {
    env: {
      host_now: (): number => Date.now(),
      host_log: (ptr: number, len: number): void => {
        if (quiet || !memory) return;
        const line = new TextDecoder().decode(new Uint8Array(memory.buffer, ptr, len));
        post({ type: "log", id: running, line });
      },
    },
  };
  const instance = new WebAssembly.Instance(m, imports);
  const e = instance.exports;
  const fn = (name: string): ((...args: number[]) => number) => {
    const f = e[name];
    if (typeof f !== "function") throw new Error(`the wasm build has no ${name}`);
    return f as (...args: number[]) => number;
  };
  if (!(e.memory instanceof WebAssembly.Memory)) throw new Error("the wasm build has no memory");
  memory = e.memory;
  return {
    memory,
    alloc: fn("alloc"),
    repl_start: fn("repl_start"),
    repl_input: fn("repl_input"),
    repl_complete: fn("repl_complete"),
    repl_query: fn("repl_query"),
    output_ptr: fn("output_ptr"),
    output_len: fn("output_len"),
  };
}

function current(): Exports {
  if (exports) return exports;
  if (!module) throw new Error("no wasm module yet");
  exports = instantiate(module);
  return exports;
}

/** Puts `text` in the module's input buffer, giving its place. */
function put(e: Exports, text: string): [number, number] {
  const bytes = new TextEncoder().encode(text);
  const ptr = e.alloc(bytes.length);
  new Uint8Array(e.memory.buffer, ptr, bytes.length).set(bytes);
  return [ptr, bytes.length];
}

function output(e: Exports): unknown {
  const bytes = new Uint8Array(e.memory.buffer, e.output_ptr(), e.output_len());
  return JSON.parse(new TextDecoder().decode(bytes)) as unknown;
}

function start(id: number, source: string, fuel: number, replay: ReplReplay[]): void {
  const e = current();
  running = id;
  e.repl_start(...put(e, source), fuel);
  const first = output(e);
  const answer = first === null ? null : toReplAnswer(first);
  let replaying: number | null = null;
  try {
    quiet = true;
    for (const [i, input] of replay.entries()) {
      replaying = i;
      e.repl_input(...put(e, input.text), input.fuel);
    }
  } catch (error) {
    exports = null;
    post({ type: "trap", id, message: String(error), replaying });
    return;
  } finally {
    quiet = false;
  }
  post({ type: "started", id, answer, replayed: replay.length });
}

function input(id: number, text: string, fuel: number): void {
  const e = current();
  running = id;
  e.repl_input(...put(e, text), fuel);
  post({ type: "answer", id, answer: toReplAnswer(output(e)) });
}

function complete(id: number, text: string): void {
  const e = current();
  e.repl_complete(...put(e, text));
  post({ type: "complete", id, completion: toReplCompletion(output(e)) });
}

/** `repl_query`'s kinds. */
const QUERIES: Record<ReplQuery, number> = { completions: 0, hover: 1, signature: 2 };

function query(id: number, what: ReplQuery, text: string, at: number): void {
  // Before a session starts, there's nothing to ask: not even a new one.
  if (!exports) return post({ type: "query", id, value: null });
  const e = exports;
  const [ptr, len] = put(e, text);
  e.repl_query(QUERIES[what], at, ptr, len);
  post({ type: "query", id, value: output(e) });
}

self.onmessage = (event: MessageEvent<ToRepl>) => {
  const message = event.data;
  if (message.type === "init") {
    module = message.module;
    return;
  }
  try {
    if (message.type === "start") start(message.id, message.source, message.fuel, message.replay);
    else if (message.type === "input") input(message.id, message.text, message.fuel);
    else if (message.type === "query") query(message.id, message.what, message.text, message.at);
    else complete(message.id, message.text);
  } catch (error) {
    exports = null;
    post({ type: "trap", id: message.id, message: String(error), replaying: null });
  }
};
