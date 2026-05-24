use crate::span::Span;
use thiserror::Error;

/// Errors produced by the parser and borrow checker.
///
/// Each variant carries the relevant variable name and
/// a [`Span`] pointing at the offending source location.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum BorrowError {
    /// Reading a value that has already been moved.
    #[error("use of moved value: `{name}`")]
    UseAfterMove { name: String, span: Span },

    /// Moving a value while it is still borrowed.
    #[error("cannot move out of `{name}` because it is borrowed")]
    MoveWhileBorrowed { name: String, span: Span },

    /// Taking a second mutable borrow of a variable.
    #[error("cannot borrow `{name}` as mutable more than once at a time")]
    MultipleMutableBorrows { name: String, span: Span },

    /// Taking a mutable borrow while there is an active immutable borrow.
    #[error("cannot borrow `{name}` as mutable because it is also borrowed as immutable")]
    MutableBorrowWhileImmutable { name: String, span: Span },

    /// Taking an immutable borrow while there is an active mutable borrow.
    #[error("cannot borrow `{name}` as immutable because it is also borrowed as mutable")]
    ImmutableBorrowWhileMutable { name: String, span: Span },

    /// Borrowing a value that has already been moved.
    #[error("borrow of moved value: `{name}`")]
    BorrowAfterMove { name: String, span: Span },

    /// Assigning to a variable that is currently borrowed.
    #[error("cannot assign to `{name}` because it is borrowed")]
    AssignWhileBorrowed { name: String, span: Span },

    /// Attempting to move out of a shared (`&`) reference.
    #[error("cannot move out of a shared reference")]
    MoveOutOfSharedRef { span: Span },

    /// Attempting to move out of a mutable (`&mut`) reference.
    #[error("cannot move out of a mutable reference")]
    MoveOutOfMutRef { span: Span },

    /// A value is used after it has been consumed by a move.
    #[error("value used after it has been moved")]
    ValueUsedAfterMove { span: Span },

    /// Reference to a variable that does not exist in scope.
    #[error("variable `{name}` not found")]
    VariableNotFound { name: String, span: Span },

    /// Call to a function that has not been defined.
    #[error("function `{name}` not found")]
    FunctionNotFound { name: String, span: Span },

    /// A borrow outlives the borrowed value's scope.
    #[error("lifetime mismatch: borrowed value does not live long enough")]
    LifetimeMismatch { span: Span },

    /// The input could not be parsed.
    #[error("parse error: {message}")]
    ParseError { message: String, span: Span },
}

impl BorrowError {
    /// Format this error with a visual caret pointing at the offending
    /// source location within `source`.
    ///
    /// ```
    /// # use borrow_checker::error::BorrowError;
    /// # use borrow_checker::span::Span;
    /// let err = BorrowError::UseAfterMove {
    ///     name: "x".into(),
    ///     span: Span::new(12, 13),
    /// };
    /// let msg = err.format_with_source("fn main() { let x = 1; let y = x; let z = x; }");
    /// assert!(msg.contains("use of moved value"));
    /// assert!(msg.contains("--> line 1:13"));
    /// ```
    pub fn format_with_source(&self, source: &str) -> String {
        let span = self.span();
        let (line_num, col, line) = Self::locate(source, span.start);

        // Cap underline at a reasonable visual width — the variable name
        // is usually what the error is about, not the whole expression.
        let underline_len = (span.end.saturating_sub(span.start)).max(1).min(20);
        let padding = " ".repeat(col.saturating_sub(1));
        let carets = "^".repeat(underline_len);

        format!(
            "  --> line {}:{}\n{:4} |\n{:4} | {}\n     | {}{} {}\n",
            line_num, col, "", line_num, line, padding, carets, self,
        )
    }

    /// Extract the [`Span`] from whichever variant this error is.
    pub fn span(&self) -> Span {
        match self {
            BorrowError::UseAfterMove { span, .. }
            | BorrowError::MoveWhileBorrowed { span, .. }
            | BorrowError::MultipleMutableBorrows { span, .. }
            | BorrowError::MutableBorrowWhileImmutable { span, .. }
            | BorrowError::ImmutableBorrowWhileMutable { span, .. }
            | BorrowError::BorrowAfterMove { span, .. }
            | BorrowError::AssignWhileBorrowed { span, .. }
            | BorrowError::MoveOutOfSharedRef { span }
            | BorrowError::MoveOutOfMutRef { span }
            | BorrowError::ValueUsedAfterMove { span }
            | BorrowError::VariableNotFound { span, .. }
            | BorrowError::FunctionNotFound { span, .. }
            | BorrowError::LifetimeMismatch { span }
            | BorrowError::ParseError { span, .. } => *span,
        }
    }

    fn locate<'s>(source: &'s str, offset: usize) -> (usize, usize, &'s str) {
        let mut line_start = 0;
        for (i, line) in source.lines().enumerate() {
            let line_end = line_start + line.len() + 1; // +1 for '\n'
            if offset < line_end || (i == source.lines().count() - 1 && offset <= line_start + line.len()) {
                return (i + 1, offset - line_start + 1, line);
            }
            line_start = line_end;
        }
        let last = source.lines().last().unwrap_or("");
        (source.lines().count(), offset.saturating_sub(line_start) + 1, last)
    }
}

/// Convenience alias for `Result<T, BorrowError>`.
pub type Result<T> = std::result::Result<T, BorrowError>;
