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
 * @param {string | null} [label]
 * @returns {Tagged}
 */
function tagged(label) {
  label ??= null;
  return new Tagged(label);
}

/**
 * @param {number} c
 * @returns {number}
 */
function weight(c) {
  const base = c === 42 ? 0 : 6;
  return ((((c > 50 ? 1 : 2) + base) | 0) + c) | 0;
}

/**
 * @param {Shape} s
 * @returns {string}
 */
function shapeName(s) {
  const name = s instanceof Dot ? "dot" : s instanceof Line ? "line" : panic();
  return name;
}

/**
 * @param {number} n
 * @returns {Shape}
 */
function shapeOf(n) {
  const shape = n > 1 ? new Line(n) : new Dot(n);
  return shape;
}

/**
 * @param {Shape} s
 * @returns {readonly number[]}
 */
function sizes(s) {
  /** @type {readonly number[]} */
  const found = s instanceof Dot ? Object.freeze([/** @type {Dot} */ (/** @type {unknown} */ (s)).size, /** @type {Dot} */ (/** @type {unknown} */ (s)).size]) : Object.freeze([]);
  return found;
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
 * @param {readonly number[]} digits
 * @param {boolean} short
 * @param {number} c
 * @returns {number}
 */
function byteAt(digits, short, c) {
  const byte = short ? Math.imul(listedGet(/** @type {number[]} */ (digits), c), 17) : (Math.imul(listedGet(/** @type {number[]} */ (digits), Math.imul(2, c)), 16) + listedGet(/** @type {number[]} */ (digits), (Math.imul(2, c) + 1) | 0)) | 0;
  return byte;
}

/**
 * @param {readonly number[]} digits
 * @returns {boolean}
 */
function ascending(digits) {
  if (digits.length > 1) {
    return ((listedGet(/** @type {number[]} */ (digits), 0) + 1) | 0) === listedGet(/** @type {number[]} */ (digits), 1);
  } else {
    return false;
  }
}

/**
 * @param {readonly number[]} digits
 * @returns {number}
 */
function firstDrop(digits) {
  let i = 1;
  while (i < digits.length && listedGet(/** @type {number[]} */ (digits), (i - 1) | 0) <= listedGet(/** @type {number[]} */ (digits), i)) {
    i = (i + 1) | 0;
  }
  return i;
}

/** @returns {number} */
function tick() {
  ticks = (ticks + 1) | 0;
  return ticks;
}

/**
 * @param {boolean} short
 * @returns {number}
 */
function tickIf(short) {
  const n = short ? tick() : (Math.imul(tick(), 10) + tick()) | 0;
  return n;
}

/** @returns {boolean} */
function ticked() {
  if (ticks > 0) {
    return ((Math.imul(tick(), 10) + tick()) | 0) > 0;
  } else {
    return false;
  }
}

/** @returns {number} */
function tickWhile() {
  let i = 0;
  while (i < 3 && tick() < tick()) {
    i = (i + 1) | 0;
  }
  return i;
}

/**
 * @template T
 * @param {readonly T[]} items
 * @returns {number}
 */
export function sizeWith(items) {
  return (items.length + extras.length) | 0;
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
  /** @type {Casing} */
  static last;

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

  /**
   * @param {string} name
   * @returns {Casing}
   */
  static named(name) {
    return new Casing(name);
  }
}

export class Shape extends type() {
  /** @type {number} */
  size = /** @type {any} */ (undefined);
}

export class Dot extends type(Shape) {
  /** @type {number} */
  #size;

  /** @param {number} size */
  constructor(size) {
    super();
    this.#size = size;
  }

  /** @returns {number} */
  get size() {
    return this.#size;
  }

  /** @returns {Dot | null} */
  get bigger() {
    if (this.#size < 3) {
      return new Dot((this.#size + 1) | 0);
    } else {
      return null;
    }
  }

  /** @returns {number | null} */
  get grown() {
    {
      const subject = this.bigger;
      if (subject == null) {
        return null;
      } else {
        return subject.scaled(10);
      }
    }
  }

  /**
   * @param {number} k
   * @returns {number}
   */
  scaled(k) {
    return Math.imul(this.#size, k);
  }
}

export class Line extends type(Shape) {
  /** @type {number} */
  #size;

  /** @param {number} size */
  constructor(size) {
    super();
    this.#size = size;
  }

  /** @returns {number} */
  get size() {
    return this.#size;
  }
}

export class Carry extends type() {
  /** @type {boolean} */
  #carrying;
  /** @type {number} */
  #state;

  /** @param {number} start */
  constructor(start) {
    super();
    this.#carrying = false;
    this.#state = 0;
    if (start !== 0) {
      this.#carrying = true;
      this.#state = start;
    }
  }

  /** @returns {boolean} */
  get carrying() {
    return this.#carrying;
  }

  /** @returns {number} */
  get state() {
    return this.#state;
  }
}

export class Tagged extends type() {
  /** @type {string | null} */
  #label;

  /** @param {string | null} label */
  constructor(label) {
    super();
    this.#label = label;
  }

  /** @returns {string | null} */
  get label() {
    return this.#label;
  }
}

Casing.first = new Casing("first");
Casing.last = Casing.named("last");
let ticks = 0;
/** @type {readonly number[]} */
const extras = Object.freeze([1]);
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
/** @type {readonly number[]} */
const digits = Object.freeze([1, 2, 4, 3]);
console.log(`${byteAt(digits, true, 1)} ${byteAt(digits, false, 0)} ${ascending(digits)} ${firstDrop(digits)} ${sizeWith(digits)}`);
console.log(`${tickIf(true)} ${tickIf(false)} ${ticked()} ${tickWhile()}`);
/** @type {string} */
let t$7;
{
  const part4 = new Dot(1).grown;
  t$7 = part4 == null ? "null" : part4.toString();
}
/** @type {string} */
let t$8;
{
  const part5 = new Dot(3).grown;
  t$8 = part5 == null ? "null" : part5.toString();
}
console.log(`${shapeName(shapeOf(1))} ${shapeName(shapeOf(2))} ${sizes(shapeOf(1)).length} ${sizes(shapeOf(2)).length} ${t$7} ${t$8} ${Casing.last.toString()}`);
const carry = new Carry(3);
console.log(`${carry.carrying} ${carry.state} ${new Carry(0).carrying} ${tagged().label ?? "null"} ${tagged("x").label ?? "null"} ${weight(42)} ${weight(60)}`);
