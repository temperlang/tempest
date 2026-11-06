//! The editor queries in `analysis`: hover, go to definition,
//! completion, and diagnostics' spans, over one settled library.

use temper_analysis::{Analysis, DeclKind, Position, byte_offset};
use temper_frontend::library::{FileId, Level, Library};
use temper_frontend::project::load_imports;
use temper_frontend::settle::settle;
use temper_frontend::sources::Sources;
use temper_syntax::lex::SourceKind;

const MAIN: FileId = FileId(0);

fn analyze(source: &str) -> Analysis {
    let mut library = Library::new();
    library.add_file("main.temper", source, SourceKind::Standalone);
    load_imports(&Sources::memory(&[]), &mut library);
    Analysis::new(settle(library))
}

/// The byte offset of the `nth` (from 0) `needle` in `source`, plus `ahead`.
fn at(source: &str, needle: &str, nth: usize, ahead: u32) -> u32 {
    let (i, _) = source.match_indices(needle).nth(nth).expect(needle);
    i as u32 + ahead
}

const STACK: &str = r#"
/** A last-in, first-out stack. */
export class Stack<T> {
  private items: ListBuilder<T> = new ListBuilder<T>();

  /** Adds an item on top. */
  public push(item: T): Void { items.add(item); }

  public get size(): Int { items.length }
}

/** Doubles `n`. */
export let twice(n: Int, extra: Int = 0): Int { n * 2 + extra }

var count = 0;
let stack = new Stack<String>();
stack.push("a");
count = twice(stack.size);
let label = "n: ${count.toString()}";
"#;

#[test]
fn hover_on_a_local() {
    let a = analyze(STACK);
    let h = a.hover(MAIN, at(STACK, "stack.push", 0, 2)).unwrap();
    assert_eq!(h.name.as_deref(), Some("stack"));
    assert_eq!(h.kind, Some(DeclKind::Let));
    assert_eq!(h.ty.as_deref(), Some("Stack<String>"));
    let start = at(STACK, "stack.push", 0, 0);
    assert_eq!((h.span.start, h.span.end), (start, start + 5));

    let h = a.hover(MAIN, at(STACK, "count =", 1, 1)).unwrap();
    assert_eq!(h.kind, Some(DeclKind::Var));
    assert_eq!(h.ty.as_deref(), Some("Int32"));
}

#[test]
fn hover_on_a_declaration() {
    let a = analyze(STACK);
    let h = a.hover(MAIN, at(STACK, "count", 0, 0)).unwrap();
    assert_eq!(h.name.as_deref(), Some("count"));
    assert_eq!(h.kind, Some(DeclKind::Var));
    let h = a.hover(MAIN, at(STACK, "Stack<T>", 0, 1)).unwrap();
    assert_eq!(h.kind, Some(DeclKind::Class));
    assert_eq!(h.ty.as_deref(), Some("Stack<T>"));
    assert_eq!(h.doc.as_deref(), Some("A last-in, first-out stack."));
}

#[test]
fn hover_on_a_parameter() {
    let a = analyze(STACK);
    let h = a.hover(MAIN, at(STACK, "n * 2", 0, 0)).unwrap();
    assert_eq!(h.name.as_deref(), Some("n"));
    assert_eq!(h.kind, Some(DeclKind::Parameter));
    assert_eq!(h.ty.as_deref(), Some("Int32"));
    let h = a.hover(MAIN, at(STACK, "add(item)", 0, 5)).unwrap();
    assert_eq!(h.kind, Some(DeclKind::Parameter));
    assert_eq!(h.ty.as_deref(), Some("T"));
}

#[test]
fn hover_on_members() {
    let a = analyze(STACK);
    let h = a.hover(MAIN, at(STACK, "push(\"a\")", 0, 1)).unwrap();
    assert_eq!(h.name.as_deref(), Some("push"));
    assert_eq!(h.kind, Some(DeclKind::Method));
    assert_eq!(h.ty.as_deref(), Some("push(item: T): Void"));
    assert_eq!(h.doc.as_deref(), Some("Adds an item on top."));
    let h = a.hover(MAIN, at(STACK, "stack.size", 0, 7)).unwrap();
    assert_eq!(h.name.as_deref(), Some("size"));
    assert_eq!(h.kind, Some(DeclKind::Getter));
    let h = a.hover(MAIN, at(STACK, "items.add", 0, 1)).unwrap();
    assert_eq!(h.kind, Some(DeclKind::Field));
    assert_eq!(h.ty.as_deref(), Some("ListBuilder<T>"));
    // A builtin type's member.
    let h = a.hover(MAIN, at(STACK, "toString", 0, 2)).unwrap();
    assert_eq!(h.name.as_deref(), Some("toString"));
    assert_eq!(h.kind, Some(DeclKind::Builtin));
}

