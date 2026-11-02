import {
  listedFilter,
  listedJoin,
  listedMap,
  stringSplit,
} from "@temperlang/core";

/** @type {readonly number[]} */
const ls = Object.freeze([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
console.log(`All  ${listedJoin(/** @type {number[]} */ (ls), ", ", (x) => x.toString())}`);
console.log(`Even ${listedJoin(/** @type {number[]} */ (listedFilter(/** @type {number[]} */ (ls), (x) => (x & 1) === 0)), ", ", (x) => x.toString())}`);
console.log(`Odd  ${listedJoin(/** @type {number[]} */ (listedFilter(/** @type {number[]} */ (ls), (x) => (x & 1) !== 0)), ", ", (x) => x.toString())}`);
console.log(`Neg  ${listedJoin(/** @type {number[]} */ (listedMap(/** @type {number[]} */ (ls), (x) => -x | 0)), ", ", (x) => x.toString())}`);
/** @type {readonly string[]} */
const strs = stringSplit("1 ; 2 ; 3", " ; ");
console.log(`Splt ${listedJoin(/** @type {string[]} */ (strs), ", ", (x) => x)}`);
