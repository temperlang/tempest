import { eqFloat64 } from "@temperlang/core";
import {
  TestHelper,
  Vec2,
  assertMoreThings,
  normSquared,
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
