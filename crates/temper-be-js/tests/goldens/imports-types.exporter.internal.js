import { type } from "@temperlang/core";

export class ExportedClass extends type() {
  /** @type {number} */
  #foo;

  /** @param {number} foo */
  constructor(foo) {
    super();
    this.#foo = foo;
  }

  /** @returns {number} */
  get foo() {
    return this.#foo;
  }
}
