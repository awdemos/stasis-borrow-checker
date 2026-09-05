use std::collections::{HashMap, HashSet};

use crate::ast::{BinOp, Expr, Func, Program, Stmt};
use crate::borrow_graph::{Borrow, BorrowGraph};
use crate::error::{BorrowError, Result};
use crate::ownership::{OwnershipState, Scope};
use crate::span::Span;

/// The borrow checker — validates ownership, borrowing, and move semantics
/// for a simplified Rust subset.
///
/// After construction, call [`check_program`](Self::check_program) or
/// [`check_source`](Self::check_source) to run the checker against AST or
/// source text, respectively.
///
/// # Errors
///
/// Returns a [`BorrowError`] describing the first rule violation found.
///
/// # Example
///
/// ```
/// use borrow_checker::BorrowChecker;
///
/// let mut checker = BorrowChecker::new();
/// let result = checker.check_source(
///     "fn main() {
///         let x = 42;
///         let r1 = &x;
///         let r2 = &x;  // two immutable borrows — OK
///     }"
/// );
/// assert!(result.is_ok());
///
/// let result = checker.check_source(
///     "fn main() {
///         let x = \"hello\";
///         let r = &x;
///         let y = x;     // ERROR: can't move while borrowed
///     }"
/// );
/// assert!(result.is_err());
/// ```
pub struct BorrowChecker {
    /// The current ownership state of all variables in scope.
    pub scope: Scope,
    /// The graph of active borrow relationships.
    pub borrow_graph: BorrowGraph,
    temp_counter: usize,
    block_locals: Vec<Vec<String>>,
    copy_types: HashSet<String>,
    var_types: HashMap<String, String>,
}

