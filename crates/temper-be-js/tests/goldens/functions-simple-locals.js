/**
 * @param {number} i
 * @param {(arg0: number) => number} f
 * @returns {number}
 */
function callIt(i, f) {
  return Math.imul(3, f(i));
}

/** @param {number} i */
function enclose(i) {
  const result = callIt(2, (j) => (i + j) | 0);
  console.log(result.toString());
}

enclose(1);
