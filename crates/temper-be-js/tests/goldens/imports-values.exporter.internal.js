import { listedGet } from "@temperlang/core";

/** @returns {readonly number[]} */
function foo() {
  return Object.freeze([(21 + 21) | 0, 42]);
}

let a = listedGet(/** @type {number[]} */ (foo()), 0);
a = a;
export const magicNumber = a;
