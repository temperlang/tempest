//! Rust's output tree and its printer.
//!
//! The printer lays code out as rustfmt does with its default settings,
//! for the constructs the translator makes: lines of at most 100
//! columns, a call's arguments on one line only while they take at most
//! 60 (rustfmt's `fn_call_width`), a chain of method calls on one line
//! only while it takes at most 60 (`chain_width`), an `if` … `else` as a
//! value on one line only after `=` and within 50 (`single_line_if_else_max_width`),
//! a last argument that's a closure, or an only argument that's a call,
//! overflowing onto the lines after its call's, and otherwise one
//! argument per line. The tests check that rustfmt leaves the goldens as
//! they are. It adds parentheses only where Rust's precedence needs
//! them.
//!
//! No printing crate is shared among backends yet: Python's tree and
//! printer (in temper-be-py) share no layout rules with this one beyond
//! indentation, so a shared crate would hold little (notes/tmpl/overview.md,
//! "Boundaries").

const MAX: usize = 100;
const CALL_WIDTH: usize = 60;
const CHAIN_WIDTH: usize = 60;
const IF_WIDTH: usize = 50;

/// A binary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bin {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    BitOr,
    BitXor,
    BitAnd,
    Shl,
    Shr,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

impl Bin {
    fn text(self) -> &'static str {
        match self {
            Bin::Or => "||",
            Bin::And => "&&",
            Bin::Eq => "==",
            Bin::Ne => "!=",
            Bin::Lt => "<",
            Bin::Le => "<=",
            Bin::Gt => ">",
            Bin::Ge => ">=",
            Bin::BitOr => "|",
            Bin::BitXor => "^",
            Bin::BitAnd => "&",
            Bin::Shl => "<<",
            Bin::Shr => ">>",
            Bin::Add => "+",
            Bin::Sub => "-",
            Bin::Mul => "*",
            Bin::Div => "/",
            Bin::Rem => "%",
        }
    }

    fn prec(self) -> u8 {
        match self {
            Bin::Or => 3,
            Bin::And => 4,
            Bin::Eq | Bin::Ne | Bin::Lt | Bin::Le | Bin::Gt | Bin::Ge => COMPARE,
            Bin::BitOr => 6,
            Bin::BitXor => 7,
            Bin::BitAnd => 8,
            Bin::Shl | Bin::Shr => 9,
            Bin::Add | Bin::Sub => 10,
            Bin::Mul | Bin::Div | Bin::Rem => 11,
        }
    }
}

/// A prefix operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unary {
    Not,
    Neg,
    Deref,
    Ref,
}

impl Unary {
    fn text(self) -> &'static str {
        match self {
            Unary::Not => "!",
            Unary::Neg => "-",
            Unary::Deref => "*",
            Unary::Ref => "&",
        }
    }
}

// Precedences, loosest first, as Rust's reference has them.
const JUMP: u8 = 0;
const COMPARE: u8 = 5;
const AS: u8 = 12;
const UNARY: u8 = 13;
const POSTFIX: u8 = 14;
const ATOM: u8 = 15;

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    /// A name or a path, as is: `fib`, `temper_core::int_div`.
    Path(String),
    /// A literal other than a string's, as is: `0`, `-1`, `1.5`, `true`.
    Lit(String),
    /// A string literal, quoted when printed. A format string is one
    /// too, with its braces already in place.
    Str(String),
    Call(Box<Expr>, Vec<Expr>),
    Method(Box<Expr>, String, Vec<Expr>),
    /// A macro's name, without its `!`, and its arguments.
    Macro(String, Vec<Expr>),
    Bin(Bin, Box<Expr>, Box<Expr>),
    Unary(Unary, Box<Expr>),
    Cast(Box<Expr>, String),
    Try(Box<Expr>),
    If(Box<Expr>, Block, Block),
    Block(Block),
    Closure(Vec<String>, Block),
    Return(Option<Box<Expr>>),
}

impl Expr {
    pub fn path(name: impl Into<String>) -> Expr {
        Expr::Path(name.into())
    }

