# /// script
# requires-python = ">=3.11"
# ///
"""Installs the checkers CI runs on generated code, and Mocha, and puts them on PATH.

Pinned by default, as PR CI runs them:

- TypeScript (tsc), pyright, and Mocha from package.json and
  package-lock.json here, by `npm ci --ignore-scripts`
- mypy at MYPY, by `uv tool install`
- LuaLS at LUALS, its download checked against LUALS_SHA256

With `--latest NAME` (typescript, pyright, mypy, or luals), that one
checker is the latest release instead, ahead of the pinned one on PATH,
for the nightly early-warning job. Each run prints the versions it
installed, and adds them to the job summary. Bump a pin by editing it
here (mypy, LuaLS) or package.json and `npm install --package-lock-only
--ignore-scripts` (the rest); docs/contributing/ci.md has the details.
"""

import argparse
import hashlib
import os
import re
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
from pathlib import Path

MYPY = "2.4.0"
LUALS = "3.19.2"
LUALS_SHA256 = "93ad326b10f6c00911dd8c450fa098332dbd8f9f45d1052f5636a2d54479b947"
LUALS_RELEASES = "https://github.com/LuaLS/lua-language-server/releases"

HERE = Path(__file__).resolve().parent
LATEST = ("typescript", "pyright", "mypy", "luals")


def run(*args: str | Path, **kwargs) -> str:
    print("+", " ".join(str(a) for a in args), flush=True)
    return subprocess.run([str(a) for a in args], check=True, text=True, stdout=subprocess.PIPE, **kwargs).stdout


def first_line(*args: str | Path) -> str:
    return run(*args).strip().splitlines()[0]


def add_path(path: Path, paths: list[Path]) -> None:
    """Puts `path` ahead of the others on PATH, for this script and later steps."""
    paths.append(path)
    os.environ["PATH"] = f"{path}{os.pathsep}{os.environ['PATH']}"


def latest_luals() -> str:
    """The tag that releases/latest redirects to."""
    request = urllib.request.Request(f"{LUALS_RELEASES}/latest", method="HEAD")
    with urllib.request.urlopen(request, timeout=60) as response:
        tag = response.url.rstrip("/").rsplit("/", 1)[-1]
    if not re.fullmatch(r"\d+\.\d+\.\d+", tag):
        sys.exit(f"install.py: unexpected LuaLS release tag {tag!r}")
    return tag


def install_luals(version: str, sha256: str | None, dest: Path) -> Path:
    name = f"lua-language-server-{version}-linux-x64.tar.gz"
    url = f"{LUALS_RELEASES}/download/{version}/{name}"
    print("+ download", url, flush=True)
    with urllib.request.urlopen(url, timeout=300) as response:
        data = response.read()
    digest = hashlib.sha256(data).hexdigest()
    if sha256 is None:
        print(f"LuaLS {version}: sha256 {digest} (latest; not checked)")
    elif digest != sha256:
        sys.exit(f"install.py: {name} has sha256 {digest}, not the recorded {sha256}")
    dest.mkdir(parents=True, exist_ok=True)
    archive = dest / name
    archive.write_bytes(data)
    with tarfile.open(archive) as tar:
        tar.extractall(dest, filter="data")
    archive.unlink()
    return dest / "bin"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--latest", choices=LATEST, help="this checker at its latest release")
    parser.add_argument("--dir", type=Path, help="where to install the latest checker and LuaLS (default $RUNNER_TEMP)")
    args = parser.parse_args()
    base = (args.dir or Path(os.environ.get("RUNNER_TEMP") or tempfile.mkdtemp())).resolve() / "ci-tools"
    paths: list[Path] = []

    # TypeScript, pyright, and Mocha, from the lockfile; no install scripts.
    run("npm", "ci", "--ignore-scripts", "--no-audit", "--no-fund", cwd=HERE)
    add_path(HERE / "node_modules/.bin", paths)
    if args.latest in ("typescript", "pyright"):
        latest = base / "npm-latest"
        latest.mkdir(parents=True, exist_ok=True)
        run("npm", "install", "--prefix", latest, "--ignore-scripts", "--no-audit", "--no-fund",
            "--no-save", f"{args.latest}@latest")
        add_path(latest / "node_modules/.bin", paths)

    # mypy, as a uv tool.
    mypy = "mypy" if args.latest == "mypy" else f"mypy=={MYPY}"
    run("uv", "tool", "install", "--force", mypy)
    add_path(Path(run("uv", "tool", "dir", "--bin").strip()), paths)

    # LuaLS, checked against its recorded hash unless it's the latest.
    if args.latest == "luals":
        luals_bin = install_luals(latest_luals(), None, base / "luals-latest")
    else:
        luals_bin = install_luals(LUALS, LUALS_SHA256, base / "luals")
    add_path(luals_bin, paths)

    versions = {
        "TypeScript": first_line("tsc", "--version"),
        "pyright": first_line("pyright", "--version"),
        "mypy": first_line("mypy", "--version"),
        "LuaLS": first_line("lua-language-server", "--version"),
        "Mocha": first_line("mocha", "--version"),
    }
    lines = [f"{k}: {v}" + (" (latest)" if args.latest and args.latest in k.lower() else "") for k, v in versions.items()]
    print("\n".join(lines))
    if github_path := os.environ.get("GITHUB_PATH"):
        # Each line is put ahead of the earlier ones, so the latest checker wins.
        with open(github_path, "a") as f:
            f.writelines(f"{p}\n" for p in paths)
    if summary := os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(summary, "a") as f:
            f.write("### Checkers\n\n" + "".join(f"- {line}\n" for line in lines))
    if output := os.environ.get("GITHUB_OUTPUT"):
        with open(output, "a") as f:
            f.write(f"versions={'; '.join(lines)}\n")


if __name__ == "__main__":
    main()
