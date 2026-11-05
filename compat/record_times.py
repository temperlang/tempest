# /// script
# requires-python = ">=3.11"
# ///
"""Turns build and test step times into rows of notes/size-and-speed.csv.

Reads a tab-separated log of step times, one row per step of a CI-style
run, with the columns

    date  commit  dirty  step  wall_s  cpu_s  result

(`date` an ISO time, `dirty` `clean` or `dirty`, `result` `ok` or `0` for
a step that passed), and prints, or with --record appends, a row per
passed step: its wall seconds as `value`, under the step's metric id,
and its CPU seconds (the step's processes', children included) as the
same id with `-cpu-s` for `-s`. Ids: `clippy-s`; a backend crate's
`cargo test -p temper-be-X` is `funtest-X-s`, since its functional tests
are most of it; another crate's is `test-<crate>-s`; the wasm build is
`build-wasm-small-s`; other steps keep their names, with `-s`. A run
starts where a step repeats, so a log of many runs splits into them;
--last takes only the last run. Failed steps aren't recorded.

Or times one command itself, as `--time ID -- COMMAND...`: its wall and
CPU seconds as a row of each (a build from clean, an incremental build).

    uv run compat/record_times.py LOG [--last] [--load TEXT] [--record]
    uv run compat/record_times.py --time ID [--load TEXT] [--record] -- CMD...

`--load` adds `load=TEXT` to the rows' conditions (`load=busy`, or
`load=2-agents`); a dirty tree adds `tree=dirty`.
"""

import argparse
import csv
import datetime
import resource
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CSV = ROOT / "notes/size-and-speed.csv"
FIELDS = ["date", "commit", "what", "value", "value_max", "unit", "conditions", "notes"]


def metric(step: str) -> str:
    if step == "clippy":
        return "clippy-s"
    if step == "wasm":
        return "build-wasm-small-s"
    if step.startswith("temper-be-") and step != "temper-be-testkit":
        return f"funtest-{step.removeprefix('temper-be-')}-s"
    if step.startswith("temper-") or step == "tempest":
        return f"test-{step}-s"
    return f"{step}-s"


def runs(rows: list[dict[str, str]]) -> list[list[dict[str, str]]]:
    """The log's runs: a new one wherever a step repeats."""
    out: list[list[dict[str, str]]] = []
    seen: set[str] = set()
    for row in rows:
        if not out or row["step"] in seen:
            out.append([])
            seen = set()
        out[-1].append(row)
        seen.add(row["step"])
    return out


def conditions(dirty: bool, load: str | None) -> str:
    parts = []
    if load:
        parts.append(f"load={load}")
    if dirty:
        parts.append("tree=dirty")
    return ";".join(parts)


def rows_for(what: str, date: str, commit: str, wall: float, cpu: float, cond: str, note: str) -> list[dict[str, str]]:
    base = {"date": date, "commit": commit, "value_max": "", "unit": "s", "conditions": cond, "notes": note}
    return [
        {**base, "what": what, "value": f"{wall:.1f}"},
        {**base, "what": what.removesuffix("-s") + "-cpu-s", "value": f"{cpu:.1f}"},
    ]


def from_log(path: Path, last: bool, load: str | None) -> list[dict[str, str]]:
    with path.open(newline="") as f:
        logged = list(csv.DictReader(f, delimiter="\t"))
    chosen = runs(logged)
    if last:
        chosen = chosen[-1:]
    out = []
    for run in chosen:
        for row in run:
            if row["result"] not in ("ok", "0"):
                continue
            out += rows_for(
                metric(row["step"]),
                row["date"][:10],
                row["commit"],
                float(row["wall_s"]),
                float(row["cpu_s"]),
                conditions(row["dirty"] == "dirty", load),
                "record_times.py",
            )
    return out


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()


def timed(what: str, command: list[str], load: str | None) -> list[dict[str, str]]:
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    start = time.monotonic()
    done = subprocess.run(command)
    wall = time.monotonic() - start
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    if done.returncode != 0:
        sys.exit(f"{' '.join(command)} failed ({done.returncode}); nothing recorded")
    cpu = (after.ru_utime - before.ru_utime) + (after.ru_stime - before.ru_stime)
    dirty = bool(git("status", "--porcelain"))
    return rows_for(
        what,
        datetime.date.today().isoformat(),
        git("rev-parse", "--short", "HEAD"),
        wall,
        cpu,
        conditions(dirty, load),
        "record_times.py --time",
    )


def main() -> None:
    parser = argparse.ArgumentParser(description="Turns build and test step times into CSV rows.")
    parser.add_argument("log", nargs="?", type=Path, help="a times log (TSV)")
    parser.add_argument("--last", action="store_true", help="only the log's last run")
    parser.add_argument("--time", metavar="ID", help="time the command after --, as metric ID")
    parser.add_argument("--load", help="what else was running, as load=TEXT")
    parser.add_argument("--record", action="store_true", help="append to notes/size-and-speed.csv")
    argv = sys.argv[1:]
    split = argv.index("--") if "--" in argv else len(argv)
    args = parser.parse_args(argv[:split])
    command = argv[split + 1:]
    if args.time:
        if not command:
            parser.error("--time needs a command after --")
        rows = timed(args.time, command, args.load)
    elif args.log:
        rows = from_log(args.log, args.last, args.load)
    else:
        parser.error("give a log or --time")
    if args.record:
        with CSV.open("a", newline="") as f:
            csv.DictWriter(f, FIELDS, lineterminator="\n").writerows(rows)
    else:
        writer = csv.DictWriter(sys.stdout, FIELDS, lineterminator="\n")
        writer.writerows(rows)


if __name__ == "__main__":
    main()
