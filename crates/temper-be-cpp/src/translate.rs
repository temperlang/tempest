//! The translation from tmpl to C++'s output tree, for subset A, with
//! several modules, imports, and tests.
//!
//! A module is a namespace, the library's (Kotlin puts every module of a
//! library in it), with its `.cpp` and its header. Its header includes
//! the headers of the modules it imports, the library's own and other
//! libraries' (`<other/helpers.hpp>`), and its init function first calls
//! theirs, each running once. Another module's declarations of the
//! library are named as they are, in the same namespace; another
//! library's, with its namespace (`other::twice`, std's
//! `temper_std::Test`). A `var` of another module's is read where it is,
//! without its module's lock: tmpl reads one only where an import copies
//! it, in the init function, which the header's note allows.
//!
//! **Tests** are functions of the module's, `void NAME(std::shared_ptr<
//! temper_std::Test> const &test)`, as Kotlin's, but only where
//! `TEMPER_TESTS` is defined: at the end of the `.cpp`, after an include
//! of what only they import (std's `testing`), and declared in the header,
//! so the library builds and runs without std, which tmpl doesn't
//! translate yet, and a test build ([`crate::test_main`]) has them. Functions are
//! functions, `static` unless exported, which the header declares; the
//! init block is the body of the module's init function,
//! `global_init_init()`, which runs it once, through `std::call_once`, so
//! threads that call it at once wait for one run, where Kotlin's checks
//! and sets a plain `static bool`. Module `let`s and `var`s that functions
//! use, or that are exported, are namespace-scope variables, `static`
//! unless exported, which the header declares `extern`, as Kotlin's are,
//! assigned by the init function; the rest are its locals.
//!
//! **Module `var`s that functions write** are shared between threads, so
//! functions read and write them only under the module's lock, a
//! `std::mutex`, where Kotlin's are plain variables, a data race (which
//! C++ leaves undefined) when two threads call such functions at once. A
//! run of statements that read or write them and call nothing
//! ([`calls_nothing`]) runs under one lock, a `std::lock_guard` to the
//! end of its block (`ticks = ticks + 1`, or `bump`'s two updates and its
//! `return`), so concurrent updates are never lost; elsewhere a read is
//! `load(x)`, which takes the lock to copy the value, and a store of a
//! value that calls something is `store(x, value)`, computed first and
//! stored under the lock, since a call could take the lock again. The
//! init function needs no lock: it runs once, before anything else may
//! use the module, as Kotlin's requires of its callers.
//!
//! **Order of evaluation:** C++ leaves the order of a call's arguments,
//! and of most operators' operands, unspecified (g++ often goes right to
//! left), where Temper's is left to right. Where the order would show
//! ([`Effects`]: one operand writes module state or logs, and another
//! writes too, reads what may be written, or may bubble), each operand
//! before the last such one is computed first into a local named by
//! [`naming::hints`] (`int32_t int32 = bump();`), in order. Kotlin's C++
//! (at the vendored commit) leaves the order to the compiler, so
//! `module-state`'s `report()` logs `hi 1 of 3 10` under g++.
//!
//! C++ has no labeled loops: a `break` or `continue` of a loop that isn't
//! the innermost is a `goto` to a label after the loop or at the end of
//! its body, outside a block holding the body, so the jump never skips a
//! local's initialization. A self-assignment (`zero = zero`) is left out:
//! it does nothing.

use crate::NotYet;
use crate::connected::{self, Connection};
use crate::names;
use crate::ops::{self, Spelling};
use crate::tree::{self, Bin, Expr, Item, Labels, Stmt, Unary};
use std::collections::{HashMap, HashSet};
use temper_tmpl::analysis::{
    Effect, Effects, Import, calls_nothing, module_imports, module_refs, module_state,
    module_writes, test_imports,
};
use temper_tmpl::naming::{self, NameMap};
use temper_tmpl::qname::{is_external, library_of};
use temper_tmpl::{
    DeclId, DeclKind, Facts, Module, Node, NodeId, Program, QNames, Scalar, TypeData, TypeId,
};

type R<T> = Result<T, NotYet>;

fn not_yet<T>(what: impl Into<String>) -> R<T> {
    Err(NotYet { what: what.into() })
}

/// The alias of `temper::core` in a module's `.cpp`, which the naming
/// rules reserve.
const CORE: &str = "core";

/// A module translated: its `.cpp`, and what its header declares.
pub struct Translated {
    pub source: tree::Source,
    /// Exported variables, as `extern` declarations' type and name, and
    /// whether functions write them.
    pub exported_variables: Vec<(String, String, bool)>,
    /// Exported functions' prototypes, without the `;`.
    pub exported_functions: Vec<String>,
    /// The headers of the modules it imports, as `<lib/helpers.hpp>`.
    pub header_includes: Vec<String>,
    /// The headers that only its tests import.
    pub test_includes: Vec<String>,
    /// Its tests' prototypes, without the `;`.
    pub test_prototypes: Vec<String>,
}

pub struct Translator<'a> {
    p: &'a Program,
    names: &'a NameMap,
    qnames: QNames,
    effects: Effects,
    /// Module `let`s and `var`s kept at namespace scope.
    statics: HashSet<DeclId>,
    /// Those that functions write, which functions use under the lock.
    shared: HashSet<DeclId>,
    /// Every name the module's declarations take, and the reserved ones.
    taken: HashSet<String>,
    /// The module's mutex and its helpers, if used, by name.
    mutex: Option<String>,
    load: Option<String>,
    store: Option<String>,
    /// Whether translating a function, not the init block.
    in_function: bool,
    /// Whether the module's lock is held where translating.
    locked: bool,
    /// The enclosing loops, innermost last.
    loops: Vec<DeclId>,
    /// Labels printed, by loop: for a `continue` and a `break` from an
    /// inner loop, only where one needs them.
    labels: HashMap<DeclId, Labels>,
    /// Temporaries for the statement being translated, in order.
    pending: Vec<Stmt>,
    /// How many places enclose the expression being translated where a
    /// temporary can't go before its statement: the right of `&&` or
    /// `||`, a branch of `?:`, or a loop's condition.
    deferred: usize,
    /// The names in the function or init block being translated.
    locals: HashSet<String>,
    /// Each test's function's name, by its declaration.
    pub test_names: HashMap<DeclId, String>,
}

