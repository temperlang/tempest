import { type } from "@temperlang/core";

class Greeter extends type() {
  /** @type {() => void} */
  greet = /** @type {any} */ (undefined);
}

class HiGreeter extends type(Greeter) {
  greet() {
    console.log("Hi, virtually!");
  }
}

/** @template T */
class Wrapper extends type() {
  /** @type {(item: T) => T} */
  wrap = /** @type {any} */ (undefined);
}

class Box extends type() {
  /** @type {string} */
  #label;

  /** @param {string} label */
  constructor(label) {
    super();
    this.#label = label;
  }

  /** @returns {string} */
  get label() {
    return this.#label;
  }
}

class BoxWrapper extends type(Wrapper) {
  /**
   * @param {Box} item
   * @returns {Box}
   */
  wrap(item) {
    item = new Box(`[${item.label}]`);
    return item;
  }
}

class IntWrapper extends type(Wrapper) {
  /**
   * @param {number} item
   * @returns {number}
   */
  wrap(item) {
    return (item + 1) | 0;
  }
}

const greeter = new HiGreeter();
greeter.greet();
/** @type {Wrapper<Box>} */
const boxWrapper = new BoxWrapper();
/** @type {Wrapper<number>} */
const intWrapper = new IntWrapper();
console.log(boxWrapper.wrap(new Box("box")).label);
console.log(intWrapper.wrap(41).toString());
