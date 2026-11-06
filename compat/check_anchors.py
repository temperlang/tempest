# /// script
# requires-python = ">=3.11"
# ///
"""Checks the links in docs/ and its folders, so they can't silently rot.

    uv run compat/check_anchors.py [DOCS_DIR]

- Every relative link must lead to a file or directory that exists.
- A link to a heading, `(page.md#some-heading)` or `(#some-heading)`, must
  match a heading in that Markdown file, by GitHub's anchors: lowercase,
  spaces to `-`, most punctuation dropped, and `-1`, `-2`, … on repeats.
- A link to code names what it points at, ``[`name`](../path/file.rs)``,
  and links the file, not a line, since line numbers drift. The file must
  define the name (the last identifier in the label, so `Typing::expr`
  needs `expr`): `fn name`, `struct name`, `const name`, a `name!` macro,
  `mod name`, and so on. A label that is the file's own name, as
  `main.rs` or `strings` for `strings.rs`, passes too.
- Links never carry line numbers (`#L10`).

Exits with 1 if anything is wrong. DOCS_DIR defaults to the repo's docs/.
"""

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LINK = re.compile(r"\]\(([^)\s]+)\)")
CODE_LINK = re.compile(r"\[`([^`]+)`\]\(([^)\s]+)\)")
DEFINERS = r"fn|struct|enum|const|type|trait|static|mod|macro_rules!|function|class|interface"


def ident(label: str) -> str:
    """The last identifier in a link's label: `naming::assign` gives
    `assign`, `schema!` gives `schema`."""
    words = re.findall(r"[A-Za-z_][A-Za-z0-9_]*", label)
    return words[-1] if words else label


def defines(text: str, name: str) -> bool:
    n = re.escape(name)
    return bool(re.search(rf"\b({DEFINERS})\s+{n}\b|^\s*{n}!\s*\{{", text, re.M))


def slug(heading: str) -> str:
    """A heading's anchor, as GitHub makes it."""
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", heading.strip().lower())
    text = re.sub(r"[^\w\- ]", "", text)
    return text.replace(" ", "-")


def anchors(page: Path) -> set[str]:
    found: set[str] = set()
    counts: dict[str, int] = {}
    fenced = False
    for line in page.read_text().splitlines():
        if line.startswith("```"):
            fenced = not fenced
        elif not fenced and (m := re.match(r"#{1,6}\s+(.*)", line)):
            base = slug(m.group(1))
            n = counts.get(base, 0)
            counts[base] = n + 1
            found.add(base if n == 0 else f"{base}-{n}")
    return found


def check(page: Path) -> list[str]:
    problems = []
    prose = re.sub(r"```.*?```", "", page.read_text(), flags=re.S)
    for target in LINK.findall(prose):
        if re.match(r"[a-z]+:", target):
            continue
        path, _, anchor = target.partition("#")
        file = (page.parent / path).resolve() if path else page
        if not file.exists():
            problems.append(f"{page.name}: ({target}): no such file")
        elif re.fullmatch(r"L\d+(-L\d+)?", anchor):
            problems.append(f"{page.name}: ({target}): link the file and name the item, not a line")
        elif anchor and file.suffix == ".md" and anchor not in anchors(file):
            problems.append(f"{page.name}: ({target}): no such heading")
    for label, target in CODE_LINK.findall(prose):
        path = target.partition("#")[0]
        file = (page.parent / path).resolve()
        if re.match(r"[a-z]+:", target) or not file.is_file() or file.suffix == ".md":
            continue
        if label in (file.name, file.stem) or (file.name in ("lib.rs", "main.rs") and label == file.parent.parent.name):
            continue
        name = ident(label)
        if not defines(file.read_text(errors="replace"), name):
            problems.append(f"{page.name}: [`{label}`]({target}): {path} doesn't define {name}")
    return problems


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("docs", nargs="?", default=ROOT / "docs", type=Path)
    args = parser.parse_args()
    problems = []
    for page in sorted(args.docs.rglob("*.md")):
        problems += check(page)
    for p in problems:
        print(p)
    print(f"{len(problems)} problems" if problems else "Every link leads where it says.")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
