//! A property test: random valid programs, from a small deterministic
//! generator, validate without problems, write as text that reads back to
//! an equal program, and write again the same. Programs use core's
//! members through a `uses` section (classes, an interface, methods, a
//! getter, a static function, and optional parameters), call them with
//! `CallMethod` and `Omitted`, log through a compiler-made `~console`, and
//! carry qname overrides, module paths, and connected and `@keep` marks.
//! Set `TMPL_FUZZ_SEED` and `TMPL_FUZZ_ROUNDS` to vary it.

use std::env;
use temper_tmpl::ops::{OPS, Op};
use temper_tmpl::qname::QNames;
use temper_tmpl::{
    Builder, DeclId, DeclKind, Facts, NodeId, Profile, Program, Span, TypeId, difference, text,
    validate,
};

const TYPES: [TypeId; 5] = [
    TypeId::BOOLEAN,
    TypeId::INT32,
    TypeId::INT64,
    TypeId::FLOAT64,
    TypeId::STRING,
];
const NAMES: [&str; 9] = ["a", "x", "count", "for", "if", "odd name", "é", "a`b", "_9"];

/// A small xorshift generator, so runs repeat for a given seed.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n.max(1) as u64) as usize
    }

    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }
}

/// A value in scope: a local, parameter, or module `let`.
#[derive(Clone, Copy)]
struct Value {
    decl: DeclId,
    ty: TypeId,
    var: bool,
    module_let: bool,
}

/// Something callable: a function, or with a receiver, a method.
#[derive(Clone)]
struct Callee {
    decl: DeclId,
    /// A method's receiver type; `None` for a function.
    receiver: Option<TypeId>,
    result: TypeId,
    bubbles: bool,
    params: Vec<TypeId>,
    optional: Vec<bool>,
}

/// Core's members the programs use.
struct Core {
    console: TypeId,
    log: DeclId,
    get_console: DeclId,
}

struct Gen {
    rng: Rng,
    b: Builder,
    functions: Vec<Callee>,
    core: Option<Core>,
    /// The module's `~console`.
    console: DeclId,
    /// Counts qname overrides, so each is unique.
    overrides: usize,
    /// Values in scope, innermost last.
    scope: Vec<Value>,
    /// Enclosing loops' labels.
    loops: Vec<DeclId>,
    owner: DeclId,
    may_bubble: bool,
    /// std's `testing` module's `Test`, once a test needs it.
    test_type: Option<TypeId>,
}

