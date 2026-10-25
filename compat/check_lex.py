# /// script
# requires-python = ">=3.11"
# ///
"""Compare temper-syntax's tokens with Kotlin Temper's lexer.

Runs both on every .temper and .temper.md file under the given directories
(default: the temper, chromarite and ordable checkouts beside this repository) and on the cases in
crates/temper-syntax/tests/*.cases, and reports inputs that differ, apart
from those listed in deviations.py, where we deliberately differ. For the
cases, it also checks that the expected tokens saved with them are current. Each side prints
one line per token (see compat/oracle/LexTokens.java).

    uv run compat/check_lex.py [--verbose] [DIR ...]

Needs an installed Kotlin Temper (TEMPER_LIB, default below), a JDK and cargo.
"""

import argparse
import difflib
import os
import subprocess
import sys
from pathlib import Path

from deviations import REASONS, known, known_file

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
DEFAULT_LIB = Path(__file__).resolve().parents[2] / "temper/cli/build/install/temper/lib"
DEFAULT_DIRS = [
    Path(__file__).resolve().parents[2] / "temper",
    Path(__file__).resolve().parents[2] / "chromarite",
    Path(__file__).resolve().parents[2] / "ordable",
]
SKIP_PARTS = {"build", "node_modules", "temper.out"}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("dirs", nargs="*", type=Path, default=DEFAULT_DIRS)
    parser.add_argument("--verbose", action="store_true", help="show diffs")
    args = parser.parse_args()

    kotlin_command, rust_command = build()
    cases = write_cases()
    files = temper_files(args.dirs) + sorted(cases)
    kotlin = by_file(run(kotlin_command, files))
    rust = by_file(run(rust_command, files))
    for path, saved in cases.items():
        if rust.get(path, []) != saved:
            print(f"STALE: saved tokens for {CASE_NAMES[path]} (cargo test shows them)")
    differ = compare(files, kotlin, rust, args.verbose)
    tokens = sum(len(lines) for lines in kotlin.values())
    print(f"{len(files) - differ} of {len(files)} inputs match Kotlin Temper ({tokens} tokens)")
    return 1 if differ else 0


def compare(files: list[str], kotlin: dict, rust: dict, verbose: bool, check: str = "lex") -> int:
    """Reports differences except deliberate ones. Returns how many inputs
    differ, counting deliberate differences as matching."""
    differ = 0
    expected: dict[str, int] = {}
    for path in files:
        theirs, ours = kotlin.get(path, []), rust.get(path, [])
        reason = known(CASE_NAMES.get(path, path), check)
        if theirs == ours:
            # Whole files of cases may hold some that match.
            if reason and not known_file(CASE_NAMES.get(path, path)):
                print(f"LISTED BUT MATCHES: {CASE_NAMES.get(path, path)} ({reason})")
            continue
        if reason:
            expected[reason] = expected.get(reason, 0) + 1
            if verbose:
                print(f"EXPECTED ({reason}): {CASE_NAMES.get(path, path)}")
            continue
        differ += 1
        print(f"DIFFERS: {CASE_NAMES.get(path, path)} ({path})")
        if verbose:
            diff = difflib.unified_diff(theirs, ours, "kotlin", "rust", lineterm="")
            print("\n".join(list(diff)[:30]))
    for reason, count in sorted(expected.items()):
        print(f"{count} inputs differ as expected: {REASONS[reason]} ({reason})")
    return differ


def build(oracle: str = "LexTokens", printer: str = "lex-tokens") -> tuple[list[str], list[str]]:
    """Builds a Java oracle and our matching printer, returning the commands
    to run them."""
    lib = Path(os.environ.get("TEMPER_LIB", DEFAULT_LIB))
    classpath = os.pathsep.join(str(jar) for jar in sorted(lib.glob("*.jar")))
    classes = build_oracle(classpath, oracle)
    subprocess.run(
        ["cargo", "build", "-q", "--release", "-p", "temper-syntax"],
        cwd=ROOT,
        check=True,
    )
    kotlin = ["java", "-cp", f"{classes}{os.pathsep}{classpath}", oracle]
    return kotlin, [str(ROOT / "target/release" / printer)]


CASES = ROOT / "crates/temper-syntax/tests"
CASE_INPUTS = ROOT / "target/lex-cases"
# For each case's input file, "file.cases: case name".
CASE_NAMES: dict[str, str] = {}


def read_cases(path: Path, separator: str = "tokens") -> list[tuple[str, bool, str, list[str]]]:
    """Reads a .cases file as (name, markdown, input, expected lines) per
    case, in the format described in crates/temper-syntax/tests/lex.rs."""
    cases = []
    text = path.read_text("utf-8")
    for chunk in ("\n" + text).split("\n=== ")[1:]:
        header, _, rest = chunk.partition("\n")
        before, _, tokens = ("\n" + rest).partition(f"\n--- {separator}\n")
        name = header.removesuffix(" (markdown)")
        lines = [line for line in tokens.split("\n") if line]
        cases.append((name, name != header, before[1:], lines))
    return cases


def write_cases(
    cases: Path = CASES, inputs: Path = CASE_INPUTS, separator: str = "tokens"
) -> dict[str, list[str]]:
    """Writes each case's input to its own file under target/, returning the
    saved output for each file."""
    saved = {}
    for path in sorted(cases.glob("*.cases")):
        names = [name for name, *_ in read_cases(path, separator)]
        directory = inputs / path.stem
        directory.mkdir(parents=True, exist_ok=True)
        for f in directory.iterdir():
            f.unlink()
        for i, (_, markdown, text, tokens) in enumerate(read_cases(path, separator)):
            file = directory / (f"{i:03}.temper" + (".md" if markdown else ""))
            file.write_text(text, "utf-8", newline="")
            saved[str(file)] = tokens
            CASE_NAMES[str(file)] = f"{path.name}: {names[i]}"
    return saved


def temper_files(dirs: list[Path]) -> list[str]:
    """All .temper and .temper.md files under the directories, sorted."""
    return sorted(
        str(path)
        for directory in dirs
        for pattern in ("*.temper", "*.temper.md")
        for path in directory.rglob(pattern)
        if path.is_file() and not SKIP_PARTS & set(path.parts)
    )


def build_oracle(classpath: str, name: str = "LexTokens") -> Path:
    """Compiles a Java oracle if it's missing or out of date."""
    source = HERE / f"oracle/{name}.java"
    classes = HERE / "oracle/classes"
    compiled = classes / f"{name}.class"
    if not compiled.exists() or source.stat().st_mtime > compiled.stat().st_mtime:
        classes.mkdir(parents=True, exist_ok=True)
        subprocess.run(
            ["javac", "-nowarn", "-cp", classpath, "-d", str(classes), str(source)],
            check=True,
        )
    return classes


def run(command: list[str], files: list[str]) -> str:
    """Runs a token printer on all files at once and returns its output."""
    result = subprocess.run(command + files, check=True, capture_output=True)
    return result.stdout.decode("utf-8")


def by_file(output: str) -> dict[str, list[str]]:
    """Splits printer output into lines per file, keyed by path."""
    tokens: dict[str, list[str]] = {}
    current: list[str] = []
    for line in output.split("\n"):
        if line.startswith("# "):
            current = tokens.setdefault(line[2:], [])
        elif line:
            current.append(line)
    return tokens


if __name__ == "__main__":
    sys.exit(main())
