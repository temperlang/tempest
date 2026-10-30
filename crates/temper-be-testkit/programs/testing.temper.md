# Testing

Tests, each a function of a `Test` of std's `testing` module, which
backends run after the module's init, in their own test forms. Their
bodies use only what tmpl translates so far: no assertions yet, since
those are calls of `Test`'s methods, which wait for classes. Running the
tests needs std's `testing` on each backend, so backends check only their
form here.

    export let twice(n: Int): Int { n * 2 }
    var runs = 0;
    let base = 10;

    test("twice works") {
      let x = twice(2);
      runs = runs + x + base;
    }

    test("names: punctuation, 'quotes', and ünïcode") { t =>
      twice(runs);
    }

    console.log("init ${twice(1).toString()}");

```log
init 2
```
