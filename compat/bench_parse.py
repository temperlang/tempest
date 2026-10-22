# /// script
# requires-python = ">=3.11"
# ///
"""Times temper-syntax's lexer and parser against Kotlin Temper's.

Like bench_lex.py, but each pass lexes and parses all files: for Kotlin,
lang.temper.parser.parse with its token adapters and post-processing, and
for us, temper_syntax::parse::parse.

    uv run compat/bench_parse.py [--runs N] [--rounds N] [DIR ...]

Needs the same setup as check_lex.py.
"""

import sys

from bench_lex import main

if __name__ == "__main__":
    sys.exit(main("ParseTree", "parse-tree", __doc__))
