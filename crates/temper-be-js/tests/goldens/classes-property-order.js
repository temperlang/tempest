import { type } from "@temperlang/core";

class C extends type() {
  /** @type {number} */
  #backed;

  /** @param {number | null} [backed] */
  constructor(backed) {
    super();
    if (backed == null) {
      let t$0;
      {
        console.log("initializing backed");
        t$0 = 1;
      }
      backed = t$0;
    }
    this.#backed = backed;
  }

  /** @returns {number} */
  get backed() {
    return this.#backed;
  }

  /** @returns {number} */
  get computed() {
    console.log("computing computed");
    return 1;
  }
}

console.log("about to create instance");
const c = new C();
console.log("about to get backed");
console.log(`c.backed=${c.backed}`);
console.log("about to get computed");
console.log(`c.computed=${c.computed}`);
console.log("done");
