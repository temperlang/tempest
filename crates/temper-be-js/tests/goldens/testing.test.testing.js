import { base, runs, runs$set, twice } from "../testing.internal.js";
import { Test } from "@temperlang/std/testing";

it("twice works", function () {
  const test = new Test();
  try {
    const x = twice(2);
    runs$set((((runs + x) | 0) + base) | 0);
  } finally {
    test.softFailToHard();
  }
});

it("names: punctuation, 'quotes', and ünïcode", function () {
  const t = new Test();
  try {
    twice(runs);
  } finally {
    t.softFailToHard();
  }
});