/// A part of a string concatenation, before it's written.
enum Part {
    Lit(String),
    /// A string-typed expression.
    Str(NodeId),
    /// An `Int32` or `Int64` that `toString()` converts, its receiver.
    Number(NodeId),
    /// A `Boolean` that `toString()` converts, its receiver.
    Boolean(NodeId),
}

impl<'a> Translator<'a> {
    pub fn new(p: &'a Program, names: &'a NameMap, module: &Module) -> Translator<'a> {
        let mut taken: HashSet<String> = (p.decls.ids())
            .filter(|&d| p.decls.contains(d))
            .map(|d| names.get(d).to_string())
            .filter(|n| !n.is_empty())
            .collect();
        for list in names::RULES.reserved {
            taken.extend(list.iter().map(|s| s.to_string()));
        }
        // Calls reach other modules' functions, so effects are the
        // library's: every module's functions, as one module's.
        let all = Module {
            decl: module.decl,
            path: String::new(),
            items: (p.modules.iter())
                .flat_map(|m| m.items.iter().copied())
                .collect(),
            init: module.init,
            tests: Vec::new(),
        };
        Translator {
            p,
            names,
            qnames: QNames::derive(p),
            effects: Effects::of(p, &all),
            statics: HashSet::new(),
            shared: HashSet::new(),
            taken,
            mutex: None,
            load: None,
            store: None,
            in_function: false,
            locked: false,
            loops: Vec::new(),
            labels: HashMap::new(),
            pending: Vec::new(),
            deferred: 0,
            locals: HashSet::new(),
            test_names: HashMap::new(),
        }
    }

    /// A module-level declaration: its name, or another library's,
    /// qualified by its namespace.
    fn top(&self, decl: DeclId) -> Expr {
        let name = self.name(decl);
        match external_library(self.p, decl) {
            Some(library) => Expr::name(format!("{}::{name}", names::namespace(&library))),
            None => Expr::name(name),
        }
    }

    pub(crate) fn node(&self, id: NodeId) -> Node {
        self.p.nodes.get(id)
    }

    /// A name of the runtime's, relative to `temper::core`, or of the
    /// standard library's, qualified.
    pub(crate) fn core(&self, path: &str) -> Expr {
        if path.starts_with("std::") {
            Expr::name(path)
        } else {
            Expr::name(format!("{CORE}::{path}"))
        }
    }

    fn name(&self, decl: DeclId) -> String {
        self.names.get(decl).to_string()
    }

    fn qname(&self, decl: DeclId) -> String {
        self.qnames
            .get(decl)
            .map(|q| q.to_string())
            .unwrap_or_default()
    }

    /// A type as C++ writes it in a `.cpp`, `temper::core` as `core`.
    fn ty(&self, ty: TypeId) -> R<String> {
        Ok(match self.p.types.get(ty) {
            TypeData::Scalar(s) => match s {
                Scalar::Void => "void",
                Scalar::Boolean => "bool",
                Scalar::Int32 => "int32_t",
                Scalar::Int64 => "int64_t",
                Scalar::Float64 => "double",
                Scalar::String => "std::string",
                Scalar::AnyValue => return not_yet("`AnyValue`"),
            }
            .to_string(),
            TypeData::Nominal { decl } => {
                let qname = self.qname(decl);
                match connected::lookup_type(&qname) {
                    Some(c) => c.to_string(),
                    None => match external_library(self.p, decl) {
                        // Another library's class, as Kotlin's are.
                        Some(library) if self.p.decls.get(decl).kind == DeclKind::Class => {
                            let ns = names::namespace(&library);
                            format!("std::shared_ptr<{ns}::{}>", self.name(decl))
                        }
                        _ => return not_yet(format!("a type for {qname}")),
                    },
                }
            }
            TypeData::Function { .. } => return not_yet("a function type"),
        })
    }

    fn exported(&self, decl: DeclId) -> bool {
        self.p.decls.get(decl).facts.contains(Facts::EXPORTED)
    }

    /// A name for a helper, the first of `base`, `base2`, … that nothing
    /// in the module takes.
    fn fresh_module_name(&mut self, base: &str) -> String {
        let name = (1..)
            .map(|n| match n {
                1 => base.to_string(),
                n => format!("{base}{n}"),
            })
            .find(|n| !self.taken.contains(n))
            .unwrap_or_default();
        self.taken.insert(name.clone());
        name
    }

    fn mutex(&mut self) -> String {
        if self.mutex.is_none() {
            self.mutex = Some(self.fresh_module_name("moduleMutex"));
        }
        self.mutex.clone().unwrap_or_default()
    }

    /// Translates a module.
    pub fn module(&mut self, module: &Module) -> R<Translated> {
        let p = self.p;
        for d in p.decls.ids().filter(|&d| p.decls.contains(d)) {
            let name = self.names.get(d);
            if !is_external(p, d) && names::reserved_for_implementation(name) {
                return not_yet(format!("a name C++ reserves, {name}"));
            }
        }
        let mut state = module_state(p, module);
        let mut written = HashSet::new();
        for &item in &module.items {
            if let Node::Function { .. } = p.nodes.get(item) {
                written.extend(module_writes(p, item));
            }
        }
        // Tests are functions too.
        for &test in &module.tests {
            written.extend(module_writes(p, test));
            state.used_by_functions.extend(module_refs(p, test));
        }
        let mut variables = Vec::new();
        let mut exported_variables = Vec::new();
        let mut init_locals = Vec::new();
        for &item in &module.items {
            if let Node::ModuleLet { decl } = p.nodes.get(item) {
                let d = p.decls.get(decl);
                let ty = self.ty(d.ty)?;
                let name = self.name(decl);
                if self.exported(decl) || state.used_by_functions.contains(&decl) {
                    self.statics.insert(decl);
                    if d.facts.contains(Facts::VAR) && written.contains(&decl) {
                        self.shared.insert(decl);
                    }
                    if self.exported(decl) {
                        exported_variables.push((
                            ty.replace("core::", "temper::core::"),
                            name.clone(),
                            self.shared.contains(&decl),
                        ));
                    }
                    variables.push(Item::Variable {
                        exported: self.exported(decl),
                        ty,
                        name,
                    });
                } else if !state.initialized_at_top.contains(&decl) {
                    // Assigned only inside the init block's blocks, so
                    // declared first.
                    init_locals.push(Stmt::Local {
                        ty,
                        name,
                        value: None,
                    });
                }
            }
        }
        // Functions, in source order, and the private ones called before
        // they're defined, which need prototypes.
        let mut functions = Vec::new();
        let mut exported_functions = Vec::new();
        let mut defined: HashSet<DeclId> = HashSet::new();
        let mut early: Vec<DeclId> = Vec::new();
        for &item in &module.items {
            match p.nodes.get(item) {
                Node::Function { decl, .. } => {
                    defined.insert(decl);
                    temper_tmpl::analysis::walk(p, item, &mut |id| {
                        if let Node::CallFunction { callee, .. } = p.nodes.get(id) {
                            if !is_external(p, callee)
                                && !defined.contains(&callee)
                                && !early.contains(&callee)
                            {
                                early.push(callee);
                            }
                        }
                    });
                    let f = self.function(item)?;
                    if f.exported {
                        exported_functions.push(header_signature(&f));
                    }
                    functions.push(f);
                }
                Node::BodilessFunction { decl, .. } => {
                    return not_yet(format!("a connected function, {}", p.name(decl)));
                }
                _ => {}
            }
        }
        let mut test_functions = Vec::new();
        let mut test_prototypes = Vec::new();
        for &test in &module.tests {
            let f = self.function(test)?;
            test_prototypes.push(header_signature(&f));
            test_functions.push(f);
        }
        self.in_function = false;
        self.begin(module.init);
        self.locals = self.taken.clone();
        let mut init = init_locals;
        // The modules it imports first, each once.
        let imports = module_imports(p, module);
        for import in &imports {
            let init_fn = match external_library(p, import.from) {
                Some(library) => format!(
                    "{}::{}",
                    names::namespace(&library),
                    init_name(p, import.from)
                ),
                None => init_name(p, import.from),
            };
            init.push(Stmt::Expr(Expr::name(init_fn).call(Vec::new())));
        }
        match p.nodes.get(module.init) {
            Node::Block { stmts } => {
                for &s in p.nodes.list(stmts) {
                    match p.nodes.get(s) {
                        Node::InitLet { target, value } if !self.statics.contains(&target) => {
                            let ty = self.ty(p.decls.get(target).ty)?;
                            let value = self.expr(value)?;
                            init.append(&mut self.pending);
                            init.push(Stmt::Local {
                                ty,
                                name: self.name(target),
                                value: Some(value),
                            });
                        }
                        _ => self.stmt(s, &mut init)?,
                    }
                }
            }
            _ => self.stmt(module.init, &mut init)?,
        }
        let mut items = variables;
        if let Some(mutex) = &self.mutex {
            items.push(Item::Variable {
                exported: false,
                ty: "std::mutex".into(),
                name: mutex.clone(),
            });
            if let Some(load) = &self.load {
                items.push(Item::Raw(format!(
                    "// Reads a variable that functions write, under the module's lock.\n\
                     template <class T>\n\
                     static T {load}(T const &variable) {{\n  \
                       std::lock_guard<std::mutex> lock({mutex});\n  \
                       return variable;\n\
                     }}\n"
                )));
            }
            if let Some(store) = &self.store {
                items.push(Item::Raw(format!(
                    "// Writes a variable that functions write, under the module's lock.\n\
                     template <class T, class V>\n\
                     static void {store}(T &variable, V &&value) {{\n  \
                       std::lock_guard<std::mutex> lock({mutex});\n  \
                       variable = std::forward<V>(value);\n\
                     }}\n"
                )));
            }
        }
        for decl in early {
            if let Some(f) = functions.iter().find(|f| f.name == self.name(decl)) {
                if !f.exported {
                    items.push(Item::Prototype(f.clone()));
                }
            }
        }
        items.extend(functions.into_iter().map(Item::Function));
        let header_includes = includes(p, &imports);
        let test_includes: Vec<String> = includes(p, &test_imports(p, module))
            .into_iter()
            .filter(|i| !header_includes.contains(i))
            .collect();
        Ok(Translated {
            source: tree::Source {
                includes: Vec::new(),
                namespace: String::new(),
                aliases: vec![(CORE.into(), "temper::core".into())],
                items,
                init: String::new(),
                init_body: reachable(init),
                tests: test_functions,
            },
            exported_variables,
            exported_functions,
            header_includes,
            test_includes,
            test_prototypes,
        })
    }

    fn function(&mut self, item: NodeId) -> R<tree::Function> {
        let p = self.p;
        let (decl, params, body, test) = match p.nodes.get(item) {
            Node::Function { decl, params, body } => (decl, params, body, false),
            Node::Test {
                decl, params, body, ..
            } => (decl, params, body, true),
            _ => return not_yet("a function that isn't one"),
        };
        self.locals = self.taken.clone();
        temper_tmpl::analysis::walk(p, item, &mut |id| match p.nodes.get(id) {
            Node::Param { decl: d } | Node::Let { local: d, .. } => {
                self.locals.insert(self.names.get(d).to_string());
            }
            _ => {}
        });
        let mut ps = Vec::new();
        for &param in p.nodes.list(params) {
            if let Node::Param { decl } = p.nodes.get(param) {
                let mut ty = self.ty(p.decls.get(decl).ty)?;
                let mut name = self.name(decl);
                if test {
                    // The test's `Test`, as Kotlin passes it; unnamed if
                    // unused, which `-Wunused-parameter` would say.
                    ty.push_str(" const &");
                    let mut used = false;
                    temper_tmpl::analysis::walk(p, body, &mut |id| {
                        used |= matches!(p.nodes.get(id), Node::Local { local } if local == decl);
                    });
                    if !used {
                        name.clear();
                    }
                }
                ps.push((ty, name));
            }
        }
        let result = match p.signature(decl) {
            Some(sig) => sig.result,
            None => TypeId::VOID,
        };
        let result_ty = self.ty(result)?;
        self.in_function = true;
        self.begin(item);
        let mut stmts = reachable(self.stmts(body)?);
        // A `return` without a value that ends a function says nothing.
        if result == TypeId::VOID && stmts.last() == Some(&Stmt::Return(None)) {
            stmts.pop();
        }
        let name = match self.test_names.get(&decl) {
            Some(name) if test => name.clone(),
            _ => self.name(decl),
        };
        Ok(tree::Function {
            exported: test || self.exported(decl),
            result: result_ty,
            name,
            params: ps,
            body: stmts,
        })
    }

    /// Starts translating a function or the init block: plans which loops
    /// need labels, those that a jump from an inner loop leaves or
    /// continues. A label takes its loop's source name, or for a
    /// compiler-made one, `outer`, then `End` for a `break` and `Next` for
    /// a `continue`, numbered only if another label has it.
    fn begin(&mut self, scope: NodeId) {
        fn visit(
            p: &Program,
            id: NodeId,
            loops: &mut Vec<DeclId>,
            jumps: &mut Vec<(DeclId, bool)>,
        ) {
            match p.nodes.get(id) {
                Node::While { label, .. } | Node::DoWhile { label, .. } => {
                    loops.push(label);
                    for &kid in p.nodes.kids(id) {
                        visit(p, kid, loops, jumps);
                    }
                    loops.pop();
                }
                Node::Break { label } | Node::Continue { label } => {
                    let brk = matches!(p.nodes.get(id), Node::Break { .. });
                    if loops.last() != Some(&label) && !jumps.contains(&(label, brk)) {
                        jumps.push((label, brk));
                    }
                }
                _ => {
                    for &kid in p.nodes.kids(id) {
                        visit(p, kid, loops, jumps);
                    }
                }
            }
        }
        let p = self.p;
        let mut jumps = Vec::new();
        visit(p, scope, &mut Vec::new(), &mut jumps);
        self.loops.clear();
        self.labels.clear();
        let mut taken: HashSet<String> = HashSet::new();
        for (l, brk) in jumps {
            let d = p.decls.get(l);
            let base = if d.facts.contains(Facts::COMPILER_MADE) || p.name(l).is_empty() {
                "outer".to_string()
            } else {
                p.name(l).to_string()
            };
            let base = format!("{base}{}", if brk { "End" } else { "Next" });
            let name = (1..)
                .map(|n| match n {
                    1 => base.clone(),
                    n => format!("{base}{n}"),
                })
                .find(|n| !taken.contains(n))
                .unwrap_or_default();
            taken.insert(name.clone());
            let labels = self.labels.entry(l).or_default();
            if brk {
                labels.end = Some(name);
            } else {
                labels.next = Some(name);
            }
        }
    }

    fn stmts(&mut self, id: NodeId) -> R<Vec<Stmt>> {
        let p = self.p;
        let list: Vec<NodeId> = match p.nodes.get(id) {
            Node::Block { stmts } => p.nodes.list(stmts).to_vec(),
            _ => vec![id],
        };
        let mut out = Vec::new();
        let mut i = 0;
        while i < list.len() {
            // A run of statements that use shared state and call nothing
            // runs under one lock.
            let run = if self.in_function && !self.locked {
                self.lock_run(&list[i..])
            } else {
                0
            };
            if run == 0 {
                self.stmt(list[i], &mut out)?;
                i += 1;
                continue;
            }
            let mutex = self.mutex();
            let guard = Stmt::Guard {
                name: self.fresh_local("lock"),
                mutex,
            };
            let mut locked = vec![guard];
            self.locked = true;
            for &s in &list[i..i + run] {
                self.stmt(s, &mut locked)?;
            }
            self.locked = false;
            i += run;
            if i == list.len() {
                out.append(&mut locked);
            } else {
                out.push(Stmt::Block(locked));
            }
        }
        Ok(reachable(out))
    }

    /// How many statements at the start of `list` run under one lock: a
    /// run that calls nothing, of assignments, `return`s, and, if the run
    /// reaches the end of its block, locals' declarations, at least one
    /// of which reads or writes a module `var` that functions write;
    /// none if not.
    fn lock_run(&self, list: &[NodeId]) -> usize {
        let p = self.p;
        let shared = |id: NodeId| {
            let mut found = false;
            temper_tmpl::analysis::walk(p, id, &mut |n| match p.nodes.get(n) {
                Node::GetModuleLet { target } | Node::SetModuleLet { target, .. } => {
                    found |= self.shared.contains(&target);
                }
                _ => {}
            });
            found
        };
        let mut run = 0;
        let mut uses = false;
        let mut locals = false;
        for &s in list {
            let fits = match p.nodes.get(s) {
                Node::Assign { value, .. } | Node::SetModuleLet { value, .. } => {
                    calls_nothing(p, value)
                }
                Node::Let { init, .. } => {
                    locals = true;
                    init.is_none_or(|v| calls_nothing(p, v))
                }
                Node::Return { value } => value.is_none_or(|v| calls_nothing(p, v)),
                _ => false,
            };
            if !fits {
                break;
            }
            uses |= shared(s);
            run += 1;
            if let Node::Return { .. } = p.nodes.get(s) {
                break;
            }
        }
        // Trailing statements that don't use shared state stay outside.
        while run > 0 && !shared(list[run - 1]) && run < list.len() {
            run -= 1;
        }
        if run == 0 {
            return 0;
        }
        let to_end = run == list.len() || matches!(p.nodes.get(list[run - 1]), Node::Return { .. });
        if !uses || (locals && !to_end) { 0 } else { run }
    }

    /// A name for a local the translator adds, the first of `base`,
    /// `base2`, … that nothing in its function takes.
    fn fresh_local(&mut self, base: &str) -> String {
        let name = (1..)
            .map(|n| match n {
                1 => base.to_string(),
                n => format!("{base}{n}"),
            })
            .find(|n| !self.locals.contains(n))
            .unwrap_or_default();
        self.locals.insert(name.clone());
        name
    }

    fn loop_body(&mut self, label: DeclId, body: NodeId) -> R<Vec<Stmt>> {
        self.loops.push(label);
        let body = self.stmts(body);
        self.loops.pop();
        body
    }

    /// A condition evaluated again and again, where no temporary can go.
    fn condition(&mut self, cond: NodeId) -> R<Expr> {
        self.deferred += 1;
        let cond = self.expr(cond);
        self.deferred -= 1;
        cond
    }

    fn stmt(&mut self, id: NodeId, out: &mut Vec<Stmt>) -> R<()> {
        let p = self.p;
        let s = match p.nodes.get(id) {
            Node::Block { .. } => Stmt::Block(self.stmts(id)?),
            Node::Let { local, init } => {
                let ty = self.ty(p.decls.get(local).ty)?;
                let value = init.map(|v| self.expr(v)).transpose()?;
                Stmt::Local {
                    ty,
                    name: self.name(local),
                    value,
                }
            }
            Node::SetModuleLet { target, value }
                if self.in_function && !self.locked && self.shared.contains(&target) =>
            {
                let value = self.expr(value)?;
                if self.store.is_none() {
                    self.mutex();
                    self.store = Some(self.fresh_module_name("store"));
                }
                let store = self.store.clone().unwrap_or_default();
                Stmt::Expr(Expr::name(store).call(vec![Expr::name(self.name(target)), value]))
            }
            Node::Assign {
                local: target,
                value,
            }
            | Node::SetModuleLet { target, value }
            | Node::InitLet { target, value } => {
                let value = self.expr(value)?;
                let target = self.name(target);
                // `x = x` does nothing.
                if value == Expr::Name(target.clone()) {
                    out.append(&mut self.pending);
                    return Ok(());
                }
                Stmt::Assign { target, value }
            }
            Node::If { cond, then, els } => {
                let cond = self.expr(cond)?;
                let hoisted = std::mem::take(&mut self.pending);
                let then = self.stmts(then)?;
                let els = match els {
                    Some(els) => self.stmts(els)?,
                    None => Vec::new(),
                };
                self.pending = hoisted;
                Stmt::If { cond, then, els }
            }
            Node::While { label, cond, body } => {
                let cond = self.condition(cond)?;
                let body = self.loop_body(label, body)?;
                Stmt::While {
                    cond,
                    body,
                    labels: self.labels.get(&label).cloned().unwrap_or_default(),
                }
            }
            Node::DoWhile { label, body, cond } => {
                let body = self.loop_body(label, body)?;
                let cond = self.condition(cond)?;
                Stmt::DoWhile {
                    body,
                    cond,
                    labels: self.labels.get(&label).cloned().unwrap_or_default(),
                }
            }
            Node::Break { label } | Node::Continue { label } => {
                let brk = matches!(p.nodes.get(id), Node::Break { .. });
                if self.loops.last() == Some(&label) {
                    if brk { Stmt::Break } else { Stmt::Continue }
                } else {
                    let labels = self.labels.get(&label);
                    let target = if brk {
                        labels.and_then(|l| l.end.clone())
                    } else {
                        labels.and_then(|l| l.next.clone())
                    };
                    match target {
                        Some(l) => Stmt::Goto(l),
                        None => return not_yet("a jump to a loop that doesn't enclose it"),
                    }
                }
            }
            Node::Return { value } => match value.map(|v| (v, p.nodes.get(v))) {
                None | Some((_, Node::VoidLit {})) => Stmt::Return(None),
                Some((_, Node::Error { .. })) => Stmt::Bubble(self.core("bubble").call(Vec::new())),
                Some((v, _)) => Stmt::Return(Some(self.expr(v)?)),
            },
            Node::ExprStatement { expr } => {
                if let Node::Error { .. } = p.nodes.get(expr) {
                    Stmt::Bubble(self.core("bubble").call(Vec::new()))
                } else {
                    let e = self.expr(expr)?;
                    if !e.is_statement() {
                        return not_yet("an expression statement that isn't a call");
                    }
                    Stmt::Expr(e)
                }
            }
            other => return not_yet(format!("a {} statement", other.kind().name())),
        };
        out.append(&mut self.pending);
        out.push(s);
        Ok(())
    }

    /// A call of a connected declaration, by the connection table; `args`
    /// has a method's receiver first.
    fn connected(&mut self, decl: DeclId, args: &[NodeId]) -> R<Expr> {
        let qname = self.qname(decl);
        let Some(connection) = connected::lookup(&qname) else {
            return not_yet(format!("a connection for {qname}"));
        };
        match connection {
            Connection::Call(f) => {
                let args = self.args(args)?;
                Ok(self.core(f).call(args))
            }
            Connection::Translate(f) => f(self, args),
        }
    }

    /// Arguments, those left out at the end dropped, as C++'s default
    /// arguments and overloads leave them out, in Temper's order.
    fn args(&mut self, args: &[NodeId]) -> R<Vec<Expr>> {
        let p = self.p;
        let given = args
            .iter()
            .rposition(|&a| !matches!(p.nodes.get(a), Node::Omitted {}))
            .map_or(0, |i| i + 1);
        if args[..given]
            .iter()
            .any(|&a| matches!(p.nodes.get(a), Node::Omitted {}))
        {
            return not_yet("a default parameter before a given one");
        }
        self.operands(&args[..given])
    }

    /// Operands, each before the last whose order shows computed first
    /// into a temporary, in order, since C++ evaluates them in any order.
    pub(crate) fn operands(&mut self, ids: &[NodeId]) -> R<Vec<Expr>> {
        let effects: Vec<Effect> = ids
            .iter()
            .map(|&id| self.effects.node(self.p, id))
            .collect();
        let mut out = Vec::new();
        for (i, &id) in ids.iter().enumerate() {
            let hoist = effects[i + 1..]
                .iter()
                .any(|&later| effects[i].conflicts(later));
            let e = self.expr(id)?;
            out.push(if hoist { self.temporary(id, e)? } else { e });
        }
        Ok(out)
    }

    /// Computes an expression first, into a local named by its hints, and
    /// gives the local.
    fn temporary(&mut self, id: NodeId, value: Expr) -> R<Expr> {
        if self.deferred > 0 {
            return not_yet("an operand whose order needs a temporary where none can go yet");
        }
        let ty = self.ty(self.p.nodes.ty(id))?;
        let mut name = None;
        for hint in naming::hints(self.p, id) {
            if !self.locals.contains(&hint) {
                name = Some(hint);
                break;
            }
        }
        let name = match name {
            Some(n) => {
                self.locals.insert(n.clone());
                n
            }
            None => self.fresh_local("value"),
        };
        self.pending.push(Stmt::Local {
            ty,
            name: name.clone(),
            value: Some(value),
        });
        Ok(Expr::Name(name))
    }

    /// A read of a module `let`: under the lock, in a function, where
    /// functions write it.
    fn read_module(&mut self, target: DeclId) -> Expr {
        let name = self.top(target);
        if !self.in_function || self.locked || !self.shared.contains(&target) {
            return name;
        }
        if self.load.is_none() {
            self.mutex();
            self.load = Some(self.fresh_module_name("load"));
        }
        Expr::name(self.load.clone().unwrap_or_default()).call(vec![name])
    }

    pub(crate) fn expr(&mut self, id: NodeId) -> R<Expr> {
        let p = self.p;
        Ok(match p.nodes.get(id) {
            Node::BoolLit { value } => Expr::Bool(value),
            Node::Int32Lit { value } => Expr::Int(value),
            Node::Int64Lit { value } => Expr::Long(value),
            Node::Float64Lit { value } => Expr::Float(value),
            Node::StringLit { value } => Expr::Str(p.strings.get(value).to_string()),
            Node::Local { local: decl } => Expr::name(self.name(decl)),
            Node::GetModuleLet { target } => self.read_module(target),
            Node::CallFunction { callee, args, .. } => {
                let args = p.nodes.list(args);
                if is_external(p, callee) && library_of(p, callee) == "core" {
                    return self.connected(callee, args);
                }
                let args = self.args(args)?;
                self.top(callee).call(args)
            }
            Node::CallMethod {
                method,
                receiver,
                args,
                ..
            } => {
                if !is_external(p, method) {
                    return not_yet("a method of the program's own");
                }
                let mut all = vec![receiver];
                all.extend_from_slice(p.nodes.list(args));
                self.connected(method, &all)?
            }
            Node::Op { op, args, .. } => {
                let Some(spelling) = ops::spelling(op) else {
                    return not_yet("an operator without a spelling");
                };
                if spelling == Spelling::Cat {
                    return self.cat(id);
                }
                let arg_ids = p.nodes.list(args);
                // A negated literal is a literal, wrapped as the type
                // wraps (`-(-2147483648)` is itself).
                if let [arg] = arg_ids {
                    let neg = matches!(
                        spelling,
                        Spelling::Unary(Unary::Neg) | Spelling::Call("Int::neg" | "Int64::neg")
                    );
                    match (neg, p.nodes.get(*arg)) {
                        (true, Node::Int32Lit { value }) => {
                            return Ok(Expr::Int(value.wrapping_neg()));
                        }
                        (true, Node::Int64Lit { value }) => {
                            return Ok(Expr::Long(value.wrapping_neg()));
                        }
                        (true, Node::Float64Lit { value }) => return Ok(Expr::Float(-value)),
                        _ => {}
                    }
                }
                let literal = |i: usize| match arg_ids.get(i).map(|&a| p.nodes.get(a)) {
                    Some(Node::Int32Lit { value }) => Some(i64::from(value)),
                    Some(Node::Int64Lit { value }) => Some(value),
                    _ => None,
                };
                let strings = arg_ids.iter().any(|&a| p.nodes.ty(a) == TypeId::STRING);
                let mut args = self.operands(arg_ids)?;
                match spelling {
                    Spelling::Bin(op) => {
                        let b = args.pop().unwrap_or(Expr::Bool(false));
                        let mut a = args.pop().unwrap_or(Expr::Bool(false));
                        // Two literals would compare pointers.
                        if strings && a.is_c_string() && b.is_c_string() {
                            a = a.std_string();
                        }
                        Expr::bin(op, a, b)
                    }
                    Spelling::Unary(op) => {
                        let a = args.pop().unwrap_or(Expr::Bool(false));
                        match (op, a) {
                            // `!(a < b)` reads better as `a >= b`; no
                            // comparison here has a NaN operand (floats'
                            // compare through the runtime).
                            (Unary::Not, Expr::Bin(cmp, x, y)) if cmp.negated().is_some() => {
                                Expr::Bin(cmp.negated().unwrap_or(cmp), x, y)
                            }
                            (op, a) => Expr::unary(op, a),
                        }
                    }
                    Spelling::Call(f) => {
                        if strings && f == "Compare::cmp" && args.iter().any(Expr::is_c_string) {
                            // A literal would deduce `const char*`.
                            self.core("Compare::cmp<std::string>").call(args)
                        } else {
                            self.core(f).call(args)
                        }
                    }
                    Spelling::Divide(op, f) => match literal(1) {
                        Some(d) if d != 0 && d != -1 => {
                            let b = args.pop().unwrap_or(Expr::Int(1));
                            let a = args.pop().unwrap_or(Expr::Int(1));
                            Expr::bin(op, a, b)
                        }
                        _ => self.core(f).call(args),
                    },
                    Spelling::ShiftRight(f) => {
                        let width = if f.starts_with("Int64") { 64 } else { 32 };
                        match literal(1) {
                            Some(n) if (0..width).contains(&n) => {
                                let b = args.pop().unwrap_or(Expr::Int(0));
                                let a = args.pop().unwrap_or(Expr::Int(0));
                                Expr::bin(Bin::Shr, a, b)
                            }
                            _ => self.core(f).call(args),
                        }
                    }
                    Spelling::Cat => return self.cat(id),
                }
            }
            Node::And { left, right } => {
                let left = self.expr(left)?;
                self.deferred += 1;
                let right = self.expr(right);
                self.deferred -= 1;
                Expr::bin(Bin::And, left, right?)
            }
            Node::Or { left, right } => {
                let left = self.expr(left)?;
                self.deferred += 1;
                let right = self.expr(right);
                self.deferred -= 1;
                Expr::bin(Bin::Or, left, right?)
            }
            Node::IfValue { cond, then, els } => {
                let cond = self.expr(cond)?;
                self.deferred += 1;
                let branches = self.expr(then).and_then(|t| Ok((t, self.expr(els)?)));
                self.deferred -= 1;
                let (then, els) = branches?;
                Expr::Cond {
                    cond: Box::new(cond),
                    then: Box::new(then),
                    els: Box::new(els),
                }
            }
            Node::Error { .. } => {
                let ty = self.ty(p.nodes.ty(id))?;
                self.core(&format!("bubble<{ty}>")).call(Vec::new())
            }
            other => return not_yet(format!("a {} expression", other.kind().name())),
        })
    }

    /// A chain of `String.cat` as one concatenation: literals merged; an
    /// `Int32.toString()` without a radix as `std::to_string`, and a
    /// `Boolean.toString()` as the runtime's; `+` between them, with the
    /// first a `std::string` where the second is a literal too.
    fn cat(&mut self, id: NodeId) -> R<Expr> {
        let mut parts = Vec::new();
        self.cat_parts(id, &mut parts);
        parts.retain(|p| !matches!(p, Part::Lit(s) if s.is_empty()));
        // What each part evaluates, for its order.
        let nodes: Vec<Option<NodeId>> = parts
            .iter()
            .map(|p| match p {
                Part::Lit(_) => None,
                Part::Str(n) | Part::Number(n) | Part::Boolean(n) => Some(*n),
            })
            .collect();
        let effects: Vec<Effect> = (nodes.iter())
            .map(|n| n.map_or(Effect::default(), |n| self.effects.node(self.p, n)))
            .collect();
        let mut out = Vec::new();
        for (i, part) in parts.into_iter().enumerate() {
            let hoist = effects[i + 1..]
                .iter()
                .any(|&later| effects[i].conflicts(later));
            let (n, convert) = match part {
                Part::Lit(s) => {
                    out.push(Expr::Str(s));
                    continue;
                }
                Part::Str(n) => (n, None),
                Part::Number(n) => (n, Some("std::to_string")),
                Part::Boolean(n) => (n, Some("core::Boolean::toString")),
            };
            let e = self.expr(n)?;
            let e = if hoist { self.temporary(n, e)? } else { e };
            out.push(match convert {
                Some(f) => Expr::name(f).call(vec![e]),
                None => e,
            });
        }
        if out.is_empty() {
            return Ok(Expr::Str(String::new()));
        }
        if out.len() == 1 {
            return Ok(out.pop().unwrap_or(Expr::Str(String::new())));
        }
        if out[0].is_c_string() && out[1].is_c_string() {
            out[0] = out[0].clone().std_string();
        }
        Ok(Expr::Concat(out))
    }

    fn cat_parts(&mut self, id: NodeId, parts: &mut Vec<Part>) {
        let p = self.p;
        match p.nodes.get(id) {
            Node::Op { op, args, .. } if ops::spelling(op) == Some(Spelling::Cat) => {
                for &a in p.nodes.list(args) {
                    self.cat_parts(a, parts);
                }
            }
            Node::StringLit { value } => {
                let text = p.strings.get(value);
                if let Some(Part::Lit(last)) = parts.last_mut() {
                    last.push_str(text);
                } else {
                    parts.push(Part::Lit(text.to_string()));
                }
            }
            Node::CallMethod {
                method,
                receiver,
                args,
                ..
            } if p
                .nodes
                .list(args)
                .iter()
                .all(|&a| matches!(p.nodes.get(a), Node::Omitted {})) =>
            {
                match self.qname(method).as_str() {
                    "core.type Int32.toString()" | "core.type Int64.toString()" => {
                        parts.push(Part::Number(receiver));
                    }
                    "core.type Boolean.toString()" => parts.push(Part::Boolean(receiver)),
                    _ => parts.push(Part::Str(id)),
                }
            }
            _ => parts.push(Part::Str(id)),
        }
    }
}

