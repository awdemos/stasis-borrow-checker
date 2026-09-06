use crate::ast::{BinOp, Expr, Func, Param, Pattern, Program, Stmt, Ty};
use crate::error::{BorrowError, Result};
use crate::span::Span;

/// A hand-written recursive descent parser for the simplified Rust subset.
///
/// Supports:
/// - Function definitions (`fn foo(x: int) -> bool { ... }`)
/// - `let` bindings with optional type annotations
/// - Blocks, `if`/`else`, `while`, `return`
/// - Reference (`&`, `&mut`) and dereference (`*`) expressions
/// - Method calls and `.clone()` chaining
/// - Full operator precedence (assignment < `||` < `&&` < comparison
///   < equality < additive < multiplicative < unary < postfix < primary)
///
/// # Errors
///
/// Returns [`BorrowError::ParseError`] on invalid syntax with a
/// [`Span`] pointing at the problem location.
pub struct Parser {
    input: Vec<char>,
    pos: usize,
}

impl Parser {
    /// Create a new parser for the given source text.
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    /// Parse the full source text into a [`Program`].
    ///
    /// Consumes all input until EOF. The program may contain zero
    /// or more function definitions.
    pub fn parse(&mut self) -> Result<Program> {
        let start = self.pos;
        let mut functions = Vec::new();
        self.skip_ws();
        while self.pos < self.input.len() {
            functions.push(self.parse_function()?);
            self.skip_ws();
        }
        Ok(Program {
            functions,
            span: Span::new(start, self.pos),
        })
    }

