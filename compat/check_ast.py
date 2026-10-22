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

    uv run compat/check_ast.py [--verbose] [--module | --docs] [DIR ...]

With --module (or --docs), both sides also do the rest of Kotlin's
ParseStage for a library (or documentation): call joins rewritten
(rewriteCallJoins) and metadata converted to JSON with InvalidMetadata
errors; the inputs then include crates/temper-syntax/tests/module/*.cases.

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
MODULE_CASES = ROOT / "crates/temper-syntax/tests/module"
MODULE_CASE_INPUTS = ROOT / "target/module-cases"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("dirs", nargs="*", type=Path, default=DEFAULT_DIRS)
    parser.add_argument("--verbose", action="store_true", help="show diffs")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--module", action="store_true", help="add ParseStage's steps, for a library")
    mode.add_argument("--docs", action="store_true", help="add ParseStage's steps, for documentation")
    args = parser.parse_args()

    flag = "--module" if args.module else "--docs" if args.docs else None
    kotlin_command, rust_command = commands()
    if flag:
        kotlin_command.append(flag)
        rust_command.append(flag)
    cases = write_cases(PARSE_CASES, PARSE_CASE_INPUTS, "tree") | write_cases(CASES, CASE_INPUTS, "ast")
    if flag:
        cases |= write_cases(MODULE_CASES, MODULE_CASE_INPUTS, "module")
    files = temper_files(args.dirs) + sorted(cases)
    kotlin = by_file(run(kotlin_command, files))
    rust = by_file(run(rust_command, files))
    if flag:
        kotlin = {path: mask_delegate_spans(lines) for path, lines in kotlin.items()}
        rust = {path: mask_delegate_spans(lines) for path, lines in rust.items()}
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


def mask_delegate_spans(lines: list[str]) -> list[str]:
    """Masks the spans of the function, call and name that rewriting a call
    join makes, as `fn (f#0) { f#0(...) }`, where we deliberately differ
    (REASONS["join-spans"]). Their declaration's spans are compared."""
    def is_temporary(line: str, kind: str) -> bool:
        parts = line.split("\t")
        return len(parts) >= 6 and parts[0] == "L" and parts[2] == kind and "#" in parts[5]

    def mask(line: str) -> str:
        parts = line.split("\t")
        return "\t".join(parts[:3] + ["*", "*"] + parts[5:])

    out = list(lines)
    for i, line in enumerate(lines):
        rest = lines[i + 1 : i + 3]
        if line.startswith("N\t") and line.split("\t")[2] == "Fun" and len(rest) == 2:
            if rest[0].split("\t")[2:3] == ["Decl"] and is_temporary(rest[1], "LeftName"):
                out[i] = mask(line)
        elif line.startswith("N\t") and line.split("\t")[2] == "Call" and rest:
            if is_temporary(rest[0], "RightName"):
                out[i] = mask(line)
        elif is_temporary(line, "RightName"):
            out[i] = mask(line)
    return out


def commands() -> tuple[list[str], list[str]]:
    """Builds both tree printers, returning the commands to run them."""
    return build("AstTree", "ast-tree")


if __name__ == "__main__":
    sys.exit(main())
