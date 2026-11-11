# CI and supply chain

CI runs other people's code: GitHub Actions, compilers, and the
checkers of generated code. Each is pinned, so a re-pointed tag or a
compromised release can't reach our builds, or our release token,
unnoticed; and each pin moves only on purpose, to a release at least two
weeks old, so a bad release has time to be caught first.

## What runs where

[ci.yml](../../.github/workflows/ci.yml) runs on every push and pull
request, in two jobs:

- **`check`, on Linux** (`ubuntu-24.04`): formatting, clippy, every
  crate's tests with every backend's toolchain and checker, the wasm
  build, the playground, the VS Code extension, the docs, and the
  functional tests. Nearly all of tempest is the same on every
  platform, so this is where it's tested.
- **`platforms`, on the other platforms tempest is released for:**
  the code that differs by platform, natively, on each OS and
  architecture's own GitHub runner, one job each from one definition:

  | Target                      | Runner             |
  | --------------------------- | ------------------ |
  | `aarch64-unknown-linux-gnu` | `ubuntu-24.04-arm` |
  | `aarch64-apple-darwin`      | `macos-15`         |
  | `x86_64-pc-windows-msvc`    | `windows-2025`     |
  | `aarch64-pc-windows-msvc`   | `windows-11-arm`   |

  `x86_64-unknown-linux-gnu` is `check`'s, which runs these tests
  among the workspace's. There's no Intel macOS (`x86_64-apple-darwin`)
  binary, as in Kotlin Temper's regular bundles; Intel Macs can build
  tempest from source with cargo.

  The code: the REPL's Ctrl-C (SIGINT on Unix, the console's handler
  on Windows), ANSI color on Windows' console,
  and where history is kept (`~/Library/Application Support` on macOS,
  `%APPDATA%` on Windows); and child processes' (`temper-tmpl`'s
  `children.rs`): process groups and time limits on Unix, and time
  limits alone on Windows, which has no process groups yet. Each job
  builds `tempest` with every backend, runs the REPL's tests and
  `children.rs`'s, and then all of `tempest`'s, built without backends
  (`--no-default-features`): their tests would run whatever toolchains
  the image has, without the setup `check` gives them (JUnit's jars,
  crates for generated Rust, .NET 6), and fail rather than skip.
  Testing the backends on these platforms is a job of its own, for
  later.

  Tests that need Linux or Unix skip by `cfg`: the terminal test needs
  Linux's `script` (so it runs on Linux Arm64 too), and the Ctrl-C test
  sends SIGINT. On Windows, a test of its own sends Ctrl-Break, which,
  unlike Ctrl-C, can go to one console process group, and the REPL
  takes as Ctrl-C. A UTF-8 test, on every platform, checks piped
  output's bytes, so no code page converts them. So that a platform
  can't pass by leaving tests out,
  [tests-ran.sh](../../.github/ci-tools/tests-ran.sh) fails a step
  unless at least a given number of tests passed (for the REPL's
  command, all of them on macOS and Windows); raise its counts as
  tests are added. Windows checks text files out with CRLF line ends,
  so the job turns that off (`core.autocrlf false`) before checking
  out. Warnings are errors (`RUSTFLAGS`), so code only these platforms
  compile stays as clean as clippy keeps Linux's.

  It's lean, as runners' minutes cost differently: on GitHub's
  standard runners, a Windows minute counts as about twice a Linux
  one, and a macOS minute about ten times, while Arm64 Linux is
  cheaper than x64 (check GitHub's current billing page before adding
  macOS jobs). So: a debug build, cached by rust-cache, with no
  checkers, toolchains, or clippy, and a 30-minute limit. Should
  GitHub stop offering an image, drop its row and keep building that
  target in binaries.yml.

[binaries.yml](../../.github/workflows/binaries.yml) builds the released
binaries nightly, cross-compiling Windows Arm64 on x64, and runs
nothing on them; `platforms` is where they're tested.
[nightly.yml](../../.github/workflows/nightly.yml) runs on Linux.

## What's pinned

- **Actions,** GitHub's own and third parties', by full commit SHA with
  the version in a comment (`uses: actions/checkout@<sha> # v4.4.0`).
  `dtolnay/rust-toolchain` is pinned to the commit of its `1.85.0`
  branch, which names the toolchain; it has no version tags.
