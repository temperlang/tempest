/** @returns {number} */
function bump() {
  count = (count + 1) | 0;
  total = (total + count) | 0;
  return count;
}

export function tick() {
  ticks = (ticks + 1) | 0;
}

/** @returns {string} */
export function report() {
  return `${greeting} ${bump()} of ${limit} ${total}`;
}

export const greeting = "hi";
let count = 0;
const limit = 3;
export let total = 10;
export let ticks = 0;
let local = 1;
local = (local + limit) | 0;
console.log(report());
console.log(report());
console.log(`local ${local}`);
