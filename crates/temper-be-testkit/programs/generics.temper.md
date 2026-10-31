# Generics

Generic functions, a generic class with a bounded type parameter and a
generic method, a generic interface, and core's generic types: list
literals, `Listed`'s and `List`'s members, `ListBuilder`, `Pair`, `Map`,
and `MapBuilder`, with nullables mixed in. The log is what tempest's
interpreter gives.

    export interface Holder<T> {
      public get held(): T;
    }

    export class Box<T>(public value: T) extends Holder<T> {
      public get held(): T { value }
      public pairWith(other: Int): Pair<T, Int> { new Pair(value, other) }
    }

    export class Named(public name: String) {}

    export class Shelf<T extends Named>(public items: List<T>) {
      public firstName(): String? {
        if (items.isEmpty) { null } else { items[0].name }
      }
    }

    export let first<T>(items: Listed<T>, fallback: T): T {
      if (items.isEmpty) { fallback } else { items[0] }
    }

    let held<T>(h: Holder<T>): T { h.held }

    let total(ns: List<Int>): Int {
      var sum = 0;
      for (var i = 0; i < ns.length; i += 1) {
        sum += ns[i];
      }
      sum
    }

    let ns: List<Int> = [1, 2, 3];
    let none: List<Int> = [];
    console.log("${total(ns)} ${first(ns, 0)} ${first(none, -1)}");
    console.log(first(["a", "b"], "z"));
    let box = new Box<String>("boxed");
    console.log(held(box));
    let pair = box.pairWith(7);
    console.log("${pair.key} ${pair.value}");
    let shelf = new Shelf([new Named("ann"), new Named("bo")]);
    console.log(shelf.firstName() ?? "none");
    let noNames: List<Named> = [];
    let empty = new Shelf(noNames);
    console.log(empty.firstName() ?? "none");
    let builder = new ListBuilder<String>();
    builder.add("x");
    builder.add("y");
    builder.add("w", 0);
    console.log("${builder.length} ${builder[0]} ${builder.removeLast()}");
    let built = builder.toList();
    console.log("${built.length} ${built.getOr(5, "?")} ${built.slice(1, 2)[0]}");
    let m = new Map([new Pair("a", 1), new Pair("b", 2)]);
    console.log("${m["b"] orelse -1} ${m["c"] orelse -1} ${m.has("a")} ${m.length}");
    let mb = new MapBuilder<String, Int?>();
    mb.set("q", 5);
    mb.set("n", null);
    console.log("${mb.getOr("q", 0) ?? -2} ${mb.getOr("n", 0) ?? -2} ${mb.has("n")}");
    console.log((mb.remove("q") orelse 0)?.toString() ?? "null");
    console.log(mb.keys().length.toString());

```log
6 1 -1
a
boxed
boxed 7
ann
none
3 w y
2 ? x
2 -1 true 2
5 -2 true
5
1
```