/// An exported function's prototype for the header, `temper::core`
/// spelled out.
/// The library a declaration of another library's is from, as its
/// `uses` section names it; `None` for the program's own.
pub(crate) fn external_library(p: &Program, decl: DeclId) -> Option<String> {
    let d = p.decls.get(decl);
    let of_library = d.kind == DeclKind::Library
        || (d.kind == DeclKind::Module && !d.owner.is_none())
        || is_external(p, decl);
    if !of_library {
        return None;
    }
    let mut d = decl;
    for _ in 0..p.decls.len() {
        let entry = p.decls.get(d);
        if entry.kind == DeclKind::Library {
            return Some(p.name(d).to_string());
        }
        d = entry.owner;
        if !p.decls.contains(d) {
            break;
        }
    }
    None
}

/// A module's path in its library, empty at the root: the program's own
/// or another library's (a `Library` itself is its root).
pub(crate) fn module_path(p: &Program, module: DeclId) -> String {
    let d = p.decls.get(module);
    if d.kind == DeclKind::Library {
        return String::new();
    }
    if !d.owner.is_none() {
        return p.name(module).to_string();
    }
    (p.modules.iter())
        .find(|m| m.decl == module)
        .map(|m| m.path.clone())
        .unwrap_or_default()
}

/// A module's file, without `.hpp` or `.cpp`, in its library's
/// directory: `init` at the root, else its path, as Kotlin's.
pub(crate) fn module_stem(p: &Program, module: DeclId) -> String {
    match module_path(p, module).as_str() {
        "" => "init".into(),
        path => path.into(),
    }
}

