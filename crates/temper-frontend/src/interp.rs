//! A small tree interpreter over core constructs, for running functional
//! tests in-process. Values are dynamically typed until types are in the
//! settle loop. Error nodes fail when run, carrying their message, as a
//! backend's generated code would.
//!
//! Frames follow scopes: a block with declarations, or a function call,
//! gets a frame of slots, one per symbol, linked to the frame it was made
//! in. Closures keep their frame. Function declarations in a block are set
//! before its statements run, so functions may call ones declared later.

use crate::builtins::{Builtin, Constant, Function};
use crate::core::{self, LET_VAR, LOOP_NEXT};
use crate::scope::{ScopeId, SymbolKind};
use crate::settle::Settled;
use std::cell::RefCell;
use std::rc::Rc;
use temper_syntax::ast::Value;
use temper_syntax::tree::{NodeId, NodeKind};

/// A run-time value.
#[derive(Clone, Debug)]
pub enum Val {
    Void,
    Null,
    Bool(bool),
    Int(i32),
    Int64(i64),
    Float(f64),
    Str(Rc<str>),
    Fn(Rc<Closure>),
    Builtin(Function),
    /// A method of a value, as `x.toString` before it's called.
    Method(Rc<(Val, String)>),
    Console,
    /// A type name used as a value; types aren't run yet.
    Type,
    /// A declared local not yet given a value.
    Uninit,
}

#[derive(Debug)]
pub struct Closure {
    node: NodeId,
    env: Env,
}

#[derive(Debug)]
struct Frame {
    scope: ScopeId,
    slots: RefCell<Vec<Val>>,
    parent: Env,
}

type Env = Option<Rc<Frame>>;

/// How evaluation stopped early.
#[derive(Debug)]
enum Abrupt {
    Break(NodeId),
    Continue(NodeId),
    Return(NodeId, Val),
    /// A failure, as from `bubble()`, with a message for error nodes.
    Fail(Option<String>),
    /// Something `orelse` can't catch: a panic, or the interpreter's limits.
    Panic(String),
}

type R = Result<Val, Abrupt>;

/// What running a library printed, and how it failed, if it did.
#[derive(Debug, Default)]
pub struct Run {
    pub output: Vec<String>,
    pub failure: Option<String>,
}

/// Limits, so that any input finishes: calls in progress, evaluation
/// nesting (which bounds the stack used; about 1 MB in release builds and
/// several in debug builds, so run on a thread with a big enough stack),
/// and the default number of evaluation steps.
const MAX_DEPTH: u32 = 200;
const MAX_NESTING: u32 = 3000;
pub const FUEL: u64 = 50_000_000;

/// Runs a settled library's top-level code.
pub fn run(settled: &Settled) -> Run {
    run_with_fuel(settled, FUEL)
}

/// Runs, stopping after `fuel` evaluation steps.
pub fn run_with_fuel(settled: &Settled, fuel: u64) -> Run {
    let mut interp = Interp { output: Vec::new(), fuel, depth: 0, nesting: 0 };
    let frame = interp.frame(settled, settled.module, None);
    let mut failure = None;
    for file in &settled.library.files {
        if let Err(abrupt) = interp.statements(settled, file.root, &frame) {
            failure = Some(match abrupt {
                Abrupt::Fail(None) => "Failed".to_string(),
                Abrupt::Fail(Some(message)) => format!("Failed: {message}"),
                Abrupt::Panic(message) => format!("Panicked: {message}"),
                Abrupt::Return(..) | Abrupt::Break(_) | Abrupt::Continue(_) => "Jumped out of the module".to_string(),
            });
            break;
        }
    }
    Run { output: interp.output, failure }
}

struct Interp {
    output: Vec<String>,
    fuel: u64,
    depth: u32,
    nesting: u32,
}

fn fail(message: impl Into<String>) -> Abrupt {
    Abrupt::Fail(Some(message.into()))
}

impl Interp {
    fn frame(&self, s: &Settled, scope: ScopeId, parent: Env) -> Env {
        let size = s.scopes.get(scope).names.len();
        Some(Rc::new(Frame { scope, slots: RefCell::new(vec![Val::Uninit; size]), parent }))
    }

