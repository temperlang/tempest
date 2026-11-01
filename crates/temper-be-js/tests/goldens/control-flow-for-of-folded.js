import { listedFilter, listedGet } from "@temperlang/core";

/**
 * @param {readonly number[]} xs
 * @returns {number}
 */
function total(xs) {
  let t = 0;
  {
    const length = xs.length;
    let i = 0;
    while (i < length) {
      const x = listedGet(/** @type {number[]} */ (xs), i);
      i = (i + 1) | 0;
      t = (t + x) | 0;
    }
  }
  return t;
}

/**
 * @param {readonly number[]} xs
 * @returns {number}
 */
function totalForEach(xs) {
  let t = 0;
  {
    const length = xs.length;
    let i = 0;
    while (i < length) {
      const x = listedGet(/** @type {number[]} */ (xs), i);
      i = (i + 1) | 0;
      t = (t + x) | 0;
    }
  }
  return t;
}

/**
 * @param {readonly number[]} xs
 * @returns {number}
 */
function totalFiltered(xs) {
  let t = 0;
  /**
   * @param {number} x
   * @returns {boolean}
   */
  function fn(x) {
    t = (t + x) | 0;
    return true;
  }
  listedFilter(/** @type {number[]} */ (xs), fn);
  return t;
}

/**
 * @param {readonly (readonly number[])[]} xss
 * @returns {number}
 */
function totalNested(xss) {
  let t = 0;
  {
    const length = xss.length;
    let i = 0;
    while (i < length) {
      /** @type {readonly number[]} */
      const xs = listedGet(/** @type {(readonly number[])[]} */ (xss), i);
      i = (i + 1) | 0;
      {
        const length2 = xs.length;
        let i2 = 0;
        while (i2 < length2) {
          const x = listedGet(/** @type {number[]} */ (xs), i2);
          i2 = (i2 + 1) | 0;
          t = (t + x) | 0;
        }
      }
    }
  }
  return t;
}

/**
 * @param {readonly number[]} xs
 * @param {number} n
 * @returns {number}
 */
function firstOver(xs, n) {
  {
    const length = xs.length;
    let i = 0;
    while (i < length) {
      const x = listedGet(/** @type {number[]} */ (xs), i);
      i = (i + 1) | 0;
      if (x > n) {
        return x;
      }
    }
  }
  return -1;
}

console.log(`total ${total(Object.freeze([1, 2, 3]))}`);
console.log(`forEach ${totalForEach(Object.freeze([1, 2, 3]))}`);
console.log(`filter ${totalFiltered(Object.freeze([1, 2, 3]))}`);
console.log(`nested ${totalNested(Object.freeze([Object.freeze([1, 2]), Object.freeze([3]), Object.freeze([])]))}`);
console.log(`firstOver ${firstOver(Object.freeze([1, 2, 3]), 1)} ${firstOver(Object.freeze([1, 2, 3]), 5)}`);
