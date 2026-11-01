import { type } from "@temperlang/core";

/** @param {string} how */
function hello(how) {
  console.log(`Called ${how}!`);
}

/** @param {(arg0: string) => void} f */
function callIt(f) {
  f("by a function");
}

/**
 * @param {number} i
 * @returns {boolean}
 */
function isOdd(i) {
  if (i === 0) {
    return false;
  } else {
    return isEven((i - 1) | 0);
  }
}

/**
 * @param {number} i
 * @returns {boolean}
 */
function isEven(i) {
  if (i === 0) {
    return true;
  } else {
    return isOdd((i - 1) | 0);
  }
}

class DelayedFunction extends type() {
  /** @type {(arg0: string) => void} */
  #f;

  /** @param {(arg0: string) => void} f */
  constructor(f) {
    super();
    this.#f = f;
  }

  callIt() {
    this.#f("from a method");
  }
}

hello("directly");
callIt(hello);
new DelayedFunction(hello).callIt();
console.log(`Is 5 even? ${isEven(5)}`);
console.log(`Is 6 even? ${isEven(6)}`);
