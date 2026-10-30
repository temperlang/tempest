import {
  bitwiseShrUnsigned32,
  cmpInt32,
  cmpString,
  divIntInt,
  modIntInt,
} from "@temperlang/core";

/**
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
function quot(a, b) {
  return divIntInt(a, b);
}

/**
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
function rem(a, b) {
  return modIntInt(a, b);
}

/**
 * @param {string} a
 * @param {string} b
 * @returns {string}
 */
function words(a, b) {
  if (cmpString(a, b) < 0) {
    return `${a} < ${b}`;
  }
  if (cmpString(a, b) <= 0) {
    return `${a} == ${b}`;
  } else {
    return `${a} > ${b}`;
  }
}

const one = 1;
const big = Math.imul(2147483647, one);
const small = Math.imul(-2147483648, one);
console.log(`${(big + 1) | 0} ${(small - 1) | 0} ${Math.imul(big, 4)} ${-small | 0}`);
console.log(`${Math.imul(-7, one) >> 1} ${bitwiseShrUnsigned32(Math.imul(-7, one), 28)} ${one << 33} ${~one} ${5 & 3} ${5 | 3} ${5 ^ 3}`);
console.log(`${cmpInt32(3, 5)} ${cmpString("b", "a")}`);
console.log(`${quot(-7, 2)} ${rem(-7, 2)} ${quot(small, -1)}`);
console.log(words("apple", "banana"));
console.log(words("pear", "pear"));
console.log(words("zoo", "yak"));
console.log((words("a", "b") === "a < b").toString());
let n = 0;
let odd = 0;
do {
  n = (n + 1) | 0;
  if (modIntInt(n, 2) !== 0) {
    odd = (odd + n) | 0;
  }
} while (n < 9);
console.log(`odd ${odd}`);
let found = -1;
let i = 0;
while (i < 5 && found < 0) {
  let j = 0;
  while (j < 5) {
    if (Math.imul(i, j) === 6) {
      found = (Math.imul(i, 10) + j) | 0;
      break;
    }
    j = (j + 1) | 0;
  }
  i = (i + 1) | 0;
}
console.log(`found ${found}`);
const empty = "";
const x = "x";
console.log((!empty && !!x).toString());
