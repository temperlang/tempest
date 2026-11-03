# Renamed

Exports whose names backends' namings change, each for a keyword or a
builtin of some target: a function, a `let`, and a class with a field,
a method, and a getter. `imports-library` uses them, spelling each as
this library's own naming spelled it, from its interface, never
deriving it again (notes/tmpl/open-questions.md, 46).

    export let lambda(n: Int): Int { n + 1 }
    export let native = 2;

    export class Checked(public end: Int) {
      public delete(): Int { end * 2 }
      public get local(): Int { end + native }
    }
