//! The validator: checks a program against the model's invariants and a
//! profile (notes/tmpl/nodes.md, "Level 3: The validator's rules").
//!
//! Structure (S) runs first; the other groups run only on a structurally
//! sound program, so one mistake isn't reported many times. Problems come
//! sorted by writing order, each with its rule's code. Subset A's rules:
//!
//! - **S1** children fit the kind's fields, come before their parent, and
//!   only calls carry `MAY_BUBBLE`; **S2** one parent per node; **S3**
//!   statements, expressions, blocks, parameters, and module items in
//!   their places, and `InitLet` only in an init block; **S4** the
//!   profile allows each kind.
//! - **T1** typed expressions and declarations; **T2** the local rules'
//!   types; **T3** each child's type is its role's exactly; **T5** types
//!   well formed. `Error` nodes are exempt from T2 and T3.
//! - **R1** ids exist; **R2** references name the right kind of
//!   declaration; **R3** locals read and assigned in scope, within their
//!   function; **R4** jumps inside their loop; **R5** definite assignment;
//!   **R6** only a `var` assigned twice, and a module `let` initialized
//!   once.
//! - **F1** no falling off the end of a function with a result; **F2**
//!   `return` has a value exactly when the result isn't `Void`; **F3**
//!   `MAY_BUBBLE` exactly on calls that may bubble; **F4** a call that may
//!   bubble only in a function that may (or in an init block, where it
//!   fails the module's initialization); **F5** no reading a module `let`
//!   before its initializer.
//! - **D1** compiler-made declarations have a hint; **D2** owners form a
//!   tree rooted at modules.
//! - **P1** in a program with spans, every node and declaration has one.

use crate::builtins::Key;
use crate::decls::{DeclKind, Facts};
use crate::ids::{DeclId, NodeId, Span, StrId, TypeId};
use crate::nodes::{Flags, Kids};
use crate::profile::Profile;
use crate::schema::{Family, Kind, Node, Place, Slot, TypeRule};
use crate::text::writing_order;
use crate::types::TypeData;
use crate::{Module, Program};
use std::fmt;

/// A validator rule, by its code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rule {
    S1,
    S2,
    S3,
    S4,
    T1,
    T2,
    T3,
    T5,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    F1,
    F2,
    F3,
    F4,
    F5,
    D1,
    D2,
    P1,
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// What a problem is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum At {
    /// A node, and its number in writing order.
    Node(NodeId, u32),
    Decl(DeclId),
}

/// One broken rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub rule: Rule,
    pub at: At,
    pub message: String,
}

/// As golden tests list them: `R5 @12: …` for a node by its writing
/// order, `D1 #4: …` for a declaration.
impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.at {
            At::Node(_, n) => write!(f, "{} @{n}: {}", self.rule, self.message),
            At::Decl(d) => write!(f, "{} #{}: {}", self.rule, d.0, self.message),
        }
    }
}

/// Checks a program against the model's invariants and a profile.
pub fn validate(p: &Program, profile: &Profile) -> Vec<Problem> {
    let order = writing_order(p);
    let mut position = vec![u32::MAX; p.nodes.len()];
    for (n, &id) in order.iter().enumerate() {
        position[id.index()] = n as u32;
    }
    let mut v = Validator {
        position,
        problems: Vec::new(),
        parents: vec![NodeId::NONE; p.nodes.len()],
        body: Body::default(),
        scope: vec![false; p.decls.len()],
        defined: vec![false; p.decls.len()],
        slot: vec![u32::MAX; p.decls.len()],
    };
    for module in &p.modules {
        v.structure_module(p, module, profile);
    }
    if v.problems.is_empty() {
        v.declarations(p);
        for module in &p.modules {
            v.module(p, module);
        }
        v.spans(p, &order);
    }
    v.sorted(p)
}

/// The state of checking one function body or init block.
#[derive(Default)]
struct Body {
    /// The function or module that owns its locals and labels.
    owner: DeclId,
    /// The function's result, or `NONE` in an init block.
    result: TypeId,
    /// Whether a call may bubble here.
    may_bubble: bool,
    in_init: bool,
    /// Declarations whose assignment is tracked, by slot.
    tracked: Vec<DeclId>,
    /// Locals in scope, innermost last.
    in_scope: Vec<DeclId>,
    /// Enclosing loops, innermost last, with the flow at their breaks and
    /// continues.
    loops: Vec<Loop>,
}

