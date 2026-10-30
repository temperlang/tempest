/**
 * @param {boolean} b
 * @returns {string}
 */
function explicit(b) {
  if (b) {
    return "true";
  }
  return "false";
}

/**
 * @param {boolean} b
 * @returns {string}
 */
function halfAndHalf(b) {
  if (b) {
    return "true";
  }
  return "false";
}

/**
 * @param {boolean} b
 * @returns {string}
 */
function implicit(b) {
  if (b) {
    return "true";
  } else {
    return "false";
  }
}

let t = true;
t = t;
let f = !t;
console.log(`explicit    true  -> ${explicit(t)}`);
console.log(`explicit    false -> ${explicit(f)}`);
console.log(`halfAndHalf true  -> ${halfAndHalf(t)}`);
console.log(`halfAndHalf false -> ${halfAndHalf(f)}`);
console.log(`implicit    true  -> ${implicit(t)}`);
console.log(`implicit    false -> ${implicit(f)}`);
