# /// script
# requires-python = ">=3.11"
# ///
"""Measures temper-rs binary size and peak memory, with Kotlin for reference.

Sizes are for the `small` profile (size-optimized, LTO, stripped) and the
plain release profile, plus `temper-wasm` built for wasm32, raw and gzipped. Peak memory is the maximum resident set size of one
`--bench 1` run over all Temper files (read everything, then lex and parse it
all once, and also build grammar trees for it all), for our binaries and
for Kotlin's oracles on the JVM.

    uv run compat/measure_size.py [DIR ...]

Record results in notes/size-and-speed.md, with the commit measured.
"""

import argparse
import gzip
import os
import subprocess
import sys
from pathlib import Path

from check_lex import DEFAULT_DIRS, ROOT, temper_files
from check_ast import commands as ast_commands
from check_parse import commands

BINARIES = ["lex-tokens", "parse-tree", "ast-tree"]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("dirs", nargs="*", type=Path, default=DEFAULT_DIRS)
    args = parser.parse_args()

    kotlin, _ = commands()  # Also builds the release binaries.
    kotlin_ast, _ = ast_commands()
    subprocess.run(["cargo", "build", "-q", "--profile", "small", "-p", "temper-syntax"], cwd=ROOT, check=True)
    for name in BINARIES:
        small = (ROOT / "target/small" / name).stat().st_size
        release = (ROOT / "target/release" / name).stat().st_size
        print(f"{name}: {kib(small)} small, {kib(release)} release")

    subprocess.run(
        ["cargo", "build", "-q", "--profile", "small", "--target", "wasm32-unknown-unknown", "-p", "temper-wasm"],
        cwd=ROOT,
        check=True,
    )
    wasm = (ROOT / "target/wasm32-unknown-unknown/small/temper_wasm.wasm").read_bytes()
    print(f"temper-wasm: {kib(len(wasm))}, {kib(len(gzip.compress(wasm, 9)))} gzipped")

    files = temper_files(args.dirs)
    total = sum(os.path.getsize(f) for f in files)
    print(f"{len(files)} files, {kib(total)}")
    for what, printer, oracle in (
        ("lex and parse", "parse-tree", kotlin),
        ("lex, parse and build trees for", "ast-tree", kotlin_ast),
    ):
        rust = peak_kib([str(ROOT / "target/small" / printer), "--bench", "1", *files])
        jvm = peak_kib(oracle + ["--bench", "1", *files])
        print(f"peak memory, {what} all: rust {rust / 1024:.1f} MiB, kotlin (JVM) {jvm / 1024:.1f} MiB")
    return 0


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


if __name__ == "__main__":
    sys.exit(main())