    pub fn call(self, args: Vec<Expr>) -> Expr {
        Expr::Call(Box::new(self), args)
    }

    pub fn method(self, name: impl Into<String>, args: Vec<Expr>) -> Expr {
        Expr::Method(Box::new(self), name.into(), args)
    }

    pub fn bin(op: Bin, left: Expr, right: Expr) -> Expr {
        Expr::Bin(op, Box::new(left), Box::new(right))
    }

    pub fn unary(op: Unary, operand: Expr) -> Expr {
        Expr::Unary(op, Box::new(operand))
    }

    pub fn cast(self, ty: impl Into<String>) -> Expr {
        Expr::Cast(Box::new(self), ty.into())
    }

    pub fn try_(self) -> Expr {
        Expr::Try(Box::new(self))
    }

    /// `Ok(self)`.
    pub fn ok(self) -> Expr {
        Expr::path("Ok").call(vec![self])
    }

    fn prec(&self) -> u8 {
        match self {
            Expr::Lit(text) if text.starts_with('-') => UNARY,
            Expr::Path(_) | Expr::Lit(_) | Expr::Str(_) | Expr::Macro(..) | Expr::Block(_) => ATOM,
            Expr::Call(..) | Expr::Method(..) | Expr::Try(_) => POSTFIX,
            Expr::Unary(..) => UNARY,
            Expr::Cast(..) => AS,
            Expr::Bin(op, ..) => op.prec(),
            // Never an operand or a receiver without parentheses.
            Expr::If(..) | Expr::Closure(..) | Expr::Return(_) => JUMP,
        }
    }
}

/// Statements, then maybe a value.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail: Option<Box<Expr>>,
}

impl Block {
    pub fn new(stmts: Vec<Stmt>, tail: Option<Expr>) -> Block {
        Block {
            stmts,
            tail: tail.map(Box::new),
        }
    }

    /// Just a value.
    pub fn of(value: Expr) -> Block {
        Block::new(Vec::new(), Some(value))
    }

    fn is_empty(&self) -> bool {
        self.stmts.is_empty() && self.tail.is_none()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    /// `let [mut] name[: ty][ = value];`
    Let {
        name: String,
        mutable: bool,
        ty: Option<String>,
        value: Option<Expr>,
    },
    /// `place = value;`
    Assign(Expr, Expr),
    /// `e;`
    Expr(Expr),
    If {
        cond: Expr,
        then: Block,
        els: Option<Block>,
    },
    While {
        label: Option<String>,
        cond: Expr,
        body: Block,
    },
    Loop {
        label: Option<String>,
        body: Block,
    },
    /// A labeled block, `'label: { … }`.
    Labeled {
        label: String,
        body: Block,
    },
    Break(Option<String>),
    Continue(Option<String>),
    Return(Option<Expr>),
    /// A `static` in a function's body.
    Static {
        name: String,
        ty: String,
        value: Expr,
    },
    Block(Block),
}

/// A module's item.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Use(String),
    Fn {
        /// `pub`, `pub(crate)`, or nothing.
        vis: &'static str,
        name: String,
        /// Each written out: `mut i: i32`.
        params: Vec<String>,
        result: Option<String>,
        body: Block,
    },
    Static {
        vis: &'static str,
        name: String,
        ty: String,
        value: Expr,
    },
    /// `pub mod name;`, a module in a file of its own.
    Mod(String),
    /// `#[cfg(test)] mod tests { use …; … }`: its `use`s, and each test
    /// function, returning `temper_core::Result<()>`.
    Tests {
        uses: Vec<String>,
        tests: Vec<(String, Block)>,
    },
}

/// A source file's items, printed with a blank line between items, but
/// none between `use`s, nor between `mod`s.
pub fn file(items: &[Item]) -> String {
    let mut out = String::new();
    let mut last = 0;
    for (i, item) in items.iter().enumerate() {
        // `use`s together, and `mod`s.
        let group = match item {
            Item::Use(_) => 1,
            Item::Mod(_) => 2,
            _ => 0,
        };
        if i > 0 && !(group != 0 && group == last) {
            out.push('\n');
        }
        last = group;
        out.push_str(&self::item(item));
    }
    out
}

