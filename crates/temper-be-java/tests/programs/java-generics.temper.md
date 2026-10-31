# Java generics

Generic values where Java's boxes could go wrong: two `Integer`s from a
list or a generic function compared with `==` would compare references,
so `1000 == 1000` would be false; and lists may hold null, which
`List.of` and `List.copyOf` reject.

    let id<T>(x: T): T { x }

    let a: List<Int> = [1000, 2];
    let b: List<Int> = [1000, 3];
    console.log((a[0] == b[0]).toString());
    console.log((a[1] != b[1]).toString());
    console.log((id(1000) == id(1000)).toString());
    let maybe: List<Int?> = [1, null];
    console.log((maybe[1] ?? -1).toString());
    let builder = new ListBuilder<Int?>();
    builder.add(null);
    console.log(builder.toList().length.toString());

```log
true
true
true
-1
1
```
