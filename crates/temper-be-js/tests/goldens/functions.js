import {
  listedFilter,
  listedGet,
  listedJoin,
  listedMap,
  listedSorted,
  type,
} from "@temperlang/core";

/**
 * @param {number} start
 * @returns {() => number}
 */
function makeCounter(start) {
  let count = start;
  /** @returns {number} */
  function fn() {
    count = (count + 1) | 0;
    return count;
  }
  return fn;
}

/**
 * @param {(arg0: number) => number} f
 * @returns {(arg0: number) => number}
 */
function twice(f) {
  return (x) => f(f(x));
}

/**
 * @param {(arg0: number) => number} f
 * @param {number} x
 * @returns {number}
 */
function applyTo(f, x) {
  return f(x);
}

/**
 * @param {number} x
 * @returns {number}
 */
function addOne(x) {
  return (x + 1) | 0;
}

class Scaler extends type() {
  /**
   * @param {number} x
   * @returns {number}
   */
  static triple(x) {
    return Math.imul(x, 3);
  }
}

class Button extends type() {
  /** @type {string} */
  #label;
  /** @type {() => number} */
  #onPress;

  /**
   * @param {{ label: string, onPress: () => number }} props
   * @returns {Button}
   */
  static new(props) {
    return new Button(props.label, props.onPress);
  }

  /**
   * @param {string} label
   * @param {() => number} onPress
   */
  constructor(label, onPress) {
    super();
    this.#label = label;
    this.#onPress = onPress;
  }

  /** @returns {string} */
  get label() {
    return this.#label;
  }

  /** @returns {() => number} */
  get onPress() {
    return this.#onPress;
  }

  /** @returns {string} */
  press() {
    return `${this.#label} ${this.#onPress()}`;
  }
}

const counter = makeCounter(10);
counter();
console.log(`count ${counter()}`);
const other = makeCounter(0);
console.log(`other ${other()} count ${counter()}`);
const addTwo = twice(addOne);
console.log(`twice ${addTwo(5)} ${applyTo(addOne, 1)}`);
console.log(`triple ${applyTo(Scaler.triple, 4)} ${twice(Scaler.triple)(2)}`);
/** @type {readonly number[]} */
const ns = Object.freeze([5, 2, 8, 1, 4]);
/** @type {readonly number[]} */
const doubled = listedMap(/** @type {number[]} */ (ns), (n) => Math.imul(n, 2));
/** @type {readonly number[]} */
const big = listedFilter(/** @type {number[]} */ (doubled), (n) => n > 4);
console.log(listedJoin(/** @type {number[]} */ (big), ", ", (n) => n.toString()));
/** @type {readonly number[]} */
const order = listedSorted(/** @type {number[]} */ (ns), (a, b) => (a - b) | 0);
console.log(listedJoin(/** @type {number[]} */ (order), " ", (n) => n.toString()));
let total = 0;
{
  const length = ns.length;
  let i2 = 0;
  while (i2 < length) {
    const n = listedGet(/** @type {number[]} */ (ns), i2);
    i2 = (i2 + 1) | 0;
    total = (total + n) | 0;
  }
}
console.log(`total ${total}`);
const button = new Button("hits", makeCounter(0));
button.press();
console.log(button.press());
/** @type {(() => number)[]} */
const makers = [];
{
  let i = 0;
  while (i < 3) {
    const k = Math.imul(i, 10);
    makers.push(() => k);
    i = (i + 1) | 0;
  }
}
console.log(listedJoin(makers, " ", (c) => c().toString()));
