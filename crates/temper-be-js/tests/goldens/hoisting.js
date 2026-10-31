import { bubble, eqFloat64 } from "@temperlang/core";

/**
 * @param {number} n
 * @returns {number}
 */
function noted(n) {
  calls = (calls + 1) | 0;
  console.log(`noted ${n}`);
  return n;
}

/**
 * @param {number} n
 * @returns {number}
 */
function failing(n) {
  if (n > 2) {
    bubble();
  }
  return n;
}

/**
 * @param {number} a
 * @param {number} b
 * @returns {string}
 */
function pair(a, b) {
  return `${a},${b}`;
}

/**
 * @param {number | null} a
 * @param {number | null} b
 * @returns {boolean}
 */
function sameFloat(a, b) {
  return a == null ? b == null : b != null && eqFloat64(a, b);
}

let calls = 0;
const t$1 = noted(1);
let t$0;
try {
  t$0 = failing(3);
} catch {
  t$0 = 9;
}
console.log(pair(t$1, t$0));
const t = true;
const f = false;
let t$3 = f;
if (t$3) {
  let t$2;
  try {
    t$2 = failing(5);
  } catch {
    t$2 = 1;
  }
  t$3 = t$2 === 1;
}
console.log(t$3.toString());
let t$5 = t;
if (t$5) {
  let t$4;
  try {
    t$4 = failing(1);
  } catch {
    t$4 = 2;
  }
  t$5 = t$4 === 1;
}
console.log(t$5.toString());
let t$7 = t;
if (!t$7) {
  let t$6;
  try {
    t$6 = noted(7);
  } catch {
    t$6 = 0;
  }
  t$7 = t$6 === 0;
}
console.log(t$7.toString());
let t$8;
if (t) {
  try {
    t$8 = failing(4);
  } catch {
    t$8 = 4;
  }
} else {
  t$8 = 0;
}
console.log(t$8.toString());
let i = 0;
while (true) {
  let t$9;
  try {
    t$9 = failing(i);
  } catch {
    t$9 = 100;
  }
  if (t$9 >= 3) {
    break;
  }
  i = (i + 1) | 0;
}
console.log(i.toString());
console.log(`${sameFloat(null, null)} ${sameFloat(0.5, null)} ${sameFloat(0.5, 0.5)}`);
console.log(calls.toString());
