import { type } from "@temperlang/core";

class C extends type() {
  /** @returns {string} */
  get place() {
    return "Hilo, HI";
  }
}

const c = new C();
console.log(`Hello, ${c.place}!`);
