# Content tags

A content tag whose contexts are worked out at compile time
(`temper-frontend`'s `autoescape`, as Kotlin's
`OptimizeContextualAutoescapingBlocks`): a toy tag following the
conventions Kotlin's pass expects of a
`ContextualAutoescapingAccumulator`, whose tagged strings call the
escapers it picks directly, and one whose accumulator a closure uses,
which propagates its contexts at run time as written. Both log what
the tag's own methods would. The log is what tempest's interpreter
gives.

    // A toy content tag, following Kotlin's pass's conventions: a value
    // is escaped as [x], and inside single quotes as ([x]), and a fixed "!"
    // becomes a mark.
    export interface Effect { enactOn(c: Buffer): Void; }
    export class AppendParseEffect(public text: String) extends Effect {
      public enactOn(c: Buffer): Void { c.append(text) }
    }
    export class EventParseEffect(public event: Mark) extends Effect {
      public enactOn(c: Buffer): Void { c.mark(event) }
    }
    export class Mark(public label: String) {}
    export class After(public effects: List<Effect>, public stateAfter: Boolean) {}
    export class Propagator(public quote: String) {}
    export let Callout = fn (String, Boolean): Void;
    let ignore(problem: String, isError: Boolean): Void { if (isError) { console.log(problem); } }
    export let propagateOver(
      p: Propagator, before: Boolean, known: String?,
      same: fn (Boolean, Boolean): Boolean, callout: Callout,
    ): After {
      if (known == null) { return new After([], before); }
      if (known == "!") { return new After([new EventParseEffect(new Mark("bang"))], before); }
      let angles = known.split("<").length;
      if (angles > 1) { callout("angle bracket in fixed text", false); }
      if (!same(before, before)) { callout("states differ from themselves", true); }
      let quotes = known.split(p.quote).length - 1;
      new After([new AppendParseEffect(known)], if (quotes % 2 == 1) { !before } else { before })
    }
    // Escapers' overrides answer to `apply`, as their interface marks it.
    export interface Escaper {
      @overload("apply")
      applyString(x: String): String;
    }
    export class TextEscaper extends Escaper {
      public static instance: TextEscaper = new TextEscaper();
      public applyString(x: String): String { "[${x}]" }
    }
    export class QuoteEscaper extends Escaper {
      public static instance: QuoteEscaper = new QuoteEscaper();
      public applyString(x: String): String { "(${x})" }
    }
    // Composed escapers apply `first`, then `second`.
    export class Both(public first: Escaper, public second: Escaper) extends Escaper {
      public applyString(x: String): String { second.applyString(first.applyString(x)) }
    }
    export class Picker(public strict: Boolean) {
      public escaperFor(inQuote: Boolean, callout: Callout): Escaper {
        if (strict && inQuote) { callout("a value in quotes", false); }
        if (inQuote) { new Both(TextEscaper.instance, QuoteEscaper.instance) } else { TextEscaper.instance }
      }
    }
    export class Buffer {
      private sb: StringBuilder = new StringBuilder();
      public append(x: String): Void { sb.append(x) }
      public mark(m: Mark): Void { sb.append("{${m.label}}") }
      public toString(): String { sb.toString() }
    }
    export interface ContextualAutoescapingAccumulator {
      public get accumulated(): String;
    }
    export class Quoting extends ContextualAutoescapingAccumulator {
      public static newCollector(): Buffer { new Buffer() }
      public static initialState(): Boolean { false }
      public static propagator(): Propagator { new Propagator("'") }
      public static picker(): Picker { new Picker(false) }
      public static sameStatePredicate(): fn (Boolean, Boolean): Boolean {
        fn (a: Boolean, b: Boolean): Boolean { a == b }
      }
      public static mergeStates(a: Boolean, b: Boolean, callout: Callout): Boolean {
        if (a != b) { callout("branches end in different contexts", true); }
        a
      }
      public static checkEndState(inQuote: Boolean, callout: Callout): Void {
        if (inQuote) { callout("ends inside quotes", true); }
      }
      public static fromCollector(c: Buffer): String { c.toString() }
      public static enact(m: Mark, c: Buffer): Void { c.mark(m) }
      private var inQuote: Boolean = false;
      private collector: Buffer = new Buffer();
      public appendSafe(x: String): Void {
        let after = propagateOver(Quoting.propagator(), inQuote, x, Quoting.sameStatePredicate(), ignore);
        inQuote = after.stateAfter;
        for (let e of after.effects) { e.enactOn(collector); }
      }
      public append(x: String): Void {
        let escaped = Quoting.picker().escaperFor(inQuote, ignore).apply(x);
        collector.append(escaped);
      }
      public get accumulated(): String { collector.toString() }
    }

    let q = Quoting;

    // Worked out at compile time: each value's escapers are called directly,
    // and the fixed "!" becomes the mark it enacts.
    let greet(name: String, who: String): String { q"say 'hi ${name}' to ${who}!" }
    console.log(greet("Ann", "Bob"));
    // Branches whose contexts agree.
    let branch(c: Boolean, x: String): String {
      q"""
        ~${x}
        : if (c) {
        ~''
        : }
        ~ '${x}'
    }
    console.log(branch(true, "a"));
    console.log(branch(false, "b"));
    // The accumulator is used in a closure, so contexts propagate at run time.
    let each(xs: List<String>): String {
      q"""
        : for (let x of xs) {
        ~'${x}' ${x};
        : }
    }
    console.log(each(["c", "d"]));

```log
say 'hi ([Ann])' to [Bob]{bang}
[a]'' '([a])'
[b] '([b])'
'([c])' [c];'([d])' [d];
```