    fn eval(&mut self, s: &Settled, node: NodeId, env: &Env) -> R {
        if self.fuel == 0 {
            return Err(Abrupt::Panic("ran out of steps".into()));
        }
        if self.nesting >= MAX_NESTING {
            return Err(Abrupt::Panic("nested too deeply".into()));
        }
        self.fuel -= 1;
        self.nesting += 1;
        let result = self.eval_node(s, node, env);
        self.nesting -= 1;
        result
    }

    fn eval_node(&mut self, s: &Settled, node: NodeId, env: &Env) -> R {
        let tree = &s.library.tree;
        let children = tree.children(node);
        match tree.kind(node) {
            NodeKind::Value => literal(s.library.value(node).unwrap()),
            NodeKind::RightName => self.read(s, node, env),
            NodeKind::Block => {
                let env = match s.scopes.of_node(node) {
                    Some(scope) if !s.scopes.get(scope).names.is_empty() => self.frame(s, scope, env.clone()),
                    _ => env.clone(),
                };
                match self.statements(s, node, &env) {
                    Err(Abrupt::Break(target)) if target == node => Ok(Val::Void),
                    result => result,
                }
            }
            NodeKind::Let => {
                let (name, _, init) = core::let_parts(tree, node);
                if let Some(init) = init {
                    let value = self.eval(s, init, env)?;
                    self.write(s, name, value, env)?;
                }
                Ok(Val::Void)
            }
            NodeKind::Assign => {
                let value = self.eval(s, children[1], env)?;
                self.write(s, children[0], value.clone(), env)?;
                Ok(value)
            }
            NodeKind::If => match self.eval(s, children[0], env)? {
                Val::Bool(true) => self.eval(s, children[1], env),
                Val::Bool(false) => self.eval(s, children[2], env),
                other => Err(fail(format!("Expected a Boolean, not {}", type_name(&other)))),
            },
            NodeKind::Loop => loop {
                match self.eval(s, children[0], env) {
                    Ok(_) => {}
                    Err(Abrupt::Continue(target)) if target == node => {}
                    Err(Abrupt::Break(target)) if target == node => return Ok(Val::Void),
                    Err(abrupt) => return Err(abrupt),
                }
                if tree.data(node) & LOOP_NEXT != 0 {
                    self.eval(s, children[1], env)?;
                }
            },
            NodeKind::Break => Err(Abrupt::Break(NodeId(tree.data(node)))),
            NodeKind::Continue => Err(Abrupt::Continue(NodeId(tree.data(node)))),
            NodeKind::Return => {
                let value = self.eval(s, children[0], env)?;
                Err(Abrupt::Return(NodeId(tree.data(node)), value))
            }
            NodeKind::Orelse => match self.eval(s, children[0], env) {
                Err(Abrupt::Fail(_)) => self.eval(s, children[1], env),
                result => result,
            },
            NodeKind::Fail => Err(Abrupt::Fail(None)),
            NodeKind::Fn => Ok(Val::Fn(Rc::new(Closure { node, env: env.clone() }))),
            NodeKind::Call => self.call(s, node, env),
            NodeKind::Error => Err(fail(s.library.error_text(node))),
            kind => Err(fail(format!("Not settled: {kind:?}"))),
        }
    }

    /// Runs a block's statements in `env`, which has the block's frame.
    fn statements(&mut self, s: &Settled, block: NodeId, env: &Env) -> R {
        let tree = &s.library.tree;
        // Set function declarations first, so they can call later ones.
        for &statement in tree.children(block) {
            if tree.kind(statement) == NodeKind::Let && tree.data(statement) & LET_VAR == 0 {
                let (name, _, init) = core::let_parts(tree, statement);
                if let Some(init) = init.filter(|&i| tree.kind(i) == NodeKind::Fn) {
                    let closure = Val::Fn(Rc::new(Closure { node: init, env: env.clone() }));
                    self.write(s, name, closure, env)?;
                }
            }
        }
        let mut value = Val::Void;
        for &statement in tree.children(block) {
            value = self.eval(s, statement, env)?;
        }
        Ok(value)
    }

