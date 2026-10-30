/**
 * @param {number} i
 * @returns {number}
 */
function fib(i) {
  let a = 0;
  let b = 1;
  while (i > 0) {
    const c = (a + b) | 0;
    a = b;
    b = c;
    i = (i - 1) | 0;
  }
  return a;
}

let zero = 0;
zero = zero;
console.log(`fib(1)=${fib((zero + 1) | 0)}`);
console.log(`fib(10)=${fib((zero + 10) | 0)}`);
console.log(`fib(20)=${fib((zero + 20) | 0)}`);
