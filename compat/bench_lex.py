# /// script
# requires-python = ">=3.11"
# ///
"""Times temper-syntax's lexer against Kotlin Temper's on the same files.

Each side reads all files first, then lexes them all once per round, so this
times lexing alone, without file reads or JVM startup. Kotlin gets warmup
rounds that don't count. Runs alternate between the two sides, since timing
on a busy machine drifts, and the report gives the range of per-run medians.

    uv run compat/bench_lex.py [--runs N] [--rounds N] [DIR ...]

Needs the same setup as check_lex.py.
"""

import argparse
import statistics
import subprocess
import sys
from pathlib import Path

from check_lex import DEFAULT_DIRS, build, temper_files

WARMUP_ROUNDS = 30


def main(
    oracle: str = "LexTokens", printer: str = "lex-tokens", doc: str = __doc__, flags: list[str] = []
) -> int:
    parser = argparse.ArgumentParser(description=doc.splitlines()[0])
    parser.add_argument("dirs", nargs="*", type=Path, default=DEFAULT_DIRS)
    parser.add_argument("--runs", type=int, default=3, help="runs per side")
    parser.add_argument("--rounds", type=int, default=20, help="timed rounds per run")
    for flag in flags:
        parser.add_argument(flag, action="store_true")
    args = parser.parse_args()

    kotlin_command, rust_command = build(oracle, printer)
    for flag in flags:
        if getattr(args, flag.removeprefix("--")):
            kotlin_command.append(flag)
            rust_command.append(flag)
    files = temper_files(args.dirs)
    medians: dict[str, list[float]] = {"rust": [], "kotlin": []}
    header = ""
    for _ in range(args.runs):
        for side, command, warmup in (
            ("rust", rust_command, 1),
            ("kotlin", kotlin_command, WARMUP_ROUNDS),
        ):
            header, times = bench(command, warmup, args.rounds, files)
            medians[side].append(statistics.median(times))

    print(header.removeprefix("# "))
    for side, values in medians.items():
        print(f"{side}: median {spread(values)} ms per pass over all files")
    ratios = [k / r for k, r in zip(medians["kotlin"], medians["rust"])]
    print(f"kotlin / rust: {spread(ratios, 'x')}")
    return 0


def bench(command: list[str], warmup: int, rounds: int, files: list[str]) -> tuple[str, list[float]]:
    """Runs one side, returning its header line and timed rounds in ms."""
    total = warmup + rounds
    output = subprocess.run(
        command + ["--bench", str(total)] + files, check=True, capture_output=True, text=True
    ).stdout
    lines = output.splitlines()
    times = [float(line.split("\t")[0]) for line in lines[1:]]
    return lines[0], times[warmup:]


def spread(values: list[float], unit: str = "") -> str:
    low, high = min(values), max(values)
    return f"{low:.1f}{unit}" if f"{low:.1f}" == f"{high:.1f}" else f"{low:.1f}–{high:.1f}{unit}"


if __name__ == "__main__":
    sys.exit(main())
