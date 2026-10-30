import { type } from "@temperlang/core";

class Foo extends type() {
  /** @type {string} */
  #backing;

  /** @param {string} backing */
  constructor(backing) {
    super();
    this.#backing = backing;
  }

  /** @returns {string} */
  get bar() {
    return this.#backing;
  }

  /** @param {string} value */
  set bar(value) {
    this.#backing = value;
  }
}

class Bar extends type() {
  /** @type {string} */
  #backing;

  /** @param {string} backing */
  constructor(backing) {
    super();
    this.#backing = backing;
  }

  /** @returns {string} */
  get bar() {
    return this.#backing;
  }

  /** @param {string} value */
  set #bar(value) {
    this.#backing = value;
  }

  /** @param {string} value */
  usingSet(value) {
    this.#bar = value;
  }
}

export class Whatever extends type() {
  /** @type {number} */
  something = /** @type {any} */ (undefined);
}

export class ConcreteWhatever extends type(Whatever) {
  /** @type {number} */
  #something;

  /** @param {number} something */
  constructor(something) {
    super();
    this.#something = something;
  }

  /** @returns {number} */
  get something() {
    return this.#something;
  }

  /** @param {number} value */
  set something(value) {
    this.#something = value;
  }
}

console.log(new Foo("initial").bar);
const foo = new Foo("initial");
foo.bar = "next";
console.log(foo.bar);
const bar = new Bar("initial");
bar.usingSet("next");
console.log(bar.bar);
const what = new ConcreteWhatever(4);
what.something = 5;
console.log(`something set to ${what.something}`);
