import { adaptAwaiter } from "@temperlang/core";

/** @param {() => Generator<{}>} body */
function f(body) {
  /** @type {Generator<{}>} */
  const generator = body();
  console.log("f0");
  generator.next();
  console.log("f1");
  generator.next();
  console.log("f2");
  generator.next();
  console.log("f3");
}

/**
 * @param {(p: Promise<unknown>) => void} doAwait
 * @returns {Generator<any>}
 */
function* fn(doAwait) {
  let i = 0;
  while (true) {
    console.log(`generated ${i}`);
    i = (i + 1) | 0;
    yield;
  }
}

f(adaptAwaiter(fn));
