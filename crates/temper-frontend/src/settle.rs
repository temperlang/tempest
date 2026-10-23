//! The settle loop: name resolution and macro expansion as work items in
//! the [`Engine`], until every node is a core construct ([`crate::core`])
//! or an error.
//!
//! Two kinds of work:
//!
//! - **Reading a block** (phase 0): its statements in order, each expanded
//!   outside-in only as far as needed to see what it declares. A statement
//!   whose callee names a macro is expanded right away, and again if the
//!   result is another macro call; a declaration is declared. Then the
//!   block's scope is complete. A macro name resolves against what's
//!   declared so far, and waits only on enclosing scopes.
//! - **Visiting an expression** (phase 1): resolving its names, expanding
//!   macro calls (callee first: nothing under a macro call is touched until
//!   it has expanded), and queuing its parts. Scopes come from the node
//!   kinds: blocks, functions and loops make them.
//!
//! The only fact so far is a scope's completion. When the loop gets stuck
//! the engine forces the earliest stuck item, whose lookup then treats
//! scopes as complete; a declaration that turns up later where such a lookup
//! passed is an error, so no resolved name ever changes.
//!
//! Debug builds check that no step read a declaration from a node that a
//! later step rewrote.

use crate::builtins::Builtin;
use crate::core;
use crate::engine::{Engine, ItemId, Next};
use crate::library::{Level, Library, Message};
use crate::scope::{BUILTINS, SymbolId, SymbolKind, Lookup, ScopeId, ScopeKind, Scopes};
use std::collections::HashSet;
use temper_syntax::ast::{Builtin as SyntaxBuiltin, Value};
use temper_syntax::names::NameId;
use temper_syntax::tree::{NodeId, NodeKind, Span};

/// What a work item can wait on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Fact {
    /// All of a scope's declarations are known.
    ScopeComplete(ScopeId),
}

#[derive(Clone, Copy, Debug)]
enum Work {
    Block { node: NodeId, scope: ScopeId, next: u32 },
    Expr { node: NodeId, scope: ScopeId },
}

const READ_BLOCKS: u8 = 0;
const VISIT: u8 = 1;

/// A settled library: its tree, now of core constructs, and the scopes and
/// symbols its names resolved to.
#[derive(Debug)]
pub struct Settled {
    pub library: Library,
    pub scopes: Scopes,
    pub module: ScopeId,
    /// Steps the loop took.
    pub steps: u64,
}

/// Settles a library's files.
pub fn settle(library: Library) -> Settled {
    let mut library = library;
    let scopes = Scopes::new(&mut library.names);
    let budget = 1000 + 100 * library.tree.len() as u64;
    let mut s = Settle {
        lib: library,
        scopes,
        engine: Engine::new(budget),
        work: Vec::new(),
        module: BUILTINS,
        module_blocks: 0,
        passed: Vec::new(),
        removed: HashSet::new(),
        step: 0,
        #[cfg(debug_assertions)]
        reads: Vec::new(),
        #[cfg(debug_assertions)]
        rewrites: Vec::new(),
    };
    let roots: Vec<NodeId> = s.lib.files.iter().map(|f| f.root).collect();
    s.module = s.scopes.add(BUILTINS, ScopeKind::Module, roots.first().copied().unwrap_or(NodeId(0)), roots.is_empty());
    for &root in &roots {
        s.module_blocks += 1;
        let item = s.add(Work::Block { node: root, scope: s.module, next: 0 }, READ_BLOCKS, root);
        s.engine.will_produce(Fact::ScopeComplete(s.module), item);
    }
    s.run();
    #[cfg(debug_assertions)]
    s.check_reads();
    Settled { steps: s.engine.steps(), library: s.lib, scopes: s.scopes, module: s.module }
}

