import { type } from "@temperlang/core";

/**
 * @param {number} n
 * @returns {number}
 */
export function lambda(n) {
  return (n + 1) | 0;
}

export class Checked extends type() {
  /** @type {number} */
  #end;

  /** @param {number} end */
  constructor(end) {
    super();
    this.#end = end;
  }

  /** @returns {number} */
  get end() {
    return this.#end;
  }

  /** @returns {number} */
  delete_() {
    return Math.imul(this.#end, 2);
  }

  /** @returns {number} */
  get local() {
    return (this.#end + native) | 0;
  }
}

export const native = 2;
