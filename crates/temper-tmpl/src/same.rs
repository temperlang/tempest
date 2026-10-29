//! Whether two programs mean the same: what `==` on [`Program`] checks.
//!
//! Declarations keep their ids everywhere, so they compare by id; nodes,
//! types, and strings compare by content, since building the same program
//! in another order gives them other ids. Spans count only when either
//! program has them.

use crate::Program;
use crate::ids::{DeclId, NodeId, StrId};
use crate::nodes::Flags;
use crate::schema::{Payload, Slot};

/// The first difference between two programs, if any, for tests.
pub fn difference(a: &Program, b: &Program) -> Option<String> {
    programs(a, b).err()
}

type Same = Result<(), String>;

fn check(same: bool, what: impl FnOnce() -> String) -> Same {
    if same { Ok(()) } else { Err(what()) }
}

fn programs(a: &Program, b: &Program) -> Same {
    check(a.library == b.library, || "libraries differ".into())?;
    check(a.profile == b.profile, || "profiles differ".into())?;
    check(a.has_spans == b.has_spans, || "only one has spans".into())?;
    check(!spans(a, b) || a.files == b.files, || "files differ".into())?;
    check(a.decls.len() == b.decls.len(), || {
        format!(
            "declaration counts differ: {} and {}",
            a.decls.len(),
            b.decls.len()
        )
    })?;
    for id in a.decls.ids() {
        decl(a, b, id)?;
    }
    check(a.modules.len() == b.modules.len(), || {
        "module counts differ".into()
    })?;
    for (m, n) in a.modules.iter().zip(&b.modules) {
        check(m.decl == n.decl, || "modules differ".into())?;
        check(m.items.len() == n.items.len(), || {
            format!("module #{}'s item counts differ", m.decl.0)
        })?;
        for (&x, &y) in m.items.iter().zip(&n.items) {
            node(a, b, x, y)?;
        }
        node(a, b, m.init, n.init)?;
    }
    Ok(())
}

fn spans(a: &Program, b: &Program) -> bool {
    a.has_spans || b.has_spans
}

fn string(a: &Program, b: &Program, x: StrId, y: StrId) -> bool {
    match (x.is_none(), y.is_none()) {
        (true, true) => true,
        (false, false) => a.strings.get(x) == b.strings.get(y),
        _ => false,
    }
}

fn decl(a: &Program, b: &Program, id: DeclId) -> Same {
    let (x, y) = (a.decls.get(id), b.decls.get(id));
    let alts = |p: &Program| -> Vec<String> {
        let alts = p.decls.alts(id).iter();
        alts.map(|&s| p.strings.get(s).to_string()).collect()
    };
    let same = x.kind == y.kind
        && string(a, b, x.name, y.name)
        && alts(a) == alts(b)
        && x.owner == y.owner
        && a.types.same(x.ty, &b.types, y.ty)
        && x.facts == y.facts
        && string(a, b, x.doc, y.doc)
        && (!spans(a, b) || (x.span == y.span && x.name_span == y.name_span));
    check(same, || format!("declarations #{} differ", id.0))
}

fn node(p: &Program, q: &Program, x: NodeId, y: NodeId) -> Same {
    let (a, b) = (&p.nodes, &q.nodes);
    if !a.contains(x) || !b.contains(y) {
        return check(a.contains(x) == b.contains(y), || {
            "a node is missing".into()
        });
    }
    let kind = a.kind(x);
    let spans = spans(p, q);
    let flags = |f: Flags| {
        if spans {
            f
        } else {
            Flags(f.0 & !Flags::DERIVED.0)
        }
    };
    let (sa, sb) = (a.slots(x), b.slots(y));
    let payload_same = kind.fields().iter().all(|field| match field.slot {
        Slot::A(Payload::Str) => string(p, q, StrId(sa.0), StrId(sb.0)),
        Slot::A(_) => sa.0 == sb.0,
        Slot::B(Payload::Str) => string(p, q, StrId(sa.1), StrId(sb.1)),
        Slot::B(_) => sa.1 == sb.1,
        Slot::AB(_) => sa == sb,
        Slot::Flag | Slot::Kid(..) => true,
    });
    let same = kind == b.kind(y)
        && p.types.same(a.ty(x), &q.types, b.ty(y))
        && flags(a.flags(x)) == flags(b.flags(y))
        && (!spans || a.span(x) == b.span(y))
        && payload_same
        && a.kids(x).len() == b.kids(y).len();
    check(same, || {
        format!("{} nodes differ ({} and {})", kind.name(), x.0, y.0)
    })?;
    for (&k, &l) in a.kids(x).iter().zip(b.kids(y)) {
        node(p, q, k, l)?;
    }
    Ok(())
}
