# CI and supply chain

CI runs other people's code: GitHub Actions, compilers, and the
checkers of generated code. Each is pinned, so a re-pointed tag or a
compromised release can't reach our builds, or our release token,
unnoticed; and each pin moves only on purpose, to a release at least two
weeks old, so a bad release has time to be caught first.

## What runs where

Linux x64 on every push and pull request, so a change is checked in
minutes; every other platform tempest is released for nightly or
weekly, and whenever someone asks (below).

### Every push and pull request: Linux x64

[ci.yml](../../.github/workflows/ci.yml) runs on every push to `main`
and every pull request, on `ubuntu-24.04`, in four parallel jobs, so a
run takes about as long as its slowest job:

| Job | What it checks |
| --- | --- |
| `lint, core` | formatting; clippy, with and without default features; the workflows parse as YAML; the docs' links; Temper's functional tests on the interpreter; the size and speed charts; then every crate's tests but the backends' and `tempest`'s, and again without default features, `tempest`'s included |
| `cli` | `tempest`'s tests with every backend, so every toolchain |
| `py, js, lua, rust` | those backends' tests, with mypy, pyright, TypeScript, Mocha, LuaLS, Lua 5.4.6 and LuaSocket, and again on LuaJIT; and the Rust runtime's own |
| `java, csharp, cpp` | those backends' tests, with Java 21 and JUnit, .NET 6, and g++ 13, sanitized too, and libcurl; and `strict`: all generated C++ at -O2 by g++ 12 and 14 and clang++ |
| `release, playground` | tempest's release build, the VS Code extension (tested in a real VS Code), the docs' examples, and the docs site; the wasm build, the playground, and its smoke test in headless Chromium |

Each check is a step of its own that runs even when an earlier one
fails, so a failure names its check, and one doesn't hide another
(unless it needs a build that failed). Each job sets up only the
toolchains its tests use, through one local action,
[.github/actions/toolchains](../../.github/actions/toolchains/action.yml),
which also says why each tool is there. The tests run with
`TEMPEST_REQUIRE_CHECKERS`, so a missing tool fails them rather than
skipping.

**What a change runs.** A first job, `changes`, lists the files changed
(`git diff` against the pull request's base, or against the commit
before a push), and the others run by what they read:

| Changed | Runs |
| --- | --- |
| only `notes/` | `lint, core`, without `core`'s tests |
| only `docs/`, `README.md`, and `notes/` | `lint, core` and `release, playground`: the REPL transcripts in the docs (`core`), the docs' examples, and the docs site |
| anything else | everything |

So notes alone skip the builds; anything outside `notes/`, `docs/`, and
`README.md`, even another Markdown file, runs everything, as do a new
branch and a base that can't be fetched. A skipped job counts as passed
for a required check (a workflow-level `paths-ignore` would leave
required checks waiting instead).

Each job caches its build with `Swatinem/rust-cache`, keyed by job (and
by group's `id`, for the backends, since keys can't have commas), saved
only from `main`; pull requests read
`main`'s. The cache holds third-party crates, built, and cargo's
downloads: tempest's own crates are rebuilt in each job, which is most
of a job's build, so there are few jobs, each running many checks. A
newer push to a pull request cancels the older run.

**Rough timings,** to be checked against the first full runs: about 15
to 25 minutes for each job, so 20 to 25 minutes for a run. A change to
notes alone takes about 10. GitHub's standard runners are free for
public repositories, so the job count is about time, not cost.

### Nightly, weekly, and on demand: the other platforms

[nightly.yml](../../.github/workflows/nightly.yml) runs its Linux jobs
and `platforms-quick` every night (03:41 UTC), `platforms` every
Sunday (05:17 UTC), and all of them whenever it's run by hand: on
GitHub, **Actions → Nightly → Run workflow**, picking a branch (or `gh
workflow run nightly.yml --ref BRANCH`). Its failures block nothing;
they're the cue to fix a platform before release.

A scheduled run skips what an earlier one already tested on the same
code: a first job, `recent`, reads the date of `main`'s latest commit,
and the nightly `cpp` and `platforms-quick` jobs run only if it's under
26 hours old (a day, with room for GitHub's delays in starting
scheduled runs), and the weekly `platforms` only if it's under 8 days.
`libraries` and `latest-checkers` run every night regardless, since
they test other repositories' `main` and the checkers' latest
releases. Run by hand, everything runs.