impl Gen {
    fn name(&mut self) -> &'static str {
        self.rng.pick(&NAMES)
    }

    fn declare(&mut self, kind: DeclKind, owner: DeclId, ty: TypeId) -> DeclId {
        if self.rng.chance(15) {
            let hints: Vec<&str> = (0..1 + self.rng.below(3)).map(|_| self.name()).collect();
            self.b.declare_made(kind, &hints, owner, ty)
        } else {
            let name = self.name();
            self.b.declare(kind, name, owner, ty)
        }
    }

    /// Sometimes overrides a declaration's qname, uniquely.
    fn qname(&mut self, decl: DeclId, suffix: &str) {
        if self.rng.chance(10) {
            self.overrides += 1;
            let tail = format!("over{}{suffix}", self.overrides);
            self.b.set_qname(decl, &tail);
        }
    }

    /// A `uses` section for core: the scalars' classes with some members,
    /// and `Console`.
    fn core(&mut self) {
        let b = &mut self.b;
        let core = b.uses("temper-core", "0.1.0");
        let member = |b: &mut Builder,
                      kind: DeclKind,
                      name: &str,
                      owner: DeclId,
                      params: &[(&str, TypeId, bool)],
                      result: TypeId,
                      bubbles: bool| {
            let types: Vec<TypeId> = params.iter().map(|p| p.1).collect();
            let optional: Vec<bool> = params.iter().map(|p| p.2).collect();
            let sig = b.function_type_with(&types, &optional, result, bubbles);
            let decl = b.declare(kind, name, owner, sig);
            b.set_connected(decl, &[]);
            let names: Vec<&str> = params.iter().map(|p| p.0).collect();
            b.set_param_names(decl, &names);
            let receiver = (kind != DeclKind::Function).then(|| b.program().decls.get(owner).ty);
            Callee {
                decl,
                receiver,
                result,
                bubbles,
                params: types,
                optional,
            }
        };
        let int32 = b.declare_type(DeclKind::Class, "Int32", core);
        let string = b.declare_type(DeclKind::Class, "String", core);
        let float64 = b.declare_type(DeclKind::Class, "Float64", core);
        let (i, s, f) = (TypeId::INT32, TypeId::STRING, TypeId::FLOAT64);
        let mut callees = vec![
            member(
                b,
                DeclKind::Method,
                "toString",
                int32,
                &[("radix", i, true)],
                s,
                false,
            ),
            member(
                b,
                DeclKind::Method,
                "max",
                int32,
                &[("other", i, false)],
                i,
                false,
            ),
            member(
                b,
                DeclKind::Function,
                "fromCodePoint",
                string,
                &[("codePoint", i, false)],
                s,
                true,
            ),
            member(
                b,
                DeclKind::Getter,
                "isEmpty",
                string,
                &[],
                TypeId::BOOLEAN,
                false,
            ),
            member(
                b,
                DeclKind::Method,
                "toInt32",
                string,
                &[("radix", i, true)],
                i,
                true,
            ),
            member(b, DeclKind::Method, "sqrt", float64, &[], f, false),
        ];
        let console = b.declare_type(DeclKind::Interface, "Console", core);
        let console_ty = b.program().decls.get(console).ty;
        let log = member(
            b,
            DeclKind::Method,
            "log",
            console,
            &[("message", s, false)],
            TypeId::VOID,
            false,
        );
        let get_console = member(
            b,
            DeclKind::Function,
            "getConsole",
            core,
            &[("name", s, true)],
            console_ty,
            false,
        );
        if self.rng.chance(30) {
            self.b
                .set_doc(get_console.decl, "Access or make a console.");
        }
        callees.push(log.clone());
        self.functions.extend(callees);
        self.core = Some(Core {
            console: console_ty,
            log: log.decl,
            get_console: get_console.decl,
        });
    }

    fn program(&mut self) -> Program {
        self.core();
        for _ in 0..1 + self.rng.below(2) {
            let module = self.b.module("m");
            if self.rng.chance(50) {
                let path = format!("p/{}", self.rng.below(100));
                self.b.set_module_path(module, &path);
            }
            // An earlier module's functions are imported only if exported.
            let p = self.b.program();
            let keep: Vec<bool> = (self.functions.iter())
                .map(|f| {
                    let d = p.decls.get(f.decl);
                    temper_tmpl::qname::is_external(p, f.decl) || d.facts.contains(Facts::EXPORTED)
                })
                .collect();
            let mut keep = keep.into_iter();
            self.functions.retain(|_| keep.next().unwrap());
            self.module(module);
        }
        let mut b = std::mem::replace(&mut self.b, Builder::new("", ""));
        if self.rng.chance(50) {
            add_spans(&mut b);
        }
        b.finish()
    }

    fn module(&mut self, module: DeclId) {
        // A compiler-made `~console`, as the frontend makes, first.
        let core = self.core.as_ref().unwrap();
        let (console_ty, get_console) = (core.console, core.get_console);
        self.console = self
            .b
            .declare_made(DeclKind::ModuleLet, &["console"], module, console_ty);
        let node = self.b.module_let(self.console);
        self.b.add_item(module, node);
        let mut lets = Vec::new();
        for _ in 0..self.rng.below(3) {
            let ty = self.rng.pick(&TYPES);
            let decl = self.declare(DeclKind::ModuleLet, module, ty);
            let var = self.rng.chance(50);
            if var {
                self.b.add_facts(decl, Facts::VAR);
            }
            self.facts(decl);
            self.qname(decl, "");
            let node = self.b.module_let(decl);
            self.b.add_item(module, node);
            lets.push(Value {
                decl,
                ty,
                var,
                module_let: true,
            });
        }
        for _ in 0..self.rng.below(4) {
            self.function(module, &lets);
        }
        // Init: each module `let` initialized in order, then statements.
        self.owner = module;
        self.may_bubble = true;
        self.scope.clear();
        let name = if self.rng.chance(50) {
            self.b.omitted(TypeId::STRING)
        } else {
            self.b.string("m")
        };
        let console = self.b.call(get_console, &[name]);
        let mut stmts = vec![self.b.init_let(self.console, console)];
        for value in lets {
            let init = self.expr(value.ty, 2);
            stmts.push(self.b.init_let(value.decl, init));
            self.scope.push(value);
        }
        for _ in 0..self.rng.below(3) {
            stmts.push(self.stmt(2));
        }
        let init = self.b.block(&stmts);
        self.b.set_init(module, init);
        for _ in 0..self.rng.below(3) {
            self.test(module, &lets_in_scope(&self.scope));
        }
    }

    /// A test: a compiler-made function of std's `Test`, which may bubble.
    fn test(&mut self, module: DeclId, lets: &[Value]) {
        let test_type = match self.test_type {
            Some(t) => t,
            None => {
                let std = self.b.uses("std", "0.6.0");
                let testing = self.b.external_module(std, "testing");
                let class = self.b.declare_type(DeclKind::Class, "Test", testing);
                let t = self.b.nominal(class);
                self.test_type = Some(t);
                t
            }
        };
        let sig = self.b.function_type(&[test_type], TypeId::VOID, true);
        let decl = self
            .b
            .declare_made(DeclKind::Function, &["test"], module, sig);
        let param = self
            .b
            .declare_made(DeclKind::Param, &["test"], decl, test_type);
        let param = self.b.param(param);
        self.owner = decl;
        self.may_bubble = true;
        self.scope = lets.to_vec();
        let stmts: Vec<NodeId> = (0..self.rng.below(3)).map(|_| self.stmt(2)).collect();
        let body = self.b.block(&stmts);
        let name = self.rng.pick(&["works", "names: 'quoted'", "ünïcode", ""]);
        let node = self.b.test(decl, name, &[param], body);
        self.b.add_test(module, node);
    }

    fn facts(&mut self, decl: DeclId) {
        if self.rng.chance(30) {
            self.b.add_facts(decl, Facts::EXPORTED);
        }
        if self.rng.chance(10) {
            self.b.add_facts(decl, Facts::KEEP);
        }
        if self.rng.chance(20) {
            let doc = self
                .rng
                .pick(&["Docs.", "Two\n\nparagraphs.", " indented", ""]);
            self.b.set_doc(decl, doc);
        }
    }

    fn function(&mut self, module: DeclId, lets: &[Value]) {
        let params: Vec<TypeId> = (0..self.rng.below(3))
            .map(|_| self.rng.pick(&TYPES))
            .collect();
        let optional: Vec<bool> = params.iter().map(|_| self.rng.chance(30)).collect();
        let result = if self.rng.chance(30) {
            TypeId::VOID
        } else {
            self.rng.pick(&TYPES)
        };
        let bubbles = self.rng.chance(30);
        let sig = self
            .b
            .function_type_with(&params, &optional, result, bubbles);
        let decl = self.declare(DeclKind::Function, module, sig);
        self.facts(decl);
        self.qname(decl, "()");
        // Connected: with no body (host code everywhere), or with a body,
        // its fallback, maybe for backends without host code.
        let connected = self.rng.chance(20);
        let bodiless = connected && self.rng.chance(50);
        if connected {
            let backends: &[&str] = if !bodiless && self.rng.chance(50) {
                &["cpp", "rust"]
            } else {
                &[]
            };
            self.b.set_connected(decl, backends);
        }
        self.functions.push(Callee {
            decl,
            receiver: None,
            result,
            bubbles,
            params: params.clone(),
            optional: optional.clone(),
        });
        self.owner = decl;
        self.may_bubble = bubbles;
        self.scope = lets.to_vec();
        let mut param_nodes = Vec::new();
        for (ty, optional) in params.into_iter().zip(optional) {
            let param = self.declare(DeclKind::Param, decl, ty);
            if optional {
                self.b.add_facts(param, Facts::OPTIONAL);
            }
            self.qname(param, "");
            param_nodes.push(self.b.param(param));
            self.scope.push(Value {
                decl: param,
                ty,
                var: false,
                module_let: false,
            });
        }
        if bodiless {
            let node = self.b.bodiless_function(decl, &param_nodes);
            self.b.add_item(module, node);
            return;
        }
        let mut stmts: Vec<NodeId> = (0..self.rng.below(4)).map(|_| self.stmt(3)).collect();
        if result != TypeId::VOID || self.rng.chance(30) {
            let value = (result != TypeId::VOID).then(|| self.expr(result, 2));
            stmts.push(self.b.return_stmt(value));
        }
        let body = self.b.block(&stmts);
        let node = self.b.function(decl, &param_nodes, body);
        self.b.add_item(module, node);
    }

    fn block(&mut self, depth: usize) -> NodeId {
        let mark = self.scope.len();
        let stmts: Vec<NodeId> = (0..self.rng.below(3)).map(|_| self.stmt(depth)).collect();
        self.scope.truncate(mark);
        self.b.block(&stmts)
    }

    fn stmt(&mut self, depth: usize) -> NodeId {
        let choice = if depth == 0 { 0 } else { self.rng.below(9) };
        match choice {
            1 => {
                let ty = self.rng.pick(&TYPES);
                let init = self.expr(ty, depth - 1);
                let local = self.declare(DeclKind::Local, self.owner, ty);
                self.qname(local, "=");
                let var = self.rng.chance(50);
                if var {
                    self.b.add_facts(local, Facts::VAR);
                }
                self.scope.push(Value {
                    decl: local,
                    ty,
                    var,
                    module_let: false,
                });
                self.b.let_stmt(local, Some(init))
            }
            2 => {
                let vars: Vec<Value> = self.scope.iter().copied().filter(|v| v.var).collect();
                if vars.is_empty() {
                    return self.stmt(0);
                }
                let target = self.rng.pick(&vars);
                let value = self.expr(target.ty, depth - 1);
                if target.module_let {
                    self.b.set_module_let(target.decl, value)
                } else {
                    self.b.assign(target.decl, value)
                }
            }
            3 => {
                let cond = self.expr(TypeId::BOOLEAN, depth - 1);
                let then = self.block(depth - 1);
                let els = if self.rng.chance(50) {
                    Some(self.block(depth - 1))
                } else {
                    None
                };
                self.b.if_stmt(cond, then, els)
            }
            4 | 5 => {
                let label = self.declare(DeclKind::Label, self.owner, TypeId::NONE);
                self.loops.push(label);
                let cond = self.expr(TypeId::BOOLEAN, depth - 1);
                let body = self.block(depth - 1);
                self.loops.pop();
                if choice == 4 {
                    self.b.while_loop(label, cond, body)
                } else {
                    self.b.do_while(label, body, cond)
                }
            }
            6 if !self.loops.is_empty() => {
                let label = self.rng.pick(&self.loops.clone());
                if self.rng.chance(50) {
                    self.b.break_loop(label)
                } else {
                    self.b.continue_loop(label)
                }
            }
            7 => self.block(depth - 1),
            _ => {
                let text = self.expr(TypeId::STRING, depth.saturating_sub(1));
                let console = self.b.get_module_let(self.console);
                let log = self.core.as_ref().unwrap().log;
                let log = self.b.call_method(log, console, &[text]);
                self.b.expr_statement(log)
            }
        }
    }

    fn expr(&mut self, ty: TypeId, depth: usize) -> NodeId {
        let choice = if depth == 0 {
            self.rng.below(2)
        } else {
            self.rng.below(9)
        };
        match choice {
            1 => {
                let values: Vec<Value> =
                    self.scope.iter().copied().filter(|v| v.ty == ty).collect();
                if values.is_empty() {
                    return self.literal(ty);
                }
                let value = self.rng.pick(&values);
                if value.module_let {
                    self.b.get_module_let(value.decl)
                } else {
                    self.b.local(value.decl)
                }
            }
            2 | 3 => {
                let may_bubble = self.may_bubble;
                let ops: Vec<usize> = (0..OPS.len())
                    .filter(|&i| OPS[i].result == ty && (may_bubble || !OPS[i].bubbles))
                    .collect();
                if ops.is_empty() {
                    return self.literal(ty);
                }
                let op = Op(self.rng.pick(&ops) as u32);
                let params = op.info().unwrap().params;
                let args: Vec<NodeId> = params.iter().map(|&p| self.expr(p, depth - 1)).collect();
                self.b.op(op, &args)
            }
            4 => {
                let may_bubble = self.may_bubble;
                let callees: Vec<Callee> = self
                    .functions
                    .iter()
                    .filter(|f| f.result == ty && (may_bubble || !f.bubbles))
                    .cloned()
                    .collect();
                if callees.is_empty() {
                    return self.literal(ty);
                }
                let callee = callees[self.rng.below(callees.len())].clone();
                let receiver = callee.receiver.map(|r| self.expr(r, depth - 1));
                let mut args = Vec::new();
                for (&p, &optional) in callee.params.iter().zip(&callee.optional) {
                    args.push(if optional && self.rng.chance(40) {
                        self.b.omitted(p)
                    } else {
                        self.expr(p, depth - 1)
                    });
                }
                match receiver {
                    Some(receiver) => self.b.call_method(callee.decl, receiver, &args),
                    None => self.b.call(callee.decl, &args),
                }
            }
            5 if ty == TypeId::BOOLEAN => {
                let left = self.expr(ty, depth - 1);
                let right = self.expr(ty, depth - 1);
                if self.rng.chance(50) {
                    self.b.and(left, right)
                } else {
                    self.b.or(left, right)
                }
            }
            6 => {
                let cond = self.expr(TypeId::BOOLEAN, depth - 1);
                let then = self.expr(ty, depth - 1);
                let els = self.expr(ty, depth - 1);
                self.b.if_value(ty, cond, then, els)
            }
            7 => {
                let mark = self.scope.len();
                let stmts: Vec<NodeId> = (0..self.rng.below(3))
                    .map(|_| self.stmt(depth - 1))
                    .collect();
                let value = self.expr(ty, depth - 1);
                self.scope.truncate(mark);
                self.b.block_value(ty, &stmts, value)
            }
            8 if self.rng.chance(20) => self.b.error(ty, "unresolved \"thing\""),
            _ => self.literal(ty),
        }
    }

    fn literal(&mut self, ty: TypeId) -> NodeId {
        match ty {
            TypeId::BOOLEAN => self.b.bool_lit(self.rng.chance(50)),
            TypeId::INT32 => {
                let value = self.rng.pick(&[0, 1, -1, i32::MIN, i32::MAX, 42]);
                self.b.int32(value)
            }
            TypeId::INT64 => {
                let value = self.rng.pick(&[0, -7, i64::MIN, i64::MAX, 1 << 40]);
                self.b.int64(value)
            }
            TypeId::FLOAT64 => {
                let value = self.rng.pick(&[
                    0.0,
                    -0.0,
                    0.1,
                    1e300,
                    -2.5e-8,
                    f64::NAN,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    5e-324,
                ]);
                self.b.float64(value)
            }
            TypeId::STRING => {
                let value = self
                    .rng
                    .pick(&["", "hi", "a \"quote\"", "tab\tline\n", "\u{1}é😀\\"]);
                self.b.string(value)
            }
            _ => self.b.void(),
        }
    }
}