    fn parse_function(&mut self) -> Result<Func> {
        let start = self.pos;
        self.expect_kw("fn")?;
        let name = self.expect_ident()?;
        self.expect_char('(')?;
        let mut params = Vec::new();
        if self.peek() != Some(')') {
            params = self.parse_params()?;
        }
        self.expect_char(')')?;
        let ret_ty = if self.eat_str("->") {
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Ok(Func {
            name,
            params,
            ret_ty,
            body,
            span: Span::new(start, self.pos),
        })
    }

    fn parse_params(&mut self) -> Result<Vec<Param>> {
        let mut params = Vec::new();
        loop {
            let start = self.pos;
            let name = self.expect_ident()?;
            self.expect_char(':')?;
            let ty = self.parse_type()?;
            params.push(Param {
                name,
                ty,
                span: Span::new(start, self.pos),
            });
            if self.eat_char(',') {
                continue;
            }
            break;
        }
        Ok(params)
    }

    fn parse_type(&mut self) -> Result<Ty> {
        if self.eat_char('&') {
            let mutable = self.eat_kw("mut");
            let inner = self.parse_type()?;
            Ok(if mutable {
                Ty::MutRef(Box::new(inner))
            } else {
                Ty::Ref(Box::new(inner))
            })
        } else if self.eat_char('(') {
            self.expect_char(')')?;
            Ok(Ty::Unit)
        } else if self.eat_str("Vec") {
            self.expect_char('<')?;
            let inner = self.parse_type()?;
            self.expect_char('>')?;
            Ok(Ty::Vec(Box::new(inner)))
        } else {
            let ident = self.expect_ident()?;
            match ident.as_str() {
                "bool" => Ok(Ty::Bool),
                "int" => Ok(Ty::Int),
                "string" => Ok(Ty::String),
                _ => Ok(Ty::Custom(ident)),
            }
        }
    }

    fn parse_block(&mut self) -> Result<Expr> {
        let start = self.pos;
        self.expect_char('{')?;
        let mut stmts = Vec::new();
        loop {
            self.skip_ws();
            if self.peek() == Some('}') || self.pos >= self.input.len() {
                break;
            }
            if self.starts_with("let") {
                stmts.push(self.parse_let_stmt()?);
            } else {
                let expr = self.parse_expr()?;
                if self.eat_char(';') {
                    stmts.push(Stmt::Expr(expr));
                } else {
                    self.skip_ws();
                    if self.peek() == Some('}') {
                        self.bump();
                        return Ok(Expr::Block {
                            stmts,
                            tail: Some(Box::new(expr)),
                            span: Span::new(start, self.pos),
                        });
                    }
                    stmts.push(Stmt::Expr(expr));
                }
            }
        }
        self.expect_char('}')?;
        Ok(Expr::Block {
            stmts,
            tail: None,
            span: Span::new(start, self.pos),
        })
    }

    fn starts_with(&self, s: &str) -> bool {
        let chars: Vec<char> = s.chars().collect();
        if self.pos + chars.len() > self.input.len() {
            return false;
        }
        for (i, &c) in chars.iter().enumerate() {
            if self.input[self.pos + i] != c {
                return false;
            }
        }
        if s.chars().all(|c| c.is_alphanumeric() || c == '_')
            && self.pos + chars.len() < self.input.len()
                && self.input[self.pos + chars.len()].is_alphanumeric()
            {
                return false;
            }
        true
    }

    fn parse_let_stmt(&mut self) -> Result<Stmt> {
        let start = self.pos;
        self.expect_kw("let")?;
        let pattern = if self.eat_char('_') {
            Pattern::Wildcard(Span::new(self.pos - 1, self.pos))
        } else {
            let name = self.expect_ident()?;
            let len = name.len();
            Pattern::Identifier(name, Span::new(self.pos - len, self.pos))
        };
        let ty = if self.eat_char(':') {
            Some(self.parse_type()?)
        } else {
            None
        };
        let init = if self.eat_char('=') {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect_char(';')?;
        Ok(Stmt::Let {
            pattern,
            ty,
            init,
            span: Span::new(start, self.pos),
        })
    }

    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_assign()
    }

    fn parse_assign(&mut self) -> Result<Expr> {
        let lhs = self.parse_or()?;
        self.skip_ws();
        if self.peek() == Some('=')
            && self.pos + 1 < self.input.len()
            && self.input[self.pos + 1] != '='
        {
            self.bump();
            let rhs = self.parse_assign()?;
            let span = lhs.span().merge(rhs.span());
            Ok(Expr::Assign {
                target: Box::new(lhs),
                value: Box::new(rhs),
                span,
            })
        } else {
            Ok(lhs)
        }
    }

    fn parse_or(&mut self) -> Result<Expr> {
        let mut left = self.parse_and()?;
        while self.eat_str("||") {
            let right = self.parse_and()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::Or,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr> {
        let mut left = self.parse_equality()?;
        while self.eat_str("&&") {
            let right = self.parse_equality()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::And,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = if self.eat_str("==") {
                BinOp::Eq
            } else if self.eat_str("!=") {
                BinOp::Ne
            } else {
                break;
            };
            let right = self.parse_comparison()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr> {
        let mut left = self.parse_additive()?;
        loop {
            let op = if self.eat_str("<=") {
                BinOp::Le
            } else if self.eat_str(">=") {
                BinOp::Ge
            } else if self.eat_char('<') {
                BinOp::Lt
            } else if self.eat_char('>') {
                BinOp::Gt
            } else {
                break;
            };
            let right = self.parse_additive()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = if self.eat_char('+') {
                BinOp::Add
            } else if self.eat_char('-') {
                BinOp::Sub
            } else {
                break;
            };
            let right = self.parse_multiplicative()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = if self.eat_char('*') {
                BinOp::Mul
            } else if self.eat_char('/') {
                BinOp::Div
            } else {
                break;
            };
            let right = self.parse_unary()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        if self.eat_char('&') {
            let mutable = self.eat_kw("mut");
            let inner = self.parse_unary()?;
            let span = inner.span();
            Ok(Expr::Ref {
                expr: Box::new(inner),
                mutable,
                span,
            })
        } else if self.eat_char('*') {
            let inner = self.parse_unary()?;
            let span = inner.span();
            Ok(Expr::Deref {
                expr: Box::new(inner),
                span,
            })
        } else {
            self.parse_postfix()
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.eat_char('.') {
                let method = self.expect_ident()?;
                if self.eat_char('(') {
                    let mut args = Vec::new();
                    if self.peek() != Some(')') {
                        args.push(self.parse_expr()?);
                        while self.eat_char(',') {
                            args.push(self.parse_expr()?);
                        }
                    }
                    self.expect_char(')')?;
                    if method == "clone" {
                        expr = Expr::Clone {
                            expr: Box::new(expr),
                            span: Span::new(self.pos - 1, self.pos),
                        };
                    } else {
                        expr = Expr::MethodCall {
                            receiver: Box::new(expr),
                            method,
                            args,
                            span: Span::new(self.pos - 1, self.pos),
                        };
                    }
                } else {
                    return Err(BorrowError::ParseError {
                        message: "unexpected field access".into(),
                        span: Span::new(self.pos, self.pos),
                    });
                }
            } else if self.eat_char('(') && matches!(expr, Expr::Path(_) | Expr::MethodCall { .. })
            {
                let mut args = Vec::new();
                if self.peek() != Some(')') {
                    args.push(self.parse_expr()?);
                    while self.eat_char(',') {
                        args.push(self.parse_expr()?);
                    }
                }
                self.expect_char(')')?;
                expr = Expr::Call {
                    func: Box::new(expr),
                    args,
                    span: Span::new(self.pos - 1, self.pos),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        self.skip_ws();
        let start = self.pos;
        match self.peek() {
            Some('0'..='9') => {
                let mut s = String::new();
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() || c == '_' {
                        s.push(c);
                        self.bump();
                    } else {
                        break;
                    }
                }
                let val = s.replace('_', "").parse::<i64>().unwrap_or(0);
                Ok(Expr::Int(val, Span::new(start, self.pos)))
            }
            Some('"') => {
                self.bump();
                let mut s = String::new();
                loop {
                    match self.bump() {
                        '\\' => match self.bump() {
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            '"' => s.push('"'),
                            '\\' => s.push('\\'),
                            c => s.push(c),
                        },
                        '"' => break,
                        c => s.push(c),
                    }
                }
                Ok(Expr::String(s, Span::new(start, self.pos)))
            }
            Some('{') => self.parse_block(),
            Some('(') => {
                self.bump();
                let expr = self.parse_expr()?;
                self.expect_char(')')?;
                Ok(expr)
            }
            Some(c) if c.is_alphabetic() || c == '_' => {
                let ident = self.expect_ident()?;
                match ident.as_str() {
                    "true" => Ok(Expr::Bool(true, Span::new(start, self.pos))),
                    "false" => Ok(Expr::Bool(false, Span::new(start, self.pos))),
                    "if" => self.parse_if(start),
                    "while" => self.parse_while(start),
                    "return" => self.parse_return(start),
                    _ => Ok(Expr::Path(crate::ast::Path {
                        segments: vec![ident],
                        span: Span::new(start, self.pos),
                    })),
                }
            }
            Some(c) => Err(BorrowError::ParseError {
                message: format!("unexpected character '{}'", c),
                span: Span::new(self.pos, self.pos + 1),
            }),
            None => Err(BorrowError::ParseError {
                message: "unexpected end of input".into(),
                span: Span::new(self.pos, self.pos),
            }),
        }
    }

    fn parse_if(&mut self, start: usize) -> Result<Expr> {
        let cond = self.parse_expr()?;
        let then_branch = self.parse_block()?;
        let else_branch = if self.eat_kw("else") {
            Some(Box::new(self.parse_block()?))
        } else {
            None
        };
        Ok(Expr::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch,
            span: Span::new(start, self.pos),
        })
    }

    fn parse_while(&mut self, start: usize) -> Result<Expr> {
        let cond = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(Expr::While {
            cond: Box::new(cond),
            body: Box::new(body),
            span: Span::new(start, self.pos),
        })
    }

    fn parse_return(&mut self, start: usize) -> Result<Expr> {
        let expr = if self
            .peek()
            .is_some_and(|c| c != ';' && c != '}' && c != ')')
        {
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };
        Ok(Expr::Return {
            expr,
            span: Span::new(start, self.pos),
        })
    }

    // Lexer helpers

    fn skip_ws(&mut self) {
        while self.pos < self.input.len() {
            let c = self.input[self.pos];
            if c == '/' && self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '/' {
                self.pos += 2;
                while self.pos < self.input.len() && self.input[self.pos] != '\n' {
                    self.pos += 1;
                }
                continue;
            }
            if !c.is_whitespace() {
                break;
            }
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn bump(&mut self) -> char {
        let c = self.input[self.pos];
        self.pos += 1;
        c
    }

    fn eat_char(&mut self, expected: char) -> bool {
        self.skip_ws();
        if self.peek() == Some(expected) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn eat_str(&mut self, s: &str) -> bool {
        self.skip_ws();
        let chars: Vec<char> = s.chars().collect();
        if self.pos + chars.len() > self.input.len() {
            return false;
        }
        for (i, &c) in chars.iter().enumerate() {
            if self.input[self.pos + i] != c {
                return false;
            }
        }
        if s.chars().all(|c| c.is_alphanumeric() || c == '_')
            && self.pos + chars.len() < self.input.len()
                && self.input[self.pos + chars.len()].is_alphanumeric()
            {
                return false;
            }
        self.pos += chars.len();
        true
    }

    fn eat_kw(&mut self, kw: &str) -> bool {
        self.eat_str(kw)
    }

    fn expect_char(&mut self, expected: char) -> Result<()> {
        self.skip_ws();
        match self.peek() {
            Some(c) if c == expected => {
                self.bump();
                Ok(())
            }
            Some(c) => Err(BorrowError::ParseError {
                message: format!("expected '{}', got '{}'", expected, c),
                span: Span::new(self.pos, self.pos + 1),
            }),
            None => Err(BorrowError::ParseError {
                message: format!("expected '{}', got EOF", expected),
                span: Span::new(self.pos, self.pos),
            }),
        }
    }

    fn expect_kw(&mut self, kw: &str) -> Result<()> {
        self.skip_ws();
        let chars: Vec<char> = kw.chars().collect();
        if self.pos + chars.len() > self.input.len() {
            return Err(BorrowError::ParseError {
                message: format!("expected '{}', got EOF", kw),
                span: Span::new(self.pos, self.pos),
            });
        }
        for (i, &c) in chars.iter().enumerate() {
            if self.input[self.pos + i] != c {
                let found: String = self.input[self.pos..self.pos + chars.len()]
                    .iter()
                    .collect();
                return Err(BorrowError::ParseError {
                    message: format!("expected '{}', got '{}'", kw, found),
                    span: Span::new(self.pos, self.pos + chars.len()),
                });
            }
        }
        if self.pos + chars.len() < self.input.len()
            && self.input[self.pos + chars.len()].is_alphanumeric()
        {
            let found: String = self.input[self.pos..self.pos + chars.len() + 1]
                .iter()
                .collect();
            return Err(BorrowError::ParseError {
                message: format!("expected '{}', got '{}'", kw, found),
                span: Span::new(self.pos, self.pos + chars.len()),
            });
        }
        self.pos += chars.len();
        Ok(())
    }

    fn expect_ident(&mut self) -> Result<String> {
        self.skip_ws();
        let start = self.pos;
        match self.peek() {
            Some(c) if c.is_alphabetic() || c == '_' => {
                self.bump();
                while let Some(c) = self.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        self.bump();
                    } else {
                        break;
                    }
                }
                Ok(self.input[start..self.pos].iter().collect())
            }
            Some(c) => Err(BorrowError::ParseError {
                message: format!("expected identifier, got '{}'", c),
                span: Span::new(self.pos, self.pos + 1),
            }),
            None => Err(BorrowError::ParseError {
                message: "expected identifier, got EOF".into(),
                span: Span::new(self.pos, self.pos),
            }),
        }
    }
}