    /// The frame slot for a name node's symbol.
    fn slot<T>(&self, s: &Settled, name: NodeId, env: &Env, f: impl FnOnce(&mut Val) -> T) -> Result<T, Abrupt> {
        let symbol = match s.scopes.resolved(name) {
            Some(id) => *s.scopes.symbol(id),
            None => return Err(fail("Unresolved name")),
        };
        let mut at = env.as_ref();
        while let Some(frame) = at {
            if frame.scope == symbol.owner {
                let mut slots = frame.slots.borrow_mut();
                return Ok(f(&mut slots[symbol.slot as usize]));
            }
            at = frame.parent.as_ref();
        }
        Err(fail(format!("{} is not in scope here", s.library.names.get(symbol.name))))
    }

    fn read(&mut self, s: &Settled, name: NodeId, env: &Env) -> R {
        let Some(id) = s.scopes.resolved(name) else {
            return Err(fail("Unresolved name"));
        };
        let symbol = *s.scopes.symbol(id);
        match symbol.kind {
            SymbolKind::Builtin(b) => builtin_value(b),
            _ => match self.slot(s, name, env, |v| v.clone())? {
                Val::Uninit => Err(fail(format!("{} has not been initialized", s.library.names.get(symbol.name)))),
                value => Ok(value),
            },
        }
    }

    fn write(&mut self, s: &Settled, name: NodeId, value: Val, env: &Env) -> Result<(), Abrupt> {
        self.slot(s, name, env, |v| *v = value)
    }

    fn call(&mut self, s: &Settled, node: NodeId, env: &Env) -> R {
        let tree = &s.library.tree;
        let children = tree.children(node);
        let Some(&callee) = children.first() else {
            return Err(fail("Malformed call"));
        };
        if is_dot(s, callee) {
            // `obj.name` read as a property.
            let (object, name) = self.member(s, node, env)?;
            return property(object, &name);
        }
        let function = if tree.kind(callee) == NodeKind::Call && tree.children(callee).first().is_some_and(|&c| is_dot(s, c)) {
            let (object, name) = self.member(s, callee, env)?;
            Val::Method(Rc::new((object, name)))
        } else {
            self.eval(s, callee, env)?
        };
        let mut args = Vec::with_capacity(children.len() - 1);
        for &arg in &children[1..] {
            if s.library.symbol(arg).is_some() {
                return Err(fail("Named arguments aren't supported yet"));
            }
            args.push(self.eval(s, arg, env)?);
        }
        self.apply(s, function, args)
    }

    /// The object and member name of `obj.name`.
    fn member(&mut self, s: &Settled, dot: NodeId, env: &Env) -> Result<(Val, String), Abrupt> {
        match *s.library.tree.children(dot) {
            [_, object, name] => {
                let object = self.eval(s, object, env)?;
                match s.library.symbol(name) {
                    Some(name) => Ok((object, s.library.names.get(name).to_string())),
                    None => Err(fail("Malformed member access")),
                }
            }
            _ => Err(fail("Malformed member access")),
        }
    }

    fn apply(&mut self, s: &Settled, function: Val, args: Vec<Val>) -> R {
        match function {
            Val::Fn(closure) => {
                if self.depth >= MAX_DEPTH {
                    return Err(Abrupt::Panic("calls nested too deeply".into()));
                }
                let Some(scope) = s.scopes.of_node(closure.node) else {
                    return Err(fail("Function not settled"));
                };
                let (params, _, body) = core::fn_parts(&s.library.tree, closure.node);
                if args.len() > params.len() {
                    return Err(fail("Too many arguments"));
                }
                let env = self.frame(s, scope, closure.env.clone());
                let given = args.len();
                for (i, value) in args.into_iter().enumerate() {
                    let (name, _, _) = core::let_parts(&s.library.tree, params[i]);
                    self.write(s, name, value, &env)?;
                }
                for &param in &params[given..] {
                    let (name, _, default) = core::let_parts(&s.library.tree, param);
                    let Some(default) = default else {
                        return Err(fail("Too few arguments"));
                    };
                    let value = self.eval(s, default, &env)?;
                    self.write(s, name, value, &env)?;
                }
                self.depth += 1;
                let result = self.eval(s, body, &env);
                self.depth -= 1;
                match result {
                    Err(Abrupt::Return(target, value)) if target == closure.node => Ok(value),
                    result => result,
                }
            }
            Val::Builtin(f) => self.builtin(f, args),
            Val::Method(method) => self.method(&method.0, &method.1, args),
            other => Err(fail(format!("{} is not a function", type_name(&other)))),
        }
    }

