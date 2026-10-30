# Modules

A library of three modules: this one, `helpers`, and `helpers/deep`,
which `helpers` imports too, and which run in that order, each after
those it imports. Imports name functions, a `let`, and a `var`, which
an import copies when it runs, as the interpreter and Kotlin's backends
read it, so later writes don't show here; an alias; and a module's own
declaration with an imported one's name.

    let { twice as double, base, bump, calls } = import("./helpers");
    let { greet } = import("./helpers/deep");

This module's `base` is its own, and not exported; the helpers' is
imported as `helperBase`.

    let { base as helperBase } = import("./helpers");
    let base = 1;

    export let sum(n: Int): Int {
      double(n) + helperBase + base
    }

    console.log(greet("modules"));
    console.log("sum ${sum(3).toString()}");
    bump();
    bump();
    console.log("calls ${calls.toString()}");

```log
hello, helpers from deep
hello, modules from deep
sum 17
calls 0
```

    console.log("calls now ${bump().toString()}");

```log
calls now 3
```
