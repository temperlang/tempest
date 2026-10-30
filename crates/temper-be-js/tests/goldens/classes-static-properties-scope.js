import { type } from "@temperlang/core";

/** @returns {string} */
function readSimpleValue() {
  return Simple.value;
}

/** @returns {string} */
function readSimpleOther() {
  return Simple.other;
}

class Simple extends type() {
  /** @type {string} */
  static value;
  /** @type {string} */
  static other;
}

Simple.value = "foo";
Simple.other = "bar";
console.log(`Simple.value is ${readSimpleValue()}`);
console.log(`Simple.other is ${readSimpleOther()}`);
