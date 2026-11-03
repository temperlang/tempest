import { type } from "@temperlang/core";

export class Stringable extends type() {
  /** @type {() => string} */
  toString = /** @type {any} */ (undefined);
}

export class Circle extends type(Stringable) {
  /** @type {number} */
  #radius;

  /** @param {number} radius */
  constructor(radius) {
    super();
    this.#radius = radius;
  }

  /** @returns {number} */
  get radius() {
    return this.#radius;
  }

  /** @returns {string} */
  toString() {
    return `Circle of radius ${this.#radius}`;
  }
}

export class Rectangle extends type() {
  /** @type {number} */
  #width;
  /** @type {number} */
  #height;

  /**
   * @param {{ width: number, height: number }} props
   * @returns {Rectangle}
   */
  static new(props) {
    return new Rectangle(props.width, props.height);
  }

  /**
   * @param {number} width
   * @param {number} height
   */
  constructor(width, height) {
    super();
    this.#width = width;
    this.#height = height;
  }

  /** @returns {number} */
  get width() {
    return this.#width;
  }

  /** @returns {number} */
  get height() {
    return this.#height;
  }
}
