//! Records the commit tempest is built from, for `tempest version`, as
//! `TEMPEST_COMMIT`, where git can say; nothing otherwise, as when built
//! from a copy of the sources outside a repository.

use std::process::Command;

fn main() {
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    if let Some(commit) = git(&["rev-parse", "--short=10", "HEAD"]) {
        println!("cargo:rustc-env=TEMPEST_COMMIT={commit}");
    }
    // Again when HEAD moves: a commit, or another branch checked out.
    for dir in [
        git(&["rev-parse", "--git-dir"]),
        git(&["rev-parse", "--git-common-dir"]),
    ]
    .into_iter()
    .flatten()
    {
        for file in ["HEAD", "refs", "packed-refs"] {
            let path = std::path::Path::new(&dir).join(file);
            if path.exists() {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
}
