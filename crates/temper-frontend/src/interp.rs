//! A small tree interpreter over core constructs, for running functional
//! tests in-process. Values are dynamically typed until types are in the
//! settle loop. Error nodes fail when run, carrying their message, as a
//! backend's generated code would.
//!
//! Frames follow scopes: a block with declarations, or a function call,
//! gets a frame of slots, one per symbol, linked to the frame it was made
//! in. Closures keep their frame. Function declarations in a block are set
//! before its statements run, so functions may call ones declared later.

mod numbers;
mod strings;

use crate::builtins::{Builtin, Constant, Function, TYPES};
use crate::core::{
    self, CLASS_INTERFACE, LET_VAR, LOOP_NEXT, MEMBER_STATIC, ROLE_CONSTRUCTOR, ROLE_GETTER, ROLE_MASK, ROLE_METHOD, ROLE_PARAM,
    ROLE_SETTER,
};
use crate::scope::{ScopeId, SymbolKind};
use crate::settle::Settled;
use std::cell::RefCell;
use std::collections::VecDeque;
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
    /// A position in a string, as a UTF-8 offset; Temper's are opaque.
    StringIndex(u32),
    /// `StringIndex.none`, the other `StringIndexOption`.
    NoStringIndex,
    StringBuilder(Rc<RefCell<String>>),
    /// An immutable `List`.
    List(Rc<[Val]>),
    ListBuilder(Rc<RefCell<Vec<Val>>>),
    Deque(Rc<RefCell<VecDeque<Val>>>),
    /// A `DenseBitVector`; bits past the end are false.
    Bits(Rc<RefCell<Vec<bool>>>),
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
    /// A builtin type, by its index in [`crate::builtins::TYPES`], for
    /// `new ListBuilder()` and `List.of`.
    BuiltinType(u8),
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

/// Limits: calls in progress and evaluation nesting, which bound the stack
/// used (about 1 MB in release builds and several in debug builds, so run
/// on a thread with a big enough stack), and, if asked, evaluation steps,
/// so that any input finishes.
const MAX_DEPTH: u32 = 200;
const MAX_NESTING: u32 = 3000;
/// A step budget for callers that must finish, such as tests and the web
/// playground; compile-time evaluation will need one too.
pub const FUEL: u64 = 50_000_000;