fn item(item: &Item) -> String {
    match item {
        Item::Use(path) => format!("use {path};\n"),
        Item::Mod(name) => format!("pub mod {name};\n"),
        Item::Fn {
            vis,
            name,
            params,
            result,
            body,
        } => function(vis, name, params, result.as_deref(), body, 0),
        Item::Tests { uses, tests } => {
            let mut out = String::from("#[cfg(test)]\nmod tests {\n");
            for u in uses {
                out.push_str(&format!("    use {u};\n"));
            }
            for (i, (name, body)) in tests.iter().enumerate() {
                if i > 0 || !uses.is_empty() {
                    out.push('\n');
                }
                out.push_str("    #[test]\n");
                let result = Some("temper_core::Result<()>");
                out.push_str(&function("", name, &[], result, body, 4));
            }
            out.push_str("}\n");
            out
        }
        Item::Static {
            vis,
            name,
            ty,
            value,
        } => {
            let head = format!("{}static {name}: {ty} = ", vis_text(vis));
            format!("{head}{};\n", layout_t(value, 0, width(&head), 1))
        }
    }
}

/// A function at an indent.
fn function(
    vis: &str,
    name: &str,
    params: &[String],
    result: Option<&str>,
    body: &Block,
    indent: usize,
) -> String {
    let vis = vis_text(vis);
    let pad = pad(indent);
    let result = result.map(|r| format!(" -> {r}")).unwrap_or_default();
    let mut sig = format!("{pad}{vis}fn {name}({}){result}", params.join(", "));
    if width(&sig) + 2 > MAX {
        sig = format!("{pad}{vis}fn {name}(\n");
        for p in params {
            sig.push_str(&format!("{pad}    {p},\n"));
        }
        sig.push_str(&format!("{pad}){result}"));
    }
    if body.is_empty() {
        format!("{sig} {{}}\n")
    } else {
        format!("{sig} {{\n{}{pad}}}\n", block(body, indent + 4))
    }
}

fn vis_text(vis: &str) -> String {
    if vis.is_empty() {
        String::new()
    } else {
        format!("{vis} ")
    }
}

fn pad(indent: usize) -> String {
    " ".repeat(indent)
}

fn width(s: &str) -> usize {
    s.chars().count()
}

/// The column after `text`, written from `col`.
fn end_col(col: usize, text: &str) -> usize {
    match text.rfind('\n') {
        Some(i) => width(&text[i + 1..]),
        None => col + width(text),
    }
}

fn first_line(text: &str) -> &str {
    text.split('\n').next().unwrap_or("")
}

/// A Rust string literal.
pub fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn paren(text: String, needed: bool) -> String {
    if needed { format!("({text})") } else { text }
}

/// An operand, in parentheses if its precedence is looser than `min`.
fn operand(e: &Expr, min: u8) -> Option<String> {
    Some(paren(flat(e)?, e.prec() < min))
}

fn bin_parts(op: Bin, a: &Expr, b: &Expr) -> (bool, bool) {
    let p = op.prec();
    let compare = p == COMPARE;
    // `a as i32 < b` would start generic arguments.
    let cast_lt = matches!(a, Expr::Cast(..)) && matches!(op, Bin::Lt | Bin::Shl);
    let left = a.prec() < p || (compare && a.prec() == p) || cast_lt;
    let right = b.prec() <= p;
    (left, right)
}

/// Arguments on one line, if each is flat and together they're narrow
/// enough.
fn flat_args(args: &[Expr]) -> Option<String> {
    let parts: Option<Vec<String>> = args.iter().map(flat).collect();
    let text = parts?.join(", ");
    (width(&text) <= CALL_WIDTH).then_some(text)
}

/// The number of method calls in a chain.
fn chain_len(e: &Expr) -> usize {
    match e {
        Expr::Method(r, ..) => 1 + chain_len(r),
        _ => 0,
    }
}

