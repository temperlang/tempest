# /// script
# requires-python = ">=3.11"
# ///
"""Compares temper-syntax's tokens with Kotlin Temper's on mutated inputs.

Takes the cases in crates/temper-syntax/tests/*.cases plus small Temper files
from the given directories, makes randomly edited copies, and lexes them all
with both lexers. Inputs that lex differently are shrunk to a small input
that still differs and saved in target/fuzz-lex/found/, ready to fix and then
add to tests/edge-cases.cases. Ours lexes with --kotlin-quirks, as in
check_lex.py, so deliberate deviations don't count. The original input is saved beside it, since
the seed alone doesn't reproduce it once the sources change.

    uv run compat/fuzz_lex.py [--seed N] [--batches N] [--size N] [DIR ...]

Needs the same setup as check_lex.py.
"""

import argparse
import random
import shutil
import sys
from pathlib import Path

from check_lex import DEFAULT_DIRS, QUIRKS, ROOT, build, by_file, run, temper_files, write_cases

WORK = ROOT / "target/fuzz-lex"
MAX_SOURCE_BYTES = 4000

# Characters that matter to the lexer, plus some non-ASCII ones. Keep in sync
# with PIECES in crates/temper-syntax/tests/lex.rs.
PIECES = [
    '"', '"""', "'", "`", "{", "}", "(", ")", "[", "]", "<", ">", "/", "*", "\\", "$", "${",
    "\\u{", ":", "~", "#", ";", ".", ",", "-", "=", "!", "0", "1e", "x", "nym`", "\n", "\r\n",
    "  ", "\t", "\n```temper\n", "\n```\n", "\n    ", "é", "ό", "️", "’", "😀", "\0",
    "return /", "/*", "*/", "//",
]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("dirs", nargs="*", type=Path, default=DEFAULT_DIRS)
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--batches", type=int, default=5)
    parser.add_argument("--size", type=int, default=2000, help="inputs per batch")
    args = parser.parse_args()

    kotlin, rust = build()
    commands = (kotlin, rust + [QUIRKS])
    files = sorted(write_cases()) + temper_files(args.dirs)
    sources = [p for p in files if Path(p).stat().st_size <= MAX_SOURCE_BYTES]
    texts = [(Path(p).read_text("utf-8"), p.endswith(".md")) for p in sources]
    rng = random.Random(args.seed)
    found = WORK / "found"
    found.mkdir(parents=True, exist_ok=True)

    checked = differ = 0
    for batch in range(args.batches):
        inputs = []
        for _ in range(args.size):
            text, markdown = rng.choice(texts)
            inputs.append((mutate(text, rng), markdown))
        for text, markdown in differing(commands, inputs, f"batch{batch}"):
            differ += 1
            # Save the original first, since regenerating it needs the same
            # sources and settings as well as the seed.
            suffix = ".temper" + (".md" if markdown else "")
            (found / f"seed{args.seed}-{differ:03}-original{suffix}").write_text(text, "utf-8", newline="")
            small = shrink(commands, text, markdown)
            name = f"seed{args.seed}-{differ:03}{suffix}"
            (found / name).write_text(small, "utf-8", newline="")
            print(f"differs: {found / name} ({len(text)} chars shrunk to {len(small)})")
        checked += len(inputs)
        print(f"batch {batch}: {checked} inputs checked, {differ} differ", flush=True)
    return 1 if differ else 0


def mutate(text: str, rng: random.Random) -> str:
    """Makes a few random edits: inserting pieces, deleting, duplicating or
    truncating."""
    for _ in range(rng.randint(1, 4)):
        at = rng.randint(0, len(text))
        match rng.randrange(4):
            case 0 | 1:
                text = text[:at] + rng.choice(PIECES) + text[at:]
            case 2:
                text = text[:at] + text[at + rng.randrange(8):]
            case _:
                end = at + rng.randrange(16)
                text = text[:end] + text[at:end] + text[end:]
    if rng.randrange(10) == 0:
        text = text[: rng.randint(0, len(text))]
    return text


def outputs(commands: tuple[list[str], list[str]], texts: list[tuple[str, bool]], directory: Path):
    """Writes the inputs to files in `directory` and returns each one's
    output from both sides, as (kotlin, rust) line lists."""
    shutil.rmtree(directory, ignore_errors=True)
    directory.mkdir(parents=True)
    paths = []
    for i, (text, markdown) in enumerate(texts):
        path = directory / (f"{i:05}.temper" + (".md" if markdown else ""))
        path.write_text(text, "utf-8", newline="")
        paths.append(str(path))
    kotlin = by_file(run(commands[0], paths))
    rust = by_file(run(commands[1], paths))
    shutil.rmtree(directory)
    return [(kotlin.get(path), rust.get(path)) for path in paths]


def differing(commands: tuple[list[str], list[str]], inputs: list[tuple[str, bool]], label: str):
    """Yields the inputs that the two lexers lex differently."""
    for item, (kotlin, rust) in zip(inputs, outputs(commands, inputs, WORK / label)):
        if kotlin != rust:
            yield item


def shrink_while(test, text: str, markdown: bool) -> str:
    """Removes chunks of text while `test` still finds a smaller input in a
    batch of candidates, trying all removals of one chunk size per batch.
    `test(candidates)` returns the first candidate that still qualifies."""
    chunk = max(1, len(text) // 2)
    while True:
        candidates = [text[:i] + text[i + chunk:] for i in range(0, len(text), chunk)]
        smaller = test([(c, markdown) for c in candidates])
        if smaller is not None:
            text = smaller[0]
            chunk = min(chunk, max(1, len(text) // 2))
        elif chunk > 1:
            chunk //= 2
        else:
            return text


def shrink(commands: tuple[list[str], list[str]], text: str, markdown: bool) -> str:
    """Shrinks an input while the lexers still differ."""
    return shrink_while(
        lambda candidates: next(iter(differing(commands, candidates, "shrink")), None), text, markdown
    )


if __name__ == "__main__":
    sys.exit(main())