struct Loop {
    label: DeclId,
    breaks: Flow,
    continues: Flow,
}

/// What's assigned at a point: definitely (`done`) and possibly
/// (`maybe`), by tracked slot. A point that can't be reached is dead.
#[derive(Clone, Debug)]
struct Flow {
    live: bool,
    done: Vec<u64>,
    maybe: Vec<u64>,
}

impl Flow {
    fn new(slots: usize) -> Flow {
        let words = slots.div_ceil(64);
        Flow {
            live: true,
            done: vec![0; words],
            maybe: vec![0; words],
        }
    }

    fn dead(slots: usize) -> Flow {
        Flow {
            live: false,
            ..Flow::new(slots)
        }
    }

    fn has(bits: &[u64], slot: u32) -> bool {
        bits[slot as usize / 64] & (1 << (slot % 64)) != 0
    }

    fn set(&mut self, slot: u32, assigned: bool) {
        let (word, bit) = (slot as usize / 64, 1u64 << (slot % 64));
        if assigned {
            self.done[word] |= bit;
            self.maybe[word] |= bit;
        } else {
            self.done[word] &= !bit;
            self.maybe[word] &= !bit;
        }
    }

    /// Where two paths meet.
    fn join(&mut self, other: &Flow) {
        if !other.live {
            return;
        }
        if !self.live {
            *self = other.clone();
            return;
        }
        for (a, b) in self.done.iter_mut().zip(&other.done) {
            *a &= b;
        }
        for (a, b) in self.maybe.iter_mut().zip(&other.maybe) {
            *a |= b;
        }
    }
}

struct Validator {
    /// Each node's number in writing order.
    position: Vec<u32>,
    problems: Vec<Problem>,
    parents: Vec<NodeId>,
    body: Body,
    /// Locals in scope, by id.
    scope: Vec<bool>,
    /// Declarations already defined by a node.
    defined: Vec<bool>,
    /// Each tracked declaration's slot in `Flow`.
    slot: Vec<u32>,
}

impl Validator {
    fn node_problem(&mut self, rule: Rule, node: NodeId, message: String) {
        let n = self.position.get(node.index()).copied().unwrap_or(u32::MAX);
        self.problems.push(Problem {
            rule,
            at: At::Node(node, n),
            message,
        });
    }

    fn decl_problem(&mut self, rule: Rule, decl: DeclId, message: String) {
        self.problems.push(Problem {
            rule,
            at: At::Decl(decl),
            message,
        });
    }

    /// Sorted by writing order; a declaration's problems come at the node
    /// that defines it.
    fn sorted(mut self, p: &Program) -> Vec<Problem> {
        let mut defined_at = vec![u32::MAX; p.decls.len()];
        for (id, &n) in self.position.iter().enumerate() {
            let id = NodeId(id as u32);
            if n == u32::MAX || p.nodes.parts(id).is_none() {
                continue;
            }
            if let Some(decl) = defined_decl(p, id) {
                if let Some(at) = defined_at.get_mut(decl.index()) {
                    *at = (*at).min(n);
                }
            }
        }
        for module in &p.modules {
            if let Some(at) = defined_at.get_mut(module.decl.index()) {
                *at = 0;
            }
        }
        let key = |problem: &Problem| match problem.at {
            At::Node(_, n) => (n, 1, 0),
            At::Decl(d) => (
                defined_at.get(d.index()).copied().unwrap_or(u32::MAX),
                0,
                d.0,
            ),
        };
        self.problems.sort_by_key(key);
        self.problems
    }

    // Structure.

    fn structure_module(&mut self, p: &Program, module: &Module, profile: &Profile) {
        for &item in &module.items {
            if self.structure(p, item, NodeId::NONE, None, profile, false) {
                let kind = p.nodes.kind(item);
                if !matches!(kind, Kind::Function | Kind::ModuleLet) {
                    let message = format!("a module item can't be a {}", kind.name());
                    self.node_problem(Rule::S3, item, message);
                }
            }
        }
        if self.structure(p, module.init, NodeId::NONE, None, profile, true)
            && p.nodes.kind(module.init) != Kind::Block
        {
            let message = format!("module #{}'s init isn't a Block", module.decl.0);
            self.node_problem(Rule::S3, module.init, message);
        }
    }

