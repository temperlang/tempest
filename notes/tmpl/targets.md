# tmpl: targets

Part of the [tmpl design](README.md).

## Level 1: Targets

**Decided:** primary targets are Kotlin Temper's backends (C++, C#,
Java, JavaScript, Lua, Python, Rust). Python is tempest's first backend
and Rust right after, so the design keeps both ends in view: Python
needs few lowerings but exact capture and scope facts, and Rust needs
results, state machines, explicit conversions, nullable as `Option`,
and ownership-friendly facts.

The stress list, kept in mind but not central:

| Target | What it stresses | How the design copes |
|---|---|---|
| C99 | No GC, generics, closures, classes, or exceptions | Error codes from the explicit bubbling paths; closure conversion; erasure or monomorphizing; structs and vtables. Reference counting relies on no cycles ([acyclic.md](../acyclic.md)) and cleanup on every exit path. Possible, not cheap. |
| PHP | `use (&$x)` captures; arrays copied by value | Capture lists; lists as runtime objects. Copes. |
| Go | No exceptions or inheritance, limited generics | Results as `(T, error)`; interfaces plus delegation for interfaces' method bodies; generic methods as functions. Copes, with some passes. |
| Swift | ARC, value types, protocols, `try` at each call | Every bubbling call is marked; ARC relies on no cycles. Copes. |
| Dart, Ruby, Kotlin | Little | Copes. |
| Elixir, BEAM; Haskell | Immutability, processes, no loops; purity, laziness | Rebinding and recursion are generic passes; [mutability.md](mutability.md)'s facts say what may cross processes and what is plain data. Shared mutable objects still need emulating. Room is left. |
| Actors and isolates generally | What state is per isolate, what may be sent | All mutable state is explicit ([mutability.md](mutability.md)). |
| MATLAB | 1-based indexing, value semantics | Indexing stays in runtime operations; handle classes; cells. Partly. |
| WASM | Structured control only | Labeled blocks and loops fit `br`; otherwise the C99 path, or WasmGC. Copes via the low end. |
