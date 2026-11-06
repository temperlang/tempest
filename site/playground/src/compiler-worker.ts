// The compiler's worker: runs the wasm build of tempest off the page's
// thread, so typing never waits on it. The page sends the compiled
// module once, then source to compile; each answer carries the request's
// id, so the page can drop stale ones. A trap (a panic, say) leaves the
// instance unusable, so the next request gets a fresh one.

import {
  type FromCompiler,
  type ToCompiler,
  toBackendOutput,
  toCompileResult,
  toStrings,
} from "./api";

/** The C-ABI exports of crates/temper-wasm, narrowed from the instance. */
interface Exports {
  memory: WebAssembly.Memory;
  alloc(len: number): number;
  compile(ptr: number, len: number, options: number, backends: number): void;
  backends(): void;
  std_library(backend: number): void;
  output_ptr(): number;
  output_len(): number;
}

const RUN = 2;
const TESTS = 4;

const post = (message: FromCompiler): void => self.postMessage(message);

let module: WebAssembly.Module | null = null;
let exports: Exports | null = null;
let backends: string[] = [];
let stdSent = false;

function instantiate(m: WebAssembly.Module): Exports {
  const instance = new WebAssembly.Instance(m, {});
  const e = instance.exports;
  const fn = (name: string): ((...args: number[]) => number) => {
    const f = e[name];
    if (typeof f !== "function") throw new Error(`the wasm build has no ${name}`);
    return f as (...args: number[]) => number;
  };
  const memory = e.memory;
  if (!(memory instanceof WebAssembly.Memory)) throw new Error("the wasm build has no memory");
  return {
    memory,
    alloc: fn("alloc"),
    compile: fn("compile"),
    backends: fn("backends"),
    std_library: fn("std_library"),
    output_ptr: fn("output_ptr"),
    output_len: fn("output_len"),
  };
}

function output(e: Exports): unknown {
  const bytes = new Uint8Array(e.memory.buffer, e.output_ptr(), e.output_len());
  return JSON.parse(new TextDecoder().decode(bytes)) as unknown;
}

function current(): Exports {
  if (exports) return exports;
  if (!module) throw new Error("no wasm module yet");
  exports = instantiate(module);
  return exports;
}

function compile(id: number, source: string): void {
  const e = current();
  const bytes = new TextEncoder().encode(source);
  const start = performance.now();
  const ptr = e.alloc(bytes.length);
  new Uint8Array(e.memory.buffer, ptr, bytes.length).set(bytes);
  e.compile(ptr, bytes.length, RUN | TESTS, (1 << backends.length) - 1);
  const ms = performance.now() - start;
  post({ type: "result", id, ms, result: toCompileResult(output(e)) });
}

/** std as JavaScript, for running tests on the generated JavaScript. */
function sendStd(): void {
  const index = backends.indexOf("js");
  if (stdSent || index < 0) return;
  stdSent = true;
  const e = current();
  e.std_library(index);
  post({ type: "std", backend: "js", output: toBackendOutput(output(e)) });
}

self.onmessage = (event: MessageEvent<ToCompiler>) => {
  const message = event.data;
  if (message.type === "init") {
    module = message.module;
    const e = current();
    e.backends();
    backends = toStrings(output(e));
    post({ type: "ready", backends });
    return;
  }
  try {
    compile(message.id, message.source);
    sendStd();
  } catch (error) {
    exports = null;
    post({ type: "failed", id: message.id, message: String(error) });
  }
};
