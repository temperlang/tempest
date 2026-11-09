# Errors

Code with errors still translates and runs: each error fails where it
runs, as `bubble()` does, so an `orelse` catches it, and everything
else runs as written (tmpl's `Error`). Code that's only ill-typed runs
on as written on the interpreter and dynamically typed backends, as in
Kotlin's interpreter, and fails there on statically typed ones, whose
compilers would reject it.

    export let good(x: Int): Int { x + 1 }

A value that doesn't fit its place (a type-only error), an undeclared
name, a misspelled member and method, and the wrong number of
arguments:

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

A missing operator is one error, and `part` is still declared, with
`cents`'s type, so only its `let` fails, where it runs:

    let dropped(cents: Int): String throws Bubble {
      let part = cents  100;
      "${part.toString()}"
    }

More type-only errors: a value stored, returned, or passed where
another type is wanted:

    let stored(x: Int): String throws Bubble { let s: String = x; "${s}!" }
    let returned(): Int throws Bubble { "text" }
    let passedOn(x: Int): Int throws Bubble { x }

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

`mismatched` and `bad` have type-only errors, which run on where
ill-typed code runs:

```log interp js lua py
2 1 -1 -1 -1 -1
1 6 3 3
```

```log cpp csharp java rust
2 -1 -1 -1 -1 -1
1 6 3 caught
```

The type-only errors, which run on where ill-typed code runs:

    let g = stored(1) orelse "caught";
    let h = returned() orelse -1;
    let i = passedOn("in") orelse -1;
    let j = dropped(1) orelse "dropped";
    console.log("${g} ${h} ${i} ${j}");

```log interp js lua py
1! text in dropped
```

```log cpp csharp java rust
caught -1 -1 dropped
```
