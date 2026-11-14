import { type } from "@temperlang/core";
import { share } from "./core.js";
import { base, bump, calls, intDoubled, twice } from "./helpers.js";
import { greet } from "./helpers/deep.js";
import { describe } from "./machine.js";

/**
 * @param {number} n
 * @returns {number}
 */
export function sum(n) {
  return (((twice(n) + base) | 0) + base2) | 0;
}

class Engine extends type() {
  /** @type {string} */
  #name;

  /** @param {string} name */
  constructor(name) {
    super();
    this.#name = name;
  }

  /** @returns {string} */
  get name() {
    return this.#name;
  }
}

const calls2 = calls;
const base2 = 1;
console.log(greet("modules"));
console.log(`sum ${sum(3)}`);
console.log(`share ${share(7, 2)} ${intDoubled(21)}`);
console.log(`${new Engine("own").name}, ${describe()}`);
bump();
bump();
console.log(`calls ${calls2}`);
console.log(`calls now ${bump()}`);
