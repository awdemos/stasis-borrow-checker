use crate::span::Span;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum BorrowError {
    #[error("use of moved value: `{name}`")]
    UseAfterMove { name: String, span: Span },

    #[error("cannot move out of `{name}` because it is borrowed")]
    MoveWhileBorrowed { name: String, span: Span },

    #[error("cannot borrow `{name}` as mutable more than once at a time")]
    MultipleMutableBorrows { name: String, span: Span },

    #[error("cannot borrow `{name}` as mutable because it is also borrowed as immutable")]
    MutableBorrowWhileImmutable { name: String, span: Span },

    #[error("cannot borrow `{name}` as immutable because it is also borrowed as mutable")]
    ImmutableBorrowWhileMutable { name: String, span: Span },

    #[error("borrow of moved value: `{name}`")]
    BorrowAfterMove { name: String, span: Span },

    #[error("cannot assign to `{name}` because it is borrowed")]
    AssignWhileBorrowed { name: String, span: Span },

    #[error("cannot move out of a shared reference")]
    MoveOutOfSharedRef { span: Span },

    #[error("cannot move out of a mutable reference")]
    MoveOutOfMutRef { span: Span },

    #[error("value used after it has been moved")]
    ValueUsedAfterMove { span: Span },

    #[error("variable `{name}` not found")]
    VariableNotFound { name: String, span: Span },

    #[error("function `{name}` not found")]
    FunctionNotFound { name: String, span: Span },

    #[error("lifetime mismatch: borrowed value does not live long enough")]
    LifetimeMismatch { span: Span },

    #[error("parse error: {message}")]
    ParseError { message: String, span: Span },
}

pub type Result<T> = std::result::Result<T, BorrowError>;
