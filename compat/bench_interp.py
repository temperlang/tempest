# /// script
# requires-python = ">=3.11"
# ///
"""Interpreter speed: tempest's `run-temper` against Kotlin Temper's
interpreter, run in one warmed-up JVM.

    uv run compat/bench_interp.py [--runs N] [--rounds N] [BENCHMARK ...]

Each program in `benchmarks/interp/` runs a loop `N` times. Running it at a
small and a large `N` and dividing the time difference by the difference in
`N` gives the cost per iteration, without startup or compiling. Ours runs
as `run-temper` processes (startup is a few milliseconds and cancels out);
Kotlin's runs through compat/oracle/InterpBench.java, which builds and runs
each program in one JVM round after round, so the first rounds, while the
JIT warms up, can be dropped. A run is one pass over every benchmark per
side; runs alternate between the sides, since timing on the dev machine
drifts, and the report gives the range of per-run medians. The "small"
column is a whole small run: ours includes process startup; Kotlin's
is compiling and running in the warm JVM, without its startup.

Needs the same setup as check_lex.py.
"""

import argparse
import os
import statistics
import subprocess
import sys
import time
from pathlib import Path

from check_lex import DEFAULT_LIB, build_oracle

ROOT = Path(__file__).resolve().parent.parent
PROGRAMS = ROOT / "benchmarks" / "interp"
RUN_TEMPER = ROOT / "target" / "release" / "run-temper"
# Small and large N per side: Kotlin's interpreter is slower, so it gets
# fewer iterations to keep runs short. fib divides them by FIB_SCALE.
SIZES = {"rust": (10_000, 1_000_000), "kotlin": (1_000, 20_000)}
FIB_SCALE = 100  # fib(12) per iteration does ~465 calls.
WARMUP_ROUNDS = 8


def sizes(side: str, name: str) -> tuple[int, int]:
    small, large = SIZES[side]
    return (small // FIB_SCALE, large // FIB_SCALE) if name == "fib" else (small, large)


def run_rust(source: str, n: int, scratch: Path) -> tuple[float, float]:
    """Runs a program with `run-temper`, returning (seconds, peak RSS MiB)."""
    path = scratch / "bench.temper"
    path.write_text(source.replace("N", str(n)))
    start = time.perf_counter()
    proc = subprocess.Popen([RUN_TEMPER, path], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    out = proc.stdout.read()
    # Reap it here to get its resource usage.
    _, status, rusage = os.wait4(proc.pid, 0)
    seconds = time.perf_counter() - start
    if status != 0 or len(out.splitlines()) != 1:
        sys.exit(f"run-temper failed:\n{out}")
    return seconds, rusage.ru_maxrss / 1024


def rust_pass(names: list[str], rounds: int, scratch: Path) -> dict[str, tuple[float, float, float]]:
    """Per benchmark, the median per-iteration and small-run seconds, and
    peak memory."""
    result = {}
    for name in names:
        source = (PROGRAMS / f"{name}.temper").read_text()
        small, large = sizes("rust", name)
        per, whole, peak = [], [], 0.0
        for _ in range(rounds):
            t_small, _ = run_rust(source, small, scratch)
            t_large, p = run_rust(source, large, scratch)
            per.append((t_large - t_small) / (large - small))
            whole.append(t_small)
            peak = max(peak, p)
        result[name] = (statistics.median(per), statistics.median(whole), peak)
    return result


def kotlin_pass(names: list[str], rounds: int, command: list[str]) -> dict[str, tuple[float, float, float]]:
    """Like `rust_pass`, through the harness: one JVM per benchmark, so each
    gets its own sizes; peak memory is the JVM's."""
    result = {}
    for name in names:
        small, large = sizes("kotlin", name)
        args = [str(WARMUP_ROUNDS + rounds), str(small), str(large), str(PROGRAMS / f"{name}.temper")]
        proc = subprocess.Popen(command + args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        out = proc.stdout.read()
        _, status, rusage = os.wait4(proc.pid, 0)
        if status != 0:
            sys.exit(f"InterpBench failed:\n{out}")
        per, whole = [], []
        for line in out.splitlines():
            _, round_, a, b = line.split("\t")
            if int(round_) >= WARMUP_ROUNDS:
                per.append((int(b) - int(a)) / 1e9 / (large - small))
                whole.append(int(a) / 1e9)
        result[name] = (statistics.median(per), statistics.median(whole), rusage.ru_maxrss / 1024)
    return result


def spread(values: list[float], scale: float, unit: str) -> str:
    def text(x: float) -> str:
        return f"{x:.0f}" if x >= 100 else f"{x:.2g}"

    low, high = text(min(values) * scale), text(max(values) * scale)
    return f"{low} {unit}" if low == high else f"{low}–{high} {unit}"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("benchmarks", nargs="*")
    parser.add_argument("--runs", type=int, default=3, help="runs per side")
    parser.add_argument("--rounds", type=int, default=5, help="timed rounds per benchmark and run")
    args = parser.parse_args()
    names = args.benchmarks or sorted(p.stem for p in PROGRAMS.glob("*.temper"))

    subprocess.run(["cargo", "build", "-q", "--release", "-p", "temper-frontend", "--bin", "run-temper"], cwd=ROOT, check=True)
    classpath = os.pathsep.join(str(jar) for jar in sorted(Path(os.environ.get("TEMPER_LIB", DEFAULT_LIB)).glob("*.jar")))
    classes = build_oracle(classpath, "InterpBench")
    kotlin = ["java", "-cp", f"{classes}{os.pathsep}{classpath}", "InterpBench"]
    scratch = ROOT / "target" / "bench-interp"
    scratch.mkdir(parents=True, exist_ok=True)

    runs: dict[str, list[dict]] = {"rust": [], "kotlin": []}
    for _ in range(args.runs):
        runs["rust"].append(rust_pass(names, args.rounds, scratch))
        runs["kotlin"].append(kotlin_pass(names, args.rounds, kotlin))

    print(f"{'benchmark':<10} {'side':<7} {'per iteration':>16} {'small run':>14} {'peak':>9}")
    for name in names:
        for side in ("rust", "kotlin"):
            per = [r[name][0] for r in runs[side]]
            whole = [r[name][1] for r in runs[side]]
            peak = max(r[name][2] for r in runs[side])
            print(f"{name:<10} {side:<7} {spread(per, 1e6, 'µs'):>16} {spread(whole, 1e3, 'ms'):>14} {peak:>6.0f}MiB")
        ratios = [k[name][0] / r[name][0] for k, r in zip(runs["kotlin"], runs["rust"])]
        print(f"{'':<10} {'ratio':<7} {spread(ratios, 1, 'x'):>16}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
