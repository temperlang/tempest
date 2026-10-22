# /// script
# requires-python = ">=3.11"
# ///
"""Times temper-syntax's lexer, parser and grammar step against Kotlin Temper's.

Like bench_lex.py, but each pass lexes, parses and builds trees for all
files: for Kotlin, lang.temper.parser.parse then lang.temper.astbuild.buildTree,
as in its ParseStage, and for us, temper_syntax::ast::build.

    uv run compat/bench_ast.py [--runs N] [--rounds N] [DIR ...]

Needs the same setup as check_lex.py.
"""

import sys

from bench_lex import main

if __name__ == "__main__":
    sys.exit(main("AstTree", "ast-tree", __doc__))
