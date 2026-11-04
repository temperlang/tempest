import {
  listBuilderAdd,
  listBuilderRemoveLast,
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
}

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
let t$0;
try {
  t$0 = mappedGet(m, "b");
} catch {
  t$0 = -1;
}
let t$1;
try {
  t$1 = mappedGet(m, "c");
} catch {
  t$1 = -1;
}
console.log(`${t$0} ${t$1} ${m.has("a")} ${m.size}`);
/** @type {Map<string, number | null>} */
const mb = new Map();
mb.set("q", 5);
mb.set("n", null);
console.log(`${(mb.has("q") ? /** @type {number | null} */ (mb.get("q")) : 0) ?? -2} ${(mb.has("n") ? /** @type {number | null} */ (mb.get("n")) : 0) ?? -2} ${mb.has("n")}`);
let t$2;
{
  /** @type {number | null} */
  let subject6;
  try {
    subject6 = mapBuilderRemove(mb, "q");
  } catch {
    subject6 = 0;
  }
  t$2 = subject6 == null ? null : subject6.toString();
}
console.log(t$2 ?? "null");
console.log(Object.freeze(Array.from(mb.keys())).length.toString());
console.log(`${firstSame(Object.freeze([1000]), Object.freeze([1000]))} ${firstSame(Object.freeze([1000]), Object.freeze([7]))}`);
