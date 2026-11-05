# /// script
# requires-python = ">=3.11"
# ///
"""Draws the size and speed history as SVG charts, for notes/size-and-speed.md.

Reads notes/size-and-speed.csv and writes notes/charts/*.svg: binary sizes
(tempest as released, and run-temper without the network code),
settle time, peak memory, Ordable's test times by runner, each backend's
functional-test time, and build times. Each point is one commit's
measurement (clean or with the chunk's changes, `tree=dirty`), at the
middle of its range, with the range as a whisker; on the timing charts
(`smooth`), each point is one run instead, with a centered rolling median
over its neighbors as the line and the spread around it as a band, since
single runs on a shared machine are noisy. Sizes in bytes show as KiB.
`record_times.py --record` redraws them too. With --check, writes nothing
and fails if the charts are out of date, as CI runs it.

    uv run compat/size_speed_charts.py [--check]
"""

import argparse
import csv
import math
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CSV = ROOT / "notes/size-and-speed.csv"
OUT = ROOT / "notes/charts"


@dataclass
class Series:
    label: str
    what: str
    # Every condition here must be among the row's; `features=no-net` also
    # matches rows from before the `net` feature existed, which had none.
    conditions: tuple[str, ...] = ()


@dataclass
class Chart:
    name: str
    title: str
    unit: str
    series: list[Series]
    note: str = ""
    # One point per row (a run), smoothed, rather than per commit.
    smooth: bool = False


CHARTS = [
    Chart("sizes", "Binary sizes", "KiB", [
        Series("tempest (small, as released)", "tempest-small", ("features=net",)),
        Series("run-temper (small, no net)", "run-temper-small", ("features=no-net",)),
        Series("wasm (small)", "wasm-small"),
        Series("wasm, gzipped", "wasm-small-gzip"),
    ], note="small: the profile releases use, optimized for size, with LTO, stripped"),
    Chart("settle-time", "Settling every Temper file", "ms", [
        Series("settle time", "lex-parse-module-settle-ms"),
    ]),
    Chart("peak-memory", "Peak memory, settling every Temper file", "MiB", [
        Series("peak memory", "peak-memory-settle-all"),
    ]),
    Chart("ordable-tests", "Ordable's tests", "s", [
        Series("tempest (release)", "ordable-tests", ("runner=tempest", "build=release")),
        Series("Python (Kotlin Temper)", "ordable-tests", ("runner=py",)),
        Series("Lua (Kotlin Temper)", "ordable-tests", ("runner=lua",)),
    ]),
    Chart("funtest-times", "Functional tests by backend", "s", [
        Series(label, f"funtest-{name}-s")
        for label, name in [("C++", "cpp"), ("C#", "csharp"), ("Java", "java"), ("JavaScript", "js"),
                            ("Lua", "lua"), ("Python", "py"), ("Rust", "rust")]
    ], note="wall time of each backend crate's cargo test, one CI run each", smooth=True),
    Chart("build-times", "Build and lint times", "s", [
        Series("release, clean", "build-tempest-release-s"),
        Series("release, incremental", "build-tempest-incremental-s"),
        Series("clippy", "clippy-s"),
        Series("wasm (small)", "build-wasm-small-s"),
    ], note="wall time; tempest's release build from clean, and after touching one frontend file",
       smooth=True),
]

# The smoothing window: a point and this many neighbors each side.
SMOOTH_REACH = 2


@dataclass
class Point:
    """One commit's measurement of one series, clean or dirty."""
    key: tuple[str, str, bool, int]  # date, commit, dirty, run (smoothed charts), else -1
    low: float
    high: float
    order: int  # the first row's place in the file

    @property
    def mid(self) -> float:
        return (self.low + self.high) / 2


@dataclass
class Data:
    keys: list[tuple[str, str, bool, int]] = field(default_factory=list)
    points: list[dict[tuple[str, str, bool, int], Point]] = field(default_factory=list)


