//! `tempest lsp`, driven as an editor drives it: a scripted JSON-RPC
//! session over the child's stdin and stdout, on a library folder with a
//! `config.temper.md` and two files.

use serde_json::{Value as Json, json};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use tempest::lsp::{path_to_uri, read_message, write_message};

struct Client {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    next: u32,
    /// Notifications that came while waiting for a response.
    notes: Vec<Json>,
}

impl Client {
    fn start() -> Client {
        let mut child = Command::new(env!("CARGO_BIN_EXE_tempest"))
            .arg("lsp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Client {
            child,
            input,
            output,
            next: 1,
            notes: Vec::new(),
        }
    }

    fn send(&mut self, message: Json) {
        write_message(&mut self.input, &message).unwrap();
    }

    fn notify(&mut self, method: &str, params: Json) {
        self.send(json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }

    /// Sends a request and gives its response's `result`, or panics with
    /// its error.
    fn request(&mut self, method: &str, params: Json) -> Json {
        let id = self.next;
        self.next += 1;
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let message = self.receive();
            if message.get("id") == Some(&json!(id)) {
                if let Some(error) = message.get("error") {
                    panic!("{method}: {error}");
                }
                return message.get("result").cloned().unwrap_or(Json::Null);
            }
            self.notes.push(message);
        }
    }

    fn receive(&mut self) -> Json {
        let text = read_message(&mut self.output).unwrap().expect("a message");
        serde_json::from_str(&text).unwrap()
    }

    /// The next diagnostics published for a URI, kept or waited for.
    fn diagnostics(&mut self, uri: &str) -> Vec<Json> {
        loop {
            let found = self.notes.iter().position(|n| {
                n.get("method").and_then(Json::as_str) == Some("textDocument/publishDiagnostics")
                    && at_path(n, &["params", "uri"]).and_then(Json::as_str) == Some(uri)
            });
            if let Some(i) = found {
                let note = self.notes.remove(i);
                let list = at_path(&note, &["params", "diagnostics"]).and_then(Json::as_array);
                return list.cloned().unwrap_or_default();
            }
            let message = self.receive();
            self.notes.push(message);
        }
    }

    fn finish(mut self) -> i32 {
        assert_eq!(self.request("shutdown", Json::Null), Json::Null);
        self.notify("exit", Json::Null);
        self.child.wait().unwrap().code().unwrap()
    }
}

/// A library in a fresh directory: its config and files, by name.
fn library(test: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tempest-lsp-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("config.temper.md"), "# Shapes\n").unwrap();
    for (name, text) in files {
        std::fs::write(dir.join(name), text).unwrap();
    }
    dir
}

fn uri(dir: &Path, name: &str) -> String {
    path_to_uri(&dir.join(name).to_string_lossy())
}

fn doc(uri: &str) -> Json {
    json!({"uri": uri})
}

/// `textDocument` and `position` params.
fn at(uri: &str, line: u32, character: u32) -> Json {
    json!({"textDocument": doc(uri), "position": {"line": line, "character": character}})
}

/// The value at a path of object keys.
fn at_path<'a>(value: &'a Json, path: &[&str]) -> Option<&'a Json> {
    path.iter().try_fold(value, |v, key| v.get(key))
}

fn open(client: &mut Client, uri: &str, text: &str) {
    client.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": uri, "languageId": "temper", "version": 1, "text": text}}),
    );
}

fn change(client: &mut Client, uri: &str, version: u32, text: &str) {
    client.notify(
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": uri, "version": version},
            "contentChanges": [{"text": text}],
        }),
    );
}

/// An incremental change: replaces a range on one line.
fn edit(client: &mut Client, uri: &str, version: u32, line: u32, from: u32, to: u32, text: &str) {
    let range = json!({
        "start": {"line": line, "character": from},
        "end": {"line": line, "character": to},
    });
    client.notify(
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": uri, "version": version},
            "contentChanges": [{"range": range, "text": text}],
        }),
    );
}

fn str_at<'a>(value: &'a Json, path: &[&str]) -> &'a str {
    at_path(value, path)
        .and_then(Json::as_str)
        .unwrap_or_else(|| panic!("{path:?} in {value}"))
}

