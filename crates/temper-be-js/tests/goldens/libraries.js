import {
  bubble,
  cmpFloat64,
  cmpString,
  divIntInt,
  float64ToString,
  listedGet,
  listedJoin,
  listedMap,
  panic,
  stringToFloat64,
  type,
} from "@temperlang/core";

/**
 * @param {string} s
 * @returns {Grid}
 */
export function parseGrid(s) {
  if (s === "") {
    bubble();
  }
  return new Grid(1, Object.freeze([stringToFloat64(s)]));
}

/**
 * @param {readonly number[]} values
 * @param {number} ncols
 * @returns {number}
 */
function rows(values, ncols) {
  try {
    return divIntInt(values.length, ncols);
  } catch {
    return panic();
  }
}

/**
 * @param {Grid} grid
 * @returns {Grid}
 */
function scaled(grid) {
  if (grid.ncols === 0) {
    bubble();
  }
  return grid;
}

/**
 * @param {Grid} grid
 * @returns {readonly number[]}
 */
function cubes(grid) {
  try {
    /** @type {readonly number[]} */
    const more = listedMap(/** @type {number[]} */ (scaled(grid).values), (x) => x * x * x);
    return more;
  } catch {
    return panic();
  }
}

/**
 * @param {readonly number[]} values
 * @param {(arg0: number) => void} f
 */
function each(values, f) {
  {
    let i = 0;
    while (i < values.length) {
      f(listedGet(/** @type {number[]} */ (values), i));
      i = (i + 1) | 0;
    }
  }
}

/**
 * @param {readonly number[]} values
 * @returns {number}
 */
function total(values) {
  let sum = 0.0;
  /** @param {number} x */
  function fn(x) {
    try {
      const part = x / 2.0;
      sum = sum + part;
    } catch {
      panic();
    }
  }
  each(values, fn);
  return sum;
}

/**
 * @param {number} x
 * @returns {number}
 */
function half(x) {
  try {
    return x / 2.0;
  } catch {
    return panic();
  }
}

/**
 * @param {number} x
 * @returns {number}
 */
function cube(x) {
  return x * 0.25;
}

/**
 * @param {Grid} grid
 * @returns {number}
 */
function corner(grid) {
  try {
    return grid.get_(1, 1);
  } catch {
    return -1.0;
  }
}

/**
 * @param {number} n
 * @returns {Grid | null}
 */
function maybe(n) {
  if (n > 0) {
    return new Grid(n, Object.freeze([1.0]));
  } else {
    return null;
  }
}

/**
 * @param {Grid} grid
 * @returns {readonly number[]}
 */
function mapped(grid) {
  /** @type {Grid} */
  let t$0;
  try {
    t$0 = scaled(grid);
  } catch {
    t$0 = panic();
  }
  /**
   * @param {number} x
   * @returns {number}
   */
  function fn(x) {
    if (cmpFloat64(x, 1.5) > 0) {
      return x * 2.0;
    } else {
      return x;
    }
  }
  return listedMap(/** @type {number[]} */ (t$0.values), fn);
}

/**
 * @param {number} n
 * @returns {string | null}
 */
function text(n) {
  if (n > 0) {
    return "x";
  } else {
    return null;
  }
}

/**
 * @param {number} n
 * @returns {number | null}
 */
function count(n) {
  if (n > 0) {
    return n;
  } else {
    return null;
  }
}

/**
 * @param {number} n
 * @returns {Casing | null}
 */
function casing(n) {
  if (n > 0) {
    return Casing.first;
  } else {
    return null;
  }
}

/**
 * @param {string} s
 * @returns {string}
 */
function captured(s) {
  /** @type {Grid | null} */
  let found;
  try {
    found = parseGrid(s);
  } catch {
    found = null;
  }
  return listedJoin(/** @type {boolean[]} */ (listedMap(/** @type {number[]} */ (Object.freeze([1, 2])), (x) => found == null)), ",", (b) => b.toString());
}

