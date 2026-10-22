# /// script
# requires-python = ">=3.11"
# ///
"""Generate temper-syntax's NFKC tables from the JDK.

Kotlin Temper checks identifiers with java.text.Normalizer, so the JDK on
PATH is the source of truth: compat/oracle/NfkcTables.java writes
crates/temper-syntax/src/nfkc_tables.rs and the test cases in
crates/temper-syntax/tests/nfkc.txt, recording the JDK's Java and Unicode
versions. The tables' Unicode notice comes from the JDK's own copy,
legal/java.base/unicode.md; the repo's NOTICE carries it too.

    uv run compat/gen_nfkc.py
"""

import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HERE = Path(__file__).resolve().parent
EXPORTS = [
    "--add-exports",
    "java.base/jdk.internal.icu.lang=ALL-UNNAMED",
    "--add-exports",
    "java.base/jdk.internal.icu.util=ALL-UNNAMED",
]


def unicode_notice() -> str:
    """The copyright and permission notice from the JDK's Unicode legal file."""
    java = Path(shutil.which("java") or sys.exit("java isn't on PATH")).resolve()
    legal = java.parent.parent / "legal/java.base/unicode.md"
    text = legal.read_text("utf-8")
    match = re.search(r"COPYRIGHT AND PERMISSION NOTICE.*?authorization of the copyright holder\.", text, re.S)
    if not match:
        sys.exit(f"no Unicode notice found in {legal}")
    return match.group(0)


def main() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        classes = Path(tmp)
        source = HERE / "oracle/NfkcTables.java"
        subprocess.run(["javac", *EXPORTS, "-d", str(classes), str(source)], check=True)
        notice = classes / "notice.txt"
        notice.write_text(unicode_notice(), "utf-8")
        tables = ROOT / "crates/temper-syntax/src/nfkc_tables.rs"
        cases = ROOT / "crates/temper-syntax/tests/nfkc.txt"
        subprocess.run(
            ["java", *EXPORTS, "-cp", str(classes), "NfkcTables", str(tables), str(cases), str(notice)],
            check=True,
        )
    print(f"wrote {tables.relative_to(ROOT)} and {cases.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