fn num_at(value: &Json, path: &[&str]) -> u32 {
    at_path(value, path)
        .and_then(Json::as_u64)
        .map(|n| n as u32)
        .unwrap_or_else(|| panic!("{path:?} in {value}"))
}

const SHAPES: &str = "/** A square's area. */
export let area(side: Int): Int { side * side }
";

#[test]
fn a_session() {
    let dir = library(
        "session",
        &[
            ("shapes.temper", SHAPES),
            ("use.temper", "let a = area(3);\n"),
        ],
    );
    let (shapes, uses) = (uri(&dir, "shapes.temper"), uri(&dir, "use.temper"));
    let mut client = Client::start();
    let init = client.request("initialize", json!({"processId": null, "capabilities": {}}));
    // Incremental sync.
    assert_eq!(
        num_at(&init, &["capabilities", "textDocumentSync", "change"]),
        2
    );
    assert_eq!(
        at_path(&init, &["capabilities", "hoverProvider"]),
        Some(&json!(true))
    );
    client.notify("initialized", json!({}));

    // Unsaved text with an error, in a file of the library.
    let text = "let a = area(3);\nlet b: String = a;\n";
    open(&mut client, &uses, text);
    let found = client.diagnostics(&uses);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(num_at(&found[0], &["range", "start", "line"]), 1);
    assert_eq!(num_at(&found[0], &["severity"]), 1);
    // Fixed, as typed: no problems.
    change(&mut client, &uses, 2, "let a = area(3);\nlet b: Int = a;\n");
    assert_eq!(client.diagnostics(&uses), Vec::<Json>::new());
    // Incremental edits: `Int` to `String` and back, as ranges.
    edit(&mut client, &uses, 3, 1, 7, 10, "String");
    assert_eq!(client.diagnostics(&uses).len(), 1);
    edit(&mut client, &uses, 4, 1, 7, 13, "Int");
    assert_eq!(client.diagnostics(&uses), Vec::<Json>::new());

    // Hover on `area`, declared in the other file.
    let hover = client.request("textDocument/hover", at(&uses, 0, 9));
    let value = str_at(&hover, &["contents", "value"]);
    assert!(
        value.contains("function area(side: Int32): Int32"),
        "{value}"
    );
    assert!(value.contains("A square's area."), "{value}");
    assert_eq!(num_at(&hover, &["range", "start", "character"]), 8);
    assert_eq!(num_at(&hover, &["range", "end", "character"]), 12);

    // Its definition, in the other file.
    let def = client.request("textDocument/definition", at(&uses, 0, 9));
    assert_eq!(str_at(&def, &["uri"]), shapes);
    assert_eq!(num_at(&def, &["range", "start", "line"]), 1);
    assert_eq!(num_at(&def, &["range", "start", "character"]), 11);

    // Completion at the start of the second line.
    let items = client.request("textDocument/completion", at(&uses, 1, 0));
    let labels: Vec<&str> = (items.as_array().unwrap().iter())
        .map(|i| str_at(i, &["label"]))
        .collect();
    assert!(
        labels.contains(&"area") && labels.contains(&"a"),
        "{labels:?}"
    );

    // References across both files, with the declaration.
    let mut params = at(&uses, 0, 9);
    params["context"] = json!({"includeDeclaration": true});
    let refs = client.request("textDocument/references", params);
    let uris: Vec<&str> = (refs.as_array().unwrap().iter())
        .map(|r| str_at(r, &["uri"]))
        .collect();
    assert_eq!(uris, [shapes.as_str(), uses.as_str()]);

    // Rename, checked first, across both files.
    let prepared = client.request("textDocument/prepareRename", at(&uses, 0, 9));
    assert_eq!(str_at(&prepared, &["placeholder"]), "area");
    let mut params = at(&uses, 0, 9);
    params["newName"] = json!("squared");
    let edit = client.request("textDocument/rename", params);
    for file in [&shapes, &uses] {
        let edits = at_path(&edit, &["changes", file.as_str()]).and_then(Json::as_array);
        let edits = edits.unwrap_or_else(|| panic!("{file} in {edit}"));
        assert_eq!(edits.len(), 1, "{edit}");
        assert_eq!(str_at(&edits[0], &["newText"]), "squared");
    }
    // Builtins can't be renamed.
    client.send(json!({
        "jsonrpc": "2.0",
        "id": 50,
        "method": "textDocument/prepareRename",
        "params": at(&shapes, 1, 23),
    }));
    let reply = loop {
        let m = client.receive();
        if m.get("id") == Some(&json!(50)) {
            break m;
        }
    };
    let why = at_path(&reply, &["error", "message"])
        .and_then(Json::as_str)
        .unwrap_or("");
    assert!(why.contains("builtin"), "{reply}");

    // The other file's outline.
    let symbols = client.request(
        "textDocument/documentSymbol",
        json!({"textDocument": doc(&shapes)}),
    );
    let first = &symbols.as_array().unwrap()[0];
    assert_eq!(str_at(first, &["name"]), "area");
    assert_eq!(num_at(first, &["kind"]), 12);
    assert_eq!(num_at(first, &["selectionRange", "start", "character"]), 11);

    // Signature help, typing a call's second line.
    change(
        &mut client,
        &uses,
        5,
        "let a = area(3);
let b: Int = area(",
    );
    let help = client.request("textDocument/signatureHelp", at(&uses, 1, 18));
    let signature = &help.get("signatures").and_then(Json::as_array).unwrap()[0];
    assert_eq!(str_at(signature, &["label"]), "area(side: Int32): Int32");
    assert_eq!(num_at(&help, &["activeParameter"]), 0);
    assert_eq!(str_at(signature, &["documentation"]), "A square's area.");

    // Unknown requests fail, but the server goes on.
    let id = 99u32;
    client.send(json!({"jsonrpc": "2.0", "id": id, "method": "tempest/nothing"}));
    let reply = loop {
        let m = client.receive();
        if m.get("id") == Some(&json!(id)) {
            break m;
        }
    };
    assert_eq!(
        at_path(&reply, &["error", "code"]).and_then(Json::as_i64),
        Some(-32601)
    );
    assert_eq!(client.finish(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn utf16_positions_and_a_lone_file() {
    let dir = std::env::temp_dir().join(format!("tempest-lsp-{}-lone", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = uri(&dir, "lone.temper");
    let mut client = Client::start();
    client.request("initialize", json!({"capabilities": {}}));
    // Not saved, and no library around it: it stands alone.
    let text = "let s = \"😀\"; let t = s;\n";
    open(&mut client, &file, text);
    // `t`, after the emoji's two UTF-16 units: character 18.
    let hover = client.request("textDocument/hover", at(&file, 0, 18));
    let value = str_at(&hover, &["contents", "value"]);
    assert!(value.contains("let t: String"), "{value}");
    assert_eq!(num_at(&hover, &["range", "start", "character"]), 18);
    let def = client.request("textDocument/definition", at(&file, 0, 22));
    assert_eq!(num_at(&def, &["range", "start", "character"]), 4);
    assert_eq!(client.diagnostics(&file), Vec::<Json>::new());
    assert_eq!(client.finish(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn exit_without_shutdown_fails() {
    let mut client = Client::start();
    client.notify("exit", Json::Null);
    assert_eq!(client.child.wait().unwrap().code(), Some(1));
}

#[test]
fn definition_into_std() {
    let dir = std::env::temp_dir().join(format!("tempest-lsp-{}-std", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = uri(&dir, "uses-std.temper");
    let mut client = Client::start();
    client.request("initialize", json!({"capabilities": {}}));
    open(
        &mut client,
        &file,
        "let { Regex } = import(\"std/regex\");\nlet r: Regex? = null;\n",
    );
    let def = client.request("textDocument/definition", at(&file, 1, 8));
    let target = str_at(&def, &["uri"]);
    assert!(
        target.contains("tempest-std-") && target.ends_with("regex.temper.md"),
        "{def}"
    );
    let path = tempest::lsp::uri_to_path(target);
    let text = std::fs::read_to_string(&path).unwrap();
    let line = num_at(&def, &["range", "start", "line"]) as usize;
    assert!(text.lines().nth(line).unwrap().contains("Regex"), "{def}");
    assert_eq!(client.finish(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}
