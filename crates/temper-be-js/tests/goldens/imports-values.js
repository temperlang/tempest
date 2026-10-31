import { magicNumber } from "./exporter.js";

/**
 * @param {number} n
 * @returns {number}
 */
export function timesMagic(n) {
  return Math.imul(n, magicNumber);
}

let a = 1;
a = a;
const c = timesMagic(a);
console.log(`number = ${c}`);
