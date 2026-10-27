//! Tests of the `tempest` binary's command line. What `test` reports is
//! tested in temper-frontend's `tests/library_tests.rs`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Runs `tempest`, giving its exit code, stdout and stderr.
fn tempest(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_tempest"))
        .args(args)
        .output()
        .unwrap();
    let text = |b: Vec<u8>| String::from_utf8(b).unwrap();
    (
        output.status.code().unwrap(),
        text(output.stdout),
        text(output.stderr),
    )
}

#[test]
fn usage() {
    assert_eq!(tempest(&["build"]).0, 2);
    assert_eq!(tempest(&[]).0, 2);
    assert_eq!(tempest(&["test", "a", "b"]).0, 2);
    assert_eq!(tempest(&["test", "--std"]).0, 2);
    assert_eq!(tempest(&["test", "--bench"]).0, 2);
    let (code, out, _) = tempest(&["--help"]);
    assert_eq!(
        (code, out.as_str()),
        (0, "usage: tempest test [--std DIR] [DIR]\n")
    );
}

#[test]
fn test_a_library() {
    let dir = std::env::temp_dir().join(format!("tempest-cli-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("config.temper.md"),
        "    export let name = \"a\";\n",
    )
    .unwrap();
    fs::write(dir.join("a.temper"), "test(\"a\") { assert(1 + 1 == 2) }\n").unwrap();
    let std_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/temper/std");
    let dir_arg = dir.to_string_lossy().into_owned();
    let (code, out, err) = tempest(&["test", "--std", &std_dir.to_string_lossy(), &dir_arg]);
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(err, "");
    assert!(
        out.ends_with("  ok      a\nTests passed: 1 of 1\n"),
        "{out}"
    );
    assert_eq!(code, 0);
}