def matches(row: dict[str, str], series: Series) -> bool:
    if row["what"] != series.what:
        return False
    conditions = set(filter(None, row["conditions"].split(";")))
    for wanted in series.conditions:
        if wanted == "features=no-net":
            if "features=net" in conditions:
                return False
        elif wanted not in conditions:
            return False
    return True


def to_unit(value: float, unit: str, wanted: str) -> float:
    if unit == "bytes" and wanted == "KiB":
        return value / 1024
    if unit != wanted:
        raise ValueError(f"can't show {unit} as {wanted}")
    return value


def runs(rows: list[dict[str, str]]) -> list[int]:
    """Each row's run: a block of rows recorded together (same date, commit,
    tree, and notes), split where a metric repeats."""
    out: list[int] = []
    run, last, seen = -1, None, set()
    for row in rows:
        here = (row["date"], row["commit"], "tree=dirty" in row["conditions"], row["notes"])
        if here != last or row["what"] in seen:
            run, last, seen = run + 1, here, set()
        seen.add(row["what"])
        out.append(run)
    return out


def gather(rows: list[dict[str, str]], chart: Chart) -> Data:
    data = Data()
    run_of = runs(rows)
    for series in chart.series:
        points: dict[tuple[str, str, bool, int], Point] = {}
        for order, row in enumerate(rows):
            if not matches(row, series):
                continue
            low = to_unit(float(row["value"]), row["unit"], chart.unit)
            high = to_unit(float(row["value_max"]), row["unit"], chart.unit) if row["value_max"] else low
            key = (row["date"], row["commit"], "tree=dirty" in row["conditions"], run_of[order] if chart.smooth else -1)
            if key in points:
                point = points[key]
                point.low, point.high = min(point.low, low), max(point.high, high)
            else:
                points[key] = Point(key, low, high, order)
        data.points.append(points)
    # The x axis: every point of every series, in the order first measured.
    firsts = {}
    for points in data.points:
        for key, point in points.items():
            firsts[key] = min(firsts.get(key, point.order), point.order)
    data.keys = sorted(firsts, key=lambda k: (k[0], firsts[k]))
    return data


# Colors: the dataviz reference palette's first categorical slots, in its
# order, which keeps neighbors distinguishable for color-blind readers, in
# light and dark; direct labels name every line too.
STYLE = """
  svg { --surface: #fcfcfb; --ink: #0b0b0b; --ink-2: #52514e; --grid: #e4e3df;
        --s1: #2a78d6; --s2: #eb6834; --s3: #1baf7a; --s4: #eda100;
        --s5: #e87ba4; --s6: #008300; --s7: #6250d6; }
  @media (prefers-color-scheme: dark) {
    svg { --surface: #1a1a19; --ink: #f0efec; --ink-2: #c3c2b7; --grid: #34342f;
          --s1: #3987e5; --s2: #d95926; --s3: #199e70; --s4: #c98500;
          --s5: #d55181; --s6: #008300; --s7: #9085e9; }
  }
  text { font: 12px system-ui, -apple-system, "Segoe UI", sans-serif; fill: var(--ink-2); }
  .title { font-size: 14px; font-weight: 600; fill: var(--ink); }
  .label { fill: var(--ink); }
"""

WIDTH, HEIGHT = 720, 296
LEFT, RIGHT, TOP, BOTTOM = 56, 190, 72, 40
LABEL_GAP = 16


