# Errors

Code with errors still translates and runs: each error fails where it
runs, as `bubble()` does, so an `orelse` catches it, and everything
else runs as written (tmpl's `Error`).

    export let good(x: Int): Int { x + 1 }

A value that doesn't fit its place, an undeclared name, a misspelled
member and method, and the wrong number of arguments:

    let mismatched(x: Int): Int throws Bubble { let s: String = x; 1 }
    let undeclared(): Int throws Bubble { missing(1) }
    let misspelled(s: String): Int throws Bubble { s.lenght }
    let misspelledMethod(s: String): Int throws Bubble { s.lengthy() }
    let wrongArity(): Int throws Bubble { good(1, 2) }

An undeclared type leaves the declaration usable, its type unknown:

    let unknownType(x: Nope): Int { 1 }

A missing bracket stays local, so what follows still parses:

    let unclosed(x: Int: Int { x * 2 }
    let afterUnclosed(x: Int): Int { x * 3 }

A method with an error fails, but not its class's others:

    class Box(public value: Int) {
      public get(): Int { value }
      public bad(): String throws Bubble { value }
    }

    let a = mismatched(1) orelse -1;
    let b = undeclared() orelse -1;
    let c = misspelled("a") orelse -1;
    let d = misspelledMethod("a") orelse -1;
    let e = wrongArity() orelse -1;
    let box = new Box(3);
    let f = box.bad() orelse "caught";
    console.log("${good(1)} ${a} ${b} ${c} ${d} ${e}");
    console.log("${unknownType(1)} ${afterUnclosed(2)} ${box.get()} ${f}");

```log
2 -1 -1 -1 -1 -1
1 6 3 caught
```
