import { divIntInt } from "@temperlang/core";

/**
 * @param {number} n
 * @param {number} d
 * @returns {number}
 */
export function share(n, d) {
  try {
    return divIntInt(n, d);
  } catch {
    return 0;
  }
}
