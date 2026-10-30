# Classes

Classes and interfaces: fields, constructors (made from a class's header,
or its own), methods, getters and setters, static fields and functions,
private members read on another instance, interfaces with abstract and
default members, `@auto("==")` and `@operator` methods, `toString`, `is`, `as`, and narrowing. Calls chain on
objects that methods return, which C++ must keep alive. The log is what
tempest's interpreter gives.

    export interface Shape {
      public get area(): Int;
      public describe(): String { "shape of area ${area}" }
    }

    export class Rect(public w: Int, public h: Int) extends Shape {
      public static unit = new Rect(1, 1);
      public get area(): Int { w * h }
      public toString(): String { "Rect(${w}, ${h})" }
      public scaled(k: Int): Rect { new Rect(w * k, h * k) }
    }

    export class Square extends Shape {
      private side: Int;
      public constructor(side: Int) {
        this.side = side;
      }
      public get area(): Int { side * side }
      public describe(): String { "square of side ${side}" }
    }

    class Counter {
      public var count: Int = 0;
      private var hidden: Int = 0;
      public get doubled(): Int { count * 2 }
      public set doubled(value: Int) { count = value - count; }
      public bump(): Counter {
        count = count + 1;
        hidden = hidden + 1;
        this
      }
      public same(other: Counter): Boolean { hidden == other.hidden }
      public static zero(): Counter { new Counter() }
    }

    @auto("==")
    class Point(public x: Int, public y: Int) {}

    class Loose(public n: Int) {
      @operator("==")
      public eq(other: Loose): Boolean { n == other.n || n == -other.n }
    }

    @auto("==")
    class Holder(public loose: Loose, public tag: String) {}

    class Version(public major: Int, public minor: Int) {
      @operator("<=>")
      public compareTo(other: Version): Int {
        if (major != other.major) { major <=> other.major } else { minor <=> other.minor }
      }
    }

    let shape(x: Shape): Shape { x }

    let r = new Rect(2, 3);
    console.log(r.toString());
    console.log(r.area.toString());
    let s = shape(r);
    console.log(s.describe());
    let q = shape(new Square(4));
    console.log(q.describe());
    console.log(q.area.toString());
    console.log(r.scaled(2).scaled(3).area.toString());
    console.log(Rect.unit.scaled(5).toString());
    let c = Counter.zero();
    c.bump().bump();
    console.log(c.count.toString());
    c.doubled = 10;
    console.log(c.doubled.toString());
    console.log(c.same(new Counter().bump().bump()).toString());
    console.log("${new Point(1, 2) == new Point(1, 2)} ${new Point(1, 2) != new Point(2, 1)}");
    console.log("${new Holder(new Loose(3), "a") == new Holder(new Loose(-3), "a")}");
    console.log("${new Version(1, 2) < new Version(1, 3)} ${new Version(2, 0) >= new Version(1, 9)}");
    console.log("${q is Square} ${s is Square}");
    let sq = q as Square;
    console.log(sq.area.toString());
    if (s is Rect) {
      console.log(s.w.toString());
    }

```log
Rect(2, 3)
6
shape of area 6
square of side 4
16
216
Rect(5, 5)
2
16
true
true true
true
true true
true false
16
2
```
