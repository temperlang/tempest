#!/usr/bin/env bash
# Runs a `cargo test` command, then fails unless at least MIN tests
# passed in all, so tests that a platform's `cfg` leaves out, or a
# filter that matches nothing, can't pass silently:
#
#     bash .github/ci-tools/tests-ran.sh MIN cargo test ...
#
# Bash, not Python, as it runs on macOS and Windows (Git Bash) runners
# without uv.
set -euo pipefail
min=$1
shift
out=$(mktemp)
"$@" 2>&1 | tee "$out"
passed=$(sed -n 's/^test result: ok\. \([0-9][0-9]*\) passed.*/\1/p' "$out" |
  awk '{ n += $1 } END { print n + 0 }')
rm -f "$out"
if [ "$passed" -lt "$min" ]; then
  echo "::error::$passed tests passed, not at least $min: $*"
  exit 1
fi
echo "$passed tests passed (at least $min expected)"
