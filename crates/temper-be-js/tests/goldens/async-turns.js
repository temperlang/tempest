import { PromiseBuilder, adaptAwaiter, runAsync, type } from "@temperlang/core";

/**
 * @param {number} n
 * @returns {Promise<number>}
 */
export function later(n) {
  /** @type {import("@temperlang/core/async.js").PromiseBuilder<number>} */
  const pb = new PromiseBuilder();
  /**
   * @param {(p: Promise<unknown>) => void} doAwait
   * @returns {Generator<any>}
   */
  function* fn(doAwait) {
    pb.complete(Math.imul(n, 2));
  }
  runAsync(adaptAwaiter(fn));
  return pb.promise;
}

/**
 * @param {() => Generator<{}>} body
 * @returns {Generator<{}>}
 */
function generated(body) {
  return body();
}

/**
 * @param {number} n
 * @param {(arg0: number) => void} each
 * @returns {Generator<{}>}
 */
export function ticks(n, each) {
  /**
   * @param {(p: Promise<unknown>) => void} doAwait
   * @returns {Generator<any>}
   */
  function* fn(doAwait) {
    {
      let i = 0;
      while (i < n) {
        each(i);
        yield;
        i = (i + 1) | 0;
      }
    }
  }
  return generated(adaptAwaiter(fn));
}

class Counter extends type() {
  /** @type {number} */
  #n;

  constructor() {
    super();
    this.#n = 0;
  }

  /** @returns {number} */
  get n() {
    return this.#n;
  }

  /** @param {number} value */
  set n(value) {
    this.#n = value;
  }

  /**
   * @param {string} tag
   * @param {number} times
   */
  bump(tag, times) {
    console.log(`${tag} start`);
    {
      let i = 0;
      while (i < times) {
        const before = this.#n;
        this.#n = (before + 1) | 0;
        i = (i + 1) | 0;
      }
    }
    console.log(`${tag} done n=${this.#n}`);
  }
}

/** @param {number} i */
function fn(i) {
  console.log(`tick ${i}`);
}

/** @type {Generator<{}>} */
const t = ticks(2, fn);
t.next();
t.next();
t.next();
const c = new Counter();
/** @type {import("@temperlang/core/async.js").PromiseBuilder<number>} */
const gateA = new PromiseBuilder();
/** @type {import("@temperlang/core/async.js").PromiseBuilder<number>} */
const gateB = new PromiseBuilder();

/**
 * @param {(p: Promise<unknown>) => void} doAwait
 * @returns {Generator<any>}
 */
function* fn2(doAwait) {
  c.bump("A", 10000);
  /** @type {number} */
  let k;
  try {
    k = yield doAwait(gateA.promise);
  } catch {
    k = -1;
  }
  c.bump(`A${k}`, 10000);
}

runAsync(adaptAwaiter(fn2));

/**
 * @param {(p: Promise<unknown>) => void} doAwait
 * @returns {Generator<any>}
 */
function* fn3(doAwait) {
  c.bump("B", 10000);
  /** @type {number} */
  let k;
  try {
    k = yield doAwait(gateB.promise);
  } catch {
    k = -1;
  }
  c.bump(`B${k}`, 10000);
}

runAsync(adaptAwaiter(fn3));

/**
 * @param {(p: Promise<unknown>) => void} doAwait
 * @returns {Generator<any>}
 */
function* fn4(doAwait) {
  console.log("opener: calling complete");
  gateA.complete(1);
  gateB.complete(2);
  console.log("opener: complete returned");
}

runAsync(adaptAwaiter(fn4));
