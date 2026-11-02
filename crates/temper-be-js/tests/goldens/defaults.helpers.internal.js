import { type } from "@temperlang/core";

/**
 * @param {string} text
 * @param {string | null} [open]
 * @param {string | null} [close]
 * @returns {string}
 */
export function wrap(text, open, close) {
  open ??= "[";
  close ??= open;
  return open + text + close;
}

export class Pad extends type() {
  /** @type {number} */
  #left;
  /** @type {number} */
  #right;

  /**
   * @param {{ left?: number | null, right?: number | null }} props
   * @returns {Pad}
   */
  static new(props) {
    return new Pad(props.left, props.right);
  }

  /**
   * @param {number | null} [left]
   * @param {number | null} [right]
   */
  constructor(left, right) {
    super();
    left ??= 1;
    right ??= 2;
    this.#left = left;
    this.#right = right;
  }

  /** @returns {number} */
  get left() {
    return this.#left;
  }

  /** @returns {number} */
  get right() {
    return this.#right;
  }

  /**
   * @param {string} text
   * @param {string | null} [fill]
   * @returns {string}
   */
  apply(text, fill) {
    fill ??= ".";
    let out = text;
    {
      let i = 0;
      while (i < this.#left) {
        out = fill + out;
        i = (i + 1) | 0;
      }
    }
    {
      let i = 0;
      while (i < this.#right) {
        out = out + fill;
        i = (i + 1) | 0;
      }
    }
    return out;
  }
}