/// An expression on one line, if it may be: no part must take several
/// lines, and every call and chain is within its width.
fn flat(e: &Expr) -> Option<String> {
    Some(match e {
        Expr::Path(s) | Expr::Lit(s) => s.clone(),
        Expr::Str(s) => quote(s),
        Expr::Call(f, args) => format!("{}({})", operand(f, POSTFIX)?, flat_args(args)?),
        Expr::Method(r, name, args) => {
            let text = format!("{}.{name}({})", operand(r, POSTFIX)?, flat_args(args)?);
            if chain_len(e) >= 2 && width(&text) > CHAIN_WIDTH {
                return None;
            }
            text
        }
        Expr::Macro(name, args) => format!("{name}!({})", flat_args(args)?),
        Expr::Bin(op, a, b) => {
            let (pa, pb) = bin_parts(*op, a, b);
            format!(
                "{} {} {}",
                paren(flat(a)?, pa),
                op.text(),
                paren(flat(b)?, pb)
            )
        }
        Expr::Unary(op, x) => format!("{}{}", op.text(), operand(x, UNARY)?),
        Expr::Cast(x, ty) => format!("{} as {ty}", operand(x, AS)?),
        Expr::Try(x) => format!("{}?", operand(x, POSTFIX)?),
        Expr::Closure(params, body) if body.stmts.is_empty() => {
            let tail = body.tail.as_deref()?;
            if matches!(tail, Expr::Block(_) | Expr::If(..)) {
                return None;
            }
            format!("|{}| {}", params.join(", "), flat(tail)?)
        }
        Expr::Return(None) => "return".into(),
        Expr::Return(Some(v)) => format!("return {}", flat(v)?),
        Expr::If(..) | Expr::Block(_) | Expr::Closure(..) => return None,
    })
}

/// `if c { a } else { b }` on one line, for after `=`: each branch just a
/// flat value, within rustfmt's width for it.
fn flat_if(e: &Expr) -> Option<String> {
    let Expr::If(c, t, f) = e else {
        return None;
    };
    let branch = |b: &Block| match (&b.stmts[..], b.tail.as_deref()) {
        ([], Some(v)) if !matches!(v, Expr::If(..)) => flat(v),
        _ => None,
    };
    let text = format!(
        "if {} {{ {} }} else {{ {} }}",
        flat(c)?,
        branch(t)?,
        branch(f)?
    );
    (width(&text) <= IF_WIDTH).then_some(text)
}

/// An expression starting at column `col`, with `indent` the indentation
/// of the line it starts on: on one line if it fits, else broken as
/// rustfmt breaks it.
fn layout(e: &Expr, indent: usize, col: usize) -> String {
    layout_t(e, indent, col, 0)
}

/// As [`layout`], with `trail` columns after it on its line (a `;`, a
/// `,`, or ` {`).
fn layout_t(e: &Expr, indent: usize, col: usize, trail: usize) -> String {
    if let Some(text) = flat(e) {
        if col + width(&text) + trail <= MAX {
            return text;
        }
    }
    broken(e, indent, col)
}

/// An expression laid out on several lines where it can be.
fn broken(e: &Expr, indent: usize, col: usize) -> String {
    match e {
        Expr::Call(f, args) => {
            let head = layout(f, indent, col);
            let head = paren(head, f.prec() < POSTFIX);
            call(&head, args, indent, col, true)
        }
        Expr::Macro(name, args) => call(&format!("{name}!"), args, indent, col, false),
        Expr::Method(..) => chain(e, indent, col),
        Expr::If(c, t, f) => if_else(c, t, f, indent, col),
        Expr::Block(b) => braced(b, indent),
        Expr::Closure(params, body) => {
            let head = format!("|{}| ", params.join(", "));
            format!("{head}{}", braced(body, indent))
        }
        Expr::Return(Some(v)) => format!("return {}", layout(v, indent, col + 7)),
        Expr::Try(x) => format!("{}?", paren(layout(x, indent, col), x.prec() < POSTFIX)),
        Expr::Unary(op, x) => {
            let inner = paren(layout(x, indent, col + 1), x.prec() < UNARY);
            format!("{}{inner}", op.text())
        }
        Expr::Cast(x, ty) => format!("{} as {ty}", paren(layout(x, indent, col), x.prec() < AS)),
        Expr::Bin(op, a, b) => {
            let (pa, pb) = bin_parts(*op, a, b);
            let left = paren(layout(a, indent, col), pa);
            let mid = format!("{left} {} ", op.text());
            let right = paren(layout(b, indent, end_col(col, &mid)), pb);
            format!("{mid}{right}")
        }
        Expr::Path(_) | Expr::Lit(_) | Expr::Str(_) | Expr::Return(None) => {
            flat(e).unwrap_or_default()
        }
    }
}