    /// Checks a node and its children; returns whether the node exists
    /// and is new, so its own place can be checked.
    fn structure(
        &mut self,
        p: &Program,
        id: NodeId,
        parent: NodeId,
        place: Option<Place>,
        profile: &Profile,
        in_init: bool,
    ) -> bool {
        let at = if parent.is_none() { id } else { parent };
        if !p.nodes.contains(id) || (!parent.is_none() && id >= parent) {
            let message = if p.nodes.contains(id) {
                format!("child {} doesn't come before its parent", id.0)
            } else {
                format!("node {} doesn't exist", id.0)
            };
            if p.nodes.contains(at) {
                self.node_problem(Rule::S1, at, message);
            } else {
                self.problems.push(Problem {
                    rule: Rule::S1,
                    at: At::Node(at, u32::MAX),
                    message,
                });
            }
            return false;
        }
        if !self.parents[id.index()].is_none() {
            let message = format!(
                "its child {} has another parent too",
                p.nodes.kind(id).name()
            );
            self.node_problem(Rule::S2, at, message);
            return false;
        }
        self.parents[id.index()] = at;
        let kind = p.nodes.kind(id);
        if let Some(place) = place {
            let fits = match place {
                Place::Param => kind == Kind::Param,
                Place::Block => kind == Kind::Block,
                Place::Stmt => kind.family() == Family::Stmt,
                Place::Expr => kind.family() == Family::Expr,
            };
            if !fits {
                let message = format!("{} can't go where {place:?} goes", kind.name());
                self.node_problem(Rule::S3, id, message);
            }
        }
        if kind == Kind::InitLet && !in_init {
            let message = "InitLet outside an init block".into();
            self.node_problem(Rule::S3, id, message);
        }
        if !profile.native.contains(kind.needs()) {
            let message = format!("the profile doesn't allow {}", kind.name());
            self.node_problem(Rule::S4, id, message);
        }
        let has_flag = kind.fields().iter().any(|f| f.slot == Slot::Flag);
        if p.nodes.flags(id).contains(Flags::MAY_BUBBLE) && !has_flag {
            let message = format!("{} can't be marked may-bubble", kind.name());
            self.node_problem(Rule::S1, id, message);
        }
        let Some(parts) = p.nodes.parts(id) else {
            let message = format!(
                "{} can't have {} children",
                kind.name(),
                p.nodes.kids(id).len()
            );
            self.node_problem(Rule::S1, id, message);
            return true;
        };
        let places = kind.fields().iter().filter_map(|f| match f.slot {
            Slot::Kid(_, place) => Some(place),
            _ => None,
        });
        for (kids, place) in parts.zip(places) {
            for &kid in p.nodes.list(kids) {
                self.structure(p, kid, id, Some(place), profile, in_init);
            }
        }
        true
    }

    // Declarations.

    fn declarations(&mut self, p: &Program) {
        for decl in p.decls.ids() {
            let d = p.decls.get(decl);
            if d.kind == DeclKind::Removed {
                continue;
            }
            if d.facts.contains(Facts::COMPILER_MADE) && p.name(decl).is_empty() {
                self.decl_problem(Rule::D1, decl, "compiler-made without a hint".into());
            }
            let owner_kind = p.decls.contains(d.owner).then(|| p.decls.get(d.owner).kind);
            let owner_fits = match d.kind {
                DeclKind::Module => d.owner.is_none(),
                DeclKind::Function | DeclKind::ModuleLet => owner_kind == Some(DeclKind::Module),
                _ => matches!(owner_kind, Some(DeclKind::Module | DeclKind::Function)),
            };
            if !owner_fits {
                let message = format!("a {} can't be owned by #{}", d.kind.name(), d.owner.0);
                self.decl_problem(Rule::D2, decl, message);
            }
            let ty = if p.types.contains(d.ty) {
                Some(p.types.get(d.ty))
            } else {
                None
            };
            match (d.kind, ty) {
                (DeclKind::Module | DeclKind::Label, _) => {}
                (DeclKind::Function, Some(TypeData::Function { .. })) => {}
                (DeclKind::Function, _) => {
                    self.decl_problem(Rule::T1, decl, "a function's type is its signature".into())
                }
                (_, Some(TypeData::Scalar(_))) => {}
                (_, Some(TypeData::Function { .. })) => self.decl_problem(
                    Rule::T5,
                    decl,
                    "function types aren't values in subset A".into(),
                ),
                (_, None) => {
                    let message = format!("a {} needs a type", d.kind.name());
                    self.decl_problem(Rule::T1, decl, message)
                }
            }
        }
    }

