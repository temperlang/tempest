# /// script
# requires-python = ">=3.11"
# ///
"""Compare temper-syntax's grammar step with Kotlin Temper's (astbuild).

Runs both on every .temper and .temper.md file under the given directories
(default: the temper and prismora checkouts beside this repository) and on the inputs in
crates/temper-syntax/tests/{parse,ast}/*.cases, and reports inputs that
differ, apart from those listed in deviations.py. Each side prints a
normalized tree (see compat/oracle/AstTree.java): calls, blocks,
declarations, functions, names, values and error nodes with their spans,
then the grammar step's diagnostics. Inputs that crash Kotlin are counted
apart.

Tree positions come from parse node spans, which differ from Kotlin's where
a separator adopts operands (the "spans" deviation), so inputs whose trees
and diagnostics match apart from positions count as that deviation. Exact
positions are compared everywhere else.

    uv run compat/check_ast.py [--verbose] [DIR ...]

Needs the same setup as check_lex.py.
"""

import argparse
import sys
from pathlib import Path

from check_lex import CASE_NAMES, DEFAULT_DIRS, ROOT, build, by_file, compare, run, temper_files, write_cases
from deviations import REASONS, known, strip_spans
from check_parse import CASES as PARSE_CASES, CASE_INPUTS as PARSE_CASE_INPUTS

CASES = ROOT / "crates/temper-syntax/tests/ast"
CASE_INPUTS = ROOT / "target/ast-cases"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("dirs", nargs="*", type=Path, default=DEFAULT_DIRS)
    parser.add_argument("--verbose", action="store_true", help="show diffs")
    args = parser.parse_args()

    kotlin_command, rust_command = commands()
    cases = write_cases(PARSE_CASES, PARSE_CASE_INPUTS, "tree") | write_cases(CASES, CASE_INPUTS, "ast")
    files = temper_files(args.dirs) + sorted(cases)
    kotlin = by_file(run(kotlin_command, files))
    rust = by_file(run(rust_command, files))
    crashed = [p for p in files if any(line.startswith("Crash") for line in kotlin.get(p, []))]
    for path in crashed:
        print(f"KOTLIN CRASHED: {CASE_NAMES.get(path, path)}")
    rest = [p for p in files if p not in crashed]
    spans = [
        p
        for p in rest
        if kotlin.get(p) != rust.get(p)
        and not known(CASE_NAMES.get(p, p), "ast")
        and strip_spans(kotlin.get(p, [])) == strip_spans(rust.get(p, []))
    ]
    for path in spans:
        rust[path] = kotlin[path]
    if spans:
        print(f"{len(spans)} inputs differ only in positions: {REASONS['spans']} (spans)")
    differ = compare(rest, kotlin, rust, args.verbose, "ast")
    nodes = sum(len(lines) for lines in kotlin.values())
    print(f"{len(rest) - differ} of {len(files)} inputs match Kotlin Temper ({nodes} lines)")
    return 1 if differ else 0


def commands() -> tuple[list[str], list[str]]:
    """Builds both tree printers, returning the commands to run them."""
    return build("AstTree", "ast-tree")


if __name__ == "__main__":
    sys.exit(main())
