# Imports a library

Imports another library by its name, `modules`, and one of its modules
by its path there; a library's tests import std's `testing` the same way.
And from `renamed`, exports whose names backends' namings change, each
spelled as `renamed`'s own naming spelled it.

    let { sum } = import("modules");
    let { twice, base } = import("modules/helpers");

    export let both(n: Int): Int { sum(n) + twice(n) + base }

    console.log("both ${both(2).toString()}");

    let { lambda, native, Checked } = import("renamed");
    let checked = new Checked(lambda(native));
    console.log(
      "renamed ${checked.delete().toString()} ${checked.local.toString()} ${checked.end.toString()}"
    );

```log
hello, helpers from deep
hello, modules from deep
sum 17
calls 0
calls now 3
both 29
renamed 6 5 3
```