/// Gives every node and declaration a span, as P1 wants.
fn add_spans(b: &mut Builder) {
    let file = b.add_file("src/fuzz.temper", [7; 16]);
    let (nodes, decls) = {
        let p = b.program();
        (p.nodes.len() as u32, p.decls.len() as u32)
    };
    for i in 0..nodes {
        let span = Span {
            file,
            start: i,
            end: i + 3,
        };
        b.set_span(NodeId(i), span, i % 5 == 0);
    }
    for i in 0..decls {
        let span = Span {
            file,
            start: i,
            end: i + 9,
        };
        let name = Span { end: i + 1, ..span };
        b.set_decl_spans(DeclId(i), span, name);
    }
}

#[test]
fn random_programs_validate_and_round_trip() {
    let seed: u64 = env::var("TMPL_FUZZ_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x5eed);
    let rounds: u64 = env::var("TMPL_FUZZ_ROUNDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300);
    let (mut kinds, mut qnames) = (std::collections::HashSet::new(), 0);
    for round in 0..rounds {
        let mut g = Gen {
            rng: Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15).wrapping_add(round) | 1),
            b: Builder::new("fuzz", "0.1.0"),
            functions: Vec::new(),
            core: None,
            console: DeclId::NONE,
            overrides: 0,
            scope: Vec::new(),
            loops: Vec::new(),
            owner: DeclId::NONE,
            test_type: None,
            may_bubble: false,
        };
        let p = g.program();
        let written = text::write(&p);
        let problems = validate(&p, &Profile::full());
        assert!(
            problems.is_empty(),
            "round {round}: {}\n{written}",
            problems[0]
        );
        let back = text::read(&written).unwrap_or_else(|e| panic!("round {round}: {e}\n{written}"));
        if let Some(d) = difference(&p, &back) {
            panic!("round {round}: {d}\n{written}");
        }
        assert_eq!(text::write(&back), written, "round {round}");
        kinds.extend(p.decls.ids().map(|d| p.decls.get(d).kind));
        let names = QNames::derive(&p);
        qnames += p.decls.ids().filter(|&d| names.get(d).is_some()).count();
    }
    // The generator reaches every declaration kind but a removed one.
    if rounds >= 100 {
        assert_eq!(kinds.len(), 11, "{kinds:?}");
        assert!(qnames > 0);
    }
}

/// The module `let`s among values in scope.
fn lets_in_scope(scope: &[Value]) -> Vec<Value> {
    scope.iter().filter(|v| v.module_let).cloned().collect()
}
