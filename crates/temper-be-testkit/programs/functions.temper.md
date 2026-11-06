# Functions

Function values: a closure counter that writes a captured `var`, a
`@fun` interface passed and returned, a static method's reference,
core's list members that take functions (`map`, `filter`, `join`, and
`sorted` with a comparator), a closure stored in a class's field, and
closures made in a loop, each capturing that time round's `let`, in a
function and in a generator, which is stepped once past its end; and
loops: `continue` in a `for … of`, and a `for` whose second
declaration reads the first. The log is what tempest's interpreter
gives.

    @fun interface Counter(): Int;
    @fun interface IntOp(x: Int): Int;

    let makeCounter(start: Int): Counter {
      var count = start;
      fn (): Int {
        count += 1;
        count
      }
    }

    let twice(f: IntOp): IntOp {
      fn (x: Int): Int { f(f(x)) }
    }

    let applyTo(f: IntOp, x: Int): Int { f(x) }

    let addOne(x: Int): Int { x + 1 }

    class Scaler {
      public static triple(x: Int): Int { x * 3 }
    }

    class Button(public label: String, public onPress: Counter) {
      public press(): String { "${label} ${onPress()}" }
    }

    let counter = makeCounter(10);
    counter();
    console.log("count ${counter()}");
    let other = makeCounter(0);
    console.log("other ${other()} count ${counter()}");
    let addTwo = twice(addOne);
    console.log("twice ${addTwo(5)} ${applyTo(addOne, 1)}");
    console.log("triple ${applyTo(Scaler.triple, 4)} ${twice(Scaler.triple)(2)}");
    let ns: List<Int> = [5, 2, 8, 1, 4];
    let doubled = ns.map { (n: Int): Int => n * 2 };
    let big = doubled.filter { (n: Int): Boolean => n > 4 };
    console.log(big.join(", ") { (n: Int): String => n.toString() });
    let order = ns.sorted { (a: Int, b: Int): Int => a - b };
    console.log(order.join(" ") { (n: Int): String => "${n}" });
    var total = 0;
    ns.forEach { (n: Int): Void => total += n; };
    console.log("total ${total}");
    let button = new Button("hits", makeCounter(0));
    button.press();
    console.log(button.press());
    let makers = new ListBuilder<Counter>();
    for (var i = 0; i < 3; i += 1) {
      let k = i * 10;
      makers.add(fn (): Int { k });
    }
    console.log(makers.join(" ") { (c: Counter): String => "${c()}" });
    let drive(steps: Int, body: fn (): SafeGenerator<Empty>): Void {
      let g = body();
      for (var s = 0; s < steps; s += 1) { g.nextSafe(); }
    }
    drive(5) { (): GeneratorResult<Empty> extends GeneratorFn =>
      let fs = new ListBuilder<Counter>();
      for (var i = 0; i < 3; i += 1) {
        let j = i * 10;
        fs.add(fn (): Int { j });
        yield();
        fs.add(fn (): Int { j + 1 });
      }
      console.log(fs.join(" ") { (c: Counter): String => "${c()}" });
      yield();
    }
    let sumFrom(xs: List<Int>, least: Int): Int {
      var sum = 0;
      for (let x of xs) {
        if (x < least) { continue; }
        sum += x;
      }
      sum
    }
    let doubles(s: String): Int {
      var n = 0;
      let end = s.end;
      for (var i = String.begin, prev = i; i < end; i = s.next(i)) {
        if (prev < i && s[prev] == s[i]) { n += 1; }
        prev = i;
      }
      n
    }
    console.log("sum ${sumFrom(ns, 4)} doubles ${doubles("bookkeeper")}");

```log
count 12
other 1 count 13
twice 7 2
triple 12 18
10, 16, 8
1 2 4 5 8
total 20
hits 2
0 10 20
0 1 10 11 20 21
sum 17 doubles 3
```
