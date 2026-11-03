# /// script
# requires-python = ">=3.11"
# ///
"""Copies Kotlin Temper's functional tests, std, core, and Python, Rust, JS, Java, C#, C++, and Lua runtimes into
vendor/temper/, so tempest's checks run without a Temper checkout lying around.

    uv run compat/sync_temper.py [TEMPER_DIR] [--commit COMMIT]

TEMPER_DIR defaults to the temper checkout beside this repository. Replaces vendor/temper/ wholly,
and records where the copy came from (branch, commit, and whether the
checkout had uncommitted changes) in vendor/temper/SOURCE.md, so a later
sync's diff shows exactly what changed upstream. With --commit, it copies
that commit's files (`git archive`), not the checkout's, so a checkout on
another branch, or with changes, can refresh or add to the vendored copy
at the commit it already has.
"""

import argparse
import io
import shutil
import subprocess
import tarfile
import tempfile
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VENDOR = ROOT / "vendor" / "temper"
# What we copy, from the Temper checkout, to where under VENDOR.
PARTS = {
    "functional-test-suite/src/commonMain/resources": "functional-tests",
    "frontend/src/commonMain/resources/std": "std",
    # Not loaded yet (tempest builds core's types in), but core and std are
    # to live in tempest's tree, and loading core.temper retires builtins.
    "frontend/src/commonMain/resources/core": "core",
    # temper_core, the Python runtime that generated Python imports, for
    # the Python backend's run tests (crates/temper-be-py).
    "be-py/src/commonMain/resources/lang/temper/be/py/temper-core": "be-py/temper-core",
    # temper-core, the Rust runtime that generated crates depend on, for the
    # Rust backend's run tests (crates/temper-be-rust). Its one dependency,
    # indexmap, is pinned by its Cargo.lock.
    "be-rust/src/commonMain/resources/lang/temper/be/rust/temper-core": "be-rust/temper-core",
    # @temperlang/core, the JS runtime that generated packages import, for
    # the JS backend's run tests and type checks (crates/temper-be-js).
    "be-js/src/commonMain/resources/lang/temper/be/js/temper-core": "be-js/temper-core",
    # temper-core for Java, the runtime that generated Java compiles
    # against, for the Java backend's run tests (crates/temper-be-java).
    # Its main sources need only the JDK; JUnit is only for its own test.
    "be-java/src/commonMain/resources/lang/temper/be/java/temper-core": "be-java/temper-core",
    # TemperLang.Core, the C# runtime that generated C# compiles against,
    # for the C# backend's run tests (crates/temper-be-csharp), and the
    # library template whose Logging.cs and Program.cs each generated
    # library gets, which its tests compare with what it writes.
    "be-csharp/src/commonMain/resources/lang/temper/be/csharp/temper-core": "be-csharp/temper-core",
    "be-csharp/src/commonMain/resources/lang/temper/be/csharp/library-template": "be-csharp/library-template",
    # temper-core for C++, the header-only runtime that generated C++
    # includes as <temper-core/core.hpp>, for the C++ backend's run tests
    # (crates/temper-be-cpp). Kotlin calls the folder core/ and copies it
    # to temper.out/cpp/temper-core/, whose name this keeps.
    "be-cpp/src/commonMain/resources/lang/temper/be/cpp/core": "be-cpp/temper-core",
    # temper-core for Lua, the runtime that generated Lua requires as
    # `temper-core`, for the Lua backend's run tests (crates/temper-be-lua).
    # Kotlin copies it to temper.out/lua/temper-core/, whose name this keeps.
    "be-lua/src/commonMain/resources/lang/temper/be/lua/temper-core": "be-lua/temper-core",
    # LuaUnit, as Kotlin's Lua backend runs tests with it (BSD, its
    # LICENSE.txt beside it).
    "be-lua/src/commonMain/resources/lang/temper/be/lua/luaunit": "be-lua/luaunit",
    # std's regex support for Rust, Java, and C#, which Kotlin writes into
    # std's own output for each (std's connected regex members call it):
    # Rust's `src/regex/support.rs` (with the `regex` crate), Java's
    # `temper/std/regex/Core.java`, and C#'s `Regex/RegexSupport.cs` and
    # `IntRangeSet.cs`. C++'s and Lua's are in their temper-core, JS's and
    # Python's in their runtimes.
    "be-rust/src/commonMain/resources/lang/temper/be/rust/std/regex": "be-rust/std/regex",
    "be-java/src/commonMain/resources/lang/temper/be/java/temper-std/src/main/java/temper/std/regex": "be-java/temper-std/regex",
    "be-csharp/src/commonMain/resources/lang/temper/be/csharp/std/Regex": "be-csharp/std/Regex",
}


def git(temper: Path, *args: str) -> str:
    return subprocess.run(["git", *args], cwd=temper, capture_output=True, text=True, check=True).stdout.strip()


def archive(temper: Path, commit: str, into: Path) -> None:
    """Extracts PARTS as they are at `commit` into `into`."""
    tar = subprocess.run(["git", "archive", commit, "--", *PARTS], cwd=temper, capture_output=True, check=True).stdout
    with tarfile.open(fileobj=io.BytesIO(tar)) as archived:
        archived.extractall(into, filter="data")


def copy_parts(source_root: Path) -> None:
    for source in PARTS:
        if not (source_root / source).is_dir():
            raise SystemExit(f"Not a Temper checkout: {source_root / source} is missing")
    shutil.rmtree(VENDOR, ignore_errors=True)
    for source, dest in PARTS.items():
        # Copied with fresh times (`copy`, not `copy2`): a copied file keeps
        # its checkout's time otherwise, which can be older than the last
        # build, so Cargo wouldn't rebuild what includes it (`include_str!`).
        shutil.copytree(
            source_root / source,
            VENDOR / dest,
            ignore=shutil.ignore_patterns("__pycache__", "target"),
            copy_function=shutil.copy,
        )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("temper", nargs="?", type=Path, default=Path(__file__).resolve().parents[2] / "temper")
    parser.add_argument("--commit", help="copy this commit's files, not the checkout's")
    args = parser.parse_args()
    temper = args.temper.expanduser().resolve()
    if args.commit:
        commit = git(temper, "rev-parse", "--verify", f"{args.commit}^{{commit}}")
        # The branch whose tip it is, or is nearest, as `name-rev` names it.
        branch = git(temper, "name-rev", "--name-only", "--refs=refs/heads/*", commit)
        dirty = False
        with tempfile.TemporaryDirectory() as tmp:
            archive(temper, commit, Path(tmp))
            copy_parts(Path(tmp))
    else:
        copy_parts(temper)
        branch = git(temper, "branch", "--show-current") or "(detached)"
        commit = git(temper, "rev-parse", "HEAD")
        dirty = bool(git(temper, "status", "--porcelain", "--", *PARTS))
    (VENDOR / "SOURCE.md").write_text(
        f"""# Source

Copied from Kotlin Temper by `compat/sync_temper.py` on {date.today()}.

- Branch: `{branch}`
- Commit: `{commit}`
- Uncommitted changes in the copied folders: {"yes" if dirty else "no"}

Folders:

{chr(10).join(f"- `{dest}/` from `{source}/`" for source, dest in PARTS.items())}

Temper and tempest are both The Temper Project's, under the same licenses
(see COPYRIGHT), except LuaUnit (`be-lua/luaunit/`), under the BSD
license in its LICENSE.txt. Refresh with `uv run compat/sync_temper.py`.
"""
    )
    print(f"Copied {branch} at {commit[:10]}{' (with uncommitted changes)' if dirty else ''} into {VENDOR.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
