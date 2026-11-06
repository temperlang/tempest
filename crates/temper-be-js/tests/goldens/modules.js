import { share } from "./core.js";
import { base, bump, calls, twice } from "./helpers.js";
import { greet } from "./helpers/deep.js";

/**
 * @param {number} n
 * @returns {number}
 */
export function sum(n) {
  return (((twice(n) + base) | 0) + base2) | 0;
}

const calls2 = calls;
const base2 = 1;
console.log(greet("modules"));
console.log(`sum ${sum(3)}`);
console.log(`share ${share(7, 2)}`);
bump();
bump();
console.log(`calls ${calls2}`);
console.log(`calls now ${bump()}`);
