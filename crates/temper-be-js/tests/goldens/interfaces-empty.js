import { requireInstanceOf, type } from "@temperlang/core";

class Greeter extends type() {}

class HiGreeter extends type(Greeter) {
  greet() {
    console.log("Hi!");
  }
}

const greeter = new HiGreeter();
requireInstanceOf(greeter, HiGreeter).greet();