/** @returns {string} */
function bidi() {
  return "look\u202eout";
}

/**
 * @param {number} n
 * @returns {string}
 */
function word(n) {
  if (n > 1) {
    return "b";
  } else {
    return "a";
  }
}

export class Grid extends type() {
  /** @type {number} */
  #ncols;
  /** @type {readonly number[]} */
  #values;

  /**
   * @param {{ ncols: number, values: readonly number[] }} props
   * @returns {Grid}
   */
  static new(props) {
    return new Grid(props.ncols, props.values);
  }

  /**
   * @param {number} ncols
   * @param {readonly number[]} values
   */
  constructor(ncols, values) {
    super();
    this.#ncols = ncols;
    this.#values = values;
  }

  /** @returns {number} */
  get ncols() {
    return this.#ncols;
  }

  /** @returns {readonly number[]} */
  get values() {
    return this.#values;
  }

  /**
   * @param {number} i
   * @param {number} j
   * @returns {number}
   */
  get_(i, j) {
    return listedGet(/** @type {number[]} */ (this.#values), (Math.imul(i, this.#ncols) + j) | 0);
  }

  /**
   * @param {string} s
   * @returns {Grid | null}
   */
  static parse(s) {
    try {
      return parseGrid(s);
    } catch {
      return null;
    }
  }
}

export class Casing extends type() {
  /** @type {string} */
  #name;
  /** @type {Casing} */
  static first;

  /** @param {string} name */
  constructor(name) {
    super();
    this.#name = name;
  }

  /** @returns {string} */
  get name() {
    return this.#name;
  }

  /**
   * @param {Casing} other
   * @returns {boolean}
   */
  eq(other) {
    return this.#name === other.#name;
  }

  /** @returns {string} */
  toString() {
    return this.#name;
  }
}

Casing.first = new Casing("first");
const grid = new Grid(2, Object.freeze([1.0, 2.0, 3.0, 4.0]));
console.log(`${rows(grid.values, 2)} ${float64ToString(half(3.0))} ${float64ToString(cube(8.0))}`);
console.log(float64ToString(corner(grid)));
console.log(float64ToString(total(grid.values)));
const none = text(0);
const some = count(3);
/** @type {string} */
let t$1;
{
  const part2 = some;
  t$1 = part2 == null ? "null" : part2.toString();
}
/** @type {string} */
let t$2;
{
  /** @type {number | null} */
  let t$3;
  {
    const subject = maybe(0);
    t$3 = subject == null ? null : subject.ncols;
  }
  const part3 = t$3;
  t$2 = part3 == null ? "null" : part3.toString();
}
console.log(`${none ?? "null"} ${t$1} ${t$2}`);
console.log(`${maybe(1) != null} ${maybe(0) != null} ${Grid.parse("") == null}`);
console.log(Casing.first.toString());
console.log(listedJoin(/** @type {number[]} */ (mapped(grid)), ",", (x) => float64ToString(x)));
console.log(listedJoin(/** @type {number[]} */ (cubes(grid)), ",", (x) => float64ToString(x)));
/** @type {boolean} */
let t$4;
{
  const left = casing(1);
  t$4 = left == null ? false : left.eq(Casing.first);
}
/** @type {boolean} */
let t$5;
{
  const left2 = casing(0);
  t$5 = left2 == null ? false : left2.eq(Casing.first);
}
/** @type {boolean} */
let t$6;
{
  const left3 = casing(0);
  const right = casing(0);
  t$6 = left3 == null ? right == null : right == null ? false : left3.eq(right);
}
console.log(`${t$4} ${t$5} ${!t$6}`);
console.log(`${captured("")} ${captured("2.5")}`);
console.log((bidi() !== "lookout").toString());
console.log(`${Math.sign(cmpString(word(1), word(2)))} ${Math.sign(cmpString(word(2), word(1)))} ${Math.sign(cmpString(word(2), word(3)))}`);
const named = word(2);
console.log((named === word(3) && `${named}!` !== named).toString());
