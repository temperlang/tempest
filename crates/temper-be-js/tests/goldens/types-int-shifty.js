import {
  bitwiseShl64,
  bitwiseShr64,
  bitwiseShrUnsigned32,
  bitwiseShrUnsigned64,
  modIntInt,
  stringCountBetween,
  stringGet,
  stringNext,
  stringStep,
} from "@temperlang/core";

/**
 * @param {string} hexString
 * @returns {string}
 */
function insertUnderscores(hexString) {
  const sb = [""];
  let idx = 0;
  const end = hexString.length;
  if (idx - end < 0) {
    const possibleSign = stringGet(hexString, idx);
    if (possibleSign === 43 || possibleSign === 45) {
      idx = stringNext(hexString, idx);
      sb[0] += hexString.substring(0, idx);
    }
  }
  const nFirstGroup = modIntInt(stringCountBetween(hexString, idx, end), 4);
  let needsUnderscore = false;
  if (nFirstGroup !== 0) {
    const groupStart = idx;
    {
      let i = 0;
      while (i < nFirstGroup) {
        idx = stringNext(hexString, idx);
        i = (i + 1) | 0;
      }
    }
    sb[0] += hexString.substring(groupStart, idx);
    needsUnderscore = true;
  }
  while (idx - end < 0) {
    const groupStart = idx;
    idx = stringStep(hexString, idx, 4);
    if (needsUnderscore) {
      sb[0] += "_";
    }
    sb[0] += hexString.substring(groupStart, idx);
    needsUnderscore = true;
  }
  return sb[0];
}

/**
 * @param {number} n
 * @returns {string}
 */
function fmt32(n) {
  return insertUnderscores(n.toString(16));
}

/**
 * @param {bigint} n
 * @returns {string}
 */
function fmt64(n) {
  return insertUnderscores(n.toString(16));
}

/**
 * @param {number} a
 * @param {number} b
 */
function and32(a, b) {
  const c = a & b;
  console.log(`0x:${fmt32(a)} & ${fmt32(b)} == ${fmt32(c)}`);
}

/**
 * @param {bigint} a
 * @param {bigint} b
 */
function and64(a, b) {
  const c = a & b;
  console.log(`0x:${fmt64(a)} & ${fmt64(b)} == ${fmt64(c)}`);
}

/**
 * @param {number} a
 * @param {number} b
 */
function or32(a, b) {
  const c = a | b;
  console.log(`0x:${fmt32(a)} | ${fmt32(b)} == ${fmt32(c)}`);
}

/**
 * @param {bigint} a
 * @param {bigint} b
 */
function or64(a, b) {
  const c = a | b;
  console.log(`0x:${fmt64(a)} | ${fmt64(b)} == ${fmt64(c)}`);
}

/** @param {number} n */
function bComp32(n) {
  console.log(`0x:~${fmt32(n)} == ${fmt32(~n)}`);
}

/** @param {bigint} n */
function bComp64(n) {
  console.log(`0x:~${fmt64(n)} == ${fmt64(~n)}`);
}

/**
 * @param {number} a
 * @param {number} b
 */
function shl32(a, b) {
  const c = a << b;
  console.log(`0x:${fmt32(a)} << ${b} == ${fmt32(c)}`);
}

/**
 * @param {bigint} a
 * @param {number} b
 */
function shl64(a, b) {
  const c = bitwiseShl64(a, b);
  console.log(`0x:${fmt64(a)} << ${b} == ${fmt64(c)}`);
}

/**
 * @param {number} a
 * @param {number} b
 */
function shr32(a, b) {
  const c = a >> b;
  console.log(`0x:${fmt32(a)} >> ${b} == ${fmt32(c)}`);
}

/**
 * @param {bigint} a
 * @param {number} b
 */
function shr64(a, b) {
  const c = bitwiseShr64(a, b);
  console.log(`0x:${fmt64(a)} >> ${b} == ${fmt64(c)}`);
}

/**
 * @param {number} a
 * @param {number} b
 */
function uShr32(a, b) {
  const c = bitwiseShrUnsigned32(a, b);
  console.log(`0x:${fmt32(a)} >>> ${b} == ${fmt32(c)}`);
}

/**
 * @param {bigint} a
 * @param {number} b
 */
function uShr64(a, b) {
  const c = bitwiseShrUnsigned64(a, b);
  console.log(`0x:${fmt64(a)} >>> ${b} == ${fmt64(c)}`);
}

