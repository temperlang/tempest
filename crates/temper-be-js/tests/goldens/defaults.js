import { type } from "@temperlang/core";
import { Pad, wrap } from "./helpers.js";

/** @returns {number} */
function next() {
  calls = (calls + 1) | 0;
  return calls;
}

/**
 * @param {number} start
 * @param {number | null} [end]
 * @param {number | null} [step]
 * @returns {string}
 */
function range(start, end, step) {
  end ??= (start + 10) | 0;
  step ??= next();
  return `${start}..${end} by ${step}`;
}

/**
 * @param {string} text
 * @param {string | null} [mark]
 * @returns {string}
 */
function tag(text, mark) {
  if (mark == null) {
    return text;
  } else {
    return mark + text;
  }
}

/**
 * @param {string | null} [name]
 * @returns {string}
 */
function named(name) {
  name ??= "nobody";
  {
    if (name == null) {
      return "null";
    } else {
      return name;
    }
  }
}

class Greeter extends type() {
  /** @type {string} */
  #greeting;
  /** @type {string} */
  #punct;

  /**
   * @param {{ greeting?: string | null, punct?: string | null }} props
   * @returns {Greeter}
   */
  static new(props) {
    return new Greeter(props.greeting, props.punct);
  }

  /**
   * @param {string | null} [greeting]
   * @param {string | null} [punct]
   */
  constructor(greeting, punct) {
    super();
    greeting ??= "Hello";
    punct ??= "!";
    this.#greeting = greeting;
    this.#punct = punct;
  }

  /** @returns {string} */
  get greeting() {
    return this.#greeting;
  }

  /** @returns {string} */
  get punct() {
    return this.#punct;
  }

  /**
   * @param {string | null} [name]
   * @param {number | null} [times]
   * @returns {string}
   */
  greet(name, times) {
    name ??= "world";
    times ??= 1;
    let out = "";
    {
      let i = 0;
      while (i < times) {
        out = `${out}${this.#greeting}, ${name}${this.#punct}`;
        i = (i + 1) | 0;
      }
    }
    return out;
  }

  /**
   * @param {string} text
   * @param {string | null} [mark]
   * @returns {string}
   */
  static shout(text, mark) {
    mark ??= "!";
    return text + mark + mark;
  }
}

class Counter extends type() {
  /** @type {number} */
  #count;

  /** @param {number | null} [start] */
  constructor(start) {
    super();
    start ??= 10;
    this.#count = start;
  }

  /** @returns {number} */
  get count() {
    return this.#count;
  }

  /** @param {number} value */
  set count(value) {
    this.#count = value;
  }

  /**
   * @param {number | null} [by]
   * @returns {number}
   */
  add(by) {
    by ??= 1;
    this.#count = (this.#count + by) | 0;
    return this.#count;
  }
}

let calls = 0;
console.log(range(1));
console.log(range(1, 5));
console.log(range(1, 5, 3));
console.log(range(2, null, 7));
console.log(`calls ${calls}`);
console.log(tag("plain"));
console.log(tag("marked", "#"));
console.log(named());
console.log(named(null));
console.log(named("Ada"));
console.log(new Greeter().greet());
console.log(new Greeter("Hi").greet("you", 2));
console.log(new Greeter(null, "?").greet(null, 2));
console.log(Greeter.shout("hey"));
const counter = new Counter();
counter.add();
console.log(`count ${counter.add(5)} ${new Counter(1).add()}`);
console.log(wrap("a"));
console.log(wrap("b", "<"));
console.log(wrap("c", "<", ">"));
const pad = new Pad();
console.log(pad.apply("x"));
console.log(new Pad(0, 3).apply("y", "-"));