impl BorrowChecker {
    /// Create a new borrow checker with an empty scope and no borrows.
    pub fn new() -> Self {
        Self {
            scope: Scope::new(),
            borrow_graph: BorrowGraph::new(),
            temp_counter: 0,
            block_locals: Vec::new(),
            copy_types: ["int", "bool", "()"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            var_types: HashMap::new(),
        }
    }

    /// Parse `source` as a program, then run the borrow checker on it.
    ///
    /// This is the simplest entry point — give it Rust source text and
    /// get back a [`Result`].
    ///
    /// ```
    /// # use borrow_checker::BorrowChecker;
    /// let mut checker = BorrowChecker::new();
    /// assert!(checker.check_source("fn main() { let x = 42; }").is_ok());
    /// ```
    pub fn check_source(&mut self, source: &str) -> Result<()> {
        let mut parser = crate::parser::Parser::new(source);
        let program = parser.parse()?;
        self.check_program(&program)
    }

    /// Check an already-parsed [`Program`] AST.
    ///
    /// Validates every function in the program against the core
    /// ownership and borrowing rules.
    pub fn check_program(&mut self, program: &Program) -> Result<()> {
        for func in &program.functions {
            self.check_function(func)?;
        }
        Ok(())
    }

    /// Check a single function definition.
    ///
    /// Resets the checker state (scope, borrows, variable types) and
    /// validates all statements and expressions in the function body.
    pub fn check_function(&mut self, func: &Func) -> Result<()> {
        self.scope = Scope::new();
        self.borrow_graph = BorrowGraph::new();
        self.temp_counter = 0;
        self.block_locals.clear();
        self.var_types.clear();

        for param in &func.params {
            self.scope
                .declare(param.name.clone(), OwnershipState::Owned);
            let ty_name = match &param.ty {
                crate::ast::Ty::Bool => "bool",
                crate::ast::Ty::Int => "int",
                crate::ast::Ty::Unit => "()",
                crate::ast::Ty::String => "string",
                _ => "unknown",
            };
            self.var_types
                .insert(param.name.clone(), ty_name.to_string());
        }

        self.check_expr(&func.body)?;
        Ok(())
    }

    /// Check a single statement for ownership/borrowing rule violations.
    pub fn check_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Let {
                pattern,
                ty,
                init,
                span,
            } => {
                if let Some(init_expr) = init {
                    self.check_expr(init_expr)?;

                    if let Expr::Ref {
                        expr,
                        mutable,
                        span: ref_span,
                    } = init_expr
                    {
                        if let Expr::Path(path) = expr.as_ref() {
                            let borrower_name = pattern.name().unwrap_or("_").to_string();
                            self.register_borrow(
                                &borrower_name,
                                &path.segments[0],
                                *mutable,
                                *ref_span,
                            )?;
                        }
                    }

                    let var_name = pattern.name();
                    let declared_type = ty.as_ref().map(|t| type_to_name(t));
                    let inferred_type = declared_type.or_else(|| infer_type(init_expr));

                    if let Some(ref name) = var_name {
                        if let Some(t) = inferred_type {
                            self.var_types.insert(name.to_string(), t);
                        }
                    }

                    if !self.is_var_copy_type(init_expr) {
                        if let Expr::Path(path) = init_expr {
                            self.try_move(&path.segments[0], *span)?;
                        }
                    }
                }

                if let Some(name) = pattern.name() {
                    self.scope.declare(name.to_string(), OwnershipState::Owned);
                    if let Some(bl) = self.block_locals.last_mut() {
                        bl.push(name.to_string());
                    }
                }

                Ok(())
            }
            Stmt::Expr(expr) => self.check_expr(expr),
        }
    }

    /// Check a single expression for ownership/borrowing rule violations.
    ///
    /// Recursively walks the expression tree, enforcing:
    /// - Use-after-move: reading a moved value
    /// - Borrow rules: mutable/immutable overlap, multiple mutable borrows
    /// - Assign-while-borrowed: writing to a borrowed variable
    /// - Move-while-borrowed: moving a value with active references
    pub fn check_expr(&mut self, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Bool(_, _) | Expr::Int(_, _) | Expr::String(_, _) | Expr::Unit(_) => Ok(()),

            Expr::Path(path) => {
                let var = &path.segments[0];
                match self.scope.get(var) {
                    None => Err(BorrowError::VariableNotFound {
                        name: var.clone(),
                        span: path.span,
                    }),
                    Some(OwnershipState::Moved { .. }) => Err(BorrowError::UseAfterMove {
                        name: var.clone(),
                        span: path.span,
                    }),
                    _ => Ok(()),
                }
            }

            Expr::Ref {
                expr: inner,
                mutable,
                span,
            } => {
                self.check_expr(inner)?;
                match inner.as_ref() {
                    Expr::Path(path) => {
                        let var = &path.segments[0];
                        self.validate_borrow(var, *mutable, *span)?;
                        let borrower = self.fresh_temp();
                        self.register_borrow(&borrower, var, *mutable, *span)
                    }
                    Expr::Deref {
                        expr: deref_inner, ..
                    } => {
                        if let Expr::Path(p) = deref_inner.as_ref() {
                            self.validate_borrow(&p.segments[0], *mutable, *span)?;
                            let borrower = self.fresh_temp();
                            self.register_borrow(&borrower, &p.segments[0], *mutable, *span)
                        } else {
                            Ok(())
                        }
                    }
                    _ => Ok(()),
                }
            }

            Expr::Deref { expr: inner, .. } => self.check_expr(inner),

            Expr::Binary {
                left, right, op: _, ..
            } => {
                self.check_expr(left)?;
                self.check_expr(right)?;
                if !self.is_var_copy_type(left) {
                    self.try_move_expr(left)?;
                }
                if !self.is_var_copy_type(right) {
                    self.try_move_expr(right)?;
                }
                Ok(())
            }

            Expr::Call {
                func,
                args,
                span: _,
            } => {
                self.check_expr(func)?;
                if let Expr::MethodCall {
                    receiver, method, ..
                } = func.as_ref()
                {
                    if method == "clone" {
                        return self.check_expr(receiver);
                    }
                }
                for arg in args {
                    self.check_expr(arg)?;
                    self.try_move_expr(arg)?;
                }
                Ok(())
            }

            Expr::MethodCall {
                receiver,
                method,
                args,
                span: _,
            } => {
                self.check_expr(receiver)?;
                if method == "clone" {
                    return Ok(());
                }
                self.try_move_expr(receiver)?;
                for arg in args {
                    self.check_expr(arg)?;
                    self.try_move_expr(arg)?;
                }
                Ok(())
            }

            Expr::Assign {
                target,
                value,
                span,
            } => {
                if let Expr::Path(path) = target.as_ref() {
                    let var = &path.segments[0];
                    if self.borrow_graph.is_borrowed(var) {
                        return Err(BorrowError::AssignWhileBorrowed {
                            name: var.clone(),
                            span: *span,
                        });
                    }
                    match self.scope.get(var) {
                        Some(OwnershipState::Moved { .. }) => {
                            return Err(BorrowError::UseAfterMove {
                                name: var.clone(),
                                span: *span,
                            });
                        }
                        _ => {}
                    }
                }
                self.check_expr(value)?;
                if let Expr::Path(path) = target.as_ref() {
                    let var = &path.segments[0];
                    if let Some(state) = self.scope.get_mut(var) {
                        *state = OwnershipState::Owned;
                    }
                }
                Ok(())
            }

            Expr::Block { stmts, tail, span } => self.check_block(stmts, tail.as_deref(), *span),

            Expr::If {
                cond,
                then_branch,
                else_branch,
                span: _,
            } => {
                self.check_expr(cond)?;
                self.check_if(then_branch, else_branch.as_deref())
            }

            Expr::While { cond, body, .. } => {
                self.check_expr(cond)?;
                self.check_while(body)
            }

            Expr::Return { expr, .. } => {
                if let Some(e) = expr {
                    self.check_expr(e)?;
                    self.try_move_expr(e)?;
                }
                Ok(())
            }

            Expr::Clone { expr, .. } => self.check_expr(expr),
        }
    }

    fn check_block(&mut self, stmts: &[Stmt], tail: Option<&Expr>, _span: Span) -> Result<()> {
        let existing_borrowers: HashSet<String> =
            self.borrow_graph.borrows.keys().cloned().collect();
        self.block_locals.push(Vec::new());

        for stmt in stmts {
            self.check_stmt(stmt)?;
        }
        if let Some(expr) = tail {
            self.check_expr(expr)?;
        }

        let local_vars = self.block_locals.pop().unwrap_or_default();
        for var in &local_vars {
            self.scope.variables.swap_remove(var);
            self.borrow_graph.remove_borrows_by(var);
            self.var_types.remove(var);
        }

        let new_borrowers: Vec<String> = self
            .borrow_graph
            .borrows
            .keys()
            .filter(|k| !existing_borrowers.contains(*k))
            .cloned()
            .collect();
        for borrower in new_borrowers {
            self.borrow_graph.remove_borrows_by(&borrower);
        }

        Ok(())
    }

    fn check_if(&mut self, then_branch: &Expr, else_branch: Option<&Expr>) -> Result<()> {
        let then_scope = self.scope.clone();
        let then_graph = self.borrow_graph.clone();
        let then_types = self.var_types.clone();
        let mut then_checker = BorrowChecker {
            scope: then_scope,
            borrow_graph: then_graph,
            temp_counter: self.temp_counter,
            block_locals: Vec::new(),
            copy_types: self.copy_types.clone(),
            var_types: then_types,
        };
        then_checker.check_expr(then_branch)?;

        let else_scope = self.scope.clone();
        let else_graph = self.borrow_graph.clone();
        let else_types = self.var_types.clone();
        let mut else_checker = BorrowChecker {
            scope: else_scope,
            borrow_graph: else_graph,
            temp_counter: self.temp_counter,
            block_locals: Vec::new(),
            copy_types: self.copy_types.clone(),
            var_types: else_types,
        };
        if let Some(else_expr) = else_branch {
            else_checker.check_expr(else_expr)?;
        }

        self.temp_counter = then_checker.temp_counter.max(else_checker.temp_counter);

        let merged = then_checker.scope.merge(else_checker.scope);

        for (name, state) in merged.variables {
            if self.scope.variables.contains_key(&name) {
                self.scope.variables.insert(name, state);
            }
        }

        self.borrow_graph = BorrowGraph::new();
        for (name, state) in &self.scope.variables {
            if matches!(state, OwnershipState::Moved { .. }) {
                self.borrow_graph.record_move(name);
            }
        }

        Ok(())
    }

    fn check_while(&mut self, body: &Expr) -> Result<()> {
        self.check_expr(body)?;
        Ok(())
    }

    fn is_type_copy(&self, ty_name: &str) -> bool {
        self.copy_types.contains(ty_name)
    }

    fn is_var_copy_type(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Bool(..) | Expr::Int(..) | Expr::Unit(..) => true,
            Expr::Path(path) => {
                let var = &path.segments[0];
                self.var_types
                    .get(var)
                    .map(|t| self.is_type_copy(t))
                    .unwrap_or(false)
            }
            Expr::Binary { op, .. }
                if matches!(
                    op,
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
                ) =>
            {
                true
            }
            _ => false,
        }
    }

    fn try_move(&mut self, var: &str, span: Span) -> Result<()> {
        if self.borrow_graph.is_borrowed(var) {
            return Err(BorrowError::MoveWhileBorrowed {
                name: var.to_string(),
                span,
            });
        }

        match self.scope.get(var) {
            None => {
                return Err(BorrowError::VariableNotFound {
                    name: var.to_string(),
                    span,
                });
            }
            Some(OwnershipState::Moved { .. }) => {
                return Err(BorrowError::UseAfterMove {
                    name: var.to_string(),
                    span,
                });
            }
            _ => {}
        }

        if let Some(state) = self.scope.get_mut(var) {
            *state = OwnershipState::Moved { moved_at: span };
        }
        self.borrow_graph.record_move(var);

        Ok(())
    }

    fn try_move_expr(&mut self, expr: &Expr) -> Result<()> {
        if self.is_var_copy_type(expr) {
            return Ok(());
        }
        match expr {
            Expr::Path(path) => self.try_move(&path.segments[0], path.span),
            Expr::Clone { .. } => Ok(()),
            _ => Ok(()),
        }
    }

    fn validate_borrow(&self, var: &str, mutable: bool, span: Span) -> Result<()> {
        match self.scope.get(var) {
            None => {
                return Err(BorrowError::VariableNotFound {
                    name: var.to_string(),
                    span,
                });
            }
            Some(OwnershipState::Moved { .. }) => {
                return Err(BorrowError::BorrowAfterMove {
                    name: var.to_string(),
                    span,
                });
            }
            _ => {}
        }

        if mutable {
            if self.borrow_graph.has_mutable_borrows(var) {
                return Err(BorrowError::MultipleMutableBorrows {
                    name: var.to_string(),
                    span,
                });
            }
            if self.borrow_graph.is_immutably_borrowed(var) {
                return Err(BorrowError::MutableBorrowWhileImmutable {
                    name: var.to_string(),
                    span,
                });
            }
        } else if self.borrow_graph.has_mutable_borrows(var) {
            return Err(BorrowError::ImmutableBorrowWhileMutable {
                name: var.to_string(),
                span,
            });
        }

        Ok(())
    }

    fn register_borrow(
        &mut self,
        borrower: &str,
        borrowed: &str,
        mutable: bool,
        span: Span,
    ) -> Result<()> {
        let borrow = Borrow {
            borrower: borrower.to_string(),
            borrowed: borrowed.to_string(),
            mutable,
            span,
        };
        self.borrow_graph.add_borrow(borrow);

        match self.scope.get_mut(borrowed) {
            Some(state) => {
                if mutable {
                    *state = OwnershipState::MutablyBorrowed { borrow_span: span };
                } else {
                    *state = OwnershipState::ImmutablyBorrowed {
                        borrow_spans: vec![span],
                    };
                }
            }
            None => {
                return Err(BorrowError::VariableNotFound {
                    name: borrowed.to_string(),
                    span,
                });
            }
        }

        Ok(())
    }

    fn fresh_temp(&mut self) -> String {
        self.temp_counter += 1;
        format!("__temp_{}", self.temp_counter)
    }
}

fn type_to_name(ty: &crate::ast::Ty) -> String {
    match ty {
        crate::ast::Ty::Bool => "bool".into(),
        crate::ast::Ty::Int => "int".into(),
        crate::ast::Ty::Unit => "()".into(),
        crate::ast::Ty::String => "string".into(),
        crate::ast::Ty::Ref(_) => "ref".into(),
        crate::ast::Ty::MutRef(_) => "mut_ref".into(),
        crate::ast::Ty::Vec(_) => "vec".into(),
        crate::ast::Ty::Custom(s) => s.clone(),
    }
}

fn infer_type(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Bool(..) => Some("bool".into()),
        Expr::Int(..) => Some("int".into()),
        Expr::String(..) => Some("string".into()),
        Expr::Unit(..) => Some("()".into()),
        Expr::Clone { expr: inner, .. } => infer_type(inner),
        _ => None,
    }
}
