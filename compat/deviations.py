"""Where temper-rs deliberately differs from Kotlin Temper, for the compat
scripts: the reasons, the inputs known to differ, and a rough classifier for
fuzzed inputs. Each reason is described in notes/testing-and-milestones.md.
This exists only while we compare with Kotlin.
"""

import re

REASONS = {
    "quoted-code": "code blocks in markdown block quotes are prose",
    "multi-quoted": "clearer rules for multi-quoted strings and scriptlet lines",
    "nested-hole": "`${` nests in a hole, as elsewhere in code",
    "spans": "spans cover all of a node or token's parts",
}

# Inputs known to differ, by corpus path suffix or "cases file: case name".
KNOWN = {
    "stage-tests/parse/bad-unicode-scalar-values/work/test/test.temper": "spans",
    "edge-cases.cases: holes in strings": "multi-quoted",
    "edge-cases.cases: blockquote with indented code then a lazy line, which crashed Kotlin;"
    " quoted code is prose": "quoted-code",
    "edge-cases.cases: quoted fenced code is prose": "quoted-code",
    "edge-cases.cases: quoted indented code is prose": "quoted-code",
    "kotlin-token-cluster-tests.cases: unclosedMQ4K": "nested-hole",
    "kotlin-token-cluster-tests.cases: dollarCurlyInsideInterpolationAndEndOfLine4A": "nested-hole",
    "kotlin-token-cluster-tests.cases: mQInStmtBlock4E": "multi-quoted",
    "kotlin-parse-tests.cases: stringEscapes": "spans",
    "kotlin-token-adapter-tests.cases: stringTemplateWithEmbeddedStatementFragments": "multi-quoted",
    "kotlin-token-adapter-tests.cases: stringsOnStringTemplateLine": "multi-quoted",
    "multi-quoted.cases: *": "multi-quoted",
}


def known(name: str) -> str | None:
    """The reason an input is known to differ, given its path or case name."""
    for key, reason in KNOWN.items():
        if key.endswith(": *") and name.startswith(key[:-1]):
            return reason
        if name == key or name.endswith("/" + key):
            return reason
    return None


def classify(text: str, markdown: bool, kotlin: list[str], rust: list[str]) -> str | None:
    """For a fuzzed input that differs, a deviation that may explain it, from
    what's in it. This is rough: it may hide real differences in inputs that
    have these features, so fuzz their own cases separately."""
    if strip_spans(kotlin) == strip_spans(rust):
        return "spans"
    if any(re.match(r"(T\t\d+\t)?Error\t", a) and a.endswith("\t$") and "{" in b.split("\t")[-1]
           for a, b in zip(kotlin, kotlin[1:])):
        return "nested-hole"
    if markdown and re.search(r"(?m)^[ \t]*>", text):
        return "quoted-code"
    if '"""' in text:
        return "multi-quoted"
    return None


def strip_spans(lines: list[str]) -> list[str]:
    """Parse tree lines without token and error positions."""
    out = []
    for line in lines:
        parts = line.split("\t")
        if parts[0] == "T":
            parts[3:5] = []
        elif parts[0] == "E":
            parts[-2:] = []
        out.append("\t".join(parts))
    return out
