# /// script
# requires-python = ">=3.11"
# ///
"""Compare temper-syntax's parse trees with Kotlin Temper's parser.

Runs both on every .temper and .temper.md file under the given directories
(default: the temper and prismora checkouts beside this repository) and on the inputs in
crates/temper-syntax/tests/parse/*.cases, and reports inputs that differ,
apart from those listed in deviations.py, where we deliberately differ.
Each side prints a normalized tree (see compat/oracle/ParseTree.java): the
operator tree with every token, the parser's errors, and comments. Inputs
that crash Kotlin are counted apart.

    uv run compat/check_parse.py [--verbose] [DIR ...]

Needs the same setup as check_lex.py.
"""

import argparse
import sys
from pathlib import Path

from check_lex import CASE_NAMES, DEFAULT_DIRS, ROOT, build, by_file, compare, run, temper_files, write_cases

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
    crashed = [p for p in files if any(line.startswith("Crash") for line in kotlin.get(p, []))]
    for path in crashed:
        # A Kotlin bug, listed in the notes, not a difference of ours.
        print(f"KOTLIN CRASHED: {CASE_NAMES.get(path, path)}")
    rest = [p for p in files if p not in crashed]
    differ = compare(rest, kotlin, rust, args.verbose, "parse")
    nodes = sum(len(lines) for lines in kotlin.values())
    print(f"{len(rest) - differ} of {len(files)} inputs match Kotlin Temper ({nodes} lines)")
    return 1 if differ else 0


def commands() -> tuple[list[str], list[str]]:
    """Builds both tree printers, returning the commands to run them."""
    return build("ParseTree", "parse-tree")


if __name__ == "__main__":
    sys.exit(main())