pub(crate) struct Settle {
    pub lib: Library,
    pub scopes: Scopes,
    engine: Engine<Fact>,
    work: Vec<Work>,
    module: ScopeId,
    /// The module's blocks, one per file, still being read.
    module_blocks: u32,
    /// Lookups that resolved past a scope that wasn't complete yet, as a
    /// statement's macro name or when forced: the scope, name and use.
    passed: Vec<(ScopeId, NameId, Span)>,
    /// Statements to drop from their blocks, like comments.
    removed: HashSet<NodeId>,
    /// The current step, for the debug check.
    step: u64,
    #[cfg(debug_assertions)]
    reads: Vec<(NodeId, u64)>,
    #[cfg(debug_assertions)]
    rewrites: Vec<(NodeId, u64)>,
}

/// What a step did.
enum Outcome {
    Done,
    Wait(Fact),
}

impl Settle {
    fn add(&mut self, work: Work, phase: u8, node: NodeId) -> ItemId {
        let id = self.engine.add(phase, self.lib.tree.span(node).start);
        debug_assert_eq!(id.0 as usize, self.work.len());
        self.work.push(work);
        id
    }

    /// Queues a visit of an expression.
    pub(crate) fn visit(&mut self, node: NodeId, scope: ScopeId) {
        self.add(Work::Expr { node, scope }, VISIT, node);
    }

    fn run(&mut self) {
        loop {
            let (id, forced) = match self.engine.next_step() {
                Next::Run(id) => (id, false),
                Next::Force(id, _) => (id, true),
                Next::Settled => break,
                Next::OutOfBudget(id) => {
                    let node = match self.work[id.0 as usize] {
                        Work::Block { node, .. } | Work::Expr { node, .. } => node,
                    };
                    let steps = self.engine.steps();
                    self.lib.report(Level::Error, Message::OutOfSteps(steps), self.lib.tree.span(node));
                    break;
                }
            };
            self.step = self.engine.steps();
            let outcome = match self.work[id.0 as usize] {
                Work::Block { node, scope, next } => self.read_block(id, node, scope, next, forced),
                Work::Expr { node, scope } => self.expr(node, scope, forced),
            };
            match outcome {
                Outcome::Done => self.engine.done(id),
                Outcome::Wait(fact) => self.engine.wait(id, fact),
            }
        }
    }

    // Reading blocks.

    fn read_block(&mut self, id: ItemId, node: NodeId, scope: ScopeId, mut next: u32, forced: bool) -> Outcome {
        let mut forced = forced;
        while let Some(&statement) = self.lib.tree.children(node).get(next as usize) {
            // `let { a, b } = x` stands for several statements.
            if self.lib.tree.kind(statement) == NodeKind::Decl {
                if let Some(statements) = self.destructure(statement) {
                    let mut children = self.lib.tree.children(node).to_vec();
                    children.splice(next as usize..=next as usize, statements);
                    self.set_children(node, &children);
                    continue;
                }
            }
            if let Err(fact) = self.head(statement, scope, forced) {
                self.work[id.0 as usize] = Work::Block { node, scope, next };
                return Outcome::Wait(fact);
            }
            forced = false;
            next += 1;
        }
        let children = self.lib.tree.children(node);
        if children.iter().any(|c| self.removed.contains(c)) {
            let kept: Vec<NodeId> = children.iter().copied().filter(|c| !self.removed.contains(c)).collect();
            self.set_children(node, &kept);
        }
        if scope == self.module {
            self.module_blocks -= 1;
            if self.module_blocks > 0 {
                return Outcome::Done;
            }
        }
        self.scopes.complete(scope);
        self.engine.publish(Fact::ScopeComplete(scope));
        Outcome::Done
    }