    // Bodies: types, references, and flow.

    fn module(&mut self, p: &Program, module: &Module) {
        for &item in &module.items {
            match p.nodes.get(item) {
                Node::Function { decl, params, body } => self.function(p, item, decl, params, body),
                Node::ModuleLet { decl } => {
                    if self.define(p, item, decl, DeclKind::ModuleLet) {
                        self.check_owner(p, item, decl, module.decl);
                    }
                }
                _ => {}
            }
        }
        // The init block tracks this module's `let`s, for F5 and R6.
        let lets: Vec<DeclId> = module
            .items
            .iter()
            .filter_map(|&item| match p.nodes.get(item) {
                Node::ModuleLet { decl } if p.decls.contains(decl) => Some(decl),
                _ => None,
            })
            .collect();
        self.start_body(p, module.decl, TypeId::NONE, true, module.init, &lets);
        self.body.in_init = true;
        let mut flow = Flow::new(self.body.tracked.len());
        self.stmt(p, module.init, &mut flow);
        self.end_body();
    }

    fn function(&mut self, p: &Program, id: NodeId, decl: DeclId, params: Kids, body: NodeId) {
        if !self.define(p, id, decl, DeclKind::Function) {
            return;
        }
        let Some((types, result, bubbles)) = p.signature(decl) else {
            return;
        };
        let params = p.nodes.list(params);
        if params.len() != types.len() {
            let message = format!(
                "the signature has {} parameters, the function {}",
                types.len(),
                params.len()
            );
            self.node_problem(Rule::T1, id, message);
        }
        self.start_body(p, decl, result, bubbles, body, &[]);
        for (i, &param) in params.iter().enumerate() {
            let Node::Param { decl: d } = p.nodes.get(param) else {
                continue;
            };
            if !self.define(p, param, d, DeclKind::Param) {
                continue;
            }
            self.check_owner(p, param, d, decl);
            if types.get(i).is_some_and(|&t| t != p.decls.get(d).ty) {
                let message = format!("{}'s type differs from the signature's", show(p, d));
                self.node_problem(Rule::T1, param, message);
            }
            self.enter_scope(d);
        }
        let mut flow = Flow::new(self.body.tracked.len());
        self.stmt(p, body, &mut flow);
        if flow.live && result != TypeId::VOID {
            let message = format!("{} can reach its end without returning", show(p, decl));
            self.node_problem(Rule::F1, id, message);
        }
        self.end_body();
    }

    /// Starts checking a body: tracks its `let`s, and `extra`.
    fn start_body(
        &mut self,
        p: &Program,
        owner: DeclId,
        result: TypeId,
        may_bubble: bool,
        body: NodeId,
        extra: &[DeclId],
    ) {
        let mut tracked = extra.to_vec();
        let mut stack = vec![body];
        while let Some(id) = stack.pop() {
            if let Node::Let { local, .. } = p.nodes.get(id) {
                if p.decls.contains(local) && self.slot[local.index()] == u32::MAX {
                    tracked.push(local);
                }
            }
            stack.extend(p.nodes.kids(id));
        }
        for (slot, &decl) in tracked.iter().enumerate() {
            self.slot[decl.index()] = slot as u32;
        }
        self.body = Body {
            owner,
            result,
            may_bubble,
            in_init: false,
            tracked,
            in_scope: Vec::new(),
            loops: Vec::new(),
        };
    }

    fn end_body(&mut self) {
        for &decl in &self.body.tracked {
            self.slot[decl.index()] = u32::MAX;
        }
        for &decl in &self.body.in_scope {
            self.scope[decl.index()] = false;
        }
    }

    fn enter_scope(&mut self, decl: DeclId) {
        self.scope[decl.index()] = true;
        self.body.in_scope.push(decl);
    }