- **Tokens:** every workflow's token is read-only (`permissions:
  contents: read`) except where a job needs more: the `release` and
  `nightly` jobs in [binaries.yml](../../.github/workflows/binaries.yml)
  (`contents: write`) and [pages.yml](../../.github/workflows/pages.yml)
  (`pages: write`, `id-token: write`).
- **Crates:** every `cargo` command over the workspace runs with
  `--locked`, so CI builds exactly `Cargo.lock`. Crates the backends'
  tests generate have no lockfile and aren't locked.
- **uv,** by version and SHA-256 (setup-uv's `version:` and
  `checksum:`).
- **Checkers of generated code,** and Mocha, which runs generated JS
  tests, installed by
  [.github/ci-tools/install.py](../../.github/ci-tools/install.py):
  TypeScript, pyright, and Mocha from the lockfile beside it, by `npm ci
  --ignore-scripts`; mypy at an exact version by `uv tool install`; and
  LuaLS at an exact version, its download checked against a recorded
  SHA-256.
- **Lua and LuaSocket,** from Ubuntu's archive (`lua5.4` and
  `lua-socket`), by pinning the jobs that install them to
  `ubuntu-24.04`, whose packages are Lua 5.4.6 and LuaSocket 3.1.0, the
  versions the tests use locally. The install step checks both
  versions, so a change fails there, not in the tests.
- **VS Code,** for the extension's tests, by `VSCODE_VERSION` in
  [editors/vscode/build.py](../../editors/vscode/build.py).

- **Runner images:** `check` is pinned to `ubuntu-24.04`, for Lua
  (above); `platforms` to the images in its table above, so a new image
  arrives as a deliberate bump, not a surprise.

Not pinned: the other jobs' runner images, and what the images and
`apt` install besides Lua; Node, Java, and .NET by major version; and
Playwright's Chromium (from its own CDN, by the version in the Mermaid
renderer's lockfile).

## Flaky downloads

Downloads from servers far from GitHub's runners sometimes fail, so CI
avoids or retries them: Lua comes from Ubuntu's mirrors, not lua.org;
`apt-get` retries (`-o Acquire::Retries=3`); VS Code is cached
(`actions/cache`, keyed on its version), and
[test/run.ts](../../editors/vscode/test/run.ts) tries a failed download
three times before running the tests, which run once, and prints each
error inside an `AggregateError` or `cause`.

## Latest checkers, nightly

New checker releases often find new things to say about generated
code. The `latest-checkers` jobs in
[nightly.yml](../../.github/workflows/nightly.yml) run the backends'
tests with one checker at a time at its latest release (TypeScript,
pyright, mypy, or LuaLS), the rest at their pins. They block nothing: a
failure, named for its checker and with the versions in its summary,
is the cue to fix the backend or the tests before bumping that pin.

## Bumping a pin

- **Actions:** Dependabot ([.github/dependabot.yml](../../.github/dependabot.yml))
  opens a monthly pull request updating the SHAs and their comments,
  for releases at least two weeks old: one for every minor and patch
  bump, and one per major bump. Read a major's release notes and check
  the inputs we pass before merging, and after merging, run Nightly and
  Binaries by hand, since only `ci.yml` runs on the pull request. By hand: resolve the release's
  tag to its commit (`git ls-remote <repo> refs/tags/<tag>`, taking the
  `^{}` line for an annotated tag), and update the SHA and the comment
  everywhere the action is used.
- **Ubuntu, for Lua:** moving a pinned job to a newer image changes
  `lua5.4` and `lua-socket`; update the versions its Lua step checks,
  and the tests' local versions, together.
- **macOS and Windows images:** `platforms`' matrix in ci.yml, once
  GitHub offers the newer image; its cache starts afresh.
- **VS Code:** `VSCODE_VERSION` in build.py; CI's cache key follows it.
- **The Rust toolchain:** take the SHA of the new version's branch
  (`git ls-remote https://github.com/dtolnay/rust-toolchain
  refs/heads/<version>`), and update the workspace's `rust-version`.
- **uv:** `version:` and `checksum:` in each workflow; the checksum for
  `x86_64-unknown-linux-gnu-<version>` is in setup-uv's
  `src/download/checksum/known-checksums.json`.
- **TypeScript, pyright, Mocha:** the version in
  [package.json](../../.github/ci-tools/package.json), then `npm install
  --package-lock-only --ignore-scripts` there.
- **mypy, LuaLS:** `MYPY`, `LUALS`, and `LUALS_SHA256` in
  [install.py](../../.github/ci-tools/install.py); the hash is
  `sha256sum` of `lua-language-server-<version>-linux-x64.tar.gz` from
  LuaLS's release.

A checker bump comes with any goldens or backend fixes it needs, after
the nightly job for it has passed.
