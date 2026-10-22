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
use crate::core::{
    self, CLASS_INTERFACE, LET_VAR, LOOP_NEXT, MEMBER_STATIC, ROLE_CONSTRUCTOR, ROLE_GETTER, ROLE_MASK, ROLE_METHOD, ROLE_PARAM,
    ROLE_SETTER,
};
use crate::scope::{ScopeId, SymbolKind};
use crate::settle::Settled;
use std::cell::RefCell;
use std::rc::Rc;
use temper_syntax::ast::Value;
use temper_syntax::names::NameId;
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
    /// An immutable `List`.
    List(Rc<[Val]>),
    Fn(Rc<Closure>),
    Builtin(Function),
    /// A method of a value, as `x.toString` before it's called.
    Method(Rc<(Val, NameId)>),
    /// A class or interface, as its name's value, for `new`, `as` and `is`.
    Class(Rc<Class>),
    Object(Rc<Object>),
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

/// A class, with the frame it was declared in.
#[derive(Debug)]
pub struct Class {
    node: NodeId,
    env: Env,
    /// Static fields, indexed like an object's fields.
    statics: RefCell<Vec<Val>>,
}

/// An instance: its fields, indexed by their symbols' slots in its class's
/// scope.
#[derive(Debug)]
pub struct Object {
    class: Rc<Class>,
    fields: RefCell<Vec<Val>>,
}

