import {
  listBuilderAdd,
  listBuilderRemoveLast,
  listBuilderSet,
  listBuilderToList,
  listedGet,
  listedGetOr,
  listedSlice,
  mapBuilderRemove,
  mapConstructor,
  mappedGet,
  pairConstructor,
  type,
} from "@temperlang/core";

/**
 * @template S
 * @template T
 * @param {T} x
 * @returns {T}
 */
function tagged(x) {
  return x;
}

/**
 * @template T
 * @param {number} n
 * @returns {number}
 */
function countOf(n) {
  /** @type {T[]} */
  const b = [];
  return (b.length + n) | 0;
}

/**
 * @template {Scene} S
 * @template T
 * @param {T} x
 * @returns {T}
 */
function relayed(x) {
  return tagged(x);
}

/**
 * @template T
 * @param {readonly T[]} items
 * @param {T} fallback
 * @returns {T}
 */
export function first(items, fallback) {
  if (!items.length) {
    return fallback;
  } else {
    return listedGet(/** @type {T[]} */ (items), 0);
  }
}

/**
 * @template T
 * @param {Holder<T>} h
 * @returns {T}
 */
function held(h) {
  return h.held;
}

/**
 * @param {readonly number[]} ns
 * @returns {number}
 */
function total(ns) {
  let sum = 0;
  {
    let i = 0;
    while (i < ns.length) {
      sum = (sum + listedGet(/** @type {number[]} */ (ns), i)) | 0;
      i = (i + 1) | 0;
    }
  }
  return sum;
}

function indexed() {
  /** @type {number[]} */
  const counts = [];
  counts.push(1);
  counts.push(10);
  let at = 0;
  {
    const k = at;
    listBuilderSet(counts, k, (listedGet(counts, k) + 4) | 0);
  }
  {
    listBuilderSet(counts, 1, Math.imul(listedGet(counts, 1), 3));
  }
  /** @type {number} */
  let t$0;
  {
    /** @type {number} */
    let t$1;
    {
      const t = at;
      at = (t + 1) | 0;
      t$1 = t;
    }
    const k2 = t$1;
    const t2 = listedGet(counts, k2);
    listBuilderSet(counts, k2, (t2 + 1) | 0);
    t$0 = t2;
  }
  const was = t$0;
  /** @type {number} */
  let t$2;
  {
    const k3 = at;
    const t3 = (listedGet(counts, k3) + 1) | 0;
    listBuilderSet(counts, k3, t3);
    t$2 = t3;
  }
  const pre = t$2;
  /** @type {number} */
  let t$3;
  {
    const t4 = listedGet(counts, 0);
    listBuilderSet(counts, 0, (t4 - 1) | 0);
    t$3 = t4;
  }
  const post = t$3;
  /** @type {number} */
  let t$4;
  {
    const t5 = (listedGet(counts, 0) - 1) | 0;
    listBuilderSet(counts, 0, t5);
    t$4 = t5;
  }
  const down = t$4;
  console.log(`${was} ${pre} ${post} ${down} ${at}`);
}

/**
 * @param {string} key
 * @returns {number}
 */
function bump(key) {
  {
    tally.set(key, (mappedGet(tally, key) - 1) | 0);
  }
  /** @type {number} */
  let t$5;
  {
    const t = mappedGet(tally, key);
    tally.set(key, (t + 1) | 0);
    t$5 = t;
  }
  const before = t$5;
  /** @type {number} */
  let t$6;
  {
    const t2 = (mappedGet(tally, key) + 1) | 0;
    tally.set(key, t2);
    t$6 = t2;
  }
  const after = t$6;
  return (before + after) | 0;
}

/**
 * @param {readonly number[]} a
 * @param {readonly number[]} b
 * @returns {boolean}
 */
function firstSame(a, b) {
  return listedGet(/** @type {number[]} */ (a), 0) === listedGet(/** @type {number[]} */ (b), 0);
}

/** @template T */
export class Holder extends type() {
  /** @type {T} */
  held = /** @type {any} */ (undefined);
}

/** @template T */
export class Box extends type(Holder) {
  /** @type {T} */
  #value;

  /** @param {T} value */
  constructor(value) {
    super();
    this.#value = value;
  }

  /** @returns {T} */
  get value() {
    return this.#value;
  }

  /** @returns {T} */
  get held() {
    return this.#value;
  }

  /**
   * @param {number} other
   * @returns {import("@temperlang/core/pair.js").Pair<T, number>}
   */
  pairWith(other) {
    return pairConstructor(this.#value, other);
  }
}

export class Named extends type() {
  /** @type {string} */
  #name;

