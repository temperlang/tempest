# /// script
# requires-python = ">=3.11"
# ///
"""Checks the links in the docs and notes, so they can't silently rot.

    uv run compat/check_anchors.py [PATH...]

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
- Reference definitions, `[name]: target`, and HTML `<a href>` and
  `<img src>` are links too.

Exits with 1 if anything is wrong. PATH, files or folders, defaults to
the README, docs/, notes/, and the playground's README.
"""

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT = ["README.md", "docs", "notes", "site/playground/README.md"]
LINK = re.compile(r"\]\(([^)\s]+)\)|^ {0,3}\[[^\]^][^\]]*\]:\s*(\S+)|<(?:a|img)\b[^>]*?\b(?:href|src)=\"([^\"]+)\"", re.M)
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
    where = page.relative_to(ROOT) if page.is_relative_to(ROOT) else page
    prose = re.sub(r"```.*?```", "", page.read_text(), flags=re.S)
    for groups in LINK.findall(prose):
        target = next(g for g in groups if g)
        if re.match(r"[a-z]+:", target):
            continue
        path, _, anchor = target.partition("#")
        file = (page.parent / path).resolve() if path else page
        if not file.exists():
            problems.append(f"{where}: ({target}): no such file")
        elif re.fullmatch(r"L\d+(-L\d+)?", anchor):
            problems.append(f"{where}: ({target}): link the file and name the item, not a line")
        elif anchor and file.suffix == ".md" and anchor not in anchors(file):
            problems.append(f"{where}: ({target}): no such heading")
    for label, target in CODE_LINK.findall(prose):
        path = target.partition("#")[0]
        file = (page.parent / path).resolve()
        if re.match(r"[a-z]+:", target) or not file.is_file() or file.suffix == ".md":
            continue
        if label in (file.name, file.stem) or (file.name in ("lib.rs", "main.rs") and label == file.parent.parent.name):
            continue
        name = ident(label)
        if not defines(file.read_text(errors="replace"), name):
            problems.append(f"{where}: [`{label}`]({target}): {path} doesn't define {name}")
    return problems


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("paths", nargs="*", default=[ROOT / d for d in DEFAULT], type=Path)
    args = parser.parse_args()
    problems = []
    for path in args.paths:
        path = path.resolve()
        for page in sorted(path.rglob("*.md")) if path.is_dir() else [path]:
            problems += check(page)
    for p in problems:
        print(p)
    print(f"{len(problems)} problems" if problems else "Every link leads where it says.")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