    fn leave_scopes(&mut self, to: usize) {
        for decl in self.body.in_scope.drain(to..) {
            self.scope[decl.index()] = false;
        }
    }

    /// Checks that a node's declaration exists, is of its kind, and is
    /// defined only here.
    fn define(&mut self, p: &Program, id: NodeId, decl: DeclId, kind: DeclKind) -> bool {
        if !self.reference(p, id, decl, &[kind]) {
            return false;
        }
        if std::mem::replace(&mut self.defined[decl.index()], true) {
            let message = format!("{} is defined twice", show(p, decl));
            self.node_problem(Rule::R1, id, message);
        }
        true
    }

    fn check_owner(&mut self, p: &Program, id: NodeId, decl: DeclId, owner: DeclId) {
        if p.decls.get(decl).owner != owner {
            let message = format!("{}'s owner should be #{}", show(p, decl), owner.0);
            self.node_problem(Rule::D2, id, message);
        }
    }

    /// Checks that a reference names an existing declaration of one of
    /// `kinds` (R1 and R2).
    fn reference(&mut self, p: &Program, id: NodeId, decl: DeclId, kinds: &[DeclKind]) -> bool {
        if !p.decls.contains(decl) {
            let message = format!("declaration #{} doesn't exist", decl.0);
            self.node_problem(Rule::R1, id, message);
            return false;
        }
        let kind = p.decls.get(decl).kind;
        if !kinds.contains(&kind) {
            let message = format!(
                "{} needs a {}, not {}, a {}",
                p.nodes.kind(id).name(),
                kinds[0].name(),
                show(p, decl),
                kind.name()
            );
            self.node_problem(Rule::R2, id, message);
            return false;
        }
        true
    }

    /// Checks that a child's type is exactly what its role wants.
    fn fits(&mut self, p: &Program, child: NodeId, want: TypeId, role: &str) {
        let have = p.nodes.ty(child);
        if have != want && p.nodes.kind(child) != Kind::Error && !have.is_none() {
            let message = format!(
                "{role} is {}, but should be {}",
                p.types.show(have),
                p.types.show(want)
            );
            self.node_problem(Rule::T3, child, message);
        }
    }

    fn slot_of(&self, decl: DeclId) -> Option<u32> {
        let slot = self.slot[decl.index()];
        (slot != u32::MAX).then_some(slot)
    }

    /// Records an assignment, reporting a second one of a non-`var`.
    fn assigned(&mut self, p: &Program, id: NodeId, decl: DeclId, flow: &mut Flow) {
        let Some(slot) = self.slot_of(decl) else {
            return;
        };
        let var = p.decls.get(decl).facts.contains(Facts::VAR);
        if flow.live && !var && Flow::has(&flow.maybe, slot) {
            let what = match p.nodes.kind(id) {
                Kind::InitLet => "initialized",
                _ => "assigned",
            };
            let message = format!("{} isn't a var, but may be {what} twice", show(p, decl));
            self.node_problem(Rule::R6, id, message);
        }
        flow.set(slot, true);
    }