    /// Expands a statement as far as needed to see whether it declares
    /// something, then queues its visit.
    fn head(&mut self, statement: NodeId, scope: ScopeId, forced: bool) -> Result<(), Fact> {
        loop {
            match self.lib.tree.kind(statement) {
                NodeKind::Decl => {
                    self.lower_decl(statement);
                }
                NodeKind::Let | NodeKind::Class => {
                    self.declare(statement, scope);
                    self.visit(statement, scope);
                    return Ok(());
                }
                NodeKind::Call => {
                    let Some(&callee) = self.lib.tree.children(statement).first() else {
                        self.visit(statement, scope);
                        return Ok(());
                    };
                    match self.lib.tree.kind(callee) {
                        NodeKind::RightName if !self.is_temporary(callee) => {
                            let name = self.lib.name(callee).unwrap();
                            match self.lookup(scope, name, callee, true, forced) {
                                Lookup::Wait(s) => return Err(Fact::ScopeComplete(s)),
                                Lookup::Found(b) => {
                                    // Only a macro is settled here; anything else
                                    // resolves when visited, once its scope is complete.
                                    if let SymbolKind::Builtin(Builtin::Macro(m)) = self.scopes.symbol(b).kind {
                                        self.scopes.resolve(callee, b);
                                        self.expand(statement, scope, m);
                                        continue;
                                    }
                                }
                                Lookup::Missing => {}
                            }
                        }
                        NodeKind::Value => match self.lib.value(callee) {
                            Some(Value::Builtin(SyntaxBuiltin::EmbeddedComment)) => {
                                self.removed.insert(statement);
                                return Ok(());
                            }
                            Some(Value::Builtin(SyntaxBuiltin::DocComment)) => {
                                self.expand_syntax(statement, SyntaxBuiltin::DocComment);
                                continue;
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                    self.visit(statement, scope);
                    return Ok(());
                }
                _ => {
                    self.visit(statement, scope);
                    return Ok(());
                }
            }
        }
    }

    /// Declares a `Let`'s or `Class`'s name in a scope, unless it's
    /// declared already.
    pub(crate) fn declare(&mut self, let_node: NodeId, scope: ScopeId) {
        let name_node = self.lib.tree.children(let_node)[0];
        if self.scopes.resolved(name_node).is_some() {
            return;
        }
        let name = self.lib.name(name_node).unwrap();
        let is_let = self.lib.tree.kind(let_node) == NodeKind::Let;
        let mutable = is_let && self.lib.tree.data(let_node) & core::LET_VAR != 0;
        let kind = if self.lib.names.is_temporary(name) {
            SymbolKind::Synthetic
        } else {
            match self.scopes.get(scope).kind {
                ScopeKind::Module => SymbolKind::TopLevel,
                ScopeKind::Fn | ScopeKind::LoopBody => SymbolKind::Parameter,
                ScopeKind::Class => SymbolKind::Member,
                ScopeKind::TypeParams => SymbolKind::TypeParam,
                _ => SymbolKind::Local,
            }
        };
        let b = self.scopes.declare(scope, name, kind, Some(let_node), mutable);
        self.scopes.resolve(name_node, b);
        let uses: Vec<Span> =
            self.passed.iter().filter(|(s, n, _)| *s == scope && *n == name).map(|&(_, _, span)| span).collect();
        for span in uses {
            let text = self.lib.names.get(name).to_string();
            self.lib.report(Level::Error, Message::DeclaredAfterUse(text), span);
        }
    }

    /// Looks up a name used at `node`, recording lookups that resolve past an
    /// incomplete scope.
    fn lookup(&mut self, scope: ScopeId, name: NameId, node: NodeId, so_far: bool, forced: bool) -> Lookup {
        let mut from = scope;
        let mut skipped = Vec::new();
        let result = loop {
            match self.scopes.lookup(from, name, so_far && from == scope) {
                Lookup::Wait(s) if forced => {
                    skipped.push(s);
                    match self.scopes.get(s).parent {
                        Some(parent) => from = parent,
                        None => break Lookup::Missing,
                    }
                }
                other => break other,
            }
        };
        // What a statement's macro name resolves to is final, as is what a
        // forced lookup finds; other lookups in the starting scope are
        // redone once it's complete.
        let found_in = match result {
            Lookup::Found(b) => Some(self.scopes.symbol(b).owner),
            _ => None,
        };
        let is_macro = |b| matches!(self.scopes.symbol(b).kind, SymbolKind::Builtin(Builtin::Macro(_)));
        let settles = match result {
            Lookup::Found(b) => forced || is_macro(b),
            _ => false,
        };
        if !settles {
            skipped.clear();
        } else if so_far && !self.scopes.get(scope).complete {
            skipped.push(scope);
        }
        let span = self.lib.tree.span(node);
        for s in skipped {
            if found_in != Some(s) {
                self.passed.push((s, name, span));
            }
        }
        if let Lookup::Found(b) = result {
            if let Some(decl) = self.scopes.symbol(b).decl {
                self.read(decl);
            }
        }
        result
    }

    // Visiting expressions.

    fn expr(&mut self, node: NodeId, scope: ScopeId, forced: bool) -> Outcome {
        loop {
            let tree = &self.lib.tree;
            match tree.kind(node) {
                NodeKind::Value | NodeKind::Error | NodeKind::Break | NodeKind::Continue | NodeKind::Fail => {}
                NodeKind::LeftName => {}
                NodeKind::RightName => {
                    if let Err(fact) = self.resolve_name(node, scope, forced) {
                        return Outcome::Wait(fact);
                    }
                    if let Some(Builtin::Macro(m)) = self.scopes.resolved_builtin(node) {
                        let name = Builtin::Macro(m).name().to_string();
                        self.error(node, Message::MalformedSpecial(name));
                    }
                }
                NodeKind::BuiltinName => {
                    let name = self.lib.name(node).unwrap();
                    match self.scopes.builtin(name) {
                        Some(b) => self.scopes.resolve(node, b),
                        None => {
                            let text = format!("builtin {}", self.lib.names.get(name));
                            self.error(node, Message::Unsupported(text));
                        }
                    }
                }
                NodeKind::Decl => {
                    self.error(node, Message::MalformedDeclaration);
                }
                NodeKind::Let => {
                    self.declare(node, scope);
                    let parts = self.lib.tree.children(node)[1..].to_vec();
                    for part in parts {
                        self.visit(part, scope);
                    }
                }
                NodeKind::Block => self.enter_block(node, scope),
                NodeKind::Fun => {
                    self.lower_fun(node);
                    continue;
                }
                NodeKind::Fn => self.enter_fn(node, scope),
                NodeKind::Class => self.enter_class(node, scope),
                NodeKind::Loop => {
                    let loop_scope = self.scopes.add(scope, ScopeKind::Loop, node, true);
                    let parts = self.lib.tree.children(node).to_vec();
                    for part in parts {
                        self.visit(part, loop_scope);
                    }
                }
                NodeKind::Assign => {
                    let target = self.lib.tree.children(node)[0];
                    if self.lib.tree.kind(target) != NodeKind::RightName {
                        // `obj.name = value`, a field or setter.
                        let parts = self.lib.tree.children(node).to_vec();
                        for part in parts {
                            self.visit(part, scope);
                        }
                        return Outcome::Done;
                    }
                    if let Err(fact) = self.resolve_name(target, scope, forced) {
                        return Outcome::Wait(fact);
                    }
                    if let Some(b) = self.scopes.resolved(target) {
                        let symbol = *self.scopes.symbol(b);
                        // A `let` without a value may be assigned once, and a
                        // field, only in its class's constructor.
                        let assignable = if symbol.kind == SymbolKind::Member {
                            self.member_assignable(b, scope)
                        } else {
                            symbol.mutable || symbol.decl.is_some_and(|d| self.lib.tree.data(d) & core::LET_INIT == 0)
                        };
                        if !assignable {
                            let text = self.lib.names.get(symbol.name).to_string();
                            let span = self.lib.tree.span(target);
                            self.lib.report(Level::Error, Message::IllegalAssignment(text), span);
                            // It's not performed: the value stands in.
                            let value = self.lib.tree.children(node)[1];
                            self.become_node(node, value);
                            continue;
                        }
                    }
                    let value = self.lib.tree.children(node)[1];
                    self.visit(value, scope);
                }
                NodeKind::If | NodeKind::Orelse | NodeKind::Return => {
                    let parts = self.lib.tree.children(node).to_vec();
                    for part in parts {
                        self.visit(part, scope);
                    }
                }
                NodeKind::Call => match self.call(node, scope, forced) {
                    Ok(true) => continue,
                    Ok(false) => {}
                    Err(fact) => return Outcome::Wait(fact),
                },
                NodeKind::Esc => self.error(node, Message::Unsupported("quoted trees".into())),
                NodeKind::Op(_) | NodeKind::Token => unreachable!("parse node in a library tree"),
            }
            return Outcome::Done;
        }
    }

    /// Resolves a used name, unless it's resolved already.
    fn resolve_name(&mut self, node: NodeId, scope: ScopeId, forced: bool) -> Result<(), Fact> {
        if self.lib.tree.kind(node) != NodeKind::RightName || self.scopes.resolved(node).is_some() {
            return Ok(());
        }
        let name = self.lib.name(node).unwrap();
        match self.lookup(scope, name, node, false, forced) {
            Lookup::Found(b) => self.scopes.resolve(node, b),
            Lookup::Wait(s) => return Err(Fact::ScopeComplete(s)),
            Lookup::Missing => {
                let text = self.lib.names.get(name).to_string();
                self.error(node, Message::UndeclaredName(text));
            }
        }
        Ok(())
    }

    /// Visits a call: expands it if its callee is a macro, returning true,
    /// or queues its parts.
    fn call(&mut self, node: NodeId, scope: ScopeId, forced: bool) -> Result<bool, Fact> {
        let children = self.lib.tree.children(node).to_vec();
        let Some(&callee) = children.first() else {
            self.error(node, Message::Unsupported("a call without a callee".into()));
            return Ok(false);
        };
        if self.is_chain_null(callee) {
            // `a?.m(args)`: the whole call is skipped if `a` is null.
            self.expand_chain_null(node, callee);
            return Ok(true);
        }
        match self.lib.tree.kind(callee) {
            NodeKind::BuiltinName if self.is_chain_null(node) => {
                self.expand_chain_null(node, node);
                return Ok(true);
            }
            NodeKind::RightName => {
                self.resolve_name(callee, scope, forced)?;
                if let Some(Builtin::Macro(m)) = self.scopes.resolved_builtin(callee) {
                    self.expand(node, scope, m);
                    return Ok(true);
                }
            }
            NodeKind::Value => {
                if let Some(Value::Builtin(b)) = self.lib.value(callee) {
                    let b = *b;
                    if b == SyntaxBuiltin::This {
                        self.expand_this(node, scope);
                    } else {
                        self.expand_syntax(node, b);
                    }
                    return Ok(true);
                }
                self.visit(callee, scope);
            }
            _ => self.visit(callee, scope),
        }
        for &arg in &children[1..] {
            self.visit(arg, scope);
        }
        Ok(false)
    }

    /// Makes a block's scope, or a label's and then the block's, and queues
    /// reading it.
    fn enter_block(&mut self, node: NodeId, scope: ScopeId) {
        let children = self.lib.tree.children(node);
        let mut parent = scope;
        if let [label, name, ..] = *children {
            let is_label = self.lib.symbol(label).is_some_and(|s| self.lib.names.get(s) == "label");
            if is_label && self.lib.tree.kind(name) == NodeKind::LeftName {
                let name = self.lib.name(name).unwrap();
                parent = self.scopes.add(scope, ScopeKind::Label(name), node, true);
                let rest = children[2..].to_vec();
                self.set_children(node, &rest);
            }
        }
        let block_scope = self.scopes.add(parent, ScopeKind::Block, node, false);
        let item = self.add(Work::Block { node, scope: block_scope, next: 0 }, READ_BLOCKS, node);
        self.engine.will_produce(Fact::ScopeComplete(block_scope), item);
    }

    /// Makes a function's scope with its parameters, and queues its parts.
    fn enter_fn(&mut self, node: NodeId, scope: ScopeId) {
        let scope = self.enter_type_params(node, scope);
        let kind = if self.lib.tree.data(node) & core::FN_LOOP_BODY != 0 { ScopeKind::LoopBody } else { ScopeKind::Fn };
        let fn_scope = self.scopes.add(scope, kind, node, true);
        let (params, out_type, body) = core::fn_parts(&self.lib.tree, node);
        for &param in &params {
            self.declare(param, fn_scope);
            let parts = self.lib.tree.children(param)[1..].to_vec();
            for part in parts {
                self.visit(part, fn_scope);
            }
        }
        if let Some(t) = out_type {
            self.visit(t, fn_scope);
        }
        self.visit(body, fn_scope);
    }

    /// Makes a scope for a generic `Fn`'s or `Class`'s type parameters, if
    /// any, declaring them and queueing their bounds, which may use them.
    /// Returns the scope for the rest.
    pub(crate) fn enter_type_params(&mut self, node: NodeId, scope: ScopeId) -> ScopeId {
        let params = core::type_params(&self.lib.tree, node).to_vec();
        if params.is_empty() {
            return scope;
        }
        let inner = self.scopes.add(scope, ScopeKind::TypeParams, node, true);
        for &param in &params {
            self.declare(param, inner);
        }
        for param in params {
            let parts = self.lib.tree.children(param)[1..].to_vec();
            for part in parts {
                self.visit(part, inner);
            }
        }
        inner
    }

    // Rewriting, with the records for the debug check.

    pub(crate) fn rewrite(&mut self, node: NodeId, kind: NodeKind, data: u32, children: &[NodeId]) {
        #[cfg(debug_assertions)]
        self.rewrites.push((node, self.step));
        self.lib.tree.replace(node, kind, data, children);
    }

    pub(crate) fn set_children(&mut self, node: NodeId, children: &[NodeId]) {
        #[cfg(debug_assertions)]
        self.rewrites.push((node, self.step));
        self.lib.tree.set_children(node, children);
    }

    /// Rewrites a node into an error node.
    pub(crate) fn error(&mut self, node: NodeId, message: Message) {
        #[cfg(debug_assertions)]
        self.rewrites.push((node, self.step));
        self.lib.error(node, message);
    }

    /// Makes `node` a copy of `other`, which becomes garbage.
    pub(crate) fn become_node(&mut self, node: NodeId, other: NodeId) {
        let tree = &self.lib.tree;
        let (kind, data, children) = (tree.kind(other), tree.data(other), tree.children(other).to_vec());
        self.rewrite(node, kind, data, &children);
        if let Some(b) = self.scopes.resolved(other) {
            self.scopes.resolve(node, b);
        }
    }

    fn read(&mut self, _node: NodeId) {
        #[cfg(debug_assertions)]
        self.reads.push((_node, self.step));
    }

    #[cfg(debug_assertions)]
    fn check_reads(&self) {
        let mut last_rewrite = std::collections::HashMap::new();
        for &(node, step) in &self.rewrites {
            last_rewrite.insert(node, step);
        }
        for &(node, step) in &self.reads {
            if let Some(&rewritten) = last_rewrite.get(&node) {
                assert!(rewritten <= step, "step {step} read node {node:?}, rewritten later at step {rewritten}");
            }
        }
    }

    pub(crate) fn is_temporary(&self, node: NodeId) -> bool {
        self.lib.name(node).is_some_and(|n| self.lib.names.is_temporary(n))
    }

    /// The scopes around `scope` up to its function, for jump targets.
    pub(crate) fn enclosing(&self, scope: ScopeId) -> Vec<(ScopeKind, NodeId)> {
        self.scopes.within_function(scope).map(|s| (s.kind, s.node)).collect()
    }

    /// A pre-resolved name node for a builtin that macros use.
    pub(crate) fn builtin_name(&mut self, symbol: SymbolId, span: Span, like: NodeId) -> NodeId {
        let name = self.scopes.symbol(symbol).name;
        let node = self.lib.add_name(NodeKind::RightName, name, span, like);
        self.scopes.resolve(node, symbol);
        node
    }
}
