import { type } from "@temperlang/core";

class Simple extends type() {
  /** @type {string} */
  #name;
  /** @type {Simple} */
  static simon;
  /** @type {string} */
  static value;
  /** @type {number} */
  static other;

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

class C extends type() {
  static sayHi() {
    console.log(C.#compose("C", "'hi.'"));
  }

  /**
   * @param {string} subject
   * @param {string} message
   * @returns {string}
   */
  static #compose(subject, message) {
    return `${subject} says ${message}`;
  }
}

class I extends type() {
  static sayBye() {
    console.log("I says 'bye.'");
  }
}

Simple.simon = new Simple("foo");
Simple.value = Simple.simon.name;
Simple.other = 22;
console.log(`Simple.simon.name is ${Simple.simon.name}`);
console.log(`Simple.value is ${Simple.value}`);
console.log(`Simple.other is ${Simple.other}`);
C.sayHi();
I.sayBye();