    fn stmt(&mut self, p: &Program, id: NodeId, flow: &mut Flow) {
        match p.nodes.get(id) {
            Node::Block { stmts } => {
                let mark = self.body.in_scope.len();
                for &stmt in p.nodes.list(stmts) {
                    self.stmt(p, stmt, flow);
                }
                self.leave_scopes(mark);
            }
            Node::Let { local, init } => {
                if let Some(init) = init {
                    self.expr(p, init, flow);
                }
                if !self.define(p, id, local, DeclKind::Local) {
                    return;
                }
                self.check_owner(p, id, local, self.body.owner);
                if let Some(init) = init {
                    self.fits(p, init, p.decls.get(local).ty, "the initializer");
                }
                if let Some(slot) = self.slot_of(local) {
                    // A fresh binding each time it runs, as in a loop.
                    flow.set(slot, init.is_some());
                }
                self.enter_scope(local);
            }
            Node::Assign { local, value } => {
                self.expr(p, value, flow);
                if !self.reference(p, id, local, &[DeclKind::Local]) {
                    return;
                }
                self.in_scope(p, id, local);
                self.fits(p, value, p.decls.get(local).ty, "the value");
                self.assigned(p, id, local, flow);
            }
            Node::SetModuleLet { target, value } => {
                self.expr(p, value, flow);
                if !self.reference(p, id, target, &[DeclKind::ModuleLet]) {
                    return;
                }
                self.fits(p, value, p.decls.get(target).ty, "the value");
                if !p.decls.get(target).facts.contains(Facts::VAR) {
                    let message = format!("{} isn't a var", show(p, target));
                    self.node_problem(Rule::R6, id, message);
                }
                if let Some(slot) = self.slot_of(target) {
                    flow.set(slot, true);
                }
            }
            Node::InitLet { target, value } => {
                self.expr(p, value, flow);
                if !self.reference(p, id, target, &[DeclKind::ModuleLet]) {
                    return;
                }
                self.fits(p, value, p.decls.get(target).ty, "the value");
                if p.decls.get(target).owner != self.body.owner {
                    let message = format!("{} is another module's", show(p, target));
                    self.node_problem(Rule::R3, id, message);
                }
                self.assigned(p, id, target, flow);
            }
            Node::If { cond, then, els } => {
                self.condition(p, cond, flow);
                let mut other = flow.clone();
                self.stmt(p, then, flow);
                if let Some(els) = els {
                    self.stmt(p, els, &mut other);
                }
                flow.join(&other);
            }
            Node::While { label, cond, body } => {
                self.repeat(p, id, label, flow);
                self.condition(p, cond, flow);
                let mut exit = if is_true(p, cond) {
                    Flow::dead(flow.done.len() * 64)
                } else {
                    flow.clone()
                };
                self.stmt(p, body, flow);
                self.end_loop(&mut exit);
                *flow = exit;
            }
            Node::DoWhile { label, body, cond } => {
                self.repeat(p, id, label, flow);
                self.stmt(p, body, flow);
                if let Some(l) = self.body.loops.last_mut() {
                    flow.join(&l.continues.clone());
                }
                self.condition(p, cond, flow);
                let mut exit = if is_true(p, cond) {
                    Flow::dead(flow.done.len() * 64)
                } else {
                    flow.clone()
                };
                self.end_loop(&mut exit);
                *flow = exit;
            }
            Node::Break { label } | Node::Continue { label } => {
                if self.reference(p, id, label, &[DeclKind::Label]) {
                    let is_break = p.nodes.kind(id) == Kind::Break;
                    match self.body.loops.iter_mut().rev().find(|l| l.label == label) {
                        Some(l) if is_break => l.breaks.join(flow),
                        Some(l) => l.continues.join(flow),
                        None => {
                            let message = format!("{} isn't an enclosing loop", show(p, label));
                            self.node_problem(Rule::R4, id, message);
                        }
                    }
                }
                flow.live = false;
            }
            Node::Return { value } => {
                if let Some(value) = value {
                    self.expr(p, value, flow);
                }
                let result = self.body.result;
                if self.body.in_init {
                    self.node_problem(Rule::F2, id, "a return outside a function".into());
                } else if result == TypeId::VOID && value.is_some() {
                    let message = "a Void function's return has no value".into();
                    self.node_problem(Rule::F2, id, message);
                } else if result != TypeId::VOID && value.is_none() {
                    let message = format!("the return needs a {}", p.types.show(result));
                    self.node_problem(Rule::F2, id, message);
                } else if let Some(value) = value {
                    self.fits(p, value, result, "the returned value");
                }
                flow.live = false;
            }
            Node::ExprStatement { expr } => self.expr(p, expr, flow),
            _ => {}
        }
    }

    /// Starts a loop: defines its label, and since its body may run
    /// again, counts what it assigns as maybe assigned already.
    fn repeat(&mut self, p: &Program, id: NodeId, label: DeclId, flow: &mut Flow) {
        if self.define(p, id, label, DeclKind::Label) {
            self.check_owner(p, id, label, self.body.owner);
        }
        let mut stack = p.nodes.kids(id).to_vec();
        while let Some(node) = stack.pop() {
            let target = match p.nodes.get(node) {
                Node::Assign { local: d, .. }
                | Node::SetModuleLet { target: d, .. }
                | Node::InitLet { target: d, .. } => Some(d),
                _ => None,
            };
            let target = target.filter(|&d| p.decls.contains(d));
            if let Some(slot) = target.and_then(|d| self.slot_of(d)) {
                flow.maybe[slot as usize / 64] |= 1 << (slot % 64);
            }
            stack.extend(p.nodes.kids(node));
        }
        let slots = flow.done.len() * 64;
        self.body.loops.push(Loop {
            label,
            breaks: Flow::dead(slots),
            continues: Flow::dead(slots),
        });
    }

