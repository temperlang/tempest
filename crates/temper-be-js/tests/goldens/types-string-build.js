import {
  stringBuilderAppendCodePoint,
  stringStep,
  type,
} from "@temperlang/core";

class StringBuilderOwner extends type() {
  /** @type {string[]} */
  #builder;

  /** @param {string[]} builder */
  constructor(builder) {
    super();
    this.#builder = builder;
  }

  /** @returns {string[]} */
  get builder() {
    return this.#builder;
  }
}

const builder = [""];
builder[0] += "Hi ";
stringBuilderAppendCodePoint(builder, 127759);
const source = "Wow!! Neato!";
const bangIndex = stringStep(source, 0, 3);
const spaceIndex = stringStep(source, bangIndex, 2);
builder[0] += source.substring(bangIndex, spaceIndex);
const built = builder[0];
builder[0] += "\n";
builder[0] += built;
console.log(builder[0]);
try {
  stringBuilderAppendCodePoint(builder, 1114112);
} catch {
  console.log("High code blocked.");
}
try {
  stringBuilderAppendCodePoint(builder, 55296);
} catch {
  console.log("Surrogate blocked.");
}
console.log(builder[0]);
builder[0] = "";
builder[0] += "Back to square zero.";
console.log(builder[0]);
const owner = new StringBuilderOwner([""]);
owner.builder[0] += "Hi from inside.";
console.log(owner.builder[0]);
