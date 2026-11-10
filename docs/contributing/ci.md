# CI and supply chain

CI runs other people's code: GitHub Actions, compilers, and the
checkers of generated code. Each is pinned, so a re-pointed tag or a
compromised release can't reach our builds, or our release token,
unnoticed; and each pin moves only on purpose, to a release at least two
weeks old, so a bad release has time to be caught first.

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
