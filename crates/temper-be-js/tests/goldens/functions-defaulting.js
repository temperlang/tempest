import { stringCountBetween, type } from "@temperlang/core";

/** @returns {number} */
function f() {
  fCalls = (fCalls + 1) | 0;
  return 42;
}

/**
 * @param {number | null} [x]
 * @returns {number}
 */
function g(x) {
  x ??= f();
  return x;
}

/** @returns {number | null} */
function noInline() {
  console.log("noInline");
  return null;
}

/**
 * @param {StringHolder | null} a
 * @returns {string}
 */
function maybeEmpty(a) {
  inked = (inked + 1) | 0;
  {
    let t$0;
    {
      let t$1;
      {
        const subject3 = a == null ? null : a.string;
        t$1 = subject3 == null ? null : !subject3;
      }
      const subject2 = t$1;
      t$0 = subject2 == null ? null : subject2.toString();
    }
    const subject = t$0;
    if (subject == null) {
      return "null";
    } else {
      return subject;
    }
  }
}

/**
 * @param {string | null} a
 * @returns {number | null}
 */
function maybeLength(a) {
  inked = (inked + 1) | 0;
  {
    if (a == null) {
      return null;
    } else {
      return stringCountBetween(a, 0, a.length);
    }
  }
}

/** @returns {number | null} */
function maybeInc() {
  if (inked === 10) {
    return null;
  } else {
    inked = (inked + 1) | 0;
    return inked;
  }
}

/**
 * @param {string | null} a
 * @returns {string}
 */
function chainMore(a) {
  let t$2;
  {
    const subject2 = maybeLength(a);
    t$2 = subject2 == null ? null : Math.max(subject2, 0);
  }
  return (t$2 ?? -1).toString();
}

class Thing extends type() {
  /**
   * @param {string | null} [a]
   * @param {string | null} [b]
   */
  say(a, b) {
    a ??= "";
    b ??= "";
    console.log(`saying ${a}${b}`);
  }

  /**
   * @param {string} a
   * @param {string | null} [b]
   */
  static sayStatic(a, b) {
    b ??= "";
    console.log(`static ${a}${b}`);
  }
}

class StringHolder extends type() {
  /** @type {string} */
  #string;

  /** @param {string} string */
  constructor(string) {
    super();
    this.#string = string;
  }

  /** @returns {string} */
  get string() {
    return this.#string;
  }
}

let fCalls = 0;
console.log(`fCalls=${fCalls}`);
console.log(`g(   )=${g()}`);
console.log(`fCalls=${fCalls}`);
console.log(`g(123)=${g(123)}`);
console.log(`fCalls=${fCalls}`);
console.log(`g(null)=${g(noInline())}`);
console.log(`fCalls=${fCalls}`);
new Thing().say("hi");
new Thing().say(null, "there");
new Thing().say("by", "e");
Thing.sayStatic("yall");
let inked = 0;
console.log(`maybeInc() -> ${maybeInc() ?? -1}`);
console.log(`inked: ${inked}`);
console.log(`lengthish "ab": ${chainMore("ab")}`);
console.log(`lengthish null: ${chainMore(null)}`);
console.log(`inked: ${inked}`);
console.log(`maybeEmpty "": ${maybeEmpty(new StringHolder(""))}`);
console.log(`maybeEmpty null: ${maybeEmpty(null)}`);
console.log(`inked: ${inked}`);
