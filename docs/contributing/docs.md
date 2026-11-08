# Writing these docs

The docs are Markdown that reads well on GitHub, which is their first
renderer. A site, like [temperlang.dev](https://temperlang.dev/), can
be rendered from the same files later, without changing them.

## Where things go

| Folder | For |
|---|---|
| [guide/](../guide/README.md) | Learning Temper: the tour, getting started, and the tutorial |
| [language/](../language/README.md) | The language reference |
| [std/](../std/README.md) | The standard library, a page per module |
| [backends/](../backends/README.md) | What each target language gets |
| [tools/](../tools/README.md) | The command line, editors, and the playground |
| [contributing/](README.md) | Working on tempest, and these docs |
| [internals/](../internals/README.md) | How tempest works, stage by stage |
| [release-notes/](../release-notes/README.md) | What changed, release by release |
| [`../notes/`](../../notes/README.md) | Design notes and plans, kept apart from the docs |

Each folder's `README.md` is its index. Images go in
[images/](../images/); keep them small, and give each a real `alt`
text. Language logos and their terms are in
[images/lang-icons](../images/lang-icons/README.md).

## Conventions

Plain GitHub Flavored Markdown, plus a few conventions that a later
renderer can style:

- **Callouts** are GitHub alerts: `> [!NOTE]`, `> [!TIP]`,
  `> [!IMPORTANT]`, `> [!WARNING]`, and `> [!CAUTION]`. (mkdocs'
  `!!! note` became these.)
- **Inconsistencies** are flagged in place with a one-line
  `> **Note:** …` that links to its entry in
  [inconsistencies.md](../inconsistencies.md). Remove both when it's
  resolved.
- **Alternatives,** such as one example in each language, go in
  `<details><summary>Python</summary> … </details>` blocks, one per
  language, with a blank line after `<summary>` so the Markdown inside
  renders. Use `<details open>` for the one to show first. (mkdocs'
  `=== "Tab"` tabs became these.)
- **Grammar** is written out as text, in `ebnf` blocks:
  `Rule := Other "token" | …`. (These were railroad diagrams, drawn from
  the same text; a renderer could draw them again.)
- **Diagrams** are [Mermaid](https://mermaid.js.org/) blocks, which
  GitHub draws.
- **Temper examples** are ```` ```temper ```` blocks, and each ends
  with a marker comment saying what it does, as below. Literate Temper
  files are ```` ```tempermd ````, interactive sessions ```` ```console ````,
  and Temper that's only shown, never compiled, ```` ```temper inert ````.
- Links are relative, to files in this repository, and never carry line
  numbers; link a file, and name what's in it.
- Write "Temper" for the language and its tools, and "tempest" only for
  this compiler or its `tempest` binary.

### Examples that are checked

`compat/check_doc_examples.py` runs every ```` ```temper ```` block in
the docs (all but `internals/` and `release-notes/`), each as its own
one-file library, with `tempest test -v`, on the interpreter. The
marker on a block's last line says what should happen:

| Marker | Means | Checked |
|---|---|---|
| `// ✅` or `// ✅ value` | It runs without errors | It runs cleanly, and logs what its `//!outputs` comments say |
| `// ❌` or `// ❌ errors` | It's an error | It reports an error |
| `// ⏸️` | A fragment, or it needs context | Not run |
| none | Usually part of a longer example | Run, but only counted |

A `//!outputs "text"` comment (a JSON string, or a list of them) after
a statement says what it logs. The values after `✅` and the errors
after `❌` aren't compared yet.

    uv run compat/check_doc_examples.py [--verbose] [PATH...]

It uses `target/release/tempest`, else `target/small/tempest`, or
`--tempest BIN`, and runs in CI. `--verbose` also lists the unmarked
blocks that don't run alone, mostly tutorial steps that build on the
ones before.

A marked example known to disagree with tempest gets an HTML comment,
hidden on GitHub, on the line right before its fence, naming its
[inconsistencies](../inconsistencies.md) entries, such as
`<!-- known: I10 I20 -->`. It's then counted as known, not fatal, and
the checker lists the known examples by entry. Once one agrees, the
checker fails until you remove the comment and update the entry. Today, 14 are known
([I10](../inconsistencies.md#i10-examples-tempest-doesnt-run-as-marked)).

## Where they came from

Most of the guide, reference, and release notes were Temper's mkdocs
site, built from `docs/for-users/` and `build-user-docs/` in the
[temperlang/temper](https://github.com/temperlang/temper) repository.
There, the reference pages (types, builtins, and syntax) were woven
together from doc comments in the compiler's Kotlin source and std,
with each example's marker and value filled in by running it. Here,
they're plain Markdown, imported once from that generated output and
edited by hand since. If you change a builtin, update its page here.

The old site's addresses map to these pages, for redirects:

| temperlang.dev | Here |
|---|---|
| `/` | [guide/introduction.md](../guide/introduction.md) |
| `/tour/` | [guide/tour.md](../guide/tour.md) |
| `/why/` | [guide/why.md](../guide/why.md) |
| `/tutorial/` | [guide/getting-started.md](../guide/getting-started.md) |
| `/tutorial/01-classfun/` | [guide/tutorial/01-classes-and-functions.md](../guide/tutorial/01-classes-and-functions.md) |
| `/tutorial/02-interface/` | [guide/tutorial/02-interfaces.md](../guide/tutorial/02-interfaces.md) |
| `/tutorial/03-controlflow/` | [guide/tutorial/03-control-flow.md](../guide/tutorial/03-control-flow.md) |
| `/tutorial/04-modlib/` | [guide/tutorial/04-modules-and-libraries.md](../guide/tutorial/04-modules-and-libraries.md) |
| `/tutorial/05-builtin/` | [guide/tutorial/05-builtin-types.md](../guide/tutorial/05-builtin-types.md) |
| `/tutorial/06-memory/` | [guide/tutorial/06-memory.md](../guide/tutorial/06-memory.md) |
| `/tutorial/07-regex/` | [guide/tutorial/07-regex.md](../guide/tutorial/07-regex.md) |
| `/reference/` | [language/README.md](../language/README.md) |
| `/reference/types/` | [language/types.md](../language/types.md) |
| `/reference/builtins/` | [language/builtins.md](../language/builtins.md) |
| `/reference/syntax/` | [language/syntax.md](../language/syntax.md) |
| `/reference/modules/` | [language/modules.md](../language/modules.md) |
| `/reference/target-languages/` | [backends/README.md](../backends/README.md) |
| `/reference/backend/` | [internals/kotlin/writing-a-backend.md](../internals/kotlin/writing-a-backend.md) |
| `/reference/out-grammar/` | [internals/kotlin/out-grammars.md](../internals/kotlin/out-grammars.md) |
| `/legal/playground-terms/` | [legal/playground-terms.md](../legal/playground-terms.md) |
| `/blog/` | stays in its own repository, `temperlang/temperlang-blog` |

Anchors changed too: the old pages linked generated anchors, such as
`#type-Boolean`; these link GitHub's heading anchors, such as
`#class-boolean`.

## Rendering a site, later

The source stays Markdown either way. The options:

| | For | Against |
|---|---|---|
| **mkdocs** (Material), as before | Proven, search, navigation, and themes built in; the old site's look | A Python toolchain beside Rust; needs plugins or rewriting for GitHub alerts and `<details>`; its own Markdown dialect differs from GitHub's in places |
| **A small custom renderer** (a Rust binary, `pulldown-cmark` or `comrak`) | Fast, one toolchain; could run examples and draw grammar while rendering | We'd write and keep navigation, search, and themes; a new dependency |
| **Kelpto**, the lossless Markdown and HTML library written in Temper | Dogfooding: Temper's docs rendered by a Temper library, through tempest, in any of seven languages; GFM conformant, with HTML sanitizing and link checks; lossless edits could keep the docs tidy (rewrite links, number the inconsistencies) | Young; navigation, search, and highlighting are ours to add; its build depends on tempest building it |

**Recommendation: Kelpto,** run as a small Temper program that tempest
builds, starting with a static site: one page per file, a sidebar from
each folder's `README.md`, GitHub alerts and `<details>` styled, Mermaid
drawn in the browser, and Temper highlighted with the VS Code
extension's grammar. It shows off exactly what Temper is for, and what
it needs (navigation, highlighting) is useful to Kelpto's users too.
Fall back to mkdocs only if the site must ship before Kelpto can do
that. Either way, the renderer can run `check_doc_examples.py` first,
and could someday show each example's translations, made by tempest at
build time, in place of the old tabs.

## To do

- Make the known examples zero, by fixing tempest or the docs.
- Compare the values after `✅` and the errors after `❌`, as Temper's
  first doc generator did.
- Make tutorial steps checkable: mark fragments `⏸️`, or run a page's
  unmarked blocks together, in order, as one literate file.
- Write the reference for what the old generator left as "TODO", such as
  `@protected` and `@public`, `await`, and the Java backend's notes.
- Reference pages for the standard library generated from its doc
  comments, or kept by hand as now.
