/**
 * @param {number} n
 * @returns {number}
 */
function search(n) {
  let found = -1;
  let i = 0;
  outer: while (i < n) {
    let j = 0;
    while (true) {
      if (j >= n) {
        break;
      }
      if (Math.imul(i, j) === 6) {
        found = (Math.imul(i, 10) + j) | 0;
        break outer;
      }
      j = (j + 1) | 0;
    }
    i = (i + 1) | 0;
  }
  return found;
}

/**
 * @param {number} n
 * @returns {number}
 */
function odds(n) {
  let k = 0;
  let sum = 0;
  do {
    k = (k + 1) | 0;
    if ((k & 1) === 0) {
      continue;
    }
    sum = (sum + k) | 0;
  } while (k < n);
  return sum;
}

console.log(search(5).toString());
console.log(odds(9).toString());
