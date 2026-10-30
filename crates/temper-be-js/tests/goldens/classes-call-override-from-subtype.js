import { type } from "@temperlang/core";

/**
 * @param {Slippy} slipster
 * @returns {boolean}
 */
function checkFromOutside(slipster) {
  return slipster.thingums === 5;
}

class Slappy extends type() {
  /** @type {number} */
  thingums = /** @type {any} */ (undefined);
}

class Slippy extends type(Slappy) {
  /** @returns {boolean} */
  checkOverridden() {
    return this.thingums === 1;
  }

  /** @returns {boolean} */
  checkOwn() {
    return this.thingumses === 2;
  }

  /** @returns {number} */
  get thingums() {
    return 3;
  }

  /** @returns {number} */
  get thingumses() {
    return 4;
  }
}

class A extends type() {
  /** @returns {string} */
  a() {
    return "A wins!";
  }

  /** @returns {string} */
  get thing() {
    return "A has thing!";
  }
}

class B extends type(A) {}

class C extends type(A) {
  /** @returns {string} */
  a() {
    return "C wins!";
  }

  /** @returns {string} */
  get thing() {
    return "C has thing!";
  }
}

class D extends type(B, C) {
  /** @returns {string} */
  a() {
    return C.prototype.a.call(this);
  }

  /** @returns {string} */
  get thing() {
    return Reflect.get(C.prototype, "thing", this);
  }
}

class ASplit extends type() {
  /** @returns {string} */
  a() {
    return "A wins!";
  }
}

class BSplit extends type(ASplit) {}

class CSplit extends type() {
  /** @returns {string} */
  a() {
    return "C wins!";
  }
}

class DSplit extends type(BSplit, CSplit) {
  /** @returns {string} */
  a() {
    return CSplit.prototype.a.call(this);
  }
}

class ADeep extends type() {
  /** @returns {string} */
  a() {
    return "A wins!";
  }
}

class B1Deep extends type(ADeep) {
  /** @returns {string} */
  a() {
    return "B1 wins!";
  }
}

class B2Deep extends type(B1Deep) {}

class B3Deep extends type(B2Deep) {}

class CDeep extends type(ADeep) {}

class DDeep extends type(B2Deep, CDeep) {
  /** @returns {string} */
  a() {
    return B1Deep.prototype.a.call(this);
  }
}

class EDeep extends type(CDeep, B2Deep) {
  /** @returns {string} */
  a() {
    return ADeep.prototype.a.call(this);
  }
}

const slippy = new Slippy();
const slappy = slippy;
console.log(slippy.thingums.toString());
console.log(slappy.thingums.toString());
console.log(slippy.thingumses.toString());
console.log(slippy.checkOverridden().toString());
console.log(slippy.checkOwn().toString());
console.log(new D().a());
console.log(new D().thing);
console.log(new DSplit().a());
console.log(new DDeep().a());
console.log(new EDeep().a());
