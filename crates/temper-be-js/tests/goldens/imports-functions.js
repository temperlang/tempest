import {
  float64ToString,
  listedJoin,
  listedReduceFrom,
} from "@temperlang/core";
import { normSquared } from "./exporter.js";

/**
 * @param {number} x
 * @param {number} y
 * @returns {number}
 */
export function norm(x, y) {
  return Math.sqrt(normSquared(x, y));
}

/**
 * @param {readonly number[]} vals
 * @returns {number}
 */
export function something(vals) {
  return /** @type {<T, R>(ls: readonly T[], initial: R, accumulate: (prev: R, cur: T) => R) => R} */ (listedReduceFrom)(/** @type {number[]} */ (vals), 0.0, normSquared);
}

let a = 3.0;
a = a;
let b = a + 1.0;
const values = listedJoin(/** @type {number[]} */ (Object.freeze([a, b, norm(a, b)])), ", ", (it) => float64ToString(it));
console.log(`${values} triangle`);
console.log(float64ToString(something(Object.freeze([a, b]))));
