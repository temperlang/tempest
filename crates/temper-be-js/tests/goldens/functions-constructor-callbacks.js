import { type } from "@temperlang/core";

/**
 * @template T
 * @param {(arg0: T) => T} ha
 * @returns {Hi<T>}
 */
export function makeHi(ha) {
  return new Hi(ha);
}

/**
 * @param {number} j
 * @returns {Hi<number>}
 */
export function newIntHi(j) {
  return new Hi(((i) => (i + j) | 0));
}

/**
 * @param {number} j
 * @returns {Hi<number>}
 */
export function makeIntHi(j) {
  return makeHi((i) => (i + j) | 0);
}

/** @template T */
export class Hi extends type() {
  /** @type {(arg0: T) => T} */
  #ha;

  /** @param {(arg0: T) => T} ha */
  constructor(ha) {
    super();
    this.#ha = ha;
  }

  /** @returns {(arg0: T) => T} */
  get ha() {
    return this.#ha;
  }
}
