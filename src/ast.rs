use crate::span::Span;

/// Unique identifier for a variable in the AST
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VarId(pub usize);

/// Unique identifier for a function in the AST
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FuncId(pub usize);

/// A path expression (e.g., `a.b.c`)
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub segments: Vec<String>,
    pub span: Span,
}

/// Types in our simplified Rust
#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    Unit,
    Bool,
    Int,
    String,
    Ref(Box<Ty>),
    MutRef(Box<Ty>),
    Vec(Box<Ty>),
    Custom(String),
}

/// Patterns in let bindings
#[derive(Clone, Debug, PartialEq)]
pub enum Pattern {
    Identifier(String, Span),
    Wildcard(Span),
}

/// Expressions in our simplified Rust
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    /// Boolean literal: `true`, `false`
    Bool(bool, Span),
    /// Integer literal: `42`
    Int(i64, Span),
    /// String literal: `"hello"`
    String(String, Span),
    /// Unit literal: `()`
    Unit(Span),
    /// Variable or path reference: `x`, `foo.bar`
    Path(Path),
    /// Reference expression: `&expr`, `&mut expr`
    Ref {
        expr: Box<Expr>,
        mutable: bool,
        span: Span,
    },
    /// Dereference: `*expr`
    Deref {
        expr: Box<Expr>,
        span: Span,
    },
    /// Binary operation
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
        span: Span,
    },
    /// Function call: `foo(a, b, c)`
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    /// Method call: `obj.method(a, b)`
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        args: Vec<Expr>,
        span: Span,
    },
    /// Assignment: `x = expr`
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },
    /// Block expression: `{ stmt; stmt; expr }`
    Block {
        stmts: Vec<Stmt>,
        tail: Option<Box<Expr>>,
        span: Span,
    },
    /// If expression: `if cond { block } else { block }`
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
        span: Span,
    },
    /// While loop: `while cond { block }`
    While {
        cond: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },
    /// Return expression: `return expr`
    Return {
        expr: Option<Box<Expr>>,
        span: Span,
    },
    /// Clone expression: `expr.clone()` (explicit copy)
    Clone {
        expr: Box<Expr>,
        span: Span,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

/// Statements in our simplified Rust
#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    /// Let binding: `let pat: ty = expr;`
    Let {
        pattern: Pattern,
        ty: Option<Ty>,
        init: Option<Expr>,
        span: Span,
    },
    /// Expression statement: `expr;`
    Expr(Expr),
}

/// Function parameter: `name: ty`
#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Ty,
    pub span: Span,
}

/// Function declaration
#[derive(Clone, Debug, PartialEq)]
pub struct Func {
    pub name: String,
    pub params: Vec<Param>,
    pub ret_ty: Option<Ty>,
    pub body: Expr,
    pub span: Span,
}

/// A complete program
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub functions: Vec<Func>,
    pub span: Span,
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Bool(_, s) => *s,
            Expr::Int(_, s) => *s,
            Expr::String(_, s) => *s,
            Expr::Unit(s) => *s,
            Expr::Path(p) => p.span,
            Expr::Ref { span, .. } => *span,
            Expr::Deref { span, .. } => *span,
            Expr::Binary { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::MethodCall { span, .. } => *span,
            Expr::Assign { span, .. } => *span,
            Expr::Block { span, .. } => *span,
            Expr::If { span, .. } => *span,
            Expr::While { span, .. } => *span,
            Expr::Return { span, .. } => *span,
            Expr::Clone { span, .. } => *span,
        }
    }
}

impl Pattern {
    pub fn name(&self) -> Option<&str> {
        match self {
            Pattern::Identifier(name, _) => Some(name),
            Pattern::Wildcard(_) => None,
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Pattern::Identifier(_, s) => *s,
            Pattern::Wildcard(s) => *s,
        }
    }
}
