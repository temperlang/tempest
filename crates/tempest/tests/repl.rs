//! Tests of `tempest repl` as a command: piped transcripts, its flags,
//! the work root's libraries, and Ctrl-C. What a session prints for
//! each input is tested in temper-frontend's `tests/repl.rs`; the
//! tutorial's transcripts, in `tests/repl_docs.rs`.

use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

/// Runs `tempest repl` with `args`, in `dir` if given, piping `input` in;
/// gives its exit code, stdout and stderr.
fn repl(args: &[&str], dir: Option<&Path>, input: &str) -> (i32, String, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tempest"));
    command.arg("repl").args(args);
    if let Some(dir) = dir {
        command.current_dir(dir);
    }
    let mut child = (command.stdin(Stdio::piped()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let text = |b: Vec<u8>| String::from_utf8(b).unwrap();
    (
        output.status.code().unwrap(),
        text(output.stdout),
        text(output.stderr),
    )
}

/// A fresh directory, removed after `f`.
fn with_dir(test: &str, f: impl FnOnce(&Path)) {
    let dir = std::env::temp_dir().join(format!("tempest-repl-{}-{test}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    f(&dir);
    let _ = fs::remove_dir_all(&dir);
}

/// Piped input gives each result, and nothing else: no prompts, and no
/// indentation echoed (Kotlin's REPL echoed its auto-indent into piped
/// output and into the input, shifting snippets' columns).
#[test]
fn piped_input_prints_only_results() {
    let input = "let f(a: Int): Int {\n  if (a > 1) {\n    a * f(a - 1)\n  } else {\n    1\n  }\n}\nf(5)\n  nope\n";
    let (code, out, err) = repl(&[], None, input);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(
        out,
        "interactive#0: void\n\
         interactive#1: 120\n\
         interactive#2:1:3: error: nope has not been declared\n    \
         \x20 nope\n    \
         \x20 ^^^^\n\
         interactive#2: fail\n"
    );
}

/// `--prompt normal` prompts with `$ `, and `  ` while an input goes on;
/// the end of input exits with 0, whatever failed.
#[test]
fn prompts() {
    let (code, out, _) = repl(&["--prompt", "normal"], None, "[1,\n2]\npanic()\n");
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "$   interactive#0: [1,2]\n$ Panicked: panic()\ninteractive#1: fail\n$ \n"
    );
    let (code, out, _) = repl(&["--prompt", "hide"], None, "1\n");
    assert_eq!((code, out.as_str()), (0, "interactive#0: 1\n"));
}

/// Input that ends partway still runs, as Ctrl-D runs what's pending.
#[test]
fn the_end_of_input_runs_what_is_pending() {
    let (code, out, _) = repl(&[], None, "[1,\n2]");
    assert_eq!(code, 0);
    assert_eq!(out, "interactive#0: [1,2]\n");
    let (code, out, _) = repl(&[], None, "[1,\n2");
    assert_eq!(code, 0);
    assert!(out.ends_with("interactive#0: 2\n"), "{out}");
}

/// `-e` runs inputs first; `--separator` chooses what ends an input.
#[test]
fn flags() {
    let (_, out, _) = repl(
        &["-e", "let x = 2", "--exec-before", "let y = 3"],
        None,
        "x * y\n",
    );
    assert_eq!(
        out,
        "interactive#0: void\ninteractive#1: void\ninteractive#2: 6\n"
    );
    let input = "let ev(n: Int): Boolean { n == 0 || od(n - 1) }\n\
                 let od(n: Int): Boolean { n != 0 && ev(n - 1) } ;;\n\
                 od(3) ;;\n";
    let (_, out, _) = repl(&["--separator", "semisemi"], None, input);
    assert_eq!(out, "interactive#0: void\ninteractive#1: true\n");
    let (_, out, _) = repl(&["--separator", "eof"], None, "let a = 1;\na + 1\n");
    assert_eq!(out, "interactive#0: 2\n");
    for args in [
        &["--separator", "comma"][..],
        &["--prompt", "loud"],
        &["--fuel", "0"],
        &["extra"],
    ] {
        assert_eq!(repl(args, None, "").0, 2, "{args:?}");
    }
    let (_, out, _) = repl(&["--fuel", "1000"], None, "while (true) {}\n1\n");
    assert_eq!(
        out,
        "Panicked: ran out of steps\ninteractive#0: fail\ninteractive#1: 1\n"
    );
}

/// UTF-8 piped in comes out as the same UTF-8 bytes, on every platform:
/// names in other scripts (Latin with diacritics, CJK), and values with
/// an emoji with a skin tone, combining marks, and CJK, in results and
/// in a diagnostic's quoted line. Piped input and output are bytes, so
/// no console code page (Windows') converts them.
#[test]
fn utf8_input() {
    let input = "let näme = \"😀 e\u{301} 日本\"\nnäme\nnäme.split(\" \").length\n\
                 let 名前 = \"👋🏽 e\u{301} 日本語\"\n\
                 let café = 名前.split(\" \")\n\
                 café.length\n\
                 名前\n\
                 let x = 名前 + nope\n";
    let mut child = Command::new(env!("CARGO_BIN_EXE_tempest"))
        .arg("repl")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(input.as_bytes()).unwrap();
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    let expected = "interactive#0: void\n\
                    interactive#1: \"😀 e\u{301} 日本\"\n\
                    interactive#2: 3\n\
                    interactive#3: void\n\
                    interactive#4: void\n\
                    interactive#5: 3\n\
                    interactive#6: \"👋🏽 e\u{301} 日本語\"\n\
                    interactive#7:1:14: error: nope has not been declared\n    \
                    let x = 名前 + nope\n    \
                    \x20            ^^^^\n\
                    interactive#7: fail\n";
    // Bytes, not text, so a stray conversion can't be decoded away.
    assert_eq!(
        output.stdout,
        expected.as_bytes(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(output.stderr, b"");
    assert!(output.status.success());
}

/// Inputs import the work root's libraries by name: the work root around
/// the current directory by default, or `-w`'s.
#[test]
fn imports_from_the_work_root() {
    with_dir("workroot", |dir| {
        let lib = dir.join("lib");
        fs::create_dir_all(&lib).unwrap();
        fs::write(
            lib.join("config.temper.md"),
            "    export let name = \"greeting\";\n",
        )
        .unwrap();
        fs::write(
            lib.join("greet.temper"),
            "export let greet(who: String): String { \"Hello, ${who}!\" }\n",
        )
        .unwrap();
        let input = "let { greet } = import(\"greeting\");\ngreet(\"REPL\")\n";
        let expected = "interactive#0: void\ninteractive#1: \"Hello, REPL!\"\n";
        let (_, out, _) = repl(&[], Some(&lib), input);
        assert_eq!(out, expected);
        let (_, out, _) = repl(&["-w", &dir.to_string_lossy()], None, input);
        assert_eq!(out, expected);
        // Outside a work root, there are none.
        let (_, out, _) = repl(&[], Some(&std::env::temp_dir()), input);
        assert!(
            out.contains("error: Not supported yet: importing greeting"),
            "{out}"
        );
    });
}

/// Ctrl-C (SIGINT) stops the input running, not the session.
#[cfg(unix)]
#[test]
fn ctrl_c_stops_the_input_not_the_session() {
    use std::time::{Duration, Instant};
    // `-e` runs once Ctrl-C is handled; until then, SIGINT would end it.
    let mut child = Command::new(env!("CARGO_BIN_EXE_tempest"))
        .args(["repl", "-e", "console.log(\"ready\")"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    // What it prints, read on a thread, so the test can wait for it.
    let (send, receive) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut buffer = [0; 256];
        while let Ok(n) = stdout.read(&mut buffer) {
            if n == 0 || send.send(buffer[..n].to_vec()).is_err() {
                break;
            }
        }
    });
    let mut out = Vec::new();
    let start = Instant::now();
    while !String::from_utf8_lossy(&out).contains("ready\ninteractive#0: void\n") {
        assert!(start.elapsed() < Duration::from_secs(60), "not ready");
        if let Ok(bytes) = receive.recv_timeout(Duration::from_millis(100)) {
            out.extend(bytes);
        }
    }
    out.clear();
    stdin
        .write_all(b"var i = 0; while (true) { i += 1 }\n")
        .unwrap();
    stdin.flush().unwrap();
    std::thread::sleep(Duration::from_millis(200));
    // A SIGINT, then another every two seconds, in case the first came
    // before the loop began, until the input says it failed.
    while !String::from_utf8_lossy(&out).contains("interactive#1: fail") {
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "no interrupt: {}",
            String::from_utf8_lossy(&out)
        );
        let pid = child.id().to_string();
        let killed = Command::new("kill").args(["-INT", &pid]).status().unwrap();
        assert!(killed.success());
        let wait = Instant::now();
        while wait.elapsed() < Duration::from_secs(2) {
            match receive.recv_timeout(Duration::from_millis(100)) {
                Ok(bytes) => out.extend(bytes),
                Err(_) if String::from_utf8_lossy(&out).contains("interactive#1: fail") => break,
                Err(_) => {}
            }
        }
    }
    stdin.write_all(b"i > 0\n1 + 1\n").unwrap();
    drop(stdin);
    let status = child.wait().unwrap();
    reader.join().unwrap();
    out.extend(receive.try_iter().flatten());
    assert!(status.success());
    assert_eq!(
        String::from_utf8_lossy(&out),
        "Interrupted\ninteractive#1: fail\ninteractive#2: true\ninteractive#3: 2\n"
    );
}

/// On Windows, Ctrl-Break stops the input running, not the session, as
/// Ctrl-C does: Ctrl-C can't be sent to one process alone, but
/// Ctrl-Break can, to a console process group of its own.
#[cfg(windows)]
#[test]
fn ctrl_break_stops_the_input_not_the_session() {
    use std::os::windows::process::CommandExt;
    use std::time::{Duration, Instant};
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const CTRL_BREAK_EVENT: u32 = 1;
    unsafe extern "system" {
        fn GenerateConsoleCtrlEvent(event: u32, group: u32) -> i32;
        fn AttachConsole(process: u32) -> i32;
        fn FreeConsole() -> i32;
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_tempest"))
        .args(["repl", "-e", "console.log(\"ready\")"])
        .creation_flags(CREATE_NEW_PROCESS_GROUP)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let (send, receive) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut buffer = [0; 256];
        while let Ok(n) = stdout.read(&mut buffer) {
            if n == 0 || send.send(buffer[..n].to_vec()).is_err() {
                break;
            }
        }
    });
    let start = Instant::now();
    let mut out = Vec::new();
    let wait_for = |text: &str, out: &mut Vec<u8>| {
        while !String::from_utf8_lossy(out).contains(text) {
            let shown = String::from_utf8_lossy(out).to_string();
            assert!(
                start.elapsed() < Duration::from_secs(60),
                "no {text:?} in {shown:?}"
            );
            if let Ok(bytes) = receive.recv_timeout(Duration::from_millis(50)) {
                out.extend(bytes);
            }
        }
    };
    // `-e` runs once the handler is in.
    wait_for("ready\ninteractive#0: void\n", &mut out);
    out.clear();
    // Sent once the loop has begun, and only once, as a second press
    // before the first is handled ends tempest.
    stdin
        .write_all(b"console.log(\"looping\"); var i = 0; while (true) { i += 1 }\n")
        .unwrap();
    stdin.flush().unwrap();
    wait_for("looping\n", &mut out);
    let group = child.id();
    // SAFETY: plain console calls. Ctrl events go to processes sharing
    // the sender's console; without one (as under some CI services),
    // this process attaches to tempest's.
    let sent = unsafe {
        GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, group) != 0 || {
            FreeConsole();
            AttachConsole(group) != 0 && GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, group) != 0
        }
    };
    assert!(
        sent,
        "no Ctrl-Break sent: {}",
        std::io::Error::last_os_error()
    );
    wait_for("interactive#1: fail\n", &mut out);
    stdin.write_all(b"i > 0\n1 + 1\n").unwrap();
    drop(stdin);
    let status = child.wait().unwrap();
    reader.join().unwrap();
    out.extend(receive.try_iter().flatten());
    assert!(status.success(), "{status}");
    assert_eq!(
        String::from_utf8_lossy(&out),
        "looping\nInterrupted\ninteractive#1: fail\ninteractive#2: true\ninteractive#3: 2\n"
    );
}

/// `--color`: `auto` colors only a terminal's output, so piped output is
/// plain; `always` colors it whatever `NO_COLOR` says; `never` doesn't.
#[test]
fn color() {
    let colored =
        "\x1b[2minteractive#0:\x1b[0m 1\n\x1b[2minteractive#1:\x1b[0m \x1b[31mfail\x1b[0m\n";
    let plain = "interactive#0: 1\ninteractive#1: fail\n";
    for (args, expected) in [
        (&[][..], plain),
        (&["--color", "auto"], plain),
        (&["--color", "never"], plain),
        (&["--color", "always"], colored),
    ] {
        let (code, out, _) = repl(args, None, "1\n1 / 0\n");
        assert_eq!((code, out.as_str()), (0, expected), "{args:?}");
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_tempest"))
        .args(["repl", "--color", "always"])
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"1\n1 / 0\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(String::from_utf8(output.stdout).unwrap(), colored);
    assert_eq!(repl(&["--color", "sometimes"], None, "").0, 2);
}

/// The REPL's own functions, through the command: `help()` says what
/// input isn't from a terminal means, and `translate` shows code.
#[test]
fn own_functions() {
    let (code, out, _) = repl(&[], None, "help()\nlet x = 1\ntranslate(1, \"py\")\n");
    assert_eq!(code, 0);
    assert!(
        out.contains("Input isn't from a terminal, so lines aren't edited."),
        "{out}"
    );
    assert!(out.contains("History: none"), "{out}");
    if cfg!(feature = "py") {
        assert!(
            out.contains("# interactive/interactive/interactive.py\n"),
            "{out}"
        );
        assert!(out.contains("_x: int = 1\ninteractive#2: void\n"), "{out}");
    }
}

/// Printed values and diagnostics can't move the terminal's cursor or
/// reorder text: control characters, C1 controls, and bidirectional
/// controls are escaped.
#[test]
fn control_characters_are_escaped() {
    let input = "\"\\u{1b}[2J\\u{9b}\\u{202e}\"\nlet a = \"\x1b[31m\" + nope\n";
    let (_, out, _) = repl(&[], None, input);
    assert!(!out.contains(['\x1b', '\u{9b}', '\u{202e}']), "{out:?}");
    assert!(out.contains("\"\\u{1b}[2J\\u{9b,202e}\""), "{out}");
    assert!(out.contains("let a = \"\\u{1b}[31m\" + nope"), "{out}");
}

/// On a terminal, as `script` gives one: an input over lines, indented as
/// it goes on, a signature hint, Ctrl-C at the prompt dropping what's
/// typed, and history kept owner-only, a multi-line input as one entry.
#[cfg(target_os = "linux")]
#[test]
fn edits_lines_on_a_terminal() {
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, Instant};
    let has_script = Command::new("script").arg("--version").output();
    if !has_script.is_ok_and(|o| o.status.success()) {
        eprintln!("skipped: no `script` here to run tempest on a terminal");
        return;
    }
    with_dir("terminal", |dir| {
        let command = format!("{} repl --color never", env!("CARGO_BIN_EXE_tempest"));
        let mut child = Command::new("script")
            .args(["-qfec", &command, "/dev/null"])
            .env("XDG_DATA_HOME", dir)
            .env("TERM", "xterm")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = child.stdout.take().unwrap();
        let (send, receive) = std::sync::mpsc::channel();
        let reader = std::thread::spawn(move || {
            let mut buffer = [0; 1024];
            while let Ok(n) = stdout.read(&mut buffer) {
                if n == 0 || send.send(buffer[..n].to_vec()).is_err() {
                    break;
                }
            }
        });
        let start = Instant::now();
        let mut out = Vec::new();
        let wait_for = |text: &str, out: &mut Vec<u8>| {
            while !String::from_utf8_lossy(out).contains(text) {
                let shown = String::from_utf8_lossy(out).to_string();
                assert!(
                    start.elapsed() < Duration::from_secs(60),
                    "no {text:?} in {shown:?}"
                );
                if let Ok(bytes) = receive.recv_timeout(Duration::from_millis(50)) {
                    out.extend(bytes);
                }
            }
        };
        let mut send_keys = |keys: &str| {
            stdin.write_all(keys.as_bytes()).unwrap();
            stdin.flush().unwrap();
        };
        wait_for("$ ", &mut out);
        send_keys("let greet(name: String): String { \"hi ${name}\" }\r");
        wait_for("interactive#0: void", &mut out);
        // Ctrl-C drops what's typed.
        send_keys("1 +");
        wait_for("1 +", &mut out);
        send_keys("\x03");
        // Typed only once the next prompt is up, as keys typed between
        // inputs may be dropped.
        wait_for("1 +\x1b[?2004l\r\n\x1b[?2004h", &mut out);
        let prompts = |out: &Vec<u8>| String::from_utf8_lossy(out).matches("$ ").count();
        while prompts(&out) < 3 {
            assert!(start.elapsed() < Duration::from_secs(60), "no prompt");
            if let Ok(bytes) = receive.recv_timeout(Duration::from_millis(50)) {
                out.extend(bytes);
            }
        }
        send_keys("greet(");
        // The hint, after what's typed.
        wait_for("greet(  greet(name: String): String", &mut out);
        send_keys("\r");
        wait_for("greet(\r\n    ", &mut out);
        send_keys("\"you\")\r");
        wait_for("interactive#1: \"hi you\"", &mut out);
        send_keys("\x04");
        drop(stdin);
        let status = child.wait().unwrap();
        reader.join().unwrap();
        assert!(status.success());
        let history = dir.join("tempest").join("repl-history.txt");
        let mode = fs::metadata(&history).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        let kept = fs::read_to_string(&history).unwrap();
        assert!(kept.contains("\ngreet(\\n    \"you\")\n"), "{kept:?}");
        assert!(!kept.contains("1 +"), "{kept:?}");
    });
}
