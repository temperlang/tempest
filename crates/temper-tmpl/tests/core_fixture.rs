//! `tests/fixtures/core.tmpl`, a hand-written `uses` section for the core
//! members that subset A's old key table had (48, plus `getConsole`),
//! checked against the vendored `core.temper`: each member's name, kind,
//! parameters (names, types, and which are optional), result, bubbling,
//! and `@connected` mark. Like the frontend's `builtin_supertypes_are_cores`,
//! it reads `core.temper` as text, so this crate needn't depend on the
//! frontend.

use std::collections::HashMap;
use temper_tmpl::{DeclKind, Facts, Profile, Program, text, validate};

const FIXTURE: &str = include_str!("fixtures/core.tmpl");
const CORE: &str = include_str!("../../../vendor/temper/core/core.temper");

/// A member as `core.temper` declares it.
#[derive(Debug, PartialEq)]
struct Member {
    kind: &'static str,
    params: Vec<(String, String, bool)>,
    result: String,
    bubbles: bool,
    connected: bool,
}

/// Core's spelling of a type, as tmpl writes it.
fn type_name(core: &str) -> String {
    let t = core.trim().trim_start_matches("builtins.");
    match t {
        "Int" => "Int32".into(),
        _ => t.into(),
    }
}

/// Members of core's top-level classes and interfaces, by `Type.name`,
/// and its top-level functions by name, from single-line declarations.
fn core_members() -> HashMap<String, Member> {
    let mut members = HashMap::new();
    let mut owner: Option<String> = None;
    let mut connected = false;
    for line in CORE.lines() {
        let trimmed = line.trim();
        if line.starts_with('}') {
            owner = None;
        }
        let top = line.trim_start_matches("export ");
        let top = top.trim_start_matches("sealed ");
        for kind in ["class ", "interface "] {
            if let Some(rest) = top.strip_prefix(kind) {
                let name: String = rest.chars().take_while(|c| c.is_alphanumeric()).collect();
                owner = Some(name);
            }
        }
        if trimmed == "@connected" {
            connected = true;
            continue;
        }
        if trimmed.starts_with('@') {
            continue;
        }
        let (decl, key) = if let Some(rest) = trimmed.strip_prefix("public ") {
            match &owner {
                Some(owner) if line.starts_with("  ") || line.starts_with("   ") => {
                    (rest, Some(owner.clone()))
                }
                _ => (rest, None),
            }
        } else if let Some(rest) = line.strip_prefix("export let ") {
            (rest, Some(String::new()))
        } else {
            connected = false;
            continue;
        };
        let Some(owner) = key else {
            connected = false;
            continue;
        };
        let (kind, decl) = if let Some(rest) = decl.strip_prefix("static ") {
            ("static", rest)
        } else if let Some(rest) = decl.strip_prefix("get ") {
            ("get", rest)
        } else if owner.is_empty() {
            ("fn", decl)
        } else {
            ("method", decl)
        };
        let Some((name, rest)) = decl.split_once('(') else {
            connected = false;
            continue;
        };
        let Some((params, rest)) = rest.split_once(')') else {
            connected = false;
            continue;
        };
        // Function types and multi-line lists are left out: the fixture
        // has none.
        let params: Option<Vec<_>> = params
            .split(',')
            .filter(|p| !p.trim().is_empty())
            .map(|p| {
                let (name, ty) = p.split_once(':')?;
                let (ty, default) = match ty.split_once('=') {
                    Some((ty, _)) => (ty, true),
                    None => (ty, false),
                };
                Some((name.trim().to_string(), type_name(ty), default))
            })
            .collect();
        let Some(params) = params else {
            connected = false;
            continue;
        };
        let rest = rest.trim_start_matches(':');
        let rest = rest.split(['{', ';']).next().unwrap();
        let (result, bubbles) = match rest.split_once(" throws ") {
            Some((result, _)) => (result, true),
            None => (rest, false),
        };
        let key = if owner.is_empty() {
            name.to_string()
        } else {
            format!("{owner}.{name}")
        };
        members.insert(
            key,
            Member {
                kind,
                params,
                result: type_name(result),
                bubbles,
                connected,
            },
        );
        connected = false;
    }
    members
}

/// The fixture's members, as core's would read.
fn fixture_members(p: &Program) -> HashMap<String, Member> {
    let mut members = HashMap::new();
    for decl in p.decls.ids() {
        let d = p.decls.get(decl);
        let kind = match d.kind {
            DeclKind::Method => "method",
            DeclKind::Getter => "get",
            DeclKind::Function if p.decls.get(d.owner).kind == DeclKind::Library => "fn",
            DeclKind::Function => "static",
            _ => continue,
        };
        let sig = p.signature(decl).unwrap();
        let names = p.decls.param_names(decl);
        let params = sig
            .params
            .iter()
            .zip(sig.optional)
            .zip(names)
            .map(|((&t, &optional), &name)| {
                let ty = p.show_type(t);
                let ty = ty.split('#').next().unwrap().to_string();
                (p.strings.get(name).to_string(), ty, optional)
            })
            .collect();
        let result = p.show_type(sig.result);
        let owner = if kind == "fn" {
            String::new()
        } else {
            format!("{}.", p.name(d.owner))
        };
        members.insert(
            format!("{owner}{}", p.name(decl)),
            Member {
                kind,
                params,
                result: result.split('#').next().unwrap().to_string(),
                bubbles: sig.bubbles,
                connected: d.facts.contains(Facts::CONNECTED),
            },
        );
    }
    members
}

#[test]
fn the_fixture_is_valid_canonical_tmpl() {
    let p = text::read(FIXTURE).unwrap();
    assert_eq!(validate(&p, &Profile::full()), []);
    assert_eq!(text::write(&p), FIXTURE);
}

#[test]
fn the_fixture_matches_core_temper() {
    let p = text::read(FIXTURE).unwrap();
    let fixture = fixture_members(&p);
    // The old key table's 48 members, and `getConsole`.
    assert_eq!(fixture.len(), 49);
    let core = core_members();
    let mut names: Vec<&String> = fixture.keys().collect();
    names.sort();
    for name in names {
        let found = core.get(name);
        assert_eq!(found, Some(&fixture[name]), "{name}");
    }
}