    fn end_loop(&mut self, exit: &mut Flow) {
        if let Some(l) = self.body.loops.pop() {
            exit.join(&l.breaks);
        }
    }

    fn condition(&mut self, p: &Program, cond: NodeId, flow: &mut Flow) {
        self.expr(p, cond, flow);
        self.fits(p, cond, TypeId::BOOLEAN, "the condition");
    }

    fn in_scope(&mut self, p: &Program, id: NodeId, local: DeclId) {
        if !self.scope[local.index()] || p.decls.get(local).owner != self.body.owner {
            let message = format!("{} isn't in scope here", show(p, local));
            self.node_problem(Rule::R3, id, message);
        }
    }

    fn expr(&mut self, p: &Program, id: NodeId, flow: &mut Flow) {
        let ty = p.nodes.ty(id);
        let kind = p.nodes.kind(id);
        if ty.is_none() || !p.types.contains(ty) {
            let message = format!("{} needs a type", kind.name());
            self.node_problem(Rule::T1, id, message);
        } else if let Some(want) = self.rule_type(p, id) {
            if want != ty && kind != Kind::Error && !want.is_none() {
                let message = format!(
                    "{} is {}, not {}",
                    kind.name(),
                    p.types.show(want),
                    p.types.show(ty)
                );
                self.node_problem(Rule::T2, id, message);
            }
        }
        match p.nodes.get(id) {
            Node::StringLit { value: s } | Node::Error { message: s } => self.string(p, id, s),
            Node::Local { local } => {
                if self.reference(p, id, local, &[DeclKind::Local, DeclKind::Param]) {
                    self.in_scope(p, id, local);
                    if let Some(slot) = self.slot_of(local) {
                        if flow.live && !Flow::has(&flow.done, slot) {
                            let message =
                                format!("{} may be read before it's assigned", show(p, local));
                            self.node_problem(Rule::R5, id, message);
                        }
                    }
                }
            }
            Node::GetModuleLet { target } => {
                if self.reference(p, id, target, &[DeclKind::ModuleLet]) {
                    if let Some(slot) = self.slot_of(target) {
                        if flow.live && !Flow::has(&flow.done, slot) {
                            let message =
                                format!("{} may be read before it's initialized", show(p, target));
                            self.node_problem(Rule::F5, id, message);
                        }
                    }
                }
            }
            Node::CallFunction {
                callee,
                may_bubble,
                args,
            } => {
                for &arg in p.nodes.list(args) {
                    self.expr(p, arg, flow);
                }
                if !self.reference(p, id, callee, &[DeclKind::Function]) {
                    return;
                }
                if let Some((params, _, bubbles)) = p.signature(callee) {
                    let what = show(p, callee);
                    self.call(p, id, &what, (params, bubbles), (may_bubble, args));
                }
            }
            Node::Builtin {
                key,
                may_bubble,
                args,
            } => {
                for &arg in p.nodes.list(args) {
                    self.expr(p, arg, flow);
                }
                let Some(info) = key.info() else {
                    let message = format!("key {} doesn't exist", key.0);
                    self.node_problem(Rule::R1, id, message);
                    return;
                };
                let what = format!("%{}", info.name);
                self.call(
                    p,
                    id,
                    &what,
                    (info.params, info.bubbles),
                    (may_bubble, args),
                );
            }
            Node::And { left, right } | Node::Or { left, right } => {
                self.condition(p, left, flow);
                let mut taken = flow.clone();
                self.condition(p, right, &mut taken);
                flow.join(&taken);
            }
            Node::IfValue { cond, then, els } => {
                self.condition(p, cond, flow);
                let mut other = flow.clone();
                self.expr(p, then, flow);
                self.fits(p, then, ty, "the branch");
                self.expr(p, els, &mut other);
                self.fits(p, els, ty, "the branch");
                flow.join(&other);
            }
            Node::BlockValue { stmts, value } => {
                let mark = self.body.in_scope.len();
                for &stmt in p.nodes.list(stmts) {
                    self.stmt(p, stmt, flow);
                }
                self.expr(p, value, flow);
                self.fits(p, value, ty, "the value");
                self.leave_scopes(mark);
            }
            _ => {}
        }
    }

