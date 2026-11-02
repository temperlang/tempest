import { type } from "@temperlang/core";

export class Counter extends type() {
  /** @type {number} */
  #count;

  constructor() {
    super();
    this.#count = 0;
  }

  /** @param {number} i */
  add(i) {
    this.#count = (this.#count + i) | 0;
  }

  /** @returns {number} */
  total() {
    return (this.#count + 1000) | 0;
  }
}

export class Doubler extends type() {
  /**
   * @param {number} i
   * @returns {number}
   */
  twice(i) {
    return (Math.imul(i, 2) + 1000) | 0;
  }
}

export const counter = new Counter();
counter.add(2);
counter.add(3);
console.log(`counter.total(): ${counter.total()}`);
console.log(`new Doubler().twice(5): ${new Doubler().twice(5)}`);
