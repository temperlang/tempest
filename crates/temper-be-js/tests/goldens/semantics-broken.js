import { bubble, stringCountBetween } from "@temperlang/core";

/** @returns {number} */
function undefinedPlusArgInside() {
  return bubble();
}

/**
 * @param {string} string
 * @param {number} minSize
 * @param {string} pad
 * @returns {string}
 */
function padLeft(string, minSize, pad) {
  const needed = (minSize - stringCountBetween(string, 0, string.length)) | 0;
  if (needed <= 0) {
    return string;
  }
  bubble();
}

bubble();
