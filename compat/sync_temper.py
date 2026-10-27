# /// script
# requires-python = ">=3.11"
# ///
"""Copies Kotlin Temper's functional tests, std, and core into vendor/temper/, so
tempest's checks run without a Temper checkout lying around.

    uv run compat/sync_temper.py [TEMPER_DIR]

TEMPER_DIR defaults to the temper checkout beside this repository. Replaces vendor/temper/ wholly,
and records where the copy came from (branch, commit, and whether the
checkout had uncommitted changes) in vendor/temper/SOURCE.md, so a later
sync's diff shows exactly what changed upstream.
"""

import argparse
import shutil
import subprocess
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
}


def git(temper: Path, *args: str) -> str:
    return subprocess.run(["git", *args], cwd=temper, capture_output=True, text=True, check=True).stdout.strip()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("temper", nargs="?", type=Path, default=Path(__file__).resolve().parents[2] / "temper")
    temper = parser.parse_args().temper.expanduser().resolve()
    for source in PARTS:
        if not (temper / source).is_dir():
            raise SystemExit(f"Not a Temper checkout: {temper / source} is missing")
    shutil.rmtree(VENDOR, ignore_errors=True)
    for source, dest in PARTS.items():
        shutil.copytree(temper / source, VENDOR / dest)
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
(see COPYRIGHT). Refresh with `uv run compat/sync_temper.py`.
"""
    )
    print(f"Copied {branch} at {commit[:10]}{' (with uncommitted changes)' if dirty else ''} into {VENDOR.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