/// A module's init function, as Kotlin names it: `global_init_` and its
/// path's last part, `init` at the root.
pub(crate) fn init_name(p: &Program, module: DeclId) -> String {
    let stem = module_stem(p, module);
    let leaf = stem.rsplit('/').next().unwrap_or(&stem);
    format!(
        "global_init_{}",
        names::namespace(leaf).trim_start_matches("temper_")
    )
}

/// The headers of the modules in `imports`, as `<lib/path.hpp>`.
fn includes(p: &Program, imports: &[Import]) -> Vec<String> {
    (imports.iter())
        .map(|import| {
            let library = match external_library(p, import.from) {
                Some(library) => library,
                None => p.library.name.clone(),
            };
            format!("<{library}/{}.hpp>", module_stem(p, import.from))
        })
        .collect()
}

fn header_signature(f: &tree::Function) -> String {
    f.signature().replace("core::", "temper::core::")
}

/// Statements up to the first that can't complete normally, and the
/// statements after a bubble, which never returns.
fn reachable(mut stmts: Vec<Stmt>) -> Vec<Stmt> {
    if let Some(i) = stmts.iter().position(|s| !completes(s)) {
        stmts.truncate(i + 1);
    }
    stmts
}

/// Whether a statement can complete normally.
fn completes(s: &Stmt) -> bool {
    match s {
        Stmt::Break | Stmt::Continue | Stmt::Goto(_) | Stmt::Return(_) | Stmt::Bubble(_) => false,
        Stmt::Block(body) => body.iter().all(completes),
        Stmt::If { then, els, .. } => {
            els.is_empty() || then.iter().all(completes) || els.iter().all(completes)
        }
        // A label after a loop is reached by a `goto` from inside it.
        Stmt::While { cond, body, labels } => {
            *cond != Expr::Bool(true) || breaks(body) || labels.end.is_some()
        }
        Stmt::DoWhile { body, cond, labels } => {
            breaks(body)
                || labels.end.is_some()
                || (*cond != Expr::Bool(true)
                    && (body.iter().all(completes) || continues(body) || labels.next.is_some()))
        }
        Stmt::Expr(_) | Stmt::Local { .. } | Stmt::Assign { .. } | Stmt::Guard { .. } => true,
    }
}

/// Whether statements hold a `break` of the innermost loop.
fn breaks(body: &[Stmt]) -> bool {
    jumps(body, true)
}

fn continues(body: &[Stmt]) -> bool {
    jumps(body, false)
}

fn jumps(body: &[Stmt], brk: bool) -> bool {
    body.iter().any(|s| match s {
        Stmt::Break => brk,
        Stmt::Continue => !brk,
        Stmt::Block(b) => jumps(b, brk),
        Stmt::If { then, els, .. } => jumps(then, brk) || jumps(els, brk),
        _ => false,
    })
}
