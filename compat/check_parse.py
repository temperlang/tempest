# /// script
# requires-python = ">=3.11"
# ///
"""Compare temper-syntax's parse trees with Kotlin Temper's parser.

Runs both on every .temper and .temper.md file under the given directories
(default: the temper and prismora checkouts beside this repository) and on the inputs in
crates/temper-syntax/tests/parse/*.cases, and reports inputs that differ.
Each side prints a normalized tree (see compat/oracle/ParseTree.java): the
operator tree with every token, the parser's errors, and comments. Ours runs
with --kotlin-quirks, reproducing Kotlin where we deliberately differ (bugs
and deviations listed in notes/testing-and-milestones.md); the report counts
the inputs where that matters.

    uv run compat/check_parse.py [--verbose] [DIR ...]

Needs the same setup as check_lex.py.
"""

import argparse
import difflib
import subprocess
import sys
from pathlib import Path

from check_lex import (
    DEFAULT_DIRS, QUIRKS, ROOT, build, by_file, report_deviations, run, temper_files, write_cases
)

CASES = ROOT / "crates/temper-syntax/tests/parse"
CASE_INPUTS = ROOT / "target/parse-cases"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("dirs", nargs="*", type=Path, default=DEFAULT_DIRS)
    parser.add_argument("--verbose", action="store_true", help="show diffs")
    args = parser.parse_args()

    kotlin_command, rust_command = commands()
    cases = write_cases(CASES, CASE_INPUTS, "tree")
    files = temper_files(args.dirs) + sorted(cases)
    kotlin = by_file(run(kotlin_command, files))
    rust = by_file(run(rust_command, files))
    plain = by_file(run(rust_command[:1], files))

    differ = crashes = 0
    for path in files:
        expected, actual = kotlin.get(path, []), rust.get(path, [])
        if any(line.startswith("Crash") for line in expected):
            # A Kotlin bug, listed in the notes, not a difference of ours.
            crashes += 1
            if args.verbose:
                print(f"KOTLIN CRASHED: {path}")
        elif expected != actual:
            differ += 1
            print(f"DIFFERS: {path}")
            if args.verbose:
                diff = difflib.unified_diff(expected, actual, "kotlin", "rust", lineterm="")
                print("\n".join(list(diff)[:30]))
    nodes = sum(len(lines) for lines in kotlin.values())
    print(f"{len(files) - differ - crashes} of {len(files)} inputs match Kotlin Temper ({nodes} lines)")
    if crashes:
        print(f"Kotlin crashed on {crashes} inputs (--verbose lists them)")
    report_deviations(files, rust, plain, args.verbose)
    return 1 if differ else 0


def commands() -> tuple[list[str], list[str]]:
    """Builds both tree printers, returning the commands to run them, ours
    with Kotlin's quirks."""
    kotlin, rust = build("ParseTree", "parse-tree")
    return kotlin, rust + [QUIRKS]


if __name__ == "__main__":
    sys.exit(main())
