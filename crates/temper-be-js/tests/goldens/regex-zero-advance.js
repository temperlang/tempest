import { stringNext } from "@temperlang/core";
import {
  Begin,
  CodePoints,
  Match,
  Or,
  RegexNode,
  Repeat,
  Sequence,
  Space,
} from "@temperlang/std/regex";

/**
 * @param {string} text
 * @returns {string}
 */
export function bulletizeList(text) {
  const replaced = commaItemStartRegex.replace(text, (match) => "\n- ");
  return replaced.substring(stringNext(replaced, 0), replaced.length);
}

const commaItemStartRegex = /** @type {RegexNode} */ (/** @type {unknown} */ (new Sequence(Object.freeze([/** @type {RegexNode} */ (/** @type {unknown} */ (new Or(Object.freeze([/** @type {RegexNode} */ (/** @type {unknown} */ (Begin)), /** @type {RegexNode} */ (/** @type {unknown} */ (new CodePoints(",")))])))), /** @type {RegexNode} */ (/** @type {unknown} */ (new Repeat(/** @type {RegexNode} */ (/** @type {unknown} */ (Space)), 0, null, false)))])))).compiled();
console.log(bulletizeList("apples, bananas, cherries"));
