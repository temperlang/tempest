import { type } from "@temperlang/core";

class I extends type() {
  /** @returns {string} */
  get place() {
    return "Byron Bay";
  }
}

class C extends type(I) {}

const c = new C();
console.log(`Goodbye, gotta run, ${/** @type {I} */ (/** @type {unknown} */ (c)).place}!`);
