import { cmpInt32, requireInstanceOf, type } from "@temperlang/core";

/**
 * @param {Shape} x
 * @returns {Shape}
 */
function shape(x) {
  return x;
}

/**
 * @param {number} n
 * @returns {string}
 */
function typeNamed(n) {
  {
    if (n === 0) {
      return "undefined";
    } else if (n === 1) {
      return "boolean";
    } else if (n === 2) {
      return "number";
    } else if (n === 3) {
      return "bigint";
    } else if (n === 4) {
      return "number";
    } else {
      return "string";
    }
  }
}

/**
 * @param {string} t
 * @returns {string}
 */
function same(t) {
  return t;
}

export class Shape extends type() {
  /** @type {number} */
  area = /** @type {any} */ (undefined);

  /** @returns {string} */
  describe() {
    return `shape of area ${this.area}`;
  }
}

export class Rect extends type(Shape) {
  /** @type {number} */
  #w;
  /** @type {number} */
  #h;
  /** @type {Rect} */
  static unit;

  /**
   * @param {{ w: number, h: number }} props
   * @returns {Rect}
   */
  static new(props) {
    return new Rect(props.w, props.h);
  }

  /**
   * @param {number} w
   * @param {number} h
   */
  constructor(w, h) {
    super();
    this.#w = w;
    this.#h = h;
  }

  /** @returns {number} */
  get w() {
    return this.#w;
  }

  /** @returns {number} */
  get h() {
    return this.#h;
  }

  /** @returns {number} */
  get area() {
    return Math.imul(this.#w, this.#h);
  }

  /** @returns {string} */
  toString() {
    return `Rect(${this.#w}, ${this.#h})`;
  }

  /**
   * @param {number} k
   * @returns {Rect}
   */
  scaled(k) {
    return new Rect(Math.imul(this.#w, k), Math.imul(this.#h, k));
  }
}

export class Square extends type(Shape) {
  /** @type {number} */
  #side;

  /** @param {number} side */
  constructor(side) {
    super();
    this.#side = side;
  }

  /** @returns {number} */
  get area() {
    return Math.imul(this.#side, this.#side);
  }

  /** @returns {string} */
  describe() {
    return `square of side ${this.#side}`;
  }
}

class Counter extends type() {
  /** @type {number} */
  #count;
  /** @type {number} */
  #hidden;

  constructor() {
    super();
    this.#count = 0;
    this.#hidden = 0;
  }

  /** @returns {number} */
  get count() {
    return this.#count;
  }

  /** @param {number} value */
  set count(value) {
    this.#count = value;
  }

  /** @returns {number} */
  get doubled() {
    return Math.imul(this.#count, 2);
  }

  /** @param {number} value */
  set doubled(value) {
    this.#count = (value - this.#count) | 0;
  }

  /** @returns {Counter} */
  bump() {
    this.#count = (this.#count + 1) | 0;
    this.#hidden = (this.#hidden + 1) | 0;
    return this;
  }

  /**
   * @param {Counter} other
   * @returns {boolean}
   */
  same(other) {
    return this.#hidden === other.#hidden;
  }

  /** @returns {Counter} */
  static zero() {
    return new Counter();
  }
}

class Point extends type() {
  /** @type {number} */
  #x;
  /** @type {number} */
  #y;

  /**
   * @param {{ x: number, y: number }} props
   * @returns {Point}
   */
  static new(props) {
    return new Point(props.x, props.y);
  }

  /**
   * @param {number} x
   * @param {number} y
   */
  constructor(x, y) {
    super();
    this.#x = x;
    this.#y = y;
  }

  /** @returns {number} */
  get x() {
    return this.#x;
  }

  /** @returns {number} */
  get y() {
    return this.#y;
  }

  /**
   * @param {Point} other
   * @returns {boolean}
   */
  eq(other) {
    if (this.#x === other.#x) {
      return this.#y === other.#y;
    } else {
      return false;
    }
  }
}

class Loose extends type() {
  /** @type {number} */
  #n;

  /** @param {number} n */
  constructor(n) {
    super();
    this.#n = n;
  }

  /** @returns {number} */
  get n() {
    return this.#n;
  }

  /**
   * @param {Loose} other
   * @returns {boolean}
   */
  eq(other) {
    if (this.#n === other.#n) {
      return true;
    } else {
      return this.#n === (-other.#n | 0);
    }
  }
}

class Holder extends type() {
  /** @type {Loose} */
  #loose;
  /** @type {string} */
  #tag;

  /**
   * @param {{ loose: Loose, tag: string }} props
   * @returns {Holder}
   */
  static new(props) {
    return new Holder(props.loose, props.tag);
  }

  /**
   * @param {Loose} loose
   * @param {string} tag
   */
  constructor(loose, tag) {
    super();
    this.#loose = loose;
    this.#tag = tag;
  }

  /** @returns {Loose} */
  get loose() {
    return this.#loose;
  }

  /** @returns {string} */
  get tag() {
    return this.#tag;
  }

  /**
   * @param {Holder} other
   * @returns {boolean}
   */
  eq(other) {
    if (this.#loose.eq(other.#loose)) {
      return this.#tag === other.#tag;
    } else {
      return false;
    }
  }
}

class Version extends type() {
  /** @type {number} */
  #major;
  /** @type {number} */
  #minor;

  /**
   * @param {{ major: number, minor: number }} props
   * @returns {Version}
   */
  static new(props) {
    return new Version(props.major, props.minor);
  }

  /**
   * @param {number} major
   * @param {number} minor
   */
  constructor(major, minor) {
    super();
    this.#major = major;
    this.#minor = minor;
  }

  /** @returns {number} */
  get major() {
    return this.#major;
  }

  /** @returns {number} */
  get minor() {
    return this.#minor;
  }

  /**
   * @param {Version} other
   * @returns {number}
   */
  compareTo(other) {
    if (this.#major !== other.#major) {
      return cmpInt32(this.#major, other.#major);
    } else {
      return cmpInt32(this.#minor, other.#minor);
    }
  }
}

Rect.unit = new Rect(1, 1);
const r = new Rect(2, 3);
console.log(r.toString());
console.log(r.area.toString());
const s = shape(/** @type {Shape} */ (/** @type {unknown} */ (r)));
console.log(s.describe());
const q = shape(new Square(4));
console.log(q.describe());
console.log(q.area.toString());
console.log(r.scaled(2).scaled(3).area.toString());
console.log(Rect.unit.scaled(5).toString());
const c = Counter.zero();
c.bump().bump();
console.log(c.count.toString());
c.doubled = 10;
console.log(c.doubled.toString());
console.log(c.same(new Counter().bump().bump()).toString());
console.log(`${new Point(1, 2).eq(new Point(1, 2))} ${!new Point(1, 2).eq(new Point(2, 1))}`);
console.log(new Holder(new Loose(3), "a").eq(new Holder(new Loose(-3), "a")).toString());
console.log(`${new Version(1, 2).compareTo(new Version(1, 3)) < 0} ${new Version(2, 0).compareTo(new Version(1, 9)) >= 0}`);
console.log(`${q instanceof Square} ${s instanceof Square}`);
const sq = requireInstanceOf(q, Square);
console.log(sq.area.toString());
if (s instanceof Rect) {
  console.log(/** @type {Rect} */ (/** @type {unknown} */ (s)).w.toString());
}
const favorite = same("bigint");
let named = 0;
{
  let i = 0;
  while (i < 6) {
    typeNamed(i);
    named = (named + 1) | 0;
    i = (i + 1) | 0;
  }
}
console.log(`${named} types`);
