import {
  dequeAdd,
  dequeConstructor,
  dequeGetIsEmpty,
  dequeRemoveFirst,
} from "@temperlang/core";

function checkEmpty() {
  console.log(dequeGetIsEmpty(deque) ? "Empty" : "Not empty");
}

/** @type {string[]} */
const deque = dequeConstructor();
checkEmpty();
if (!dequeGetIsEmpty(deque)) {
  console.log(dequeRemoveFirst(deque));
}
dequeAdd(deque, "foo");
dequeAdd(deque, "bar");
console.log(dequeRemoveFirst(deque));
dequeAdd(deque, "baz");
console.log(dequeRemoveFirst(deque));
dequeAdd(deque, "boo");
checkEmpty();
while (!dequeGetIsEmpty(deque)) {
  console.log(dequeRemoveFirst(deque));
}
checkEmpty();
