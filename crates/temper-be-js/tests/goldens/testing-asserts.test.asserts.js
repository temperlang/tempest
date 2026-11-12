import { eqFloat64, float64ToString, listedGet } from "@temperlang/core";
import {
  TestHelper,
  Vec2,
  assertMoreThings,
  normSquared,
  unitVecs,
} from "../asserts.internal.js";
import { Test } from "@temperlang/std/testing";

it("a test case", function () {
  const test = new Test();
  try {
    test.assert(eqFloat64(normSquared(new Vec2(0.0, 0.0)), 0.0), () => "Oh noes");
  } finally {
    test.softFailToHard();
  }
});

it("a failing test", function () {
  const test = new Test();
  try {
    test.assert(false, () => "because it's wrong");
    const helper = new TestHelper(3, 2);
    assertMoreThings(test, helper, Object.freeze([Object.freeze(["bye"])]));
  } finally {
    test.softFailToHard();
  }
});

it("exported", function () {
  const test = new Test();
  try {
    const vec = new Vec2(1.5, 2.5);
    {
      const actual = vec.toString();
      const expected = "(1.5, 2.5) size squared 8.5";
      test.assert(actual === expected, () => `expected vec.toString() == (${expected}) not (${actual})`);
    }
  } finally {
    test.softFailToHard();
  }
});

it("module values", function () {
  const test = new Test();
  try {
    {
      const length = unitVecs.length;
      let i = 0;
      while (i < length) {
        const vec = listedGet(/** @type {Vec2[]} */ (unitVecs), i);
        i = (i + 1) | 0;
        {
          const actual = normSquared(vec);
          const expected = 1.0;
          test.assert(eqFloat64(actual, expected), () => `expected normSquared(vec) == (${float64ToString(expected)}) not (${float64ToString(actual)})`);
        }
      }
    }
  } finally {
    test.softFailToHard();
  }
});