    fn builtin(&mut self, f: Function, args: Vec<Val>) -> R {
        use Function::*;
        match (f, args.as_slice()) {
            (Cat, parts) => {
                let mut text = String::new();
                for part in parts {
                    text.push_str(&to_string(part));
                }
                Ok(Val::Str(text.into()))
            }
            (Not, [Val::Bool(b)]) => Ok(Val::Bool(!b)),
            (Sub, [Val::Int(n)]) => Ok(Val::Int(n.wrapping_neg())),
            (Sub, [Val::Int64(n)]) => Ok(Val::Int64(n.wrapping_neg())),
            (Sub, [Val::Float(n)]) => Ok(Val::Float(-n)),
            (Add, [Val::Int(n)]) => Ok(Val::Int(*n)),
            (Eq, [a, b]) => Ok(Val::Bool(equal(a, b))),
            (Ne, [a, b]) => Ok(Val::Bool(!equal(a, b))),
            (Lt | Le | Gt | Ge, [a, b]) => {
                let ordering = compare(a, b).ok_or_else(|| fail("Can't compare these"))?;
                Ok(Val::Bool(match f {
                    Lt => ordering.is_lt(),
                    Le => ordering.is_le(),
                    Gt => ordering.is_gt(),
                    _ => ordering.is_ge(),
                }))
            }
            // `i32::MIN / -1` overflows `i32` but not `i64`, and wraps back.
            (_, [Val::Int(a), Val::Int(b)]) => int_op(f, *a as i64, *b as i64).map(|n| Val::Int(n as i32)),
            (_, [Val::Int64(a), Val::Int64(b)]) => int_op(f, *a, *b).map(Val::Int64),
            (Add | Sub | Mul | Div | Mod, [Val::Float(a), Val::Float(b)]) => Ok(Val::Float(match f {
                Add => a + b,
                Sub => a - b,
                Mul => a * b,
                Div => a / b,
                _ => a % b,
            })),
            (Panic, _) => Err(Abrupt::Panic("panic()".into())),
            _ => Err(fail(format!("Can't apply {} to these arguments", Builtin::Function(f).name()))),
        }
    }

    fn method(&mut self, object: &Val, name: &str, args: Vec<Val>) -> R {
        match (object, name, args.as_slice()) {
            (Val::Console, "log", [message]) => {
                self.output.push(to_string(message));
                Ok(Val::Void)
            }
            (_, "toString", []) => Ok(Val::Str(to_string(object).into())),
            (Val::Int(a), "min", [Val::Int(b)]) => Ok(Val::Int(*a.min(b))),
            (Val::Int(a), "max", [Val::Int(b)]) => Ok(Val::Int(*a.max(b))),
            (Val::Int64(a), "min", [Val::Int64(b)]) => Ok(Val::Int64(*a.min(b))),
            (Val::Int64(a), "max", [Val::Int64(b)]) => Ok(Val::Int64(*a.max(b))),
            (Val::Float(a), "min", [Val::Float(b)]) => Ok(Val::Float(a.min(*b))),
            (Val::Float(a), "max", [Val::Float(b)]) => Ok(Val::Float(a.max(*b))),
            _ => Err(fail(format!("No method {name} for {}", type_name(object)))),
        }
    }
}

fn is_dot(s: &Settled, node: NodeId) -> bool {
    s.scopes.resolved_builtin(node) == Some(Builtin::Function(Function::Dot))
}