/// `{`, a block's lines, and `}` at `indent`; `{}` if it's empty.
fn braced(b: &Block, indent: usize) -> String {
    if b.is_empty() {
        "{}".into()
    } else {
        format!("{{\n{}{}}}", block(b, indent + 4), pad(indent))
    }
}

/// A call whose arguments don't fit on its line: its last argument
/// overflowing, if it's a closure or the only argument and a call, else
/// one argument per line, with a trailing comma unless it's a macro's.
fn call(head: &str, args: &[Expr], indent: usize, col: usize, comma: bool) -> String {
    let open = format!("{head}(");
    let after = end_col(col, &open);
    if let Some((last, init)) = args.split_last() {
        let overflows = matches!(last, Expr::Closure(..))
            || (init.is_empty() && matches!(last, Expr::Call(..) | Expr::Macro(..)));
        if overflows {
            let prefix = if init.is_empty() {
                Some(String::new())
            } else {
                flat_args(init).map(|p| format!("{p}, "))
            };
            if let Some(prefix) = prefix {
                let at = after + width(&prefix);
                let last = broken(last, indent, at);
                if at + width(first_line(&last)) <= MAX && last.contains('\n') {
                    return format!("{open}{prefix}{last})");
                }
            }
        }
    }
    if args.is_empty() {
        return format!("{open})");
    }
    let mut out = format!("{open}\n");
    for (i, a) in args.iter().enumerate() {
        let sep = if comma || i + 1 < args.len() { "," } else { "" };
        let text = layout_t(a, indent + 4, indent + 4, sep.len());
        out.push_str(&format!("{}{text}{sep}\n", pad(indent + 4)));
    }
    out.push_str(&pad(indent));
    out.push(')');
    out
}

/// A chain of method calls that doesn't fit on one line: with one call,
/// as a call of `receiver.method`; with more, each call on a line of its
/// own after the receiver.
fn chain(e: &Expr, indent: usize, col: usize) -> String {
    let mut calls = Vec::new();
    let mut base = e;
    while let Expr::Method(r, name, args) = base {
        calls.push((name, args));
        base = r;
    }
    calls.reverse();
    let receiver = paren(layout(base, indent, col), base.prec() < POSTFIX);
    if let [(name, args)] = calls[..] {
        return call(&format!("{receiver}.{name}"), args, indent, col, true);
    }
    let mut out = receiver;
    for (name, args) in calls {
        let at = indent + 4;
        let one = flat_args(args).map(|a| format!(".{name}({a})"));
        let text = match one {
            Some(text) if at + width(&text) <= MAX => text,
            _ => call(&format!(".{name}"), args, at, at, true),
        };
        out.push_str(&format!("\n{}{text}", pad(at)));
    }
    out
}

fn if_else(c: &Expr, t: &Block, f: &Block, indent: usize, col: usize) -> String {
    let cond = layout_t(c, indent, col + 3, 2);
    let mut out = format!("if {cond} {}", braced(t, indent));
    match (&f.stmts[..], f.tail.as_deref()) {
        ([], Some(Expr::If(c2, t2, f2))) => {
            let rest = if_else(c2, t2, f2, indent, indent + 7);
            out.push_str(&format!(" else {rest}"));
        }
        _ => out.push_str(&format!(" else {}", braced(f, indent))),
    }
    out
}

