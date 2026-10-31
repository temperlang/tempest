import {
  bubble,
  divIntInt,
  modIntInt,
  requireNotNull,
  stringToInt32,
} from "@temperlang/core";

/**
 * @param {number | null} x
 * @returns {number}
 */
function more(x) {
  return bubble();
}

/**
 * @param {string | null} x
 * @returns {string}
 */
function something(x) {
  {
    if (x != null) {
      return x;
    } else {
      return bubble();
    }
  }
}

/** @param {string | null} x */
function report(x) {
  /** @type {string} */
  let message;
  try {
    message = something(x);
  } catch {
    message = "null";
  }
  /** @type {string} */
  let suffix;
  try {
    let t$0;
    try {
      requireNotNull(x);
      t$0 = 1;
    } catch {
      t$0 = 0;
    }
    suffix = more(t$0).toString();
  } catch {
    suffix = "";
  }
  console.log(message + suffix);
}

/**
 * @param {number} cols
 * @param {number} total
 * @returns {number}
 */
function calcRows(cols, total) {
  console.log(`calcRows(${cols}, ${total})`);
  if (modIntInt(total, cols) !== 0) {
    bubble();
  }
  return divIntInt(total, cols);
}

/**
 * @param {string} s
 * @returns {number}
 */
function countBelow(s) {
  let i = 0;
  while (i < stringToInt32(s, null)) {
    i = (i + 1) | 0;
  }
  return i;
}

/**
 * @param {number} a
 * @param {number} b
 */
function branch(a, b) {
  const c = a === 0 ? b === 0 ? "00" : b === 1 ? "01" : b === 2 ? "02" : "0x" : a === 1 ? "1x" : a === 2 ? "2x" : a === 3 ? "3x" : "xx";
  console.log(`branch(${a}, ${b}) -> ${c}`);
}

report("hi");
report(null);
console.log(calcRows(3, 6).toString());
let t$1;
try {
  t$1 = countBelow("abc");
} catch {
  t$1 = -1;
}
console.log(t$1.toString());
branch(0, 2);
branch(2, 0);
