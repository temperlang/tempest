# /// script
# requires-python = ">=3.11"
# ///
"""Measures tempest binary size and peak memory, with Kotlin for reference.

Sizes are for the `small` profile (size-optimized, LTO, stripped) and the
plain release profile, plus `temper-wasm` built for wasm32, raw and gzipped. Peak memory is the maximum resident set size of one
`--bench 1` run over all Temper files (read everything, then lex and parse it
all once, and also build grammar trees for it all, and also settle each
file as a library), for our binaries and for Kotlin's oracles on the JVM.

    uv run compat/measure_size.py [--record] [DIR ...]

`--record` also appends the results as rows to notes/size-and-speed.csv,
with today's date, the commit, and `tree=dirty` if the work tree has
changes. Explain them in notes/size-and-speed.md only where rows can't.
"""

import argparse
import csv
import datetime
import gzip
import os
import subprocess
import sys
from pathlib import Path

from check_lex import DEFAULT_DIRS, ROOT, temper_files
from check_ast import commands as ast_commands
from check_parse import commands

BINARIES = ["lex-tokens", "parse-tree", "ast-tree", "run-temper", "tempest"]
# Binaries built with temper-frontend's default features, which include `net`.
NET = {"run-temper", "tempest"}
CSV = ROOT / "notes/size-and-speed.csv"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("dirs", nargs="*", type=Path, default=DEFAULT_DIRS)
    parser.add_argument("--record", action="store_true", help="append results to notes/size-and-speed.csv")
    args = parser.parse_args()
    rows: list[tuple[str, str, str, str]] = []  # what, value, unit, conditions

    kotlin, _ = commands()  # Also builds the release binaries.
    kotlin_ast, _ = ast_commands()
    subprocess.run(["cargo", "build", "-q", "--release", "-p", "temper-frontend", "-p", "tempest"], cwd=ROOT, check=True)
    subprocess.run(
        ["cargo", "build", "-q", "--profile", "small", "-p", "temper-syntax", "-p", "temper-frontend", "-p", "tempest"],
        cwd=ROOT,
        check=True,
    )
    for name in BINARIES:
        small = (ROOT / "target/small" / name).stat().st_size
        release = (ROOT / "target/release" / name).stat().st_size
        print(f"{name}: {kib(small)} small, {kib(release)} release")
        features = "features=net" if name in NET else ""
        rows += [(f"{name}-small", str(small), "bytes", features), (f"{name}-release", str(release), "bytes", features)]

    subprocess.run(
        ["cargo", "build", "-q", "--profile", "small", "--target", "wasm32-unknown-unknown", "-p", "temper-wasm"],
        cwd=ROOT,
        check=True,
    )
    wasm = (ROOT / "target/wasm32-unknown-unknown/small/temper_wasm.wasm").read_bytes()
    gzipped = len(gzip.compress(wasm, 9))
    print(f"temper-wasm: {kib(len(wasm))}, {kib(gzipped)} gzipped")
    rows += [("wasm-small", str(len(wasm)), "bytes", ""), ("wasm-small-gzip", str(gzipped), "bytes", "")]

    files = temper_files(args.dirs)
    total = sum(os.path.getsize(f) for f in files)
    print(f"{len(files)} files, {kib(total)}")
    count = f"files={len(files)}"
    for what, metric, printer, oracle in (
        ("lex and parse", "peak-memory-parse-all", "parse-tree", kotlin),
        ("lex, parse and build trees for", "peak-memory-ast-all", "ast-tree", kotlin_ast),
    ):
        rust = peak_kib([str(ROOT / "target/small" / printer), "--bench", "1", *files])
        jvm = peak_kib(oracle + ["--bench", "1", *files])
        print(f"peak memory, {what} all: rust {rust / 1024:.1f} MiB, kotlin (JVM) {jvm / 1024:.1f} MiB")
        rows += [(metric, mib(rust), "MiB", f"run=rust;{count}"), (metric, mib(jvm), "MiB", f"run=kotlin-jvm;{count}")]
    rust = peak_kib([str(ROOT / "target/small/run-temper"), "--bench", "1", *files])
    print(f"peak memory, lex, parse, build trees for and settle all: rust {rust / 1024:.1f} MiB")
    rows.append(("peak-memory-settle-all", mib(rust), "MiB", f"run=rust;{count}"))
    if args.record:
        record(rows)
    return 0


def record(rows: list[tuple[str, str, str, str]]) -> None:
    """Appends rows to the CSV, marking them `tree=dirty` if there are changes."""
    git = ["git", "-C", str(ROOT)]
    commit = subprocess.run([*git, "rev-parse", "--short", "HEAD"], capture_output=True, text=True, check=True)
    status = subprocess.run([*git, "status", "--porcelain"], capture_output=True, text=True, check=True)
    tree = "tree=dirty" if status.stdout.strip() else ""
    date = datetime.date.today().isoformat()
    with CSV.open("a", newline="") as f:
        writer = csv.writer(f, lineterminator="\n")
        for what, value, unit, conditions in rows:
            conditions = ";".join(c for c in (conditions, tree) if c)
            writer.writerow([date, commit.stdout.strip(), what, value, "", unit, conditions, "measure_size.py"])
    print(f"recorded {len(rows)} rows in {CSV.name}")


def peak_kib(command: list[str]) -> int:
    """Runs a command and returns its own peak resident set size in KiB."""
    process = subprocess.Popen(command, stdout=subprocess.DEVNULL)
    _, status, usage = os.wait4(process.pid, 0)
    process.returncode = os.waitstatus_to_exitcode(status)
    if process.returncode != 0:
        raise subprocess.CalledProcessError(process.returncode, command)
    return usage.ru_maxrss


def kib(n: int) -> str:
    return f"{n / 1024:.0f} KiB"


def mib(n_kib: int) -> str:
    return f"{n_kib / 1024:.1f}"


if __name__ == "__main__":
    sys.exit(main())
