/**
 * @param {number} n
 * @returns {number}
 */
export function twice(n) {
  return Math.imul(n, 2);
}

export let runs = 0;
export const base = 10;
console.log(`init ${twice(1)}`);

/** @param {number} value$ */
export function runs$set(value$) {
  runs = value$;
}
