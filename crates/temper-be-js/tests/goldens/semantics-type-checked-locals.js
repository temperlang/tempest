import { bubble } from "@temperlang/core";

function yay() {
  console.log("yay");
}

function meh() {
  console.log("meh");
}

function boo() {
  console.log("boo");
}

/**
 * @param {number} x
 * @returns {number}
 */
function f(x) {
  return x;
}

/**
 * @param {number} x
 * @returns {number}
 */
function g(x) {
  return x;
}

/**
 * @param {number} x
 * @returns {number}
 */
function i(x) {
  return x;
}

console.log("Checkpoint 0");
try {
  /** @type {number} */
  let a;
  bubble();
} catch {
  yay();
}
console.log("Checkpoint 1");
/** @type {void} */
let t$0;
{
  const a2 = 1;
  yay();
  t$0 = undefined;
}
t$0;
console.log("Checkpoint 2");
{
  const a3 = 1;
  try {
    bubble();
  } catch {
    meh();
  }
  if (a3 === 1) {
    meh();
  } else {
    meh();
  }
}
console.log("Checkpoint 3");
/** @type {void} */
let t$1;
{
  let a4 = 1;
  a4 = 2;
  t$1 = yay();
}
t$1;
console.log("Checkpoint 4");
try {
  f(0);
  yay();
} catch {
  boo();
}
try {
  bubble();
} catch {
  meh();
}
console.log("Checkpoint 5");
try {
  g(0);
  yay();
} catch {
  boo();
}
try {
  bubble();
} catch {
  meh();
}
console.log("Checkpoint 6");
try {
  i(0);
  yay();
} catch {
  boo();
}
try {
  bubble();
} catch {
  meh();
}