/// A frame for a class's scope while one of its members runs, holding just
/// `this` (the scope's first symbol), through which bare member names read:
/// an object, or for static members, the class.
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
            NodeKind::RightName | NodeKind::BuiltinName => self.read(s, node, env),
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
                if tree.kind(children[0]) == NodeKind::Call {
                    let (object, name) = self.member(s, children[0], env)?;
                    let value = self.eval(s, children[1], env)?;
                    self.set_property(s, object, name, value.clone())?;
                    return Ok(value);
                }
                let value = self.eval(s, children[1], env)?;
                self.write(s, children[0], value.clone(), env)?;
                Ok(value)
            }
            NodeKind::Class => {
                // Static fields, in order; the class was set before the block ran.
                let Val::Class(class) = self.read(s, children[0], env)? else {
                    return Err(fail("Class not set"));
                };
                let scope = s.scopes.of_node(node).ok_or_else(|| fail("Class not settled"))?;
                let this_env = self.this_frame(scope, Val::Class(class.clone()), &class);
                for &member in core::class_parts(tree, node).2 {
                    let (name, _, init) = core::let_parts(tree, member);
                    let data = tree.data(member);
                    if let (true, Some(init)) = (data & MEMBER_STATIC != 0 && data & ROLE_MASK == 0, init) {
                        let value = self.eval(s, init, &this_env)?;
                        let slot = s.scopes.resolved(name).map(|b| s.scopes.symbol(b).slot as usize);
                        class.statics.borrow_mut()[slot.ok_or_else(|| fail("Unresolved name"))?] = value;
                    }
                }
                Ok(Val::Void)
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
        // Set function and class declarations first, so they can use later
        // ones.
        for &statement in tree.children(block) {
            if tree.kind(statement) == NodeKind::Class {
                let size = s.scopes.of_node(statement).map_or(0, |scope| s.scopes.get(scope).names.len());
                let statics = RefCell::new(vec![Val::Uninit; size]);
                let class = Val::Class(Rc::new(Class { node: statement, env: env.clone(), statics }));
                self.write(s, tree.children(statement)[0], class, env)?;
            }
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
            SymbolKind::TypeParam => Ok(Val::Type),
            SymbolKind::Member => {
                let this = self.this(s, symbol.owner, env)?;
                self.get_property(s, this, symbol.name)
            }
            _ => match self.slot(s, name, env, |v| v.clone())? {
                Val::Uninit => Err(fail(format!("{} has not been initialized", s.library.names.get(symbol.name)))),
                value => Ok(value),
            },
        }
    }

    fn write(&mut self, s: &Settled, name: NodeId, value: Val, env: &Env) -> Result<(), Abrupt> {
        if let Some(id) = s.scopes.resolved(name) {
            let symbol = *s.scopes.symbol(id);
            if symbol.kind == SymbolKind::Member {
                let this = self.this(s, symbol.owner, env)?;
                return self.set_property(s, this, symbol.name, value);
            }
        }
        self.slot(s, name, env, |v| *v = value)
    }

    /// `this` for a class's scope, from the frame its member runs in.
    fn this(&self, s: &Settled, class_scope: ScopeId, env: &Env) -> R {
        let mut at = env.as_ref();
        while let Some(frame) = at {
            if frame.scope == class_scope {
                return Ok(frame.slots.borrow()[0].clone());
            }
            at = frame.parent.as_ref();
        }
        let _ = s;
        Err(fail("No this here"))
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
            return self.get_property(s, object, name);
        }
        let function = if tree.kind(callee) == NodeKind::Call && tree.children(callee).first().is_some_and(|&c| is_dot(s, c)) {
            let (object, name) = self.member(s, callee, env)?;
            Val::Method(Rc::new((object, name)))
        } else {
            self.eval(s, callee, env)?
        };
        let mut args = Vec::with_capacity(children.len() - 1);
        let mut rest = &children[1..];
        while let Some((&arg, more)) = rest.split_first() {
            rest = more;
            match s.library.symbol(arg) {
                // The interpreter ignores type arguments.
                Some(key) if s.library.names.get(key) == "typeArg" => rest = rest.get(1..).unwrap_or_default(),
                Some(_) => return Err(fail("Named arguments aren't supported yet")),
                None => args.push(self.eval(s, arg, env)?),
            }
        }
        self.apply(s, function, args)
    }

    /// The object and member name of `obj.name`.
    fn member(&mut self, s: &Settled, dot: NodeId, env: &Env) -> Result<(Val, NameId), Abrupt> {
        match *s.library.tree.children(dot) {
            [_, object, name] => {
                let object = self.eval(s, object, env)?;
                match s.library.symbol(name) {
                    Some(name) => Ok((object, name)),
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
            Val::Builtin(Function::New) => match args.split_first() {
                Some((Val::Class(class), rest)) => self.construct(s, class.clone(), rest.to_vec()),
                _ => Err(fail("new needs a class")),
            },
            Val::Builtin(f @ (Function::As | Function::Is)) => {
                let [value, ty] = <[Val; 2]>::try_from(args).map_err(|_| fail("Expected a value and a type"))?;
                let is = match (&value, &ty) {
                    (Val::Object(object), Val::Class(class)) => self.extends(s, &object.class, class)?,
                    (_, Val::Class(_)) => false,
                    // Builtin types aren't checked yet.
                    _ => true,
                };
                match f {
                    Function::Is => Ok(Val::Bool(is)),
                    _ if is => Ok(value),
                    _ => Err(Abrupt::Fail(None)),
                }
            }
            Val::Builtin(f) => self.builtin(f, args),
            Val::Method(method) => match &method.0 {
                this @ (Val::Object(_) | Val::Class(_)) => {
                    let class = class_of(this);
                    let wanted = |data| data & ROLE_MASK == ROLE_METHOD && member_of(this, data);
                    match self.find_member(s, &class, method.1, &wanted)? {
                        Some((class, decl, _)) => match core::let_parts(&s.library.tree, decl).2 {
                            Some(f) => {
                                let this = receiver(this, s.library.tree.data(decl), &class);
                                self.invoke(s, this, &class, f, args)
                            }
                            None => Err(fail("Abstract method")),
                        },
                        None => {
                            let value = self.get_property(s, method.0.clone(), method.1)?;
                            self.apply(s, value, args)
                        }
                    }
                }
                Val::List(items) => self.list_method(s, items.clone(), s.library.names.get(method.1), args),
                object => self.method(object, s.library.names.get(method.1), args),
            },
            other => Err(fail(format!("{} is not a function", type_name(&other)))),
        }
    }

    fn builtin(&mut self, f: Function, args: Vec<Val>) -> R {
        use Function::*;
        match (f, args.as_slice()) {
            (Angle, [generic, ..]) => Ok(generic.clone()),
            (List, items) => Ok(Val::List(items.into())),
            (Index, [Val::List(items), Val::Int(i)]) => list_get(items, *i),
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
            (Val::Str(text), "split", [Val::Str(sep)]) => {
                let parts: Vec<Val> = if sep.is_empty() {
                    text.chars().map(|c| Val::Str(c.to_string().into())).collect()
                } else {
                    text.split(&**sep).map(|p| Val::Str(p.into())).collect()
                };
                Ok(Val::List(parts.into()))
            }
            _ => Err(fail(format!("No method {name} for {}", type_name(object)))),
        }
    }
}

impl Interp {
    /// A `List`'s methods, which may call back.
    fn list_method(&mut self, s: &Settled, items: Rc<[Val]>, name: &str, args: Vec<Val>) -> R {
        let mut args = args.into_iter();
        let mut arg = || args.next().ok_or_else(|| fail(format!("Too few arguments to {name}")));
        match name {
            "get" => match arg()? {
                Val::Int(i) => list_get(&items, i),
                _ => Err(fail("List index isn't an Int")),
            },
            "getOr" => match (arg()?, arg()?) {
                (Val::Int(i), fallback) => Ok(usize::try_from(i).ok().and_then(|i| items.get(i).cloned()).unwrap_or(fallback)),
                _ => Err(fail("List index isn't an Int")),
            },
            "slice" => match (arg()?, arg()?) {
                (Val::Int(begin), Val::Int(end)) => {
                    let clamp = |i: i32| (i.max(0) as usize).min(items.len());
                    let (begin, end) = (clamp(begin), clamp(end));
                    Ok(Val::List(items[begin..end.max(begin)].into()))
                }
                _ => Err(fail("List indices aren't Ints")),
            },
            "toList" => Ok(Val::List(items)),
            "map" | "filter" => {
                let f = arg()?;
                let mut out = Vec::with_capacity(items.len());
                for item in items.iter() {
                    let result = self.apply(s, f.clone(), vec![item.clone()])?;
                    match (name, result) {
                        ("map", result) => out.push(result),
                        (_, Val::Bool(true)) => out.push(item.clone()),
                        (_, Val::Bool(false)) => {}
                        _ => return Err(fail("filter's predicate didn't return a Boolean")),
                    }
                }
                Ok(Val::List(out.into()))
            }
            "reduce" | "reduceFrom" => {
                let (mut result, start) = match name {
                    "reduce" => (list_get(&items, 0)?, 1),
                    _ => (arg()?, 0),
                };
                let f = arg()?;
                for item in &items[start..] {
                    result = self.apply(s, f.clone(), vec![result, item.clone()])?;
                }
                Ok(result)
            }
            "join" => {
                let (separator, f) = (arg()?, arg()?);
                let Val::Str(separator) = separator else { return Err(fail("join's separator isn't a String")) };
                let mut text = String::new();
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        text.push_str(&separator);
                    }
                    match self.apply(s, f.clone(), vec![item.clone()])? {
                        Val::Str(part) => text.push_str(&part),
                        _ => return Err(fail("join's stringify didn't return a String")),
                    }
                }
                Ok(Val::Str(text.into()))
            }
            _ => self.method(&Val::List(items), name, args.collect()),
        }
    }

    /// `new C(args)`: the header's fields from the arguments or their
    /// defaults, then other fields' initializers in order, then the
    /// constructor, if any.
    fn construct(&mut self, s: &Settled, class: Rc<Class>, args: Vec<Val>) -> R {
        let tree = &s.library.tree;
        if tree.data(class.node) & CLASS_INTERFACE != 0 {
            return Err(fail("Can't make an instance of an interface"));
        }
        let scope = s.scopes.of_node(class.node).ok_or_else(|| fail("Class not settled"))?;
        let size = s.scopes.get(scope).names.len();
        let object = Rc::new(Object { class: class.clone(), fields: RefCell::new(vec![Val::Uninit; size]) });
        let this = Val::Object(object.clone());
        let env = self.this_frame(scope, this.clone(), &class);
        let (_, _, members) = core::class_parts(tree, class.node);
        let constructor = members.iter().copied().find(|&m| tree.data(m) & ROLE_MASK == ROLE_CONSTRUCTOR);
        let mut args = args.into_iter();
        for &member in members {
            let (name, _, init) = core::let_parts(tree, member);
            let data = tree.data(member);
            if data & MEMBER_STATIC != 0 {
                continue;
            }
            let value = match data & ROLE_MASK {
                ROLE_PARAM if constructor.is_none() => match (args.next(), init) {
                    (Some(value), _) => value,
                    (None, Some(init)) => self.eval(s, init, &env)?,
                    (None, None) => return Err(fail("Too few arguments")),
                },
                0 => match init {
                    Some(init) => self.eval(s, init, &env)?,
                    None => continue,
                },
                _ => continue,
            };
            let slot = s.scopes.resolved(name).map(|b| s.scopes.symbol(b).slot as usize);
            object.fields.borrow_mut()[slot.ok_or_else(|| fail("Unresolved name"))?] = value;
        }
        match constructor.and_then(|c| core::let_parts(tree, c).2) {
            Some(f) => {
                self.invoke(s, this.clone(), &class, f, args.collect())?;
            }
            None if args.next().is_some() => return Err(fail("Too many arguments")),
            None => {}
        }
        Ok(this)
    }

    fn this_frame(&self, scope: ScopeId, this: Val, class: &Rc<Class>) -> Env {
        let slots = RefCell::new(vec![this]);
        Some(Rc::new(Frame { scope, slots, parent: class.env.clone() }))
    }

    /// Calls a member's `Fn` with `this`, an object or for static members,
    /// a class; `class` declares the member.
    fn invoke(&mut self, s: &Settled, this: Val, class: &Rc<Class>, f: NodeId, args: Vec<Val>) -> R {
        let scope = s.scopes.of_node(class.node).ok_or_else(|| fail("Class not settled"))?;
        let env = self.this_frame(scope, this, class);
        self.apply(s, Val::Fn(Rc::new(Closure { node: f, env })), args)
    }

    /// The member with a name whose `Let`'s data `wanted` accepts: the
    /// class declaring it, its `Let` and its slot. A class's own members
    /// come first, then its supertypes' breadth-first, so the nearest wins,
    /// and among equally near ones, the first in `extends` order.
    fn find_member(
        &mut self,
        s: &Settled,
        class: &Rc<Class>,
        name: NameId,
        wanted: &dyn Fn(u32) -> bool,
    ) -> Result<Option<(Rc<Class>, NodeId, usize)>, Abrupt> {
        let tree = &s.library.tree;
        let mut queue = std::collections::VecDeque::from([class.clone()]);
        let mut seen = vec![class.node];
        while let Some(class) = queue.pop_front() {
            let scope = s.scopes.of_node(class.node).ok_or_else(|| fail("Class not settled"))?;
            for &(n, b) in &s.scopes.get(scope).names {
                let symbol = s.scopes.symbol(b);
                if let (true, Some(decl)) = (n == name, symbol.decl) {
                    if wanted(tree.data(decl)) {
                        return Ok(Some((class, decl, symbol.slot as usize)));
                    }
                }
            }
            for &sup in core::class_parts(tree, class.node).1 {
                if let Val::Class(sup) = self.eval(s, sup, &class.env)? {
                    if !seen.contains(&sup.node) {
                        seen.push(sup.node);
                        queue.push_back(sup);
                    }
                }
            }
        }
        Ok(None)
    }

    /// Whether `class` is `target` or a subtype of it.
    fn extends(&mut self, s: &Settled, class: &Rc<Class>, target: &Rc<Class>) -> Result<bool, Abrupt> {
        if class.node == target.node {
            return Ok(true);
        }
        let (_, supers, _) = core::class_parts(&s.library.tree, class.node);
        for &sup in supers {
            if let Val::Class(sup) = self.eval(s, sup, &class.env)? {
                if self.extends(s, &sup, target)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// `object.name`, or `Class.name` for a static member: a field, a
    /// getter's result or a method.
    fn get_property(&mut self, s: &Settled, object: Val, name: NameId) -> R {
        if !matches!(object, Val::Object(_) | Val::Class(_)) {
            return property(object, name, s.library.names.get(name));
        }
        let class = class_of(&object);
        let readable = |data| !matches!(data & ROLE_MASK, ROLE_SETTER | ROLE_CONSTRUCTOR) && member_of(&object, data);
        let Some((class, decl, slot)) = self.find_member(s, &class, name, &readable)? else {
            return Err(fail(format!("No member {}", s.library.names.get(name))));
        };
        let data = s.library.tree.data(decl);
        match data & ROLE_MASK {
            ROLE_METHOD => Ok(Val::Method(Rc::new((object.clone(), name)))),
            ROLE_GETTER => match core::let_parts(&s.library.tree, decl).2 {
                Some(f) => self.invoke(s, receiver(&object, data, &class), &class, f, Vec::new()),
                None => Err(fail("Abstract getter")),
            },
            _ => {
                match field(&object, data, &class, |fields| fields[slot].clone())? {
                    Val::Uninit => Err(fail(format!("{} has not been initialized", s.library.names.get(name)))),
                    value => Ok(value),
                }
            }
        }
    }

    /// `object.name = value`, or `Class.name = value`: a field or a setter.
    fn set_property(&mut self, s: &Settled, object: Val, name: NameId, value: Val) -> Result<(), Abrupt> {
        if !matches!(object, Val::Object(_) | Val::Class(_)) {
            return Err(fail(format!("Can't set {} on {}", s.library.names.get(name), type_name(&object))));
        }
        let class = class_of(&object);
        let writable = |data| matches!(data & ROLE_MASK, 0 | ROLE_PARAM | ROLE_SETTER) && member_of(&object, data);
        let Some((class, decl, slot)) = self.find_member(s, &class, name, &writable)? else {
            return Err(fail(format!("No member {}", s.library.names.get(name))));
        };
        let data = s.library.tree.data(decl);
        if data & ROLE_MASK == ROLE_SETTER {
            return match core::let_parts(&s.library.tree, decl).2 {
                Some(f) => self.invoke(s, receiver(&object, data, &class), &class, f, vec![value]).map(|_| ()),
                None => Err(fail("Abstract setter")),
            };
        }
        field(&object, data, &class, |fields| fields[slot] = value)
    }
}

/// Runs `f` on where a field lives: its class's statics, or the object's
/// fields.
fn field<T>(object: &Val, data: u32, class: &Rc<Class>, f: impl FnOnce(&mut Vec<Val>) -> T) -> Result<T, Abrupt> {
    match object {
        _ if data & MEMBER_STATIC != 0 => Ok(f(&mut class.statics.borrow_mut())),
        Val::Object(obj) if Rc::ptr_eq(class, &obj.class) => Ok(f(&mut obj.fields.borrow_mut())),
        _ => Err(fail("Inherited fields aren't supported yet")),
    }
}

/// The class of an object, or a class itself, for its static members.
fn class_of(this: &Val) -> Rc<Class> {
    match this {
        Val::Object(object) => object.class.clone(),
        Val::Class(class) => class.clone(),
        _ => unreachable!("not an object or class"),
    }
}

/// Whether a member with this data can be reached from `this`: a class
/// reaches only static members.
fn member_of(this: &Val, data: u32) -> bool {
    !matches!(this, Val::Class(_)) || data & MEMBER_STATIC != 0
}

/// `this` for running a member: the declaring class for a static one.
fn receiver(this: &Val, data: u32, class: &Rc<Class>) -> Val {
    if data & MEMBER_STATIC != 0 { Val::Class(class.clone()) } else { this.clone() }
}

fn is_dot(s: &Settled, node: NodeId) -> bool {
    s.scopes.resolved_builtin(node) == Some(Builtin::Function(Function::Dot))
}

fn property(object: Val, name: NameId, text: &str) -> R {
    match (&object, text) {
        (Val::Str(s), "isEmpty") => Ok(Val::Bool(s.is_empty())),
        (Val::List(items), "isEmpty") => Ok(Val::Bool(items.is_empty())),
        (Val::List(items), "length") => Ok(Val::Int(items.len() as i32)),
        _ => Ok(Val::Method(Rc::new((object, name)))),
    }
}

/// An element of a list, panicking out of bounds, as Temper's does.
fn list_get(items: &[Val], i: i32) -> R {
    match usize::try_from(i).ok().and_then(|i| items.get(i)) {
        Some(item) => Ok(item.clone()),
        None => Err(Abrupt::Panic(format!("index {i} out of bounds for length {}", items.len()))),
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
        (Val::Object(a), Val::Object(b)) => Rc::ptr_eq(a, b),
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
        Val::List(_) => "List",
        Val::Fn(_) | Val::Builtin(_) | Val::Method(_) => "Function",
        Val::Console => "Console",
        Val::Class(_) => "Class",
        Val::Object(_) => "Object",
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
