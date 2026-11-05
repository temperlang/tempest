# Async turns

An `async` block runs until it finishes or reaches an `await`, and no
other block runs meanwhile, so blocks that share state need no locks
(after Temper PR #532's `async-one-at-a-time` and `async-complete-order`
tests). Completing a promise only queues its waiter, so code after
`complete` runs before the waiter resumes.

Host code uses the library the standard way for each language (each
backend's `host_interop` test): it awaits what `later` returns as its own
language's promise, and loops over what `ticks` returns with its own loop.
tmpl's generators yield only `Empty` so far, so `ticks` reports each
step through `each`. If `each` fails, the generator is done.

    export let later(n: Int): Promise<Int> {
      let pb = new PromiseBuilder<Int>();
      async { (): GeneratorResult<Empty> extends GeneratorFn =>
        pb.complete(n * 2);
      }
      pb.promise
    }

    @fun export interface Tick(i: Int): Void;

    let generated(body: fn (): SafeGenerator<Empty>): SafeGenerator<Empty> { body() }

    export let ticks(n: Int, each: Tick): SafeGenerator<Empty> {
      generated { (): GeneratorResult<Empty> extends GeneratorFn =>
        for (var i = 0; i < n; i += 1) {
          each(i);
          yield();
        }
      }
    }

    let t = ticks(2) { (i: Int): Void => console.log("tick ${i.toString()}"); };
    t.nextSafe();
    t.nextSafe();
    t.nextSafe();

```log
tick 0
tick 1
```

    class Counter {
      public var n: Int = 0;
      public bump(tag: String, times: Int): Void {
        console.log("${tag} start");
        for (var i = 0; i < times; ++i) {
          let before = n;
          n = before + 1;
        }
        console.log("${tag} done n=${n.toString()}");
      }
    }

    let c = new Counter();
    let gateA = new PromiseBuilder<Int>();
    let gateB = new PromiseBuilder<Int>();

Each of two blocks reads and writes the shared counter many times with no
`await` between, awaits its own gate, and does it again. A third block
opens the gates. Run in parallel, the blocks would interleave their
`start` lines and end below 40000.

    async { (): GeneratorResult<Empty> extends GeneratorFn =>
      c.bump("A", 10000);
      let k = await gateA.promise orelse -1;
      c.bump("A${k.toString()}", 10000);
    }
    async { (): GeneratorResult<Empty> extends GeneratorFn =>
      c.bump("B", 10000);
      let k = await gateB.promise orelse -1;
      c.bump("B${k.toString()}", 10000);
    }
    async { (): GeneratorResult<Empty> extends GeneratorFn =>
      console.log("opener: calling complete");
      gateA.complete(1);
      gateB.complete(2);
      console.log("opener: complete returned");
    }

```log
A start
A done n=10000
B start
B done n=20000
opener: calling complete
opener: complete returned
A1 start
A1 done n=30000
B2 start
B2 done n=40000
```