Besides its Linux jobs (more C++ compilers, real libraries, and the
checkers' latest releases; below), it tests, natively, each other
platform [binaries.yml](../../.github/workflows/binaries.yml) releases
for, one job each from one definition, in two tiers:

| Target                      | Runner             |
| --------------------------- | ------------------ |
| `aarch64-unknown-linux-gnu` | `ubuntu-24.04-arm` |
| `aarch64-apple-darwin`      | `macos-15`         |
| `x86_64-pc-windows-msvc`    | `windows-2025`     |
| `aarch64-pc-windows-msvc`   | `windows-11-arm`   |

There's no Intel macOS (`x86_64-apple-darwin`) binary, as in Kotlin
Temper's regular bundles; Intel Macs can build tempest from source with
cargo.

- **`platforms-quick`, nightly,** tests the code that differs by
  platform, which needs no backend's toolchain: first `temper-os`, the
  OS calls, linted by clippy as that platform compiles them (Linux's
  push CI lints only Linux's), with the inventory of unsafe code; the
  REPL's Ctrl-C
  (SIGINT on Unix, the console's handler on Windows), ANSI color on
  Windows' console, and where history is kept (`~/Library/Application
  Support` on macOS, `%APPDATA%` on Windows); and child processes'
  (`temper-tmpl`'s `children.rs`): process groups and time limits on
  Unix, and time limits alone on Windows, which has no process groups
  yet. Then all of `tempest`'s tests without backends.
- **`platforms`, weekly,** does the same, then each backend's tests, a
  step each, with the same toolchain action as `ci.yml`'s, and
  `tempest`'s tests with that platform's backends built in. It repeats
  the quick tier's steps, which share its build, so every run of
  either tier checks the REPL and child processes.

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
tests are added. The backends' tests run with
`TEMPEST_REQUIRE_CHECKERS`, so a missing tool fails, except what a
platform is let off by name (below). Windows checks text files out
with CRLF line ends, so the job turns that off (`core.autocrlf false`)
before checking out. Warnings are errors (`RUSTFLAGS`), so code only
these platforms compile stays as clean as clippy keeps Linux's; the
last step, `tempest`'s tests with backends, builds without, as
`tempest test -b rust` passes them on to cargo on generated crates.

**Each platform's toolchains:**

| | Linux Arm64 | macOS Arm64 | Windows x64 | Windows Arm64 |
| --- | --- | --- | --- | --- |
| Python | the image's 3.12 | 3.12 from uv | 3.12 from uv | 3.12 from uv |
| mypy, pyright, TypeScript, Mocha | pinned | pinned | pinned | pinned |
| Node | 24 | 24 | 24 | 24 |
| Lua | 5.4.6 and LuaSocket, Ubuntu's | Homebrew's, no LuaSocket | MSYS2's, no LuaSocket | **none** |
| LuaLS | **skipped** | **skipped** | **skipped** | **none** |
| Java, JUnit | 21, 5.9.2 | 21, 5.9.2 | 21, 5.9.2 | 21, 5.9.2 |
| .NET | 6 | 6 | 6 | 6 |
| C++ | g++ 13, sanitized, and `strict` by g++ 12 and 14 and clang++ | Apple's clang, through CMake | MSVC, through CMake (Ninja) | MSVC, through CMake (Ninja) |
| Rust | 1.85.0 | 1.85.0 | 1.85.0 | 1.85.0 |

What's left out, and why:

- **LuaLS** is checked against a SHA-256 recorded for each platform's
  download in [install.py](../../.github/ci-tools/install.py)'s
  `LUALS_SHA256`, and only Linux x64's is recorded. The job lets it be
  missing (`TEMPEST_ALLOW_MISSING: LuaLS`), so the Lua tests skip their
  type check, saying so. To check LuaLS on a platform, record the
  `sha256sum` of its asset (`lua-language-server-VERSION-linux-arm64.tar.gz`,
  `-darwin-arm64.tar.gz`, or `-win32-x64.zip`) there; once all three
  are, drop the allowance. LuaLS has no Windows Arm64 build.
- **Lua on Windows Arm64:** its image has no MSYS2, so the job leaves
  out the Lua backend (its tests, and the `lua` feature from
  `tempest`'s).
- **LuaSocket off Linux:** std `net`'s requests on Lua fail as Kotlin's
  suite expects of a Lua without a client, as the tests note.
- **The C++ backend's own runs on macOS and Windows** (`runs`,
  `strict`, `co_await_interop`, and `net_client_from_host`, skipped by
  name): they build with g++'s options and sanitizers. C++ is tested
  there through CMake, as users build it, by `tempest`'s tests
  (`tempest test -b cpp` and CMake's build of `tempest build`'s
  output); the C++ backend's goldens and other tests still run.
- **LuaJIT** runs on Linux x64 only, in `ci.yml`.

`TEMPEST_ALLOW_MISSING` takes comma-separated words, each matched in a
missing tool's message; the test still says what it skipped.

**Timings,** to be checked against the first runs: `platforms`' jobs
build tempest twice (with and without `RUSTFLAGS`) and each backend's
tests, so perhaps 30 to 60 minutes, Windows the slowest;
`platforms-quick`'s build tempest once, without toolchains, so perhaps
10 to 15. Standard runners are free for public repositories, on every
OS; `platforms` runs weekly, and the nightly jobs skip a day without
commits, to keep the queue short and spare GitHub's machines.
Should GitHub stop offering an image, drop its row and keep building
that target in binaries.yml.

[binaries.yml](../../.github/workflows/binaries.yml) builds the released
binaries nightly, cross-compiling Windows Arm64 on x64, and runs
nothing on them; nightly.yml's `platforms` jobs are where they're
tested.

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

- **Runner images:** ci.yml's and pages.yml's jobs are pinned to
  `ubuntu-24.04`, for Lua (above); nightly.yml's `platforms` and `platforms-quick` to the
  images in its table above, so a new image arrives as a deliberate
  bump, not a surprise.

Not pinned: the other jobs' runner images, and what the images and
`apt` install besides Lua; Node, Java, and .NET by major version; and
Playwright's Chromium (from its own CDN, by the version in the Mermaid
renderer's lockfile).

## Flaky downloads

Downloads from servers far from GitHub's runners sometimes fail, so CI
avoids or retries them: Lua comes from Ubuntu's mirrors, not lua.org;
[apt-install.sh](../../.github/ci-tools/apt-install.sh) skips packages
the image has, updates from Ubuntu's own sources only, and retries each
download three times with a 30-second timeout, so a dead mirror fails
in minutes; Playwright's Chromium is installed without `--with-deps`
(which runs apt), as the image has Chrome's libraries; VS Code is cached
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
  opens one monthly pull request updating the SHAs and their comments,
  in the workflows and the local action together, for releases at least
  two weeks old, majors included. Read each major's release notes and
  check the inputs we pass before merging, and after merging, run
  Nightly, Binaries, and Pages by hand, since only `ci.yml` runs on the
  pull request. By hand: resolve the release's
  tag to its commit (`git ls-remote <repo> refs/tags/<tag>`, taking the
  `^{}` line for an annotated tag), and update the SHA and the comment
  everywhere the action is used.
- **Ubuntu, for Lua:** moving a pinned job to a newer image changes
  `lua5.4` and `lua-socket`; update the versions its Lua step checks,
  and the tests' local versions, together.
- **macOS and Windows images:** the matrices of `platforms` and
  `platforms-quick` in nightly.yml, once GitHub offers the newer image;
  its cache starts afresh.
- **VS Code:** `VSCODE_VERSION` in build.py; CI's cache key follows it.
- **The Rust toolchain:** take the SHA of the new version's branch
  (`git ls-remote https://github.com/dtolnay/rust-toolchain
  refs/heads/<version>`), and update the workspace's `rust-version`.
- **uv:** `version:` and `checksum:` in each workflow and in
  [the toolchains action](../../.github/actions/toolchains/action.yml);
  the checksum for `x86_64-unknown-linux-gnu-<version>` is in setup-uv's
  `src/download/checksum/known-checksums.json`, which setup-uv also
  checks the other platforms' downloads against.
- **TypeScript, pyright, Mocha:** the version in
  [package.json](../../.github/ci-tools/package.json), then `npm install
  --package-lock-only --ignore-scripts` there.
- **mypy, LuaLS:** `MYPY`, `LUALS`, and `LUALS_SHA256` in
  [install.py](../../.github/ci-tools/install.py); each hash is
  `sha256sum` of that platform's asset from LuaLS's release, as
  `lua-language-server-<version>-linux-x64.tar.gz`.

A checker bump comes with any goldens or backend fixes it needs, after
the nightly job for it has passed.
