# /// script
# requires-python = ">=3.11"
# ///
"""Times the settle loop on every Temper file, each as its own library.

Alternates two passes over all files: up to the end of the module step
(`ast-tree --module --bench`), and the same plus copying into a library
arena and settling (`run-temper --bench`). The difference is the settle
loop's cost. There's no Kotlin side: its frontend stages don't map onto
ours, and this compares nothing it could run alone.

    uv run compat/bench_settle.py [--runs N] [--rounds N] [DIR ...]
"""

import argparse
import statistics
import subprocess
import sys
from pathlib import Path

from bench_lex import bench, spread
from check_lex import DEFAULT_DIRS, ROOT, temper_files


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("dirs", nargs="*", type=Path, default=DEFAULT_DIRS)
    parser.add_argument("--runs", type=int, default=3, help="runs per side")
    parser.add_argument("--rounds", type=int, default=20, help="timed rounds per run")
    args = parser.parse_args()

    subprocess.run(["cargo", "build", "-q", "--release"], cwd=ROOT, check=True)
    release = ROOT / "target/release"
    sides = {
        "module step": [str(release / "ast-tree"), "--module"],
        "settled": [str(release / "run-temper")],
    }
    files = temper_files(args.dirs)
    medians: dict[str, list[float]] = {side: [] for side in sides}
    header = ""
    for _ in range(args.runs):
        for side, command in sides.items():
            header, times = bench(command, 1, args.rounds, files)
            medians[side].append(statistics.median(times))
    print(header.removeprefix("# "))
    for side, values in medians.items():
        print(f"{side}: median {spread(values)} ms per pass over all files")
    settle = [s - m for s, m in zip(medians["settled"], medians["module step"])]
    print(f"settling alone: {spread(settle)} ms")
    return 0


if __name__ == "__main__":
    sys.exit(main())