fn property(object: Val, name: &str) -> R {
    match (&object, name) {
        (Val::Str(text), "isEmpty") => Ok(Val::Bool(text.is_empty())),
        _ => Ok(Val::Method(Rc::new((object, name.to_string())))),
    }
}

fn literal(value: &Value) -> R {
    Ok(match value {
        Value::Int32(n) => Val::Int(*n),
        Value::Int64(n) => Val::Int64(*n),
        Value::Float64(f) => Val::Float(*f),
        Value::String(s) => Val::Str(s.as_str().into()),
        Value::Bool(b) => Val::Bool(*b),
        Value::Null => Val::Null,
        Value::Void => Val::Void,
        _ => return Err(fail("Unsupported value")),
    })
}

fn builtin_value(b: Builtin) -> R {
    Ok(match b {
        Builtin::Value(Constant::True) => Val::Bool(true),
        Builtin::Value(Constant::False) => Val::Bool(false),
        Builtin::Value(Constant::Null) => Val::Null,
        Builtin::Value(Constant::Console) => Val::Console,
        Builtin::Function(f) => Val::Builtin(f),
        Builtin::Type(_) => Val::Type,
        Builtin::Macro(_) => return Err(fail("Macro used as a value")),
    })
}

/// Integer arithmetic on operands widened to `i64`, wrapping; callers
/// truncate for `Int`, which wraps as Temper's does. Division by zero fails.
fn int_op(f: Function, a: i64, b: i64) -> Result<i64, Abrupt> {
    use Function::*;
    Ok(match f {
        Add => a.wrapping_add(b),
        Sub => a.wrapping_sub(b),
        Mul => a.wrapping_mul(b),
        Div | Mod if b == 0 => return Err(Abrupt::Fail(None)),
        Div => a.wrapping_div(b),
        Mod => a.wrapping_rem(b),
        BitAnd => a & b,
        BitOr => a | b,
        BitXor => a ^ b,
        Shl => a.wrapping_shl(b as u32),
        Shr => a.wrapping_shr(b as u32),
        _ => return Err(fail(format!("Can't apply {} to numbers", Builtin::Function(f).name()))),
    })
}

fn equal(a: &Val, b: &Val) -> bool {
    match (a, b) {
        (Val::Void, Val::Void) | (Val::Null, Val::Null) => true,
        (Val::Bool(a), Val::Bool(b)) => a == b,
        (Val::Int(a), Val::Int(b)) => a == b,
        (Val::Int64(a), Val::Int64(b)) => a == b,
        (Val::Float(a), Val::Float(b)) => a == b,
        (Val::Str(a), Val::Str(b)) => a == b,
        (Val::Fn(a), Val::Fn(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

fn compare(a: &Val, b: &Val) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Val::Int(a), Val::Int(b)) => Some(a.cmp(b)),
        (Val::Int64(a), Val::Int64(b)) => Some(a.cmp(b)),
        (Val::Float(a), Val::Float(b)) => a.partial_cmp(b),
        (Val::Str(a), Val::Str(b)) => Some(a.cmp(b)),
        _ => None,
    }
}

fn type_name(value: &Val) -> &'static str {
    match value {
        Val::Void => "Void",
        Val::Null => "Null",
        Val::Bool(_) => "Boolean",
        Val::Int(_) => "Int",
        Val::Int64(_) => "Int64",
        Val::Float(_) => "Float64",
        Val::Str(_) => "String",
        Val::Fn(_) | Val::Builtin(_) | Val::Method(_) => "Function",
        Val::Console => "Console",
        Val::Type => "Type",
        Val::Uninit => "Uninitialized",
    }
}

/// A value as its `toString()` gives it.
pub fn to_string(value: &Val) -> String {
    match value {
        Val::Str(s) => s.to_string(),
        Val::Int(n) => n.to_string(),
        Val::Int64(n) => n.to_string(),
        Val::Float(f) => {
            if f.is_finite() && f.fract() == 0.0 && f.abs() < 1e15 {
                format!("{f:.1}")
            } else {
                f.to_string()
            }
        }
        Val::Bool(b) => b.to_string(),
        Val::Null => "null".into(),
        Val::Void => "void".into(),
        other => type_name(other).into(),
    }
}
