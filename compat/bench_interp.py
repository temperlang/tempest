# /// script
# requires-python = ">=3.11"
# ///
"""Interpreter speed and memory: temper-rs's `run-temper` against Kotlin
Temper's interpreter, driven through `temper repl`.

    uv run compat/bench_interp.py [--repeat N] [BENCHMARK ...]

Each program in `benchmarks/interp/` runs a loop `N` times. Running it at a
small and a large `N` and dividing the time difference by the difference in
`N` gives the cost per iteration, without process startup or compiling,
which are reported apart (the small run's time). Sizes alternate between
implementations and repeat, since timing on the dev machine is noisy; the
best of each is reported. Programs define and call a function, because the
REPL rejects a loop that follows another statement at top level.
"""

import argparse
import os
import shutil
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PROGRAMS = ROOT / "benchmarks" / "interp"
RUN_TEMPER = ROOT / "target" / "release" / "run-temper"
# Small and large N per implementation: Kotlin's interpreter is slower, so it
# gets fewer iterations to keep runs short.
SIZES = {"rs": (10_000, 1_000_000), "classic": (1_000, 40_000)}
FIB_SCALE = 100  # fib(12) per iteration does ~465 calls.


def measure(args, stdin=None):
    """Runs a command, returning (seconds, peak RSS in MiB, output)."""
    start = time.perf_counter()
    proc = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    if stdin:
        proc.stdin.write(stdin)
    proc.stdin.close()
    out = proc.stdout.read()
    # Reap it here rather than with proc.wait(), to get its resource usage,
    # which covers its children too (the JVM under `temper`'s script).
    _, _, rusage = os.wait4(proc.pid, 0)
    proc.returncode = 0
    return time.perf_counter() - start, rusage.ru_maxrss / 1024, out


def run(impl, source, n, scratch):
    text = source.replace("N", str(n))
    if impl == "rs":
        path = scratch / "bench.temper"
        path.write_text(text)
        seconds, peak, out = measure([RUN_TEMPER, path])
    else:
        seconds, peak, out = measure(["temper", "repl"], text)
    if "fail" in out or "rror" in out:
        raise RuntimeError(f"{impl} failed:\n{out}")
    return seconds, peak


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("benchmarks", nargs="*")
    parser.add_argument("--repeat", type=int, default=2)
    args = parser.parse_args()
    names = args.benchmarks or sorted(p.stem for p in PROGRAMS.glob("*.temper"))
    subprocess.run(["cargo", "build", "-q", "--release", "-p", "temper-frontend", "--bin", "run-temper"], cwd=ROOT, check=True)
    scratch = ROOT / "target" / "bench-interp"
    scratch.mkdir(parents=True, exist_ok=True)
    have_classic = shutil.which("temper") is not None
    impls = ["rs", "classic"] if have_classic else ["rs"]
    print(f"{'benchmark':<10} {'impl':<8} {'per iter':>10} {'startup':>9} {'peak':>9}")
    for name in names:
        source = (PROGRAMS / f"{name}.temper").read_text()
        best = {}
        for _ in range(args.repeat):
            for impl in impls:
                small, large = SIZES[impl]
                if name == "fib":
                    small, large = max(1, small // FIB_SCALE), max(2, large // FIB_SCALE)
                t_small, _ = run(impl, source, small, scratch)
                t_large, peak = run(impl, source, large, scratch)
                per = (t_large - t_small) / (large - small)
                old = best.get(impl)
                best[impl] = (
                    min(per, old[0]) if old else per,
                    min(t_small, old[1]) if old else t_small,
                    max(peak, old[2]) if old else peak,
                )
        for impl in impls:
            per, startup, peak = best[impl]
            print(f"{name:<10} {impl:<8} {per * 1e9:>8.0f}ns {startup:>8.2f}s {peak:>6.0f}MiB")
        if len(impls) == 2:
            print(f"{'':<10} {'ratio':<8} {best['classic'][0] / best['rs'][0]:>9.0f}x")


if __name__ == "__main__":
    main()
