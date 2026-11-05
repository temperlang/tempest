# /// script
# requires-python = ">=3.11"
# ///
"""Copies Kotlin Temper's functional tests, std, core, and Python, Rust, JS, Java, C#, C++, and Lua runtimes into
vendor/temper/, so tempest's checks run without a Temper checkout lying around.

    uv run compat/sync_temper.py [TEMPER_DIR] [--commit COMMIT]
    uv run compat/sync_temper.py --check [TEMPER_DIR]

TEMPER_DIR defaults to the temper checkout beside this repository. Replaces vendor/temper/ wholly,
and records where the copy came from (branch, commit, and whether the
checkout had uncommitted changes) in vendor/temper/SOURCE.md, so a later
sync's diff shows exactly what changed upstream. With --commit, it copies
that commit's files (`git archive`), not the checkout's, so a checkout on
another branch, or with changes, can refresh or add to the vendored copy
at the commit it already has.

After copying, it applies tempest's fixes to the copy, the patches in
vendor/temper-patches/ (in name order), and lists them in SOURCE.md. A
patch that no longer applies stops the sync, leaving vendor/temper/ as it
was; one that upstream already has (it applies in reverse) is reported,
to be dropped.

With --check, it changes nothing, and fails unless vendor/temper/ is
SOURCE.md's commit plus the patches: that the vendored tree has every
patch (each undone in turn, last first, on a copy), and SOURCE.md lists them all, always; and, where TEMPER_DIR has
that commit (CI has no checkout), that the tree equals the commit's files
with the patches applied, file for file.
"""

import argparse
import filecmp
import io
import os
import re
import shutil
import subprocess
import tarfile
import tempfile
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VENDOR = ROOT / "vendor" / "temper"
# tempest's fixes to the copy, as patches whose paths are under VENDOR
# (`a/be-js/...`), each starting with a one-line summary and an
# `Upstream:` line saying whether Kotlin has it yet.
PATCHES = ROOT / "vendor" / "temper-patches"
# What copying leaves out, and comparing ignores.
IGNORED = ("__pycache__", "target")
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
    # std's temporal support for C#, `Temporal/TemporalSupport.cs`, which
    # std's connected `Date` members call. Rust's is tempest's own (its
    # `today()` without the `time` crate); the others' are in their
    # runtimes.
    "be-csharp/src/commonMain/resources/lang/temper/be/csharp/std/Temporal": "be-csharp/std/Temporal",
}


def git(temper: Path, *args: str) -> str:
    return subprocess.run(["git", *args], cwd=temper, capture_output=True, text=True, check=True).stdout.strip()


def archive(temper: Path, commit: str, into: Path) -> None:
    """Extracts PARTS as they are at `commit` into `into`."""
    tar = subprocess.run(["git", "archive", commit, "--", *PARTS], cwd=temper, capture_output=True, check=True).stdout
    with tarfile.open(fileobj=io.BytesIO(tar)) as archived:
        archived.extractall(into, filter="data")


def copy_parts(source_root: Path, into: Path) -> None:
    for source in PARTS:
        if not (source_root / source).is_dir():
            raise SystemExit(f"Not a Temper checkout: {source_root / source} is missing")
    for source, dest in PARTS.items():
        # Copied with fresh times (`copy`, not `copy2`): a copied file keeps
        # its checkout's time otherwise, which can be older than the last
        # build, so Cargo wouldn't rebuild what includes it (`include_str!`).
        shutil.copytree(
            source_root / source,
            into / dest,
            ignore=shutil.ignore_patterns(*IGNORED),
            copy_function=shutil.copy,
        )


def patches() -> list[Path]:
    return sorted(PATCHES.glob("*.patch"))


def about(patch: Path) -> tuple[str, str]:
    """A patch's one-line summary, and its `Upstream:` status."""
    lines = patch.read_text().splitlines()
    summary = next((line for line in lines if line.strip()), "").strip()
    upstream = next((line.split(":", 1)[1].strip() for line in lines if line.startswith("Upstream:")), "")
    if not summary or not upstream or summary.startswith(("diff ", "--- ")):
        raise SystemExit(f"{patch.relative_to(ROOT)} needs a summary line and an `Upstream:` line before its diff")
    return summary, upstream


def applies(patch: Path, tree: Path, *flags: str) -> subprocess.CompletedProcess[str]:
    """`git apply` of `patch` to the plain files under `tree`, with `--check`
    among `flags` to only check. Repository discovery stops at `tree`, so
    git doesn't take paths as relative to an enclosing repository (as
    vendor/temper/'s is), and nothing touches any index."""
    env = {**os.environ, "GIT_CEILING_DIRECTORIES": str(tree.parent)}
    return subprocess.run(
        ["git", "apply", "--whitespace=nowarn", *flags, str(patch)],
        cwd=tree,
        env=env,
        capture_output=True,
        text=True,
    )


def apply_patches(tree: Path) -> list[Path]:
    """Applies every patch to `tree`, failing on any that doesn't apply.
    Gives the ones upstream already has, which it leaves be."""
    upstream = []
    for patch in patches():
        about(patch)
        if applies(patch, tree, "--check").returncode == 0:
            applies(patch, tree).check_returncode()
        elif applies(patch, tree, "--check", "--reverse").returncode == 0:
            upstream.append(patch)
        else:
            failed = applies(patch, tree, "--check")
            raise SystemExit(
                f"{patch.relative_to(ROOT)} no longer applies; vendor/temper/ is unchanged. Update or drop it:\n{failed.stderr}"
            )
    return upstream


