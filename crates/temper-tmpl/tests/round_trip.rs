//! A property test: random valid programs, from a small deterministic
//! generator, validate without problems, write as text that reads back to
//! an equal program, and write again the same. Set `TMPL_FUZZ_SEED` and
//! `TMPL_FUZZ_ROUNDS` to vary it.

use std::env;
use temper_tmpl::builtins::{KEYS, Key};
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

struct Gen {
    rng: Rng,
    b: Builder,
    functions: Vec<(DeclId, TypeId, bool, Vec<TypeId>)>,
    /// Values in scope, innermost last.
    scope: Vec<Value>,
    /// Enclosing loops' labels.
    loops: Vec<DeclId>,
    owner: DeclId,
    may_bubble: bool,
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

    fn program(&mut self) -> Program {
        for _ in 0..1 + self.rng.below(2) {
            let module = self.b.module(&format!("m/{}", self.rng.below(100)));
            self.module(module);
        }
        let mut b = std::mem::replace(&mut self.b, Builder::new("", ""));
        if self.rng.chance(50) {
            add_spans(&mut b);
        }
        b.finish()
    }

    fn module(&mut self, module: DeclId) {
        let mut lets = Vec::new();
        for _ in 0..self.rng.below(3) {
            let ty = self.rng.pick(&TYPES);
            let decl = self.declare(DeclKind::ModuleLet, module, ty);
            let var = self.rng.chance(50);
            if var {
                self.b.add_facts(decl, Facts::VAR);
            }
            self.facts(decl);
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
        let mut stmts = Vec::new();
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
    }

    fn facts(&mut self, decl: DeclId) {
        if self.rng.chance(30) {
            self.b.add_facts(decl, Facts::EXPORTED);
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
        let result = if self.rng.chance(30) {
            TypeId::VOID
        } else {
            self.rng.pick(&TYPES)
        };
        let bubbles = self.rng.chance(30);
        let sig = self.b.function_type(&params, result, bubbles);
        let decl = self.declare(DeclKind::Function, module, sig);
        self.facts(decl);
        self.functions.push((decl, result, bubbles, params.clone()));
        self.owner = decl;
        self.may_bubble = bubbles;
        self.scope = lets.to_vec();
        let mut param_nodes = Vec::new();
        for ty in params {
            let param = self.declare(DeclKind::Param, decl, ty);
            param_nodes.push(self.b.param(param));
            self.scope.push(Value {
                decl: param,
                ty,
                var: false,
                module_let: false,
            });
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
                let log = self.b.builtin(Key::named("console.log").unwrap(), &[text]);
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
                let keys: Vec<usize> = (0..KEYS.len())
                    .filter(|&i| KEYS[i].result == ty && (may_bubble || !KEYS[i].bubbles))
                    .collect();
                if keys.is_empty() {
                    return self.literal(ty);
                }
                let key = Key(self.rng.pick(&keys) as u32);
                let params = key.info().unwrap().params;
                let args: Vec<NodeId> = params.iter().map(|&p| self.expr(p, depth - 1)).collect();
                self.b.builtin(key, &args)
            }
            4 => {
                let may_bubble = self.may_bubble;
                let callees: Vec<_> = self
                    .functions
                    .iter()
                    .filter(|f| f.1 == ty && (may_bubble || !f.2))
                    .cloned()
                    .collect();
                if callees.is_empty() {
                    return self.literal(ty);
                }
                let (callee, _, _, params) =
                    self.rng.pick(&callees.iter().collect::<Vec<_>>()).clone();
                let args: Vec<NodeId> = params.iter().map(|&p| self.expr(p, depth - 1)).collect();
                self.b.call(callee, &args)
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
    for round in 0..rounds {
        let mut g = Gen {
            rng: Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15).wrapping_add(round) | 1),
            b: Builder::new("fuzz", "0.1.0"),
            functions: Vec::new(),
            scope: Vec::new(),
            loops: Vec::new(),
            owner: DeclId::NONE,
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
    }
}
