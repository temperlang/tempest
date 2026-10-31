import { type } from "@temperlang/core";

class Bar extends type() {
  /** @type {Foo | null} */
  #x;

  /** @param {Foo | null} x */
  constructor(x) {
    super();
    this.#x = x;
  }

  /** @returns {Foo | null} */
  get x() {
    return this.#x;
  }
}

class Foo extends type() {
  /** @type {Bar | null} */
  #x;

  /** @param {Bar | null} x */
  constructor(x) {
    super();
    this.#x = x;
  }

  /** @returns {Bar | null} */
  get x() {
    return this.#x;
  }
}

const emptyFoo = new Foo(null);
const emptyBar = new Bar(null);
const nonEmptyFoo = new Foo(emptyBar);
const nonEmptyBar = new Bar(emptyFoo);
const stuffedBar = new Bar(nonEmptyFoo);
console.log(`emptyFoo.x has Foo -> ${emptyFoo.x != null}`);
console.log(`nonEmptyFoo.x has Foo -> ${nonEmptyFoo.x != null}`);
