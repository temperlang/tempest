#!/usr/bin/env bash
# Installs packages from Ubuntu's archive on a GitHub-hosted Ubuntu
# runner, failing fast and saying why:
#
#     bash .github/ci-tools/apt-install.sh PACKAGE...
#
# - Packages the image already has (as ubuntu-24.04 has g++-12, g++-14,
#   and clang) are skipped, and with none left, apt isn't run at all.
# - `apt-get update` reads only Ubuntu's own sources, not the image's
#   others (Microsoft's and the like), which the packages don't need and
#   whose one unreachable host would fail the update.
# - Each download gives up after 30 seconds (retried 3 times), not apt's
#   default 120, and each apt run after 4 minutes in all. If Azure's
#   mirror (the image's default) is slow or down, the update and install
#   are tried once more from archive.ubuntu.com.
# - A compiler counts as installed when its command is on PATH (the image
#   has clang as clang-18's package, not `clang`).
#
# Bash, not Python, so jobs that don't set up uv can use it.
set -euo pipefail
missing=()
for package in "$@"; do
  case "$package" in
    clang) command="clang++" ;;
    g++-*|clang-*) command="$package" ;;
    *) command="" ;;
  esac
  if { [ -n "$command" ] && command -v "$command" >/dev/null; } \
    || dpkg-query -W -f '${Status}' "$package" 2>/dev/null | grep -q "install ok installed"; then
    echo "Already installed: $package"
  else
    missing+=("$package")
  fi
done
if [ ${#missing[@]} -eq 0 ]; then
  exit 0
fi
apt=(sudo timeout 240 apt-get -o Acquire::Retries=3 -o Acquire::http::Timeout=30 -o Acquire::https::Timeout=30)
# Ubuntu's sources: sources.list (older images), and ubuntu.sources in
# sources.list.d (24.04 and later), alone in a directory of their own.
parts=$(mktemp -d)
if [ -f /etc/apt/sources.list.d/ubuntu.sources ]; then
  cp /etc/apt/sources.list.d/ubuntu.sources "$parts/"
fi
try_install() {
  "${apt[@]}" -o Dir::Etc::SourceParts="$parts" update \
    && "${apt[@]}" -o Dir::Etc::SourceParts="$parts" install -y "${missing[@]}"
}
if ! try_install; then
  echo "apt failed or timed out; trying archive.ubuntu.com instead of the image's mirror"
  if [ -f "$parts/ubuntu.sources" ]; then
    sudo sed -i 's|^URIs: .*mirror.*|URIs: http://archive.ubuntu.com/ubuntu/|' "$parts/ubuntu.sources"
  fi
  sudo sed -i 's|http://azure.archive.ubuntu.com/ubuntu|http://archive.ubuntu.com/ubuntu|' /etc/apt/sources.list 2>/dev/null || true
  try_install
fi
rm -rf "$parts"
