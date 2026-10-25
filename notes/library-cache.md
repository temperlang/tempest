# Library cache: a stable format that skips the frontend

Part of the [rewrite plan](README.md). A preliminary assessment
(2026-10-25), not a decision. Discussed for Kotlin Temper too, but never
built there; its slow frontend is the main motivation.

## What it is

A file per built library that holds the frontend's results, so a later
build, a dependent library or a backend can start from it instead of
from source. It holds:

- **The interface:** exports with their types, signatures and docs, enough
  to compile code against the library (like `.d.ts` or Rust's `.rmeta`).
- **The bodies:** the settled program, as the shared starting tmpl that
  the frontend hands every backend (generators and `async` still
  abstract). Backends translate each library for each target, so they need
  bodies, not just the interface.
- **Metadata:** the compiler and format versions, a hash per source file,
  the library's dependencies with their hashes, and diagnostics.

The starting tmpl is the natural core: it's already meant to be one stable
form that every backend starts from, defined in its own schema crate.

## Is it useful for tempest?

Yes, though less for raw speed than in Kotlin Temper: settling all of
std's `json` (about 1,400 lines) takes about 10 ms as a whole process.
Where it helps:

- **Core and std in the binary:** embed their caches rather than their
  source, so no build settles std again.
- **Dependencies:** published libraries can ship a cache, so dependents
  never parse or settle them.
- **Watch mode and the language server:** a warm start, with unchanged
  libraries loaded rather than rebuilt.
- **Bridging the two compilers:** if Kotlin Temper's backends could read
  the format, tempest could serve as the frontend for them before tempest's
  own backends exist. That's maybe the biggest win for Kotlin Temper, and
  cheaper than building a cache inside it, where trees are object graphs
  that are harder to write out.

Open: compile-time evaluation of a dependency's functions. The interpreter
runs on the frontend's lowered form, not tmpl, so either the cache also
carries what the interpreter needs (lowered code, or a settled tree it can
lower), or the interpreter learns to run tmpl.

## Binary, text, or both?

Both, from one schema:

- **Binary as the real format:** compact and fast to load. Tempest's arena
  trees (nodes in arrays, ids instead of pointers, interned strings) write
  out almost as they are: a string table, node arrays and varints, maybe
  memory-mapped. A small format with a written spec, so Kotlin can read it
  too, rather than one tied to a Rust library's layout.
- **Text as a printed view of the same data:** for debugging, diffs and
  golden tests (as `tests/settle/*.cases` compare printed trees now), and
  for an easy first reader on the Kotlin side. Not used for real builds.

Stability: at first, a cache is valid only for the compiler version that
wrote it, and is rebuilt otherwise. A format that's stable across versions
matters once published libraries ship caches; then tmpl's schema needs
versioning and compatibility rules, perhaps tracked in `temper.keep`.

## Decompiling to Temper source

Mostly, yes. The starting tmpl is after macro expansion, so decompiling
gives valid Temper of the expanded program: desugared, without the
original macros, formatting, prose or comments. With docs and source spans
kept, it can be quite readable, which is useful for looking into a
dependency without its source. For the exact original, simply ship the
source beside the cache; that's simpler than making the format round-trip.

## Would AI save tokens?

Probably not by writing in a compact format. Models write mainstream-
looking source best, and Temper reads like the languages they know; a
binary form isn't readable to them at all, and a terse custom text form
would be unfamiliar, error-prone, and not tokenized efficiently. So
Temper source is likely already the best form for generating code.

The token win is in reading: an API summary printed from the cache
(signatures and short docs, like `.d.ts`) is far smaller than a library's
full literate source, and is what an assistant (or a person) needs to use
a dependency. That's cheap once the interface is in the cache.

## Implications now

Not to build yet: tmpl doesn't exist until milestone 5. But tmpl's design
should keep it easy to write out: ids rather than pointers, interned
strings, no closures or host objects in the tree, and deterministic order.
