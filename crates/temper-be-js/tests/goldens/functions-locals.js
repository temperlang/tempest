import {
  listedFilter,
  listedGet,
  listedJoin,
  listedMap,
  type,
} from "@temperlang/core";

/**
 * @param {readonly number[]} x
 * @returns {string}
 */
function show(x) {
  /** @type {string[]} */
  const s = [];
  s.push("[");
  let c = "";
  {
    let i = 0;
    while (i < x.length) {
      s.push(c);
      s.push(listedGet(/** @type {number[]} */ (x), i).toString());
      c = ", ";
      i = (i + 1) | 0;
    }
  }
  s.push("]");
  return listedJoin(s, "", (v) => v);
}

/**
 * @template T
 * @param {() => T} b
 * @returns {T}
 */
function nestBlock(b) {
  return b();
}

/** @param {() => void} b */
function nestBlockVoid(b) {
  b();
}

/**
 * @param {readonly number[]} x
 * @param {number} a
 */
function constantCapture(x, a) {
  const b = Math.imul(a, 1);
  /** @type {readonly number[]} */
  const out = listedMap(/** @type {number[]} */ (x), (n) => (n + b) | 0);
  console.log(`constantCapture = ${show(out)}`);
}

/**
 * @param {readonly number[]} x
 * @param {number} a
 */
function constantCaptureNested(x, a) {
  const b = Math.imul(a, 1);
  /** @type {readonly number[]} */
  const out = nestBlock(() => listedMap(/** @type {number[]} */ (x), (n) => (n + b) | 0));
  console.log(`constantCaptureNested = ${show(out)}`);
}

/** @param {readonly number[]} x */
function mutableCapture(x) {
  let sum = 0;
  /**
   * @param {number} n
   * @returns {number}
   */
  function fn(n) {
    sum = (sum + n) | 0;
    return sum;
  }
  /** @type {readonly number[]} */
  const out = listedMap(/** @type {number[]} */ (x), fn);
  console.log(`mutableCapture = ${show(out)}; sum=${sum}`);
}

/** @param {readonly number[]} x */
function mutableCaptureNested(x) {
  let sum = 0;
  /** @returns {readonly number[]} */
  function fn() {
    /**
     * @param {number} n
     * @returns {number}
     */
    function fn2(n) {
      sum = (sum + n) | 0;
      return sum;
    }
    return listedMap(/** @type {number[]} */ (x), fn2);
  }
  /** @type {readonly number[]} */
  const out = nestBlock(fn);
  console.log(`mutableCaptureNested = ${show(out)}; sum=${sum}`);
}

/**
 * @param {readonly number[]} values
 * @param {number} value
 */
export function thresh(values, value) {
  value = (value + 1) | 0;
  /** @type {readonly number[]} */
  const result = listedFilter(/** @type {number[]} */ (values), (it) => it > value);
  console.log(`thresh length = ${result.length}`);
}

/**
 * @param {readonly number[]} values
 * @param {number} value
 */
export function thresh2(values, value) {
  let value2 = value;
  value2 = (value2 + 1) | 0;
  /** @type {readonly number[]} */
  const result = listedFilter(/** @type {number[]} */ (values), (it) => it > value2);
  console.log(`thresh2 length = ${result.length}`);
}

/** @param {readonly number[]} x */
function selfRecursion(x) {
  /**
   * @param {number} i
   * @param {number[]} out
   */
  function mapper(i, out) {
    if (i < x.length) {
      out.push(Math.imul(listedGet(/** @type {number[]} */ (x), i), 2));
      mapper((i + 1) | 0, out);
    }
  }
  /** @type {number[]} */
  const out2 = [];
  mapper(0, out2);
  console.log(`selfRecursion = ${show(out2)}`);
}

/** @param {readonly number[]} x */
function selfRecursionNested(x) {
  /**
   * @param {number} i
   * @param {number[]} out
   */
  function mapper(i, out) {
    function fn2() {
      if (i < x.length) {
        out.push(Math.imul(listedGet(/** @type {number[]} */ (x), i), 2));
        mapper((i + 1) | 0, out);
      }
    }
    nestBlockVoid(fn2);
  }
  /** @type {number[]} */
  const out2 = [];
  mapper(0, out2);
  console.log(`selfRecursionNested = ${show(out2)}`);
}

/** @param {readonly number[]} x */
function mutualRecursion(x) {
  /**
   * @param {number} i
   * @param {number[]} out
   */
  function walker(i, out) {
    if (i < x.length) {
      putter(i, out);
    }
  }
  /**
   * @param {number} i2
   * @param {number[]} out2
   */
  function putter(i2, out2) {
    out2.push(Math.imul(listedGet(/** @type {number[]} */ (x), i2), 2));
    walker((i2 + 1) | 0, out2);
  }
  /** @type {number[]} */
  const out3 = [];
  walker(0, out3);
  console.log(`mutualRecursion = ${show(out3)}`);
}

/**
 * @param {Hub<number>} hub
 * @returns {number[]}
 */
function gatherHandlers(hub) {
  /** @type {number[]} */
  const items = [];
  {
    let i = 0;
    while (i < 3) {
      const i2 = i;
      /** @param {number} item */
      function fn(item) {
        items.push((item + i2) | 0);
      }
      hub.onAction(fn);
      i = (i + 1) | 0;
    }
  }
  return items;
}

/** @template T */
class Hub extends type() {
  /** @type {((arg0: T) => void)[]} */
  #handlers;

  constructor() {
    super();
    this.#handlers = [];
  }

  /** @param {(arg0: T) => void} handler */
  onAction(handler) {
    this.#handlers.push(handler);
  }

  /** @param {T} item */
  actionPerformed(item) {
    {
      let i = 0;
      while (i < this.#handlers.length) {
        listedGet(this.#handlers, i)(item);
        i = (i + 1) | 0;
      }
    }
  }
}

constantCapture(Object.freeze([1, 2, 3]), 1);
constantCaptureNested(Object.freeze([1, 2, 3]), 1);
mutableCapture(Object.freeze([1, 2, 3]));
mutableCaptureNested(Object.freeze([1, 2, 3]));
thresh(Object.freeze([4, 5, 6]), 4);
thresh2(Object.freeze([4, 5, 6]), 4);
selfRecursion(Object.freeze([1, 2, 3]));
selfRecursionNested(Object.freeze([1, 2, 3]));
mutualRecursion(Object.freeze([1, 2, 3]));
/** @type {Hub<number>} */
const hub = new Hub();
/** @type {number[]} */
const items = gatherHandlers(hub);
hub.actionPerformed(1);
