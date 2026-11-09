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

class Money extends type() {
  /** @type {number} */
  #cents;
  /** @type {number} */
  #spent;

  /** @param {number} cents */
  constructor(cents) {
    super();
    this.#cents = cents;
    this.#spent = 0;
  }

  /** @returns {number} */
  #whole() {
    return (this.#cents / 100) | 0;
  }

  /** @returns {number} */
  get #rounded() {
    return (((this.#cents + 50) | 0) / 100) | 0;
  }

  /**
   * @param {Money} other
   * @returns {Money}
   */
  plus(other) {
    return new Money((this.#cents + other.#cents) | 0);
  }

  /**
   * @param {Money} other
   * @returns {boolean}
   */
  sameWhole(other) {
    return this.#whole() === other.#whole();
  }

  /**
   * @param {Money} other
   * @returns {boolean}
   */
  sameRounded(other) {
    return this.#rounded === other.#rounded;
  }

  /** @param {Money} other */
  spendFrom(other) {
    other.#spent = (other.#spent + 1) | 0;
  }

  /** @returns {number} */
  get timesSpent() {
    return this.#spent;
  }

  /** @returns {string} */
  toString() {
    return `${this.#cents} cents`;
  }
}

class Sink extends type() {
  /** @type {() => number} */
  end = /** @type {any} */ (undefined);
}

class TenSink extends type(Sink) {
  /** @returns {number} */
  end() {
    return 10;
  }
}

class Note extends type() {
  /** @type {string} */
  #body;

  /** @param {string} body */
  constructor(body) {
    super();
    this.#body = body;
  }

  /** @returns {string} */
  get body() {
    return this.#body;
  }

  /**
   * @param {string} more
   * @returns {Note}
   */
  text(more) {
    return new Note(this.#body + more);
  }
}

class Label extends type() {
  /** @type {number} */
  #end;

  /** @param {number} end */
  constructor(end) {
    super();
    this.#end = end;
  }

  /** @returns {string} */
  get text() {
    return "label";
  }

  /**
   * @param {Sink} sink
   * @returns {number}
   */
  close(sink) {
    return (sink.end() + this.#end) | 0;
  }

  /**
   * @param {string} more
   * @returns {string}
   */
  noted(more) {
    return new Note(this.text).text(more).body;
  }
}

const c = new C();
console.log(`method1(10)=${c.method1(10)}`);
console.log(`method2(10)=${c.method2(10)}`);
const a = new Money(150);
const b = new Money(175);
console.log(`plus: ${a.plus(b).toString()}`);
console.log(`same whole: ${a.sameWhole(b)}`);
console.log(`same rounded: ${a.sameRounded(b)}`);
a.spendFrom(b);
a.spendFrom(b);
console.log(`b spent: ${b.timesSpent}`);
const label = new Label(1);
console.log(`close: ${label.close(new TenSink())}`);
console.log(`noted: ${label.noted("!")}`);
