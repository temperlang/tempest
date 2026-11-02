import * as _connected from "./_connected.js";

export const Counter = _connected.Counter;
export const Doubler = _connected.Doubler;
export const counter = new Counter();
counter.add(2);
counter.add(3);
console.log(`counter.total(): ${counter.total()}`);
console.log(`new Doubler().twice(5): ${new Doubler().twice(5)}`);
