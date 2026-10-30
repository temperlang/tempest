import { type } from "@temperlang/core";

/** @param {string} message */
function say(message) {
  console.log(message);
}

class Something extends type() {
  /** @type {string} */
  #message;

  /** @param {string} message */
  constructor(message) {
    super();
    this.#message = message;
  }

  sayMessage() {
    say(this.#message);
  }
}

const something = new Something("Hello, World!");
something.sayMessage();
