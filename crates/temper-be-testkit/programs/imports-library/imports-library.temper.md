# Imports a library

Imports another library by its name, `modules`, and one of its modules
by its path there; a library's tests import std's `testing` the same way.

    let { sum } = import("modules");
    let { twice, base } = import("modules/helpers");

    export let both(n: Int): Int { sum(n) + twice(n) + base }

    console.log("both ${both(2).toString()}");

```log
hello, helpers from deep
hello, modules from deep
sum 17
calls 0
calls now 3
both 29
```