    /// The type the local rules give a node, if they cover it.
    fn rule_type(&self, p: &Program, id: NodeId) -> Option<TypeId> {
        let (a, _) = p.nodes.slots(id);
        let decl_ty = |d: DeclId| {
            if p.decls.contains(d) {
                p.decls.get(d).ty
            } else {
                TypeId::NONE
            }
        };
        match p.nodes.kind(id).type_rule() {
            TypeRule::None | TypeRule::Written => None,
            TypeRule::Fixed(ty) => Some(ty),
            TypeRule::Decl => Some(decl_ty(DeclId(a))),
            TypeRule::Callee => Some(match p.types.signature(decl_ty(DeclId(a))) {
                Some((_, result, _)) => result,
                None => TypeId::NONE,
            }),
            TypeRule::Key => Some(Key(a).info().map_or(TypeId::NONE, |info| info.result)),
        }
    }

    fn string(&mut self, p: &Program, id: NodeId, s: StrId) {
        if !p.strings.contains(s) {
            let message = format!("string {} doesn't exist", s.0);
            self.node_problem(Rule::R1, id, message);
        }
    }

    /// A call's arguments and bubbling, against its callee's signature.
    fn call(
        &mut self,
        p: &Program,
        id: NodeId,
        what: &str,
        (params, bubbles): (&[TypeId], bool),
        (marked, args): (bool, Kids),
    ) {
        let args = p.nodes.list(args);
        if args.len() != params.len() {
            let message = format!(
                "{what} takes {} arguments, not {}",
                params.len(),
                args.len()
            );
            self.node_problem(Rule::T3, id, message);
        } else {
            for (i, (&arg, &param)) in args.iter().zip(params).enumerate() {
                self.fits(p, arg, param, &format!("argument {}", i + 1));
            }
        }
        if marked != bubbles {
            let message = if bubbles {
                format!("the call of {what} may bubble, but isn't marked")
            } else {
                format!("the call of {what} can't bubble, but is marked")
            };
            self.node_problem(Rule::F3, id, message);
        } else if bubbles && !self.body.may_bubble {
            let message = format!(
                "the call of {what} may bubble, but {} can't",
                show(p, self.body.owner)
            );
            self.node_problem(Rule::F4, id, message);
        }
    }

    // Positions.

    fn spans(&mut self, p: &Program, order: &[NodeId]) {
        if !p.has_spans {
            return;
        }
        let fits = |span: Span| {
            span.is_known() && span.file.index() < p.files.len() && span.start <= span.end
        };
        for &id in order {
            if !fits(p.nodes.span(id)) {
                let message = format!("{} has no span in a known file", p.nodes.kind(id).name());
                self.node_problem(Rule::P1, id, message);
            }
        }
        for decl in p.decls.ids() {
            let d = p.decls.get(decl);
            if d.kind != DeclKind::Removed && !(fits(d.span) && fits(d.name_span)) {
                let message = "a declaration needs its spans in a known file".into();
                self.decl_problem(Rule::P1, decl, message);
            }
        }
    }
}

/// `name#id`, for messages.
fn show(p: &Program, decl: DeclId) -> String {
    let made = p.decls.get(decl).facts.contains(Facts::COMPILER_MADE);
    let tilde = if made { "~" } else { "" };
    format!("{tilde}{}#{}", p.name(decl), decl.0)
}

/// The declaration a node defines, if it defines one.
fn defined_decl(p: &Program, id: NodeId) -> Option<DeclId> {
    match p.nodes.get(id) {
        Node::Function { decl, .. } | Node::Param { decl } | Node::ModuleLet { decl } => Some(decl),
        Node::Let { local, .. } => Some(local),
        Node::While { label, .. } | Node::DoWhile { label, .. } => Some(label),
        _ => None,
    }
}

fn is_true(p: &Program, cond: NodeId) -> bool {
    matches!(p.nodes.get(cond), Node::BoolLit { value: true })
}