def median(values: list[float]) -> float:
    ordered = sorted(values)
    n = len(ordered)
    return ordered[n // 2] if n % 2 else (ordered[n // 2 - 1] + ordered[n // 2]) / 2


def smoothed(values: list[float]) -> list[tuple[float, float, float]]:
    """Each value's centered rolling median over its neighbors, with a band
    of that median plus and minus the neighbors' median distance from it."""
    out = []
    for i in range(len(values)):
        near = values[max(0, i - SMOOTH_REACH):i + SMOOTH_REACH + 1]
        mid = median(near)
        spread = median([abs(v - mid) for v in near])
        out.append((mid, max(0.0, mid - spread), mid + spread))
    return out


def nice_max(value: float) -> tuple[float, float]:
    """A round top for the y axis and its tick step, about 4 ticks."""
    if value <= 0:
        return 1, 0.25
    raw = value / 4
    magnitude = 10 ** math.floor(math.log10(raw))
    step = next(m * magnitude for m in (1, 2, 2.5, 5, 10) if m * magnitude >= raw)
    top = step * 4
    while top < value:
        top += step
    return top, step


def number(value: float) -> str:
    return f"{value:,.0f}" if value >= 100 else f"{value:,.1f}".rstrip("0").rstrip(".")


def draw(chart: Chart, data: Data) -> str:
    plot_w, plot_h = WIDTH - LEFT - RIGHT, HEIGHT - TOP - BOTTOM
    highest = max((p.high for points in data.points for p in points.values()), default=1)
    top, step = nice_max(highest)
    n = len(data.keys)

    def x(i: int) -> float:
        return LEFT + (plot_w * i / (n - 1) if n > 1 else plot_w / 2)

    def y(v: float) -> float:
        return TOP + plot_h * (1 - v / top)

    out = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {WIDTH} {HEIGHT}" '
           f'width="{WIDTH}" height="{HEIGHT}" role="img" aria-labelledby="t d">',
           f'<title id="t">{chart.title} ({chart.unit})</title>',
           f'<desc id="d">{describe(chart, data)}</desc>',
           f"<style>{STYLE}</style>",
           f'<rect width="{WIDTH}" height="{HEIGHT}" rx="8" fill="var(--surface)"/>',
           f'<text class="title" x="{LEFT}" y="24">{chart.title}</text>',
           f'<text x="{LEFT}" y="42">{subtitle(chart)}</text>']
    if chart.note:
        out.append(f'<text x="{LEFT}" y="58">{chart.note}</text>')
    # Grid and y ticks.
    tick = 0.0
    while tick <= top + step / 1000:
        out.append(f'<line x1="{LEFT}" x2="{LEFT + plot_w}" y1="{y(tick):.1f}" y2="{y(tick):.1f}" '
                   f'stroke="var(--grid)" stroke-width="1"/>')
        out.append(f'<text x="{LEFT - 8}" y="{y(tick) + 4:.1f}" text-anchor="end">{number(tick)}</text>')
        tick += step
    # x ticks: each date, under its first measurement.
    seen = set()
    for i, (date, *_) in enumerate(data.keys):
        if date not in seen:
            seen.add(date)
            out.append(f'<text x="{x(i):.1f}" y="{TOP + plot_h + 20}" text-anchor="middle">{date[5:]}</text>')
    # Series: whiskers, then the line, then markers ringed by the surface.
    labels: list[tuple[float, str]] = []
    for index, (series, points) in enumerate(zip(chart.series, data.points)):
        color = f"var(--s{index + 1})"
        placed = [(data.keys.index(k), p) for k, p in points.items()]
        placed.sort(key=lambda ip: ip[0])
        if chart.smooth and len(placed) > 1:
            smooth = smoothed([p.mid for _, p in placed])
            upper = " ".join(f"{x(i):.1f},{y(hi):.1f}" for (i, _), (_, _, hi) in zip(placed, smooth))
            lower = " ".join(f"{x(i):.1f},{y(lo):.1f}" for (i, _), (_, lo, _) in reversed(list(zip(placed, smooth))))
            out.append(f'<polygon points="{upper} {lower}" fill="{color}" opacity="0.15"/>')
            path = " ".join(f"{'M' if j == 0 else 'L'}{x(i):.1f},{y(m):.1f}"
                            for j, ((i, _), (m, _, _)) in enumerate(zip(placed, smooth)))
            out.append(f'<path d="{path}" fill="none" stroke="{color}" stroke-width="2" stroke-linejoin="round"/>')
        for i, p in placed:
            if p.high > p.low:
                out.append(f'<line x1="{x(i):.1f}" x2="{x(i):.1f}" y1="{y(p.low):.1f}" y2="{y(p.high):.1f}" '
                           f'stroke="{color}" stroke-width="2" stroke-linecap="round" opacity="0.5"/>')
        if len(placed) > 1 and not chart.smooth:
            path = " ".join(f"{'M' if j == 0 else 'L'}{x(i):.1f},{y(p.mid):.1f}" for j, (i, p) in enumerate(placed))
            out.append(f'<path d="{path}" fill="none" stroke="{color}" stroke-width="2" stroke-linejoin="round"/>')
        for i, p in placed:
            out.append(f'<circle cx="{x(i):.1f}" cy="{y(p.mid):.1f}" r="4" fill="{color}" '
                       f'stroke="var(--surface)" stroke-width="2"><title>{series.label}, {p.key[1]}'
                       f'{" with changes" if p.key[2] else ""} ({p.key[0]}): {range_text(p, chart.unit)}</title></circle>')
        if placed:
            labels.append((y(placed[-1][1].mid), f"{series.label}: {number(placed[-1][1].mid)}"))
    # Direct labels beside each series' last point, spaced apart so they never overlap.
    labels.sort()
    for k in range(1, len(labels)):
        if labels[k][0] - labels[k - 1][0] < LABEL_GAP:
            labels[k] = (labels[k - 1][0] + LABEL_GAP, labels[k][1])
    # Then up from the bottom, so the lowest stays inside the chart.
    limit = HEIGHT - 12.0
    for k in reversed(range(len(labels))):
        below = labels[k + 1][0] - LABEL_GAP if k + 1 < len(labels) else limit
        if labels[k][0] > below:
            labels[k] = (below, labels[k][1])
    for at, text in labels:
        out.append(f'<text class="label" x="{LEFT + plot_w + 12}" y="{at + 4:.1f}">{text}</text>')
    out.append("</svg>")
    return "\n".join(out) + "\n"


