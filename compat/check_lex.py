# /// script
# requires-python = ">=3.11"
# ///
"""Compare temper-syntax's tokens with Kotlin Temper's lexer.

Runs both on every .temper and .temper.md file under the given directories
(default: the temper and prismora checkouts beside this repository), and reports files that
differ. Each side prints one line per token (see compat/oracle/LexTokens.java).

    uv run compat/check_lex.py [--verbose] [DIR ...]

Needs an installed Kotlin Temper (TEMPER_LIB, default below), a JDK and cargo.
"""

import argparse
import difflib
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
DEFAULT_LIB = Path(__file__).resolve().parents[2] / "temper/cli/build/install/temper/lib"
DEFAULT_DIRS = [Path(__file__).resolve().parents[2] / "temper", Path(__file__).resolve().parents[2] / "prismora"]
SKIP_PARTS = {"build", "node_modules", "temper.out"}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("dirs", nargs="*", type=Path, default=DEFAULT_DIRS)
    parser.add_argument("--verbose", action="store_true", help="show diffs")
    args = parser.parse_args()

    lib = Path(os.environ.get("TEMPER_LIB", DEFAULT_LIB))
    classpath = os.pathsep.join(str(jar) for jar in sorted(lib.glob("*.jar")))
    classes = build_oracle(classpath)
    subprocess.run(
        ["cargo", "build", "-q", "--release", "-p", "temper-syntax"],
        cwd=ROOT,
        check=True,
    )

    files = sorted(
        str(path)
        for directory in args.dirs
        for pattern in ("*.temper", "*.temper.md")
        for path in directory.rglob(pattern)
        if path.is_file() and not SKIP_PARTS & set(path.parts)
    )
    kotlin = by_file(run(["java", "-cp", f"{classes}{os.pathsep}{classpath}", "LexTokens"], files))
    rust = by_file(run([str(ROOT / "target/release/lex-tokens")], files))

    differ = 0
    for path in files:
        expected, actual = kotlin.get(path, []), rust.get(path, [])
        if expected != actual:
            differ += 1
            print(f"DIFFERS: {path}")
            if args.verbose:
                diff = difflib.unified_diff(expected, actual, "kotlin", "rust", lineterm="")
                print("\n".join(list(diff)[:20]))
    tokens = sum(len(lines) for lines in kotlin.values())
    print(f"{len(files) - differ} of {len(files)} files match Kotlin Temper ({tokens} tokens)")
    return 1 if differ else 0


def build_oracle(classpath: str) -> Path:
    """Compiles the Java oracle if it's missing or out of date."""
    source = HERE / "oracle/LexTokens.java"
    classes = HERE / "oracle/classes"
    compiled = classes / "LexTokens.class"
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
