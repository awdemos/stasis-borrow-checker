//! **Stasis** — a Rust borrow checker in idiomatic Rust.
//!
//! This crate provides a simplified, educational implementation of a
//! Rust-like borrow checker. It parses a subset of Rust syntax and
//! validates ownership, borrowing, and move semantics.
//!
//! # Quick start
//!
//! ```rust
//! use borrow_checker::BorrowChecker;
//!
//! let mut checker = BorrowChecker::new();
//!
//! // Accepted: two immutable borrows are fine.
//! assert!(checker.check_source(r#"
//!     fn main() {
//!         let x = 42;
//!         let a = &x;
//!         let b = &x;
//!     }
//! "#).is_ok());
//!
//! // Rejected: use after move.
//! assert!(checker.check_source(r#"
//!     fn main() {
//!         let s = "hello";
//!         let t = s;
//!         let u = s;  // ERROR: s was moved to t
//!     }
//! "#).is_err());
//! ```
//!
//! # Architecture
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`ast`] | AST types for a simplified Rust subset |
//! | [`parser`] | Hand-written recursive descent parser |
//! | [`span`] | Source location tracking |
//! | [`ownership`] | `OwnershipState` machine |
//! | [`borrow_graph`] | Active borrow relationship tracking |
//! | [`checker`] | Core borrow checking algorithm |
//! | [`error`] | Error types with source-aware formatting |
//!
//! If you already have an AST, build [`ast::Program`] manually and pass
//! it to [`BorrowChecker::check_program`]. Otherwise, use
//! [`BorrowChecker::check_source`] for the simplest path from source
//! text to validation.

pub mod ast;
pub mod borrow_graph;
pub mod checker;
pub mod error;
pub mod ownership;
pub mod parser;
pub mod span;

pub use checker::BorrowChecker;
pub use error::{BorrowError, Result};
