"""Runs many functional-test cases in one interpreter, for `functional.rs`.

Reads a JSON list of cases from stdin, each `{"key", "cwd", "path",
"package"}`, and for each, in turn: puts its `path` entries first on
`sys.path`, imports its package (which logs its output) in a top-level
turn, so its async blocks start after its top-level code, waits for its
async work, and then forgets every module the import added, so the next
case's same-named libraries load fresh. Before each case it prints a line
`\\x1eKEY`; after an exception, a line `\\x1e!` and the traceback. The
caller compares each case's lines with what it expects and reruns any
case that doesn't match in a process of its own, for its precise result.
"""

import json
import os
import sys
import traceback

import temper_core


def main() -> None:
    cases = json.load(sys.stdin)
    temper_core.init_simple_logging()
    base_path = list(sys.path)
    base_modules = set(sys.modules)
    for case in cases:
        print(f"\x1e{case['key']}", flush=True)
        os.chdir(case["cwd"])
        sys.path[:] = case["path"] + base_path
        try:
            with temper_core.top_level_turn():
                __import__(case["package"])
            temper_core.await_safe_to_exit()
        except BaseException:
            sys.stdout.flush()
            print("\x1e!", flush=True)
            traceback.print_exc(file=sys.stdout)
            sys.stdout.flush()
        for name in set(sys.modules) - base_modules:
            del sys.modules[name]
    temper_core.await_safe_to_exit()


if __name__ == "__main__":
    main()
