import { sum } from "modules";
import { base, twice } from "modules/helpers";

/**
 * @param {number} n
 * @returns {number}
 */
export function both(n) {
  return (((sum(n) + twice(n)) | 0) + base) | 0;
}

console.log(`both ${both(2)}`);
