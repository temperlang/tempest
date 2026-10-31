/** @param {readonly number[]} b */
function f(b) {
  if (!b.length) {
    console.log("empty");
  } else {
    console.log("not empty");
  }
}

f(Object.freeze([2, 3]));
f(Object.freeze([]));