/// Runs a settled library's top-level code, stopping after `fuel`
/// evaluation steps if given. Without fuel, a program runs until it ends,
/// as any language's would.
pub fn run(settled: &Settled, fuel: Option<u64>) -> Run {
    // `u64::MAX` steps take centuries, so no fuel needs no check of its own.
    let fuel = fuel.unwrap_or(u64::MAX);
    let mut interp = Interp { output: Vec::new(), fuel, depth: 0, nesting: 0 };
    let mut failure = None;
    // Each module runs in a frame linked to those before it, so that
    // imported names, resolved to their modules' symbols, are found.
    let mut frame = None;
    let mut result = Ok(Val::Void);
    for module in &settled.modules {
        frame = interp.frame(settled, module.scope, frame);
        result = interp.statements(settled, &module.top_levels, &frame);
        if result.is_err() {
            break;
        }
    }
    if let Err(abrupt) = result {
        failure = Some(match abrupt {
            Abrupt::Fail(None) => "Failed".to_string(),
            Abrupt::Fail(Some(message)) => format!("Failed: {message}"),
            Abrupt::Panic(message) => format!("Panicked: {message}"),
            Abrupt::Return(..) | Abrupt::Break(_) | Abrupt::Continue(_) => "Jumped out of the module".to_string(),
        });
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
                match self.statements(s, tree.children(node), &env) {
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
    fn statements(&mut self, s: &Settled, statements: &[NodeId], env: &Env) -> R {
        let tree = &s.library.tree;
        // Set function and class declarations first, so they can use later
        // ones.
        for &statement in statements {
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
        for &statement in statements {
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
                // A missing or null argument takes its parameter's default,
                // computed here in the callee, after earlier parameters.
                let mut args = args.into_iter();
                for &param in &params {
                    let (name, _, default) = core::let_parts(&s.library.tree, param);
                    let value = match (args.next(), default) {
                        (None | Some(Val::Null), Some(default)) => self.eval(s, default, &env)?,
                        (Some(value), _) => value,
                        (None, None) => return Err(fail("Too few arguments")),
                    };
                    self.write(s, name, value, &env)?;
                }
                self.depth += 1;
                let result = self.eval(s, body, &env);
                self.depth -= 1;
                match result {
                    Err(Abrupt::Return(target, value)) if target == closure.node => Ok(value),
                // `continue` in a `for … of` body ends its call.
                Err(Abrupt::Continue(target)) if target == closure.node => Ok(Val::Void),
                    result => result,
                }
            }
            Val::Builtin(Function::New) => match args.split_first() {
                Some((Val::Class(class), rest)) => self.construct(s, class.clone(), rest.to_vec()),
                Some((Val::BuiltinType(t), [])) if TYPES[*t as usize] == "ListBuilder" => {
                    Ok(Val::ListBuilder(Rc::new(RefCell::new(Vec::new()))))
                }
                Some((Val::BuiltinType(t), [])) if TYPES[*t as usize] == "StringBuilder" => {
                    Ok(Val::StringBuilder(Rc::new(RefCell::new(String::new()))))
                }
                Some((Val::BuiltinType(t), [])) if TYPES[*t as usize] == "Deque" => {
                    Ok(Val::Deque(Rc::new(RefCell::new(VecDeque::new()))))
                }
                Some((Val::BuiltinType(t), [Val::Int(capacity)])) if TYPES[*t as usize] == "DenseBitVector" => {
                    Ok(Val::Bits(Rc::new(RefCell::new(vec![false; (*capacity).max(0) as usize]))))
                }
                _ => Err(fail("new needs a class")),
            },
            Val::Builtin(f @ (Function::As | Function::Is)) => {
                let [value, ty] = <[Val; 2]>::try_from(args).map_err(|_| fail("Expected a value and a type"))?;
                let is = match (&value, &ty) {
                    (Val::Object(object), Val::Class(class)) => self.extends(s, &object.class, class)?,
                    (_, Val::Class(_)) => false,
                    (_, Val::BuiltinType(t)) => is_builtin_type(&value, TYPES[*t as usize]),
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
                list @ (Val::List(_) | Val::ListBuilder(_)) => self.list_method(s, list, s.library.names.get(method.1), args),
                Val::Str(text) if s.library.names.get(method.1) == "forEach" => {
                    // As `core.temper`'s `String.forEach`.
                    let text = text.clone();
                    let [f] = <[Val; 1]>::try_from(args).map_err(|_| fail("forEach takes a function"))?;
                    for c in text.chars() {
                        self.apply(s, f.clone(), vec![Val::Int(c as i32)])?;
                    }
                    Ok(Val::Void)
                }
                object => self.method(object, s.library.names.get(method.1), args),
            },
            other => Err(fail(format!("{} is not a function", type_name(&other)))),
        }
    }

    fn builtin(&mut self, f: Function, args: Vec<Val>) -> R {
        use Function::*;
        match (f, args.as_slice()) {
            (Angle, [generic, ..]) | (Nullable, [generic]) => Ok(generic.clone()),
            (List, items) => Ok(Val::List(items.into())),
            (Index, [Val::List(items), Val::Int(i)]) => list_get(items, *i),
            (Index, [Val::ListBuilder(items), Val::Int(i)]) => list_get(&items.borrow(), *i),
            (Index, [Val::Str(text), Val::StringIndex(i)]) => strings::get(text, *i),
            (Index, [bits @ Val::Bits(_), i @ Val::Int(_)]) => self.method(bits, "get", vec![i.clone()]),
            (Char, parts) => strings::char_tag(parts),
            (Ignore, [_]) => Ok(Val::Void),
            (Cmp, [a, b]) => {
                let ordering = compare(a, b).ok_or_else(|| fail("Can't compare these"))?;
                Ok(Val::Int(ordering as i32))
            }
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
            (BitNot, [Val::Int(n)]) => Ok(Val::Int(!n)),
            (BitNot, [Val::Int64(n)]) => Ok(Val::Int64(!n)),
            // Shift distances wrap at the width, as in Java; an `Int64`'s is an `Int`.
            (Shl | Shr | UShr, [Val::Int(a), Val::Int(b)]) => Ok(Val::Int(match f {
                Shl => a.wrapping_shl(*b as u32),
                Shr => a.wrapping_shr(*b as u32),
                _ => (*a as u32).wrapping_shr(*b as u32) as i32,
            })),
            (Shl | Shr | UShr, [Val::Int64(a), Val::Int(b)]) => Ok(Val::Int64(match f {
                Shl => a.wrapping_shl(*b as u32),
                Shr => a.wrapping_shr(*b as u32),
                _ => (*a as u64).wrapping_shr(*b as u32) as i64,
            })),
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
            // Like integers', and unlike IEEE 754's, float division fails
            // for a zero divisor.
            (Div | Mod, [Val::Float(_), Val::Float(b)]) if *b == 0.0 => Err(Abrupt::Fail(None)),
            (Pow, [Val::Float(a), Val::Float(b)]) => Ok(Val::Float(a.powf(*b))),
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
            (Val::StringBuilder(builder), _, _) => {
                strings::builder_method(builder, name, &args).unwrap_or_else(|| no_method(object, name))
            }
            (Val::Deque(deque), "add", [value]) => {
                deque.borrow_mut().push_back(value.clone());
                Ok(Val::Void)
            }
            (Val::Deque(deque), "removeFirst", []) => {
                deque.borrow_mut().pop_front().ok_or_else(|| Abrupt::Panic("removeFirst on an empty Deque".into()))
            }
            // As `core.temper`'s `DenseBitVector`, which grows when a bit
            // past the end is set.
            (Val::Bits(bits), "get", [Val::Int(i)]) => {
                Ok(Val::Bool(usize::try_from(*i).ok().and_then(|i| bits.borrow().get(i).copied()).unwrap_or(false)))
            }
            (Val::Bits(bits), "set", [Val::Int(i), Val::Bool(value)]) => {
                let mut bits = bits.borrow_mut();
                let Ok(i) = usize::try_from(*i) else {
                    return Err(Abrupt::Panic(format!("index {i} out of bounds for length {}", bits.len())));
                };
                if i >= bits.len() && *value {
                    bits.resize(i + 1, false);
                }
                if let Some(bit) = bits.get_mut(i) {
                    *bit = *value;
                }
                Ok(Val::Void)
            }
            (_, "toString", []) => Ok(Val::Str(to_string(object).into())),
            (Val::Int(n), "toString", [radix]) => strings::int_to_string(*n as i64, radix_or_10(radix)?),
            (Val::Int64(n), "toString", [radix]) => strings::int_to_string(*n, radix_or_10(radix)?),
            (Val::StringIndex(_) | Val::NoStringIndex, "compareTo", [other]) => {
                let ordering = compare(object, other).ok_or_else(|| fail("Can't compare these"))?;
                Ok(Val::Int(ordering as i32))
            }
            (Val::BuiltinType(t), "of", items) if TYPES[*t as usize] == "List" => Ok(Val::List(items.into())),
            (Val::BuiltinType(t), _, _) if TYPES[*t as usize] == "String" => {
                strings::static_method(name, &args).unwrap_or_else(|| no_method(object, name))
            }
            (Val::Int(a), "min", [Val::Int(b)]) => Ok(Val::Int(*a.min(b))),
            (Val::Int(a), "max", [Val::Int(b)]) => Ok(Val::Int(*a.max(b))),
            (Val::Int64(a), "min", [Val::Int64(b)]) => Ok(Val::Int64(*a.min(b))),
            (Val::Int64(a), "max", [Val::Int64(b)]) => Ok(Val::Int64(*a.max(b))),
            (Val::Float(f), _, _) => numbers::float_method(*f, name, &args).unwrap_or_else(|| no_method(object, name)),
            (Val::Int(n), _, _) => numbers::int_method(*n as i64, false, name, &args).unwrap_or_else(|| no_method(object, name)),
            (Val::Int64(n), _, _) => numbers::int_method(*n, true, name, &args).unwrap_or_else(|| no_method(object, name)),
            (Val::Str(text), _, _) => strings::string_method(text, name, &args).unwrap_or_else(|| no_method(object, name)),
            _ => no_method(object, name),
        }
    }
}

/// Whether a value has a builtin type, for `is` and `as`. Others, like
/// `Mapped`, aren't checked yet.
fn is_builtin_type(value: &Val, ty: &str) -> bool {
    match ty {
        "Int" | "Int32" => matches!(value, Val::Int(_)),
        "Int64" => matches!(value, Val::Int64(_)),
        "Float64" => matches!(value, Val::Float(_)),
        "String" => matches!(value, Val::Str(_)),
        "Boolean" => matches!(value, Val::Bool(_)),
        "List" => matches!(value, Val::List(_)),
        "ListBuilder" => matches!(value, Val::ListBuilder(_)),
        "Listed" => matches!(value, Val::List(_) | Val::ListBuilder(_)),
        "StringIndex" => matches!(value, Val::StringIndex(_)),
        "NoStringIndex" => matches!(value, Val::NoStringIndex),
        "StringIndexOption" => matches!(value, Val::StringIndex(_) | Val::NoStringIndex),
        "StringBuilder" => matches!(value, Val::StringBuilder(_)),
        "Deque" => matches!(value, Val::Deque(_)),
        "DenseBitVector" => matches!(value, Val::Bits(_)),
        "AnyValue" => !matches!(value, Val::Null),
        "Void" => matches!(value, Val::Void),
        "Never" => false,
        _ => !matches!(value, Val::Null),
    }
}

/// A radix argument, 10 if null.
fn radix_or_10(radix: &Val) -> Result<i32, Abrupt> {
    match radix {
        Val::Null => Ok(10),
        Val::Int(radix) => Ok(*radix),
        _ => Err(fail("The radix isn't an Int")),
    }
}

fn no_method(object: &Val, name: &str) -> R {
    Err(fail(format!("No method {name} for {}", type_name(object))))
}

impl Interp {
    /// `List` and `ListBuilder` methods, which may call back. Callbacks
    /// get a snapshot of a builder's items, so they may change it.
    fn list_method(&mut self, s: &Settled, this: &Val, name: &str, args: Vec<Val>) -> R {
        if let Val::ListBuilder(builder) = this {
            if BUILDER_METHODS.contains(&name) {
                return self.builder_method(s, builder, name, &args);
            }
        }
        let mut args = args.into_iter();
        let mut arg = || args.next().ok_or_else(|| fail(format!("Too few arguments to {name}")));
        // Reads that don't call back borrow the items.
        match name {
            "get" => {
                return match arg()? {
                    Val::Int(i) => with_items(this, |items| list_get(items, i)),
                    _ => Err(fail("List index isn't an Int")),
                };
            }
            "getOr" => {
                return match (arg()?, arg()?) {
                    (Val::Int(i), fallback) => Ok(with_items(this, |items| {
                        usize::try_from(i).ok().and_then(|i| items.get(i).cloned()).unwrap_or(fallback)
                    })),
                    _ => Err(fail("List index isn't an Int")),
                };
            }
            "slice" => {
                return match (arg()?, arg()?) {
                    (Val::Int(begin), Val::Int(end)) => Ok(Val::List(with_items(this, |items| {
                        let clamp = |i: i32| (i.max(0) as usize).min(items.len());
                        let (begin, end) = (clamp(begin), clamp(end));
                        items[begin..end.max(begin)].into()
                    }))),
                    _ => Err(fail("List indices aren't Ints")),
                };
            }
            "toList" => return Ok(Val::List(snapshot(this))),
            "toListBuilder" => return Ok(Val::ListBuilder(Rc::new(RefCell::new(with_items(this, <[Val]>::to_vec))))),
            _ => {}
        }
        let items = snapshot(this);
        match name {
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
            // Only `List` has `forEach` so far, as in Kotlin Temper's core.
            "forEach" if matches!(this, Val::List(_)) => {
                let f = arg()?;
                for item in items.iter() {
                    self.apply(s, f.clone(), vec![item.clone()])?;
                }
                Ok(Val::Void)
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
            "sorted" => {
                let f = arg()?;
                Ok(Val::List(self.sort(s, items.to_vec(), &f)?.into()))
            }
            _ => self.method(this, name, args.collect()),
        }
    }

    /// One of [`BUILDER_METHODS`]. A missing or null argument takes its
    /// default.
    fn builder_method(&mut self, s: &Settled, builder: &Rc<RefCell<Vec<Val>>>, name: &str, args: &[Val]) -> R {
        let len = builder.borrow().len();
        let int_or = |i: usize, default: usize| match args.get(i) {
            None | Some(Val::Null) => Ok(default as i64),
            Some(Val::Int(n)) => Ok(*n as i64),
            Some(_) => Err(fail(format!("{name}'s index isn't an Int"))),
        };
        let at = |i: usize| {
            let at = int_or(i, len)?;
            usize::try_from(at)
                .ok()
                .filter(|&at| at <= len)
                .ok_or_else(|| Abrupt::Panic(format!("index {at} out of bounds for length {len}")))
        };
        match (name, args) {
            ("add", [value, ..]) => {
                let at = at(1)?;
                builder.borrow_mut().insert(at, value.clone());
                Ok(Val::Void)
            }
            ("addAll", [values, ..]) => {
                let (at, values) = (at(1)?, snapshot(values));
                builder.borrow_mut().splice(at..at, values.iter().cloned());
                Ok(Val::Void)
            }
            ("set", [Val::Int(i), value]) => {
                match usize::try_from(*i).ok().and_then(|i| builder.borrow_mut().get_mut(i).map(|slot| *slot = value.clone())) {
                    Some(()) => Ok(Val::Void),
                    None => Err(Abrupt::Panic(format!("index {i} out of bounds for length {len}"))),
                }
            }
            ("removeLast", []) => builder.borrow_mut().pop().ok_or_else(|| Abrupt::Panic("removeLast on an empty ListBuilder".into())),
            ("reverse", []) => {
                builder.borrow_mut().reverse();
                Ok(Val::Void)
            }
            ("clear", []) => {
                builder.borrow_mut().clear();
                Ok(Val::Void)
            }
            ("sort", [f]) => {
                let items = builder.borrow().clone();
                *builder.borrow_mut() = self.sort(s, items, f)?;
                Ok(Val::Void)
            }
            ("splice", _) => {
                // The index and count are clamped.
                let index = int_or(0, 0)?.clamp(0, len as i64) as usize;
                let count = int_or(1, len)?.clamp(0, (len - index) as i64) as usize;
                let values = match args.get(2) {
                    None | Some(Val::Null) => Rc::from([]),
                    Some(values) => snapshot(values),
                };
                let removed: Vec<Val> = builder.borrow_mut().splice(index..index + count, values.iter().cloned()).collect();
                Ok(Val::List(removed.into()))
            }
            _ => Err(fail(format!("Wrong arguments to {name}"))),
        }
    }

    /// A stable merge sort by a Temper comparison function, which may fail
    /// or disagree with itself without upsetting the sort.
    fn sort(&mut self, s: &Settled, mut items: Vec<Val>, compare: &Val) -> Result<Vec<Val>, Abrupt> {
        let mut width = 1;
        let mut merged = Vec::with_capacity(items.len());
        while width < items.len() {
            for start in (0..items.len()).step_by(2 * width) {
                let middle = (start + width).min(items.len());
                let end = (start + 2 * width).min(items.len());
                let (mut i, mut j) = (start, middle);
                while i < middle && j < end {
                    // Take from the right only when it's strictly less.
                    match self.apply(s, compare.clone(), vec![items[j].clone(), items[i].clone()])? {
                        Val::Int(n) if n < 0 => {
                            merged.push(items[j].clone());
                            j += 1;
                        }
                        Val::Int(_) => {
                            merged.push(items[i].clone());
                            i += 1;
                        }
                        _ => return Err(fail("The comparison didn't return an Int")),
                    }
                }
                merged.extend_from_slice(&items[i..middle]);
                merged.extend_from_slice(&items[j..end]);
            }
            std::mem::swap(&mut items, &mut merged);
            merged.clear();
            width *= 2;
        }
        Ok(items)
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

/// `ListBuilder`'s methods that `List` lacks.
const BUILDER_METHODS: &[&str] = &["add", "addAll", "set", "removeLast", "reverse", "clear", "sort", "splice"];

/// Runs `f` on a list's or builder's items.
fn with_items<T>(list: &Val, f: impl FnOnce(&[Val]) -> T) -> T {
    match list {
        Val::List(items) => f(items),
        Val::ListBuilder(items) => f(&items.borrow()),
        _ => f(&[]),
    }
}

/// A list's items, or a copy of a builder's.
fn snapshot(list: &Val) -> Rc<[Val]> {
    match list {
        Val::List(items) => items.clone(),
        other => with_items(other, |items| Rc::from(items)),
    }
}

fn is_dot(s: &Settled, node: NodeId) -> bool {
    s.scopes.resolved_builtin(node) == Some(Builtin::Function(Function::Dot))
}

fn property(object: Val, name: NameId, text: &str) -> R {
    match (&object, text) {
        (Val::Str(s), "isEmpty") => Ok(Val::Bool(s.is_empty())),
        (Val::Str(s), "end") => Ok(Val::StringIndex(s.len() as u32)),
        (Val::BuiltinType(t), "begin") if TYPES[*t as usize] == "String" => Ok(Val::StringIndex(0)),
        (Val::BuiltinType(t), "none") if TYPES[*t as usize] == "StringIndex" => Ok(Val::NoStringIndex),
        (Val::StringBuilder(builder), "end") => Ok(Val::StringIndex(builder.borrow().len() as u32)),
        (Val::BuiltinType(t), "e") if TYPES[*t as usize] == "Float64" => Ok(Val::Float(std::f64::consts::E)),
        (Val::BuiltinType(t), "pi") if TYPES[*t as usize] == "Float64" => Ok(Val::Float(std::f64::consts::PI)),
        (Val::List(items), "isEmpty") => Ok(Val::Bool(items.is_empty())),
        (Val::List(items), "length") => Ok(Val::Int(items.len() as i32)),
        (Val::ListBuilder(items), "isEmpty") => Ok(Val::Bool(items.borrow().is_empty())),
        (Val::ListBuilder(items), "length") => Ok(Val::Int(items.borrow().len() as i32)),
        (Val::Deque(items), "isEmpty") => Ok(Val::Bool(items.borrow().is_empty())),
        (Val::BuiltinType(t), "of") if TYPES[*t as usize] == "List" => Ok(Val::Builtin(Function::List)),
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
        Builtin::Value(Constant::NaN) => Val::Float(f64::NAN),
        Builtin::Value(Constant::Infinity) => Val::Float(f64::INFINITY),
        Builtin::Function(f) => Val::Builtin(f),
        Builtin::Type(t) => Val::BuiltinType(t),
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
        _ => return Err(fail(format!("Can't apply {} to numbers", Builtin::Function(f).name()))),
    })
}

fn equal(a: &Val, b: &Val) -> bool {
    match (a, b) {
        (Val::Void, Val::Void) | (Val::Null, Val::Null) => true,
        (Val::Bool(a), Val::Bool(b)) => a == b,
        (Val::Int(a), Val::Int(b)) => a == b,
        (Val::Int64(a), Val::Int64(b)) => a == b,
        (Val::Float(a), Val::Float(b)) => numbers::float_cmp(*a, *b).is_eq(),
        (Val::Str(a), Val::Str(b)) => a == b,
        (Val::StringIndex(a), Val::StringIndex(b)) => a == b,
        (Val::NoStringIndex, Val::NoStringIndex) => true,
        (Val::StringBuilder(a), Val::StringBuilder(b)) => Rc::ptr_eq(a, b),
        (Val::Fn(a), Val::Fn(b)) => Rc::ptr_eq(a, b),
        (Val::Object(a), Val::Object(b)) => Rc::ptr_eq(a, b),
        (Val::ListBuilder(a), Val::ListBuilder(b)) => Rc::ptr_eq(a, b),
        (Val::Deque(a), Val::Deque(b)) => Rc::ptr_eq(a, b),
        (Val::Bits(a), Val::Bits(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

fn compare(a: &Val, b: &Val) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Val::Int(a), Val::Int(b)) => Some(a.cmp(b)),
        (Val::Int64(a), Val::Int64(b)) => Some(a.cmp(b)),
        (Val::Float(a), Val::Float(b)) => Some(numbers::float_cmp(*a, *b)),
        (Val::Str(a), Val::Str(b)) => Some(a.cmp(b)),
        // `StringIndex.none` sorts before every index.
        (Val::StringIndex(_) | Val::NoStringIndex, Val::StringIndex(_) | Val::NoStringIndex) => {
            let position = |v: &Val| if let Val::StringIndex(i) = v { *i as i64 } else { -1 };
            Some(position(a).cmp(&position(b)))
        }
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
        Val::StringIndex(_) => "StringIndex",
        Val::NoStringIndex => "NoStringIndex",
        Val::StringBuilder(_) => "StringBuilder",
        Val::List(_) => "List",
        Val::ListBuilder(_) => "ListBuilder",
        Val::Deque(_) => "Deque",
        Val::Bits(_) => "DenseBitVector",
        Val::Fn(_) | Val::Builtin(_) | Val::Method(_) => "Function",
        Val::Console => "Console",
        Val::Class(_) => "Class",
        Val::Object(_) => "Object",
        Val::Type | Val::BuiltinType(_) => "Type",
        Val::Uninit => "Uninitialized",
    }
}

/// A value as its `toString()` gives it.
pub fn to_string(value: &Val) -> String {
    match value {
        Val::Str(s) => s.to_string(),
        Val::Int(n) => n.to_string(),
        Val::Int64(n) => n.to_string(),
        Val::Float(f) => numbers::format_float(*f),
        Val::Bool(b) => b.to_string(),
        Val::Null => "null".into(),
        Val::Void => "void".into(),
        other => type_name(other).into(),
    }
}