/// A block's lines at `indent`, each ending in a newline.
fn block(b: &Block, indent: usize) -> String {
    let mut out = String::new();
    for s in &b.stmts {
        out.push_str(&stmt(s, indent));
    }
    if let Some(tail) = &b.tail {
        out.push_str(&format!(
            "{}{}\n",
            pad(indent),
            layout(tail, indent, indent)
        ));
    }
    out
}

/// What follows `=`: an `if` on one line only if short.
///
/// As rustfmt, a value too long for the line goes on the next, indented,
/// where it fits on one line there.
fn rhs(e: &Expr, indent: usize, col: usize) -> String {
    if let Some(text) = flat_if(e) {
        if col + width(&text) < MAX {
            return text;
        }
    }
    if let Some(text) = flat(e) {
        let next = indent + 4;
        let breakable = !matches!(e, Expr::Closure(..) | Expr::Block(_) | Expr::If(..));
        if breakable && col + width(&text) + 1 > MAX && next + width(&text) < MAX {
            return format!("\n{}{text}", pad(next));
        }
    }
    layout_t(e, indent, col, 1)
}

fn stmt(s: &Stmt, indent: usize) -> String {
    let p = pad(indent);
    let label = |l: &Option<String>| l.as_ref().map(|l| format!("{l}: ")).unwrap_or_default();
    let jump = |kw: &str, l: &Option<String>| match l {
        Some(l) => format!("{p}{kw} {l};\n"),
        None => format!("{p}{kw};\n"),
    };
    match s {
        Stmt::Let {
            name,
            mutable,
            ty,
            value,
        } => {
            let m = if *mutable { "mut " } else { "" };
            let ty = ty.as_ref().map(|t| format!(": {t}")).unwrap_or_default();
            let head = format!("let {m}{name}{ty}");
            match value {
                Some(v) => {
                    let col = indent + width(&head) + 3;
                    format!("{p}{head} = {};\n", rhs(v, indent, col)).replacen(" = \n", " =\n", 1)
                }
                None => format!("{p}{head};\n"),
            }
        }
        Stmt::Assign(place, value) => {
            let place = layout(place, indent, indent);
            let col = end_col(indent, &place) + 3;
            format!("{p}{place} = {};\n", rhs(value, indent, col)).replacen(" = \n", " =\n", 1)
        }
        Stmt::Expr(e) => format!("{p}{};\n", layout_t(e, indent, indent, 1)),
        Stmt::If { cond, then, els } => {
            let els = els.clone().unwrap_or_default();
            if els.is_empty() {
                let cond = layout_t(cond, indent, indent + 3, 2);
                format!("{p}if {cond} {}\n", braced(then, indent))
            } else {
                format!("{p}{}\n", if_else(cond, then, &els, indent, indent))
            }
        }
        Stmt::While {
            label: l,
            cond,
            body,
        } => {
            let head = format!("{}while ", label(l));
            let cond = layout_t(cond, indent, indent + width(&head), 2);
            format!("{p}{head}{cond} {}\n", braced(body, indent))
        }
        Stmt::Loop { label: l, body } => {
            format!("{p}{}loop {}\n", label(l), braced(body, indent))
        }
        Stmt::Labeled { label: l, body } => format!("{p}{l}: {}\n", braced(body, indent)),
        Stmt::Break(l) => jump("break", l),
        Stmt::Continue(l) => jump("continue", l),
        Stmt::Return(None) => format!("{p}return;\n"),
        Stmt::Return(Some(v)) => format!("{p}return {};\n", layout_t(v, indent, indent + 7, 1)),
        Stmt::Static { name, ty, value } => {
            let head = format!("static {name}: {ty} = ");
            let col = indent + width(&head);
            format!("{p}{head}{};\n", layout_t(value, indent, col, 1))
        }
        Stmt::Block(b) => format!("{p}{}\n", braced(b, indent)),
    }
}