console.log("and32");
and32(0, 0);
and32(3, 1);
and32(2, 3);
and32(-2, 7);
and32(-809434545, 901070595);
console.log("and64");
and64(0n, 0n);
and64(3n, 1n);
and64(2n, 3n);
and64(-2n, 7n);
and64(297421889964898413n, -3505189831424144594n);
console.log("or32");
or32(0, 0);
or32(0, 1);
or32(2, 1);
or32(2, 2);
or32(3, 1);
or32(2, 3);
or32(-2, 7);
or32(-809434545, 901070595);
console.log("or64");
or64(0n, 0n);
or64(0n, 1n);
or64(2n, 1n);
or64(2n, 2n);
or64(3n, 1n);
or64(2n, 3n);
or64(-2n, 7n);
or64(297421889964898413n, -3505189831424144594n);
console.log("bComp32");
bComp32(0);
bComp32(1);
bComp32(-1);
bComp32(2);
bComp32(-2);
bComp32(-809434545);
bComp32(901070595);
console.log("bComp64");
bComp64(0n);
bComp64(1n);
bComp64(-1n);
bComp64(2n);
bComp64(-2n);
bComp64(297421889964898413n);
bComp64(-3505189831424144594n);
console.log("shl32");
shl32(5, 0);
shl32(5, 1);
shl32(5, 2);
shl32(5, 3);
shl32(5, 4);
shl32(5, 5);
shl32(5, 20);
shl32(5, 28);
shl32(5, 29);
shl32(5, 30);
shl32(5, 31);
shl32(5, 32);
shl32(5, 33);
shl32(5, -1);
shl32(5, -30);
shl32(-1, 0);
shl32(-1, 1);
shl32(-1, 2);
console.log("shl64");
shl64(5n, 0);
shl64(5n, 1);
shl64(5n, 2);
shl64(5n, 3);
shl64(5n, 4);
shl64(5n, 5);
shl64(5n, 33);
shl64(5n, 62);
shl64(5n, 63);
shl64(5n, 64);
shl64(5n, -1);
shl64(5n, -60);
shl64(-1n, 0);
shl64(-1n, 1);
shl64(-1n, 2);
console.log("shr32");
shr32(128, 0);
shr32(128, 1);
shr32(128, 2);
shr32(128, 3);
shr32(128, 4);
shr32(128, 5);
shr32(128, 6);
shr32(128, 7);
shr32(128, 8);
shr32(128, 9);
shr32(128, 31);
shr32(128, 32);
shr32(128, 33);
shr32(128, -1);
shr32(128, -30);
shr32(-128, 0);
shr32(-128, 1);
shr32(-128, 2);
shr32(-128, 3);
shr32(-128, 4);
console.log("shr64");
shr64(128n, 0);
shr64(128n, 1);
shr64(128n, 2);
shr64(128n, 3);
shr64(128n, 4);
shr64(128n, 5);
shr64(128n, 6);
shr64(128n, 7);
shr64(128n, 8);
shr64(128n, 9);
shr64(128n, 32);
shr64(128n, 63);
shr64(128n, 64);
shr64(128n, 65);
shr64(128n, -1);
shr64(128n, -62);
shr64(-128n, 0);
shr64(-128n, 1);
shr64(-128n, 2);
shr64(-128n, 3);
shr64(-128n, 4);
console.log("uShr32");
uShr32(128, 0);
uShr32(128, 1);
uShr32(128, 2);
uShr32(128, 3);
uShr32(128, 4);
uShr32(128, 5);
uShr32(128, 6);
uShr32(128, 7);
uShr32(128, 8);
uShr32(128, 9);
uShr32(128, 31);
uShr32(128, 32);
uShr32(128, 33);
uShr32(128, -1);
uShr32(128, -30);
uShr32(-128, 0);
uShr32(-128, 1);
uShr32(-128, 2);
uShr32(-128, 3);
uShr32(-128, 4);
console.log("uShr64");
uShr64(128n, 0);
uShr64(128n, 1);
uShr64(128n, 2);
uShr64(128n, 3);
uShr64(128n, 4);
uShr64(128n, 5);
uShr64(128n, 6);
uShr64(128n, 7);
uShr64(128n, 8);
uShr64(128n, 9);
uShr64(128n, 32);
uShr64(128n, 63);
uShr64(128n, 64);
uShr64(128n, 65);
uShr64(128n, -1);
uShr64(128n, -62);
uShr64(-128n, 0);
uShr64(-128n, 1);
uShr64(-128n, 2);
uShr64(-128n, 3);
uShr64(-128n, 4);