#[test]
fn hover_on_a_call() {
    let a = analyze(STACK);
    let h = a.hover(MAIN, at(STACK, "twice(stack", 0, 1)).unwrap();
    assert_eq!(h.kind, Some(DeclKind::Function));
    assert_eq!(
        h.ty.as_deref(),
        Some("twice(n: Int32, extra?: Int32): Int32")
    );
    assert_eq!(h.doc.as_deref(), Some("Doubles `n`."));
    // On the call's parenthesis: the call's type.
    let h = a.hover(MAIN, at(STACK, "twice(stack", 0, 5)).unwrap();
    assert_eq!(h.name, None);
    assert_eq!(h.ty.as_deref(), Some("Int32"));
    // A builtin.
    let h = a.hover(MAIN, at(STACK, "ListBuilder<T>()", 0, 1)).unwrap();
    assert_eq!(h.kind, Some(DeclKind::Builtin));
}

#[test]
fn definitions() {
    let a = analyze(STACK);
    let d = a.definition(MAIN, at(STACK, "stack.push", 0, 1)).unwrap();
    let decl = at(STACK, "stack =", 0, 0);
    assert_eq!((d.file, d.span.start, d.span.end), (MAIN, decl, decl + 5));
    let d = a.definition(MAIN, at(STACK, "push(\"a\")", 0, 0)).unwrap();
    assert_eq!(d.span.start, at(STACK, "push(item", 0, 0));
    let d = a.definition(MAIN, at(STACK, "n * 2", 0, 0)).unwrap();
    assert_eq!(d.span.start, at(STACK, "n: Int", 0, 0));
    assert_eq!(
        a.definition(MAIN, at(STACK, "ListBuilder<T>()", 0, 1)),
        None
    );
}

#[test]
fn completions() {
    let a = analyze(STACK);
    let names: Vec<String> = (a.completions(MAIN, at(STACK, "n * 2", 0, 0)).into_iter())
        .map(|c| c.name)
        .collect();
    for name in ["n", "extra", "twice", "Stack", "console"] {
        assert!(names.contains(&name.to_string()), "{name} in {names:?}");
    }
    assert!(names.iter().position(|n| n == "n") < names.iter().position(|n| n == "console"));

    let source = format!("{STACK}stack.");
    let a = analyze(&source);
    let members = a.completions(MAIN, source.len() as u32);
    let names: Vec<&str> = members.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["push", "size"]);
    assert_eq!(members[0].detail.as_deref(), Some("push(item: T): Void"));
}

/// Diagnostics' spans as the text they cover.
fn problems(source: &str) -> Vec<(Level, String, &str)> {
    let a = analyze(source);
    (a.diagnostics().into_iter())
        .filter(|p| p.file == MAIN)
        .map(|p| {
            let text = &source[p.span.start as usize..p.span.end as usize];
            (p.level, p.message, text)
        })
        .collect()
}

#[test]
fn diagnostics_are_tight_and_all_there() {
    let source = r#"let f(a: Int): Int { a + "x" }
let g(): Void {
  let y = 1 +;
  zzz(2);
}
let h: String = f(1);
let k = f("s");
let later = f(2) + 1;
"#;
    let found = problems(source);
    let texts: Vec<&str> = found.iter().map(|p| p.2).collect();
    assert!(found.iter().all(|p| p.0 == Level::Error));
    // Parse, settle (an undeclared name), and typing errors, each where
    // it is, and later declarations still typed (keep on trucking).
    assert_eq!(
        texts,
        ["let y = 1 +", "zzz", "\"x\"", "f(1)", "\"s\""],
        "{found:?}"
    );
    let a = analyze(source);
    let h = a.hover(MAIN, at(source, "later", 0, 1)).unwrap();
    assert_eq!(h.ty.as_deref(), Some("Int32"));
}

#[test]
fn diagnostics_past_a_broken_bracket() {
    let source = "export let f(): Int { (1 + 2; }\nlet x: Int = \"s\";\nlet y = f() + 1;\n";
    let found = problems(source);
    assert!(found.len() >= 2, "{found:?}");
    assert!(found.iter().any(|p| p.2 == "\"s\""), "{found:?}");
    let a = analyze(source);
    let h = a.hover(MAIN, at(source, "y =", 0, 0)).unwrap();
    assert_eq!(h.ty.as_deref(), Some("Int32"));
}

#[test]
fn utf16_at_the_boundary() {
    let source = "let s = \"é😀\"; let t = s;";
    let t = at(source, "t =", 0, 0) as usize;
    let p = Position::of(source, t);
    assert_eq!(p.utf16, t - 3);
    assert_eq!(byte_offset(source, p.utf16), t);
    // Inside the surrogate pair: the emoji's start.
    let emoji = source.find('😀').unwrap();
    let pair = Position::of(source, emoji).utf16;
    assert_eq!(byte_offset(source, pair + 1), emoji);
    assert_eq!(byte_offset(source, 10_000), source.len());
}

#[test]
fn getters_show_their_type() {
    let a = analyze(STACK);
    let h = a.hover(MAIN, at(STACK, "stack.size", 0, 7)).unwrap();
    assert_eq!(h.ty.as_deref(), Some("Int32"));
}
