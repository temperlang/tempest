import { type } from "@temperlang/core";

class C extends type() {
  /** @type {string} */
  #x;
  /** @type {string} */
  #y;

  /**
   * @param {{ x: string, y: string }} props
   * @returns {C}
   */
  static new(props) {
    return new C(props.x, props.y);
  }

  /**
   * @param {string} x
   * @param {string} y
   */
  constructor(x, y) {
    super();
    this.#x = x;
    this.#y = y;
  }

  /** @returns {string} */
  get x() {
    return this.#x;
  }

  /** @returns {string} */
  get y() {
    return this.#y;
  }

  /** @param {string} input */
  echo(input) {
    console.log(input);
  }
}

const c = new C("Hello", "World");
console.log(`${c.x}, ${c.y}!`);
c.echo("Hello World, again!");
