import { float64Near, float64ToString, listedJoin } from "@temperlang/core";

/**
 * @param {string} label
 * @param {number} actual
 * @param {number} expected
 */
function near(label, actual, expected) {
  /** @type {string} */
  let t$0;
  {
    const subject = float64Near(actual, expected);
    t$0 = subject === true ? "✅" : `❌ wanted ${float64ToString(expected)}, was ${float64ToString(actual)}`;
  }
  const result = t$0;
  console.log(`${label}: ${result}`);
}

let one = 1.0;
one = one;
const two = one + one;
const half = one / two;
near("half.abs()", Math.abs(half), half);
near("(-half).abs()", Math.abs(-half), half);
const three = one + two;
near("half.acos()", Math.acos(half), Math.PI / three);
const pi = Math.PI;
const six = two * three;
near("half.asin()", Math.asin(half), pi / six);
const four = two * two;
near("(-one).atan()", Math.atan(-one), -pi / four);
near("one.atan2(-one)", Math.atan2(one, -one), (three * pi) / four);
near("(four / three).ceil()", Math.ceil(four / three), two);
near("(-four / three).ceil()", Math.ceil(-four / three), -one);
near("(pi / three).cos()", Math.cos(pi / three), half);
const e = Math.E;
near("one.cosh()", Math.cosh(one), (e + one / e) / two);
near("half.exp() == e.sqrt()", Math.exp(half), Math.sqrt(e));
near("half.expm1()", Math.expm1(half), Math.sqrt(e) - one);
near("(four / three).floor()", Math.floor(four / three), one);
near("(-four / three).floor()", Math.floor(-four / three), -two);
near("e.log()", Math.log(Math.E), one);
const five = one + four;
const ten = two * five;
near("ten.log10()", Math.log10(ten), one);
near("(e - one).log1p()", Math.log1p(e - one), one);

/**
 * @param {number} x
 * @returns {string}
 */
function fn(x) {
  /**
   * @param {number} y
   * @returns {string}
   */
  function fn2(y) {
    const pair = listedJoin(/** @type {number[]} */ (Object.freeze([Math.max(x, y), Math.min(x, y)])), ", ", (z) => float64ToString(z));
    return `(${pair})`;
  }
  return listedJoin(/** @type {number[]} */ (Object.freeze([NaN, two])), ", ", fn2);
}

const maxMinCombos = listedJoin(/** @type {number[]} */ (Object.freeze([NaN, one])), ", ", fn);
console.log(`maxMinCombos: ${maxMinCombos}`);
const quarter = half / two;
near("2.25 % 2.0", (two + quarter) % two, 0.25);
near("2.25 % -2.0", (two + quarter) % -two, 0.25);
near("-2.25 % 2.0", -(two + quarter) % two, -0.25);
near("-2.25 % -2.0", -(two + quarter) % -two, -0.25);
near("pi.round()", Math.round(pi), three);
near("(-pi).round()", Math.round(-pi), -three);
near("pi.sign()", Math.sign(pi), one);
near("(-pi).sign()", Math.sign(-pi), -one);
const zero = one - one;
console.log(`zero.sign(): ${float64ToString(Math.sign(zero))}`);
console.log(`(-zero).sign(): ${float64ToString(Math.sign(-zero))}`);
near("(pi / six).sin()", Math.sin(pi / six), half);
near("one.sinh()", Math.sinh(one), (e - one / e) / two);
near("(-pi / four).tan()", Math.tan(-pi / four), -one);
near("(three * pi / four).tan()", Math.tan((three * pi) / four), -one);
near("one.tanh()", Math.tanh(one), (e ** 2.0 - 1.0) / (e ** 2.0 + 1.0));
console.log(`one.near(one + 0.1): ${float64Near(one, one + 0.1)}`);
console.log(`one.near(one + 0.25, absTol = 0.25): ${float64Near(one, one + 0.25, null, 0.25)}`);
console.log(`one.near(one + 0.1 + 1e-9, absTol = 0.1): ${float64Near(one, one + 0.1 + 1e-9, null, 0.1)}`);
console.log(`one.near(one - 0.1, absTol = 0.11): ${float64Near(one, one - 0.1, null, 0.11)}`);
console.log(`ten.near(ten + 0.1, absTol = 0.011): ${float64Near(ten, ten + 0.1, null, 0.011)}`);
console.log(`ten.near(ten + 0.1, relTol = 0.011): ${float64Near(ten, ten + 0.1, 0.011)}`);
