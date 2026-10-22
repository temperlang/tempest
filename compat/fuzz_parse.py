# /// script
# requires-python = ">=3.11"
# ///
"""Compares temper-syntax's parse trees with Kotlin Temper's on mutated inputs.

Like fuzz_lex.py, but for check_parse.py's trees, with extra edits that
matter to the parser. Inputs that parse differently are shrunk and saved in
target/fuzz-parse/found/ as seedN-NNN.temper(.md), with the original beside
it. Inputs that crash Kotlin's parser are kept apart, as crash-*: they show
Kotlin bugs, not ours. So are inputs whose tokens already differ, as lex-*
(unshrunk; see fuzz_lex.py for those).

    uv run compat/fuzz_parse.py [--seed N] [--batches N] [--size N] [DIR ...]

Needs the same setup as check_lex.py.
"""

import argparse
import random
import sys
from pathlib import Path

from check_lex import DEFAULT_DIRS, QUIRKS, ROOT, build, temper_files
from check_parse import CASE_INPUTS, CASES, commands
from check_lex import write_cases
from fuzz_lex import MAX_SOURCE_BYTES, PIECES, mutate, outputs, shrink_while

WORK = ROOT / "target/fuzz-parse"

# Tokens that matter to the parser and its token adapters. Keep in sync with
# PARSE_PIECES in crates/temper-syntax/tests/parse.rs.
PARSE_PIECES = [
    "\n{", "}\n", "} else {", ";", ",", ":", "=", "@", "var ", "const ", "public ", "let ",
    "extends ", "case ", "->", "=>", "new ", "++", "-", "!", "?", "<", ">", "</", "${", "\\{",
    "\\(", "mixin(", "of ", "in ", "given ", "return ", ".", "?.", "...", "\n  :", "\n  \"",
    "\n  ~", '"""\n',
]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("dirs", nargs="*", type=Path, default=DEFAULT_DIRS)
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--batches", type=int, default=5)
    parser.add_argument("--size", type=int, default=2000, help="inputs per batch")
    args = parser.parse_args()

    both = commands()
    lex_kotlin, lex_rust = build()
    lexers = (lex_kotlin, lex_rust + [QUIRKS])
    files = sorted(write_cases(CASES, CASE_INPUTS, "tree")) + temper_files(args.dirs)
    sources = [p for p in files if Path(p).stat().st_size <= MAX_SOURCE_BYTES]
    texts = [(Path(p).read_text("utf-8"), p.endswith(".md")) for p in sources]
    rng = random.Random(args.seed)
    found = WORK / "found"
    found.mkdir(parents=True, exist_ok=True)

    def classify(results):
        """Per input: "crash" if Kotlin crashed, "differ" if the trees
        differ otherwise, else None."""
        for kotlin, rust in results:
            if kotlin and any(line.startswith("Crash") for line in kotlin):
                yield "crash"
            elif kotlin != rust:
                yield "differ"
            else:
                yield None

    def first(kind: str):
        def test(candidates):
            results = outputs(both, candidates, WORK / "shrink")
            return next((c for c, k in zip(candidates, classify(results)) if k == kind), None)
        return test

    checked = 0
    count = {"differ": 0, "crash": 0, "lex": 0}
    for batch in range(args.batches):
        inputs = []
        for _ in range(args.size):
            text, markdown = rng.choice(texts)
            inputs.append((mutate_parse(text, rng), markdown))
        results = outputs(both, inputs, WORK / f"batch{batch}")
        kinds = list(classify(results))
        # Set aside differences that start in the lexer.
        suspects = [i for i, kind in enumerate(kinds) if kind == "differ"]
        lexed = outputs(lexers, [inputs[i] for i in suspects], WORK / "lex")
        for i, (kotlin, rust) in zip(suspects, lexed):
            if kotlin != rust:
                kinds[i] = "lex"
        for (text, markdown), kind in zip(inputs, kinds):
            if kind is None:
                continue
            count[kind] += 1
            prefix = f"seed{args.seed}-" + ("" if kind == "differ" else f"{kind}-") + f"{count[kind]:03}"
            suffix = ".temper" + (".md" if markdown else "")
            # Save the original first: regenerating it needs the same sources.
            (found / f"{prefix}-original{suffix}").write_text(text, "utf-8", newline="")
            if kind == "lex" or (kind == "crash" and count[kind] > 5):
                continue  # Shrinking is slow, and these aren't parser differences.
            small = shrink_while(first(kind), text, markdown)
            (found / f"{prefix}{suffix}").write_text(small, "utf-8", newline="")
            print(f"{kind}: {found / (prefix + suffix)} ({len(text)} chars shrunk to {len(small)})")
        checked += len(inputs)
        print(
            f"batch {batch}: {checked} inputs checked, {count['differ']} differ, "
            f"{count['crash']} crash Kotlin, {count['lex']} lex differently",
            flush=True,
        )
    return 1 if count["differ"] else 0


def mutate_parse(text: str, rng: random.Random) -> str:
    """Edits like fuzz_lex.mutate, plus inserting parser pieces."""
    text = mutate(text, rng)
    for _ in range(rng.randint(0, 2)):
        at = rng.randint(0, len(text))
        text = text[:at] + rng.choice(PARSE_PIECES) + text[at:]
    return text


if __name__ == "__main__":
    sys.exit(main())