def subtitle(chart: Chart) -> str:
    if chart.smooth:
        return (f"{chart.unit}, by run; line: rolling median of {2 * SMOOTH_REACH + 1} runs, "
                "band: the spread around it")
    return f"{chart.unit}, by commit measured; whiskers show each range"


def range_text(p: Point, unit: str) -> str:
    return f"{number(p.low)} {unit}" if p.high == p.low else f"{number(p.low)}–{number(p.high)} {unit}"


def describe(chart: Chart, data: Data) -> str:
    parts = []
    for series, points in zip(chart.series, data.points):
        if points:
            ordered = sorted(points.values(), key=lambda p: data.keys.index(p.key))
            parts.append(f"{series.label}: {range_text(ordered[0], chart.unit)} at first, "
                         f"{range_text(ordered[-1], chart.unit)} latest ({ordered[-1].key[1]})")
    return "; ".join(parts) + "."


def render(check: bool = False) -> list[Path]:
    """Draws every chart from the CSV; with `check`, writes nothing and
    returns the charts that are out of date instead."""
    with CSV.open(newline="") as f:
        rows = list(csv.DictReader(f))
    stale = []
    for chart in CHARTS:
        svg = draw(chart, gather(rows, chart))
        path = OUT / f"{chart.name}.svg"
        if check:
            if not path.exists() or path.read_text() != svg:
                stale.append(path.relative_to(ROOT))
        else:
            OUT.mkdir(exist_ok=True)
            path.write_text(svg)
    return stale


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="fail if the charts are out of date")
    args = parser.parse_args()
    stale = render(args.check)
    if stale:
        print("Out of date; run `uv run compat/size_speed_charts.py`:", *stale, sep="\n  ", file=sys.stderr)
        return 1
    return 0

if __name__ == "__main__":
    sys.exit(main())
