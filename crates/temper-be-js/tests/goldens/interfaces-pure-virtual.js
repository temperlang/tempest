import { type } from "@temperlang/core";

class Greeter extends type() {
  /** @type {() => void} */
  greet = /** @type {any} */ (undefined);
}

class HiGreeter extends type(Greeter) {
  greet() {
    console.log("Hi, virtually!");
  }
}

const greeter = new HiGreeter();
greeter.greet();
