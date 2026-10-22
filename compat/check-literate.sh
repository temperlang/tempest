#!/bin/bash
# Compares temper-literate's blocks with Kotlin Temper's for every .temper.md
# file under the given directories (default: the temper and
# prismora checkouts beside this repository). Prints differing files and a summary.
#
# Needs an installed Kotlin Temper (TEMPER_LIB, default below) and a JDK.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
root=$(dirname "$here")
lib=${TEMPER_LIB:-$HOME/projects/temper/cli/build/install/temper/lib}
dirs=("$@")
[ ${#dirs[@]} -eq 0 ] && dirs=("$HOME/projects/temper" "$HOME/projects/prismora")

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cp=$(ls -d "$lib"/*.jar | tr '\n' ':')
classes="$here/oracle/classes"
if [ ! -f "$classes/MarkdownBlocks.class" ] || [ "$here/oracle/MarkdownBlocks.java" -nt "$classes/MarkdownBlocks.class" ]; then
  mkdir -p "$classes"
  javac -nowarn -cp "$cp" -d "$classes" "$here/oracle/MarkdownBlocks.java"
fi
cargo build -q --release --manifest-path "$root/Cargo.toml" -p temper-literate

find "${dirs[@]}" -name '*.temper.md' -not -path '*/build/*' -not -path '*/node_modules/*' \
  -not -path '*/temper.out/*' | sort > "$work/files"

xargs java -cp "$classes:$cp" MarkdownBlocks < "$work/files" > "$work/kotlin.txt"
xargs "$root/target/release/literate-blocks" < "$work/files" > "$work/rust.txt"

# Split per file and compare.
split_by_file() {
  awk -v dir="$2" '/^# / { f = dir "/" NR; print substr($0, 3) > (f ".name"); next } { print > (f ".txt") }' "$1"
}
mkdir -p "$work/k" "$work/r"
split_by_file "$work/kotlin.txt" "$work/k"
split_by_file "$work/rust.txt" "$work/r"

total=0
differ=0
while read -r file; do
  total=$((total + 1))
  k=$(awk -v f="$file" '$0 == "# " f { on = 1; next } /^# / { on = 0 } on' "$work/kotlin.txt")
  r=$(awk -v f="$file" '$0 == "# " f { on = 1; next } /^# / { on = 0 } on' "$work/rust.txt")
  if [ "$k" != "$r" ]; then
    differ=$((differ + 1))
    echo "DIFFERS: $file"
    if [ -n "${VERBOSE:-}" ]; then
      diff <(echo "$k") <(echo "$r") | head -20 || true
    fi
  fi
done < "$work/files"
echo "$((total - differ)) of $total files match Kotlin Temper"
[ "$differ" -eq 0 ]
