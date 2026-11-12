import { sum } from "modules";
import { base, twice } from "modules/helpers";
import { Builtins, Checked, lambda, native } from "renamed";

/**
 * @param {number} n
 * @returns {number}
 */
export function both(n) {
  return (((sum(n) + twice(n)) | 0) + base) | 0;
}

console.log(`both ${both(2)}`);
const checked = new Checked(lambda(native));
console.log(`renamed ${checked.delete_()} ${checked.local} ${checked.end}`);
const b = new Builtins(4);
console.log(`builtins ${b.hash} ${b.get(1)} ${b.set(3)} ${b.map(5)} ${b.format()} ${b.int} ${b.later()}`);
console.log(b.print());