def source_text(branch: str, commit: str, dirty: bool, upstream: list[Path]) -> str:
    listed = []
    for patch in patches():
        summary, status = about(patch)
        if patch in upstream:
            status = "upstream has it now; drop it"
        listed.append(f"- `{patch.name}`: {summary} Upstream: {status}")
    return f"""# Source

Copied from Kotlin Temper by `compat/sync_temper.py` on {date.today()}.

- Branch: `{branch}`
- Commit: `{commit}`
- Uncommitted changes in the copied folders: {"yes" if dirty else "no"}

Folders:

{chr(10).join(f"- `{dest}/` from `{source}/`" for source, dest in PARTS.items())}

Patches, tempest's fixes to the copy (behavior kept), from
`vendor/temper-patches/`, applied after copying:

{chr(10).join(listed) or "- (none)"}

Temper and tempest are both The Temper Project's, under the same licenses
(see COPYRIGHT), except LuaUnit (`be-lua/luaunit/`), under the BSD
license in its LICENSE.txt. Refresh with `uv run compat/sync_temper.py`.
"""


def differences(left: Path, right: Path, at: str = "") -> list[str]:
    """The files that differ between two trees, or are in only one."""
    compared = filecmp.dircmp(left, right, ignore=[*IGNORED, "SOURCE.md"] if not at else [*IGNORED])
    found = [f"{at}{name}: only in {left.name}" for name in compared.left_only]
    found += [f"{at}{name}: only in {right.name}" for name in compared.right_only]
    found += [f"{at}{name}: differs" for name in compared.common_files if not filecmp.cmp(left / name, right / name, shallow=False)]
    for name in compared.common_dirs:
        found += differences(left / name, right / name, f"{at}{name}/")
    return found


def check(temper: Path) -> None:
    source = (VENDOR / "SOURCE.md").read_text()
    problems = []
    listed = re.findall(r"^- `([^`]+\.patch)`", source, re.MULTILINE)
    if listed != [patch.name for patch in patches()]:
        problems.append(f"SOURCE.md lists patches {listed}, but vendor/temper-patches/ has {[p.name for p in patches()]}")
    # Undone last first, on a copy, as later patches may change what
    # earlier ones did.
    with tempfile.TemporaryDirectory() as tmp:
        undone = Path(tmp) / "temper"
        shutil.copytree(VENDOR, undone, ignore=shutil.ignore_patterns(*IGNORED))
        for patch in reversed(patches()):
            about(patch)
            if applies(patch, undone, "--reverse").returncode != 0:
                problems.append(f"vendor/temper/ lacks {patch.name} (it doesn't apply in reverse)")
    commit = re.search(r"^- Commit: `([0-9a-f]+)`", source, re.MULTILINE)
    dirty = "Uncommitted changes in the copied folders: yes" in source
    has_commit = (
        commit
        and (temper / ".git").exists()
        and subprocess.run(["git", "cat-file", "-e", f"{commit[1]}^{{commit}}"], cwd=temper, capture_output=True).returncode == 0
    )
    if dirty:
        print("Not compared file for file: SOURCE.md's copy had uncommitted changes")
    elif not has_commit:
        print(f"Not compared file for file: no Temper checkout at {temper} with SOURCE.md's commit")
    else:
        assert commit
        with tempfile.TemporaryDirectory() as tmp:
            archived, pinned = Path(tmp) / "archived", Path(tmp) / "pinned"
            archive(temper, commit[1], archived)
            copy_parts(archived, pinned)
            apply_patches(pinned)
            problems += [f"vendor/temper/ and {commit[1][:10]} plus patches: {d}" for d in differences(VENDOR, pinned)]
        if not problems:
            print(f"vendor/temper/ is {commit[1][:10]} plus {len(patches())} patch(es), file for file")
    if problems:
        raise SystemExit("\n".join(problems))
    print("vendor/temper/ has every patch, all listed in SOURCE.md")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("temper", nargs="?", type=Path, default=Path(__file__).resolve().parents[2] / "temper")
    parser.add_argument("--commit", help="copy this commit's files, not the checkout's")
    parser.add_argument("--check", action="store_true", help="check vendor/temper/ is SOURCE.md's commit plus the patches")
    args = parser.parse_args()
    temper = args.temper.expanduser().resolve()
    if args.check:
        check(temper)
        return
    with tempfile.TemporaryDirectory() as tmp:
        staged = Path(tmp) / "temper"
        if args.commit:
            commit = git(temper, "rev-parse", "--verify", f"{args.commit}^{{commit}}")
            # The branch whose tip it is, or is nearest, as `name-rev` names it.
            branch = git(temper, "name-rev", "--name-only", "--refs=refs/heads/*", commit)
            dirty = False
            archived = Path(tmp) / "archived"
            archive(temper, commit, archived)
            copy_parts(archived, staged)
        else:
            copy_parts(temper, staged)
            branch = git(temper, "branch", "--show-current") or "(detached)"
            commit = git(temper, "rev-parse", "HEAD")
            dirty = bool(git(temper, "status", "--porcelain", "--", *PARTS))
        # Patched before replacing vendor/temper/, which a failure leaves be.
        upstream = apply_patches(staged)
        (staged / "SOURCE.md").write_text(source_text(branch, commit, dirty, upstream))
        shutil.rmtree(VENDOR, ignore_errors=True)
        shutil.move(staged, VENDOR)
    print(f"Copied {branch} at {commit[:10]}{' (with uncommitted changes)' if dirty else ''} into {VENDOR.relative_to(ROOT)}")
    applied = len(patches()) - len(upstream)
    print(f"Applied {applied} patch(es) from {PATCHES.relative_to(ROOT)}")
    for patch in upstream:
        print(f"Upstream already has {patch.relative_to(ROOT)}: drop it and sync again")


if __name__ == "__main__":
    main()
