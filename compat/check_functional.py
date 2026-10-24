# /// script
# requires-python = ">=3.11"
# ///
"""Runs Kotlin Temper's functional tests on our settle loop and interpreter.

Each test is a directory under functional-test-suite/src/commonMain/resources
whose main file is named after it (`fibonacci/fibonacci.temper.md`). The
directory's own .temper and .temper.md files are built as one library with
crates/temper-frontend's `run-temper`, and what it logs is compared with the
expected output: the main file's ```log blocks, joined, as Kotlin's
MarkdownFileBasedFunctionalTestBase reads them. A test without any passes
only if it builds without errors and runs without failing, or, if its
metadata says `expectRunFailure = true`, fails. A test whose metadata says
`runAsTest = true` instead runs its `test` cases, and passes if the ones
that fail are those its `expectedFailures` names, each failure's message
containing the expected text, as Kotlin's `assertTestingTestFromJunit`
checks. Doesn't run Kotlin Temper, but takes `std` imports from its source
tree.

    uv run compat/check_functional.py [--verbose] [TEST ...]

TEST filters by path, as `algos/fibonacci` or `fibonacci`. --verbose shows
diagnostics and a diff for failing tests.
"""

import argparse
import difflib
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SUITE = Path(__file__).resolve().parents[2] / "temper/functional-test-suite/src/commonMain/resources"
STD = Path(__file__).resolve().parents[2] / "temper/frontend/src/commonMain/resources/std"
START = re.compile(r"^[ \t]*```log[ \t]*$")
END = re.compile(r"^[ \t]*```[ \t]*$")
# `"name" to """message"""` or `"name" to "message"` in `expectedFailures`.
FAILURE = re.compile(r'"((?:[^"\\]|\\.)*)"\s+to\s+(?:"""(.*?)"""|"((?:[^"\\]|\\.)*)")', re.S)


def expected_output(main: Path) -> list[str]:
    """The lines in the main file's ```log blocks."""
    lines, inside = [], False
    for line in main.read_text("utf-8").splitlines():
        if (END if inside else START).match(line):
            inside = not inside
        elif inside:
            lines.append(line)
    return lines


def expected_failures(main_text: str) -> dict[str, str]:
    """The `expectedFailures` metadata: each failing test's name and part
    of its message."""
    meta = [line.split("@meta:arg", 1)[1].strip() for line in main_text.splitlines() if "@meta:arg" in line]
    start = next((i for i, line in enumerate(meta) if line.startswith("expectedFailures")), None)
    if start is None:
        return {}
    end = next((i for i in range(start, len(meta)) if meta[i] in (")", "),")), len(meta))
    entries = FAILURE.finditer("\n".join(meta[start : end + 1]))
    return {m[1]: m[2] if m[2] is not None else m[3] for m in entries}


def test_results_ok(stdout: list[str], expected: dict[str, str]) -> bool:
    """Whether the JUnit reports in a test run's output fail as expected."""
    reports = re.findall(r"<testsuites>.*?</testsuites>", "\n".join(stdout), re.S)
    failures = []
    for report in reports:
        for case in ET.fromstring(report).iter("testcase"):
            failures += [(case.get("name", ""), f.get("message", "")) for f in case.iter("failure")]
    if not reports:
        return False
    names = sorted(name.lower() for name, _ in failures)
    if names != sorted(name.lower() for name in expected):
        return False
    return all(any(cause in message for _, message in failures) for cause in expected.values())


def tests() -> list[tuple[str, Path]]:
    """Each test's name, as `algos/fibonacci`, and its directory."""
    found = []
    for main in sorted(SUITE.rglob("*.temper.md")):
        if main.name == f"{main.parent.name}.temper.md":
            found.append((str(main.parent.relative_to(SUITE)), main.parent))
    return found


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("tests", nargs="*")
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()

    subprocess.run(["cargo", "build", "-q", "--release", "-p", "temper-frontend"], cwd=ROOT, check=True)
    runner = ROOT / "target/release/run-temper"
    passed, failed = [], []
    for name, directory in tests():
        if args.tests and not any(t in name for t in args.tests):
            continue
        files = sorted(str(p) for p in directory.iterdir() if p.name.endswith((".temper", ".temper.md")))
        expected = expected_output(directory / f"{directory.name}.temper.md")
        main_text = (directory / f"{directory.name}.temper.md").read_text("utf-8")
        expect_failure = "expectRunFailure = true" in main_text
        as_test = "runAsTest = true" in main_text
        test_flag = ["--test"] if as_test else []
        try:
            result = subprocess.run([str(runner), *test_flag, "--std", str(STD), *files], capture_output=True, timeout=60)
            actual = result.stdout.decode("utf-8", "replace").splitlines()
            errors = result.stderr.decode("utf-8", "replace")
            clean = result.returncode == 0 and not any(line.startswith("error ") for line in errors.splitlines())
        except subprocess.TimeoutExpired:
            actual, errors, clean = [], "timed out", False
        if as_test:
            ok = clean and test_results_ok(actual, expected_failures(main_text))
        else:
            ok = actual == expected and (expected or clean != expect_failure)
        if ok:
            passed.append(name)
            continue
        failed.append(name)
        if args.verbose:
            print(f"--- {name}")
            print(errors.rstrip())
            for line in difflib.unified_diff(expected, actual, "expected", "actual", lineterm="", n=1):
                print(line)
    print(f"{len(passed)} of {len(passed) + len(failed)} functional tests pass:")
    for name in passed:
        print(f"  {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
