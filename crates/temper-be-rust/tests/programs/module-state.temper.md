# Module state

Module `let`s and `var`s that functions read or write, or that are
exported, live in statics in Rust, safe to read and write from several
threads; the rest are locals of the module's init. `tick` updates `ticks`
from `ticks` alone, under one lock, so concurrent ticks aren't lost
(`crates/temper-be-rust/tests/functional.rs` runs it from 8 threads).

    export let greeting = "hi";
    var count = 0;
    let limit = 3;
    export var total = 10;
    export var ticks = 0;
    var local = 1;

    let bump(): Int {
      count = count + 1;
      total = total + count;
      count
    }

    export let tick(): Void {
      ticks = ticks + 1;
    }

    export let report(): String {
      "${greeting} ${bump().toString()} of ${limit.toString()} ${total.toString()}"
    }

    local = local + limit;
    console.log(report());
    console.log(report());
    console.log("local ${local.toString()}");

Expected output:

```log
hi 1 of 3 11
hi 2 of 3 13
local 4
```
