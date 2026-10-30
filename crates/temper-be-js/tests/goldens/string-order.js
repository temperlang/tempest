import { cmpFloat64, cmpString } from "@temperlang/core";

/**
 * @param {string} a
 * @param {string} b
 * @returns {string}
 */
function show(a, b) {
  return `${Math.sign(cmpString(a, b))} ${cmpString(a, b) < 0} ${cmpString(a, b) <= 0} ${cmpString(a, b) > 0} ${cmpString(a, b) >= 0}`;
}

/**
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
function cmp(a, b) {
  return Math.sign(cmpFloat64(a, b));
}

console.log(show("｡", "😀"));
console.log(show("😀", "｡"));
console.log(show("", "𐀀"));
console.log(show("￿", "􏿿"));
console.log(show("a｡b", "a😀"));
console.log(show("😀", "😁"));
console.log(show("😀", "😀"));
console.log(show("😀", "😀a"));
console.log(show("", "😀"));
console.log(show("", ""));
console.log(show("z", "a"));
console.log(show("abc", "abd"));
console.log(`${cmp(2.5, 1.0)} ${cmp(1.0, 2.5)} ${cmp(1.5, 1.5)}`);