  /** @param {string} name */
  constructor(name) {
    super();
    this.#name = name;
  }

  /** @returns {string} */
  get name() {
    return this.#name;
  }
}

/** @template {Named} T */
export class Shelf extends type() {
  /** @type {readonly T[]} */
  #items;

  /** @param {readonly T[]} items */
  constructor(items) {
    super();
    this.#items = items;
  }

  /** @returns {readonly T[]} */
  get items() {
    return this.#items;
  }

  /** @returns {string | null} */
  firstName() {
    if (!this.#items.length) {
      return null;
    } else {
      return listedGet(/** @type {T[]} */ (this.#items), 0).name;
    }
  }

  /**
   * @template TName
   * @param {number} extra
   * @returns {number}
   */
  tally(extra) {
    return (this.#items.length + extra) | 0;
  }
}

export class Scene extends type() {}

export class Stage extends type(Scene) {}

/** @type {readonly number[]} */
const ns = Object.freeze([1, 2, 3]);
/** @type {readonly number[]} */
const none = Object.freeze([]);
console.log(`${total(ns)} ${first(ns, 0)} ${first(none, -1)}`);
console.log(first(Object.freeze(["a", "b"]), "z"));
/** @type {Box<string>} */
const box = new Box("boxed");
console.log(held(box));
/** @type {import("@temperlang/core/pair.js").Pair<string, number>} */
const pair = box.pairWith(7);
console.log(`${pair.key} ${pair.value}`);
/** @type {Shelf<Named>} */
const shelf = new Shelf(Object.freeze([new Named("ann"), new Named("bo")]));
console.log(shelf.firstName() ?? "none");
/** @type {readonly Named[]} */
const noNames = Object.freeze([]);
/** @type {Shelf<Named>} */
const empty_ = new Shelf(noNames);
console.log(empty_.firstName() ?? "none");
const n = (shelf.tally(1) + shelf.tally(2)) | 0;
console.log(`${tagged(4)} ${tagged("t")} ${countOf(3)} ${relayed(5)} ${n}`);
/** @type {string[]} */
const builder = [];
builder.push("x");
builder.push("y");
listBuilderAdd(builder, "w", 0);
console.log(`${builder.length} ${listedGet(builder, 0)} ${listBuilderRemoveLast(builder)}`);
/** @type {readonly string[]} */
const built = listBuilderToList(builder);
console.log(`${built.length} ${listedGetOr(/** @type {string[]} */ (built), 5, "?")} ${listedGet(/** @type {string[]} */ (listedSlice(/** @type {string[]} */ (built), 1, 2)), 0)}`);
/** @type {Map<string, number>} */
const m = mapConstructor(/** @type {any} */ (Object.freeze([pairConstructor("a", 1), pairConstructor("b", 2)])));
/** @type {number} */
let t$7;
try {
  t$7 = mappedGet(m, "b");
} catch {
  t$7 = -1;
}
/** @type {number} */
let t$8;
try {
  t$8 = mappedGet(m, "c");
} catch {
  t$8 = -1;
}
console.log(`${t$7} ${t$8} ${m.has("a")} ${m.size}`);
/** @type {Map<string, number | null>} */
const mb = new Map();
mb.set("q", 5);
mb.set("n", null);
console.log(`${(mb.has("q") ? /** @type {number | null} */ (mb.get("q")) : 0) ?? -2} ${(mb.has("n") ? /** @type {number | null} */ (mb.get("n")) : 0) ?? -2} ${mb.has("n")}`);
/** @type {string | null} */
let t$9;
{
  /** @type {number | null} */
  let subject6;
  try {
    subject6 = mapBuilderRemove(mb, "q");
  } catch {
    subject6 = 0;
  }
  t$9 = subject6 == null ? null : subject6.toString();
}
console.log(t$9 ?? "null");
console.log(Object.freeze(Array.from(mb.keys())).length.toString());
indexed();
/** @type {Map<string, number>} */
const tally = new Map();
tally.set("k", 2);
/** @type {number} */
let t$10;
try {
  t$10 = bump("k");
} catch {
  t$10 = -1;
}
/** @type {number} */
let t$11;
try {
  t$11 = bump("z");
} catch {
  t$11 = -1;
}
/** @type {number} */
let t$12;
try {
  t$12 = mappedGet(tally, "k");
} catch {
  t$12 = -1;
}
console.log(`${t$10} ${t$11} ${t$12}`);
console.log(`${firstSame(Object.freeze([1000]), Object.freeze([1000]))} ${firstSame(Object.freeze([1000]), Object.freeze([7]))}`);
