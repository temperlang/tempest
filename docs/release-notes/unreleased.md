# Unreleased

- [🚨Breaking change: Rest parameter syntax removed.](#breaking-change-rest-parameter-syntax-removed)
- [Auto operator implementation](#auto-operator-implementation)
- [Rust resumes every block awaiting a promise](#rust-resumes-every-block-awaiting-a-promise)
- [Decorators for immutable types](#decorators-for-immutable-types)
- [🚨Breaking change: type parameter variance now requires `@`.](#breaking-change-type-parameter-variance-now-requires-)
- [🚨Breaking change: Multiline string syntax (yes, again)](#breaking-change-multiline-string-syntax-yes-again)
- [🆕 Bitwise operators](#-bitwise-operators)
- [🚨Breaking change: Details of rules for `<` ambiguity have changed](#breaking-change-details-of-rules-for--ambiguity-have-changed)
- [Rust crates depend on `temper-std` only when they use it](#rust-crates-depend-on-temper-std-only-when-they-use-it)
- [`++ls[i]` gives the new value](#lsi-gives-the-new-value)
- [Unassigned locals in `for … of` bodies are errors](#unassigned-locals-in-for--of-bodies-are-errors)
- [`@json` adapters](#json-adapters)
- [A class has only one constructor](#a-class-has-only-one-constructor)
- [Extension methods](#extension-methods)

## 🚨Breaking change: Rest parameter syntax removed.

Previously, Temper allowed variadic functions, functions
that can accept more arguments than the number of declared
formals, when the extra arguments could fit into a
homogeneous list that is bound to a "rest" parameter.

```temper inert
export let sum(
  ...nums: Int32  // ERROR: syntax not supported
): Int32 {
  var total = 0;
  for (let num of nums) {
    total += num;
  }
  total
}
```

These were hard to translate. Dynamic languages tend to
handle them fine, but TmpL had two special forms:
an expression for the count of rest arguments, and
an expression for the n-th rest argument.
Most backends didn't bother translating them, so
it was unclear that translation was generally possible.

But internally, the way rest parameters were represented
in function signatures complicated a lot of things.

It was also unclear how to support connected, variadic
functions.

Temper may reintroduce rest parameters in the future,
in a way that only puts a burden on backends that want
to allow rest-calling conventions, and in a way that
does not have extra moving parts for the vast majority
of function signatures that are non-resty.
(Perhaps by establishing the convention that if the last
parameter has a type like `RestList<T>` then extra
parameters can be auto grouped into a list, so rest
parameter calling is a compiler fiction for list passing.)

Some builtins are still variadic: the list constructor
and string concatenation.  But variadicity is not supported
in user code.

## Auto operator implementation

For the upcoming `@operator` feature, there's also a way to implement `==`
automatically:

```temper
@auto("==")
export class Point(
  public x: Int,
  public y: Int,
) {}
```

This automatically creates an `@operator("==")` implementation named `eq` that
checks `==` for all backed properties. This also works automatically for whole
hierachies of sealed types, which also checks the type in those cases.

See documentation for more details.

## Rust resumes every block awaiting a promise

On the Rust backend, a promise used to remember only the last `async` block
that awaited it, so when two blocks awaited the same promise, only one of them
resumed. Now every waiting block resumes, in the order it started waiting.
A `breakPromise` after `complete` no longer replaces the first result.

## Decorators for immutable types

The following decorators are now built into Temper. These tags are commitments
on the part of any type that extends them:

- `@imu` - Any type decorated with `@imu` must be deeply immutable with not even
  any internal state changes allowed. Built-in `@imu` types include *String*,
  *Int32*, and *Boolean*, among others.
- `@partialImu` - Any type decorated with `@partialImu` is also immutable when
  actual type arguments are deeply immutable. Built-in *PartialImu* types
  include *List* and *Map*.

These restrictions also apply to subtypes of types with the above decorations.

Immutable types will allow for safe transfer across threads or for compile-time
evaluation, among other possible applications. Most target languages don't have
a way to explicitly state immutability, but it's possible these tags can affect
translation in some cases.

Decorations `@imu` and `@partialImu` are checked by the Temper compiler. For
example, the following declarations are legal:

```temper inert
// `@imu` tag is valid because all properties are imu.
// And `List<Int>` is imu because *Int* is imu.
@imu class SomeValues(
  public text: String,
  public numbers: List<Int>,
) {}

// `@partialImu` is valid because this type is imu if *T* is imu.
@partialImu class SummarizedItems<T>(
  public items: List<T>,
  public summary: T,
) {}

// `@imu` is valid because *T* is constrained.
@imu interface PropHolder<@imu T> {
  public prop: T;
}

// And `@imu` is still valid here because all tags follow rules.
class SummarizedItemsHolder(
  public prop: SummarizedItems<String>,
) extends PropHolder<SummarizedItems<String>> {}
```

But the following are illegal:

```temper inert
// No commitment tag here, so legal but not imu.
class NoCommitment {}

// Compiler error because *NoCommitment* isn't imu.
@imu class BadImuClass(
  public things: List<NoCommitment>,
) {}

// Compiler error because *reassignable* is `var`.
@partialImu class BadImuClassBecauseVar<T>(
  public var reassignable: List<T>,
) {}
```

This is just a sampling of the rules which enforce `@imu` and `@partialImu`
tagging.

## 🚨Breaking change: type parameter variance now requires `@`.

Previously, a type parameter could be declared like `<out T>`.
Now you need to declare it `<@out T>` using decoration/annotation syntax.
Similarly for contravariant syntax `<@in T>`.
Invariant remains the default.

Also, both covariance and contravariance are deprecated.  It's not
clear that explicit variance is translatable to languages that infer
variance or to language's that are very aggressive about
monomorphization.

## 🚨Breaking change: Multiline string syntax (yes, again)

Early on, Temper used Python-esque triple quoted string syntax but
with rules for stripping incidental spaces from the beginning and ends
of lines.

```temper inert
let s = """
  Two spaces stripped per line
  Including here
  Can't have """ here
  """;
```

A subsequent 🚨breaking change introduced multi-quoted string syntax
that uses margin characters and to allow embedding statement fragments
using `{: ... :}`.

With margin characters, there's no need for an end delimiter.

```temper inert
let a = """
  "I am the Count who loves to ${action}!
  "{: for (let number of numbers) { :}
      " ${number}! Ha HA ha.
  "{: } :}
  ;
```

Unfortunately, that scheme was visually noisy: `{: } :}` is 7
characters for a payload of one single-character token.
And the rules around stripping newlines were confusing.

The new, final, multi-quoted string expression syntax now leans more
heavily on margin characters.

There are now three margin characters:

- The double quote (`"`) and tilde (`~`) margin characters precede
  literal character content, escape sequences, and interpolations.
  See below for why there are two.
- The colon margin character precedes lines of statement fragment
  tokens.

```temper
let numbers = ["Zero", "One", "Two", "Three"];
// Starting at zero because the Count is a vampire, not a monster.
let action = "count";

let a = """
  ~I am the Count who loves to ${action}!
  : for (let number of numbers) {
      ~ ${number}! Ha HA ha.
  : }
  ;
```

Spaces (and tabs) are still stripped from the ends of lines, so use
`${}` if you need a line to end with spaces.

But the `~` and `"` margin characters give fine-grained control over
newline stripping via one simple rules: `"` lines have them, `~` lines
don't.

### Migration Guide

To migrate old style complex string expressions to new style, keep in
mind two things.

1. The old style dropped any newline at the end of the last line.
   If you want the last line to drop its newline *now*, use the
   `~` margin character instead of `"`.
2. `{: ... :}` statement fragments need to be on their own lines,
   without the delimiters, and with a `:` margin character instead
   of `"`.
   if you have lines that mix statement fragments and character data,
   separate the parts onto separate lines with the right margin
   character, and use `~` instead of `"` for character data lines
   that were followed by other stuff before a newline.

## 🆕 Bitwise operators

Previously, Temper had `&` and `|` for operating on *Int32* and
*Int64* values but was missing some operators from the usual suite.

Temper now has these operators:

- Prefix `~` performs bitwise complement on an integer.
- Infix `^` performs bitwise exclusive-or on two same-width integers.
- Infix `<<` can take an integer of either width on the left and an
  *Int32* on the right, and shifts left.
- Infix `>>` does a sign-extending right shift, and take the same
  kinds of arguments as the left shift.
- Infix `>>>` does a non sign-extending right shift.

These are documented under the builtin operators reference.

All the shift operators ignore any bit the five (for 32b shifts) or
six (for 64b shifts) least significant bits which emulates common
hardware practice.

None of these operands panic on underflow or overflow.

Negative integers have defined semantics for shifting, as per their
2's complement representation, and shifting a positive value's bit
left into the most significant bit produces a negative integer again
per 2's complement.

## 🚨Breaking change: Details of rules for `<` ambiguity have changed

The previous parsing rules for determining when a `<` token is an
infix comparison operator and when it is part of a `<...>` type
argument group have changed.

Previously, the rule was based on speculatively parsing ahead, a
technique that made it difficult to recreate Temper's exact parsing in
tools like TextMate grammars.

Now, it is based on whether there is a space or comment token preceding.

Previously, some expressions that looked like comparisons, would require
extra parentheses.

    f(a < b, c > d)

Now they don't, but if you need a comparison operator, always put spaces
around it.

See the builtin/< reference article for details.

## Rust crates depend on `temper-std` only when they use it

A generated Rust crate no longer depends on `temper-std` when its
library doesn't use std, where it always did before. A project with a
committed `Cargo.lock` that builds such a crate sees `temper-std` drop
out of it once.

## `++ls[i]` gives the new value

`++ls[i]` and `--ls[i]` on a `ListBuilder` or `MapBuilder` now give the
element's new value, as `++x` does, where they gave `Void` before, so
they can be used as values. `ls[i]++` still gives the old value, and
`ls[i] += 1` still gives `Void`, as `ls[i] = v` does.

## Unassigned locals in `for … of` bodies are errors

A `for … of` loop's body that reads a local of the function around it,
where some path to the loop doesn't assign it, is now "x is not
initialized along branches", as it is anywhere else in the function.
It wasn't reported before, and failed when run.

```temper inert
let firsts(xs: List<Int>): Int {
  let first;
  for (let x of xs) {
    console.log(first.toString()); // ERROR: not initialized
  }
  first = 0;
  first
}
```

## `@json` adapters

`@json` on a class or sealed interface now gives it a static
`jsonAdapter()`, and, unless it writes them itself, `encodeToJson` and
`decodeFromJson`, as the [decorator's docs](../language/builtins.md#json-decorator)
describe, with `@jsonName` and `@jsonExtra`; before, it gave "No member
jsonAdapter". A module with `@json` needn't import `std/json` for them.
Not yet for generic types, nor std's `Date`.

## A class has only one constructor

Several constructors were never portable (Kotlin's JS output, for
one, was invalid), and now each one after the first is an error. For
other ways to make instances, use a static method, or an object literal
with the one constructor.

```temper inert
class Thing {
  public n: Int;
  public constructor(hi: Int) { n = hi; }
  public constructor(lo: Int) { n = lo + 1; } // ERROR: only one constructor
}
```

## Extension methods

A function marked `@extension("name")` can now be called as a method
wherever it's visible, declared or imported: `"step on no pets".isPalindrome()`
calls `stringIsPalindrome("step on no pets")`, on every backend; before,
such a call couldn't be translated. A method of the same name that
takes the arguments still wins. Two extensions that fit a call equally
well, declared side by side, are now an error, where Kotlin's Temper
picked the first one.
