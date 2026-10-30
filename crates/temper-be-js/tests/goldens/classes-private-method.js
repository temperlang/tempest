import { type } from "@temperlang/core";

class C extends type() {
  /**
   * @param {number} x
   * @returns {number}
   */
  #helper(x) {
    return (x + 1) | 0;
  }

  /**
   * @param {number} x
   * @returns {number}
   */
  method1(x) {
    return this.#helper(Math.imul(x, 2));
  }

  /**
   * @param {number} x
   * @returns {number}
   */
  method2(x) {
    return this.#helper(Math.imul(x, 3));
  }
}

const c = new C();
console.log(`method1(10)=${c.method1(10)}`);
console.log(`method2(10)=${c.method2(10)}`);
