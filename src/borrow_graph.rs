use crate::span::Span;
use indexmap::IndexMap;
use std::collections::HashSet;

/// A single borrow relationship in the borrow graph.
///
/// Records that `borrower` holds a (mutable or immutable) reference
/// to `borrowed`, with the source location for error reporting.
#[derive(Clone, Debug, PartialEq)]
pub struct Borrow {
    /// The variable holding the reference (e.g. `r` in `let r = &x`).
    pub borrower: String,
    /// The variable being borrowed (e.g. `x` in `let r = &x`).
    pub borrowed: String,
    /// Whether this is a mutable (`&mut`) or immutable (`&`) borrow.
    pub mutable: bool,
    /// Source location of the borrow expression.
    pub span: Span,
}

/// Tracks all active borrow relationships in the program.
///
/// Maintains two complementary indexes — one keyed by borrower,
/// one by borrowed — so that both "who do I borrow?" and
/// "who borrows me?" queries are O(1).
#[derive(Clone, Debug, Default)]
pub struct BorrowGraph {
    /// Active borrows, keyed by borrower name.
    pub borrows: IndexMap<String, Vec<Borrow>>,
    /// Variables that have been moved.
    pub moved: HashSet<String>,
    /// Active borrows, keyed by the variable being borrowed.
    pub borrowed_by: IndexMap<String, Vec<Borrow>>,
}

impl BorrowGraph {
    /// Create an empty borrow graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a new borrow into both indexes.
    pub fn add_borrow(&mut self, borrow: Borrow) {
        let borrowed = borrow.borrowed.clone();
        let borrower = borrow.borrower.clone();

        self.borrowed_by
            .entry(borrowed)
            .or_default()
            .push(borrow.clone());
        self.borrows.entry(borrower).or_default().push(borrow);
    }

    /// Remove all borrows originating from `borrower`.
    ///
    /// Called when the borrower goes out of scope — all its
    /// references are dropped and the borrowed variables are
    /// released.
    pub fn remove_borrows_by(&mut self, borrower: &str) {
            if let Some(borrows) = self.borrows.swap_remove(borrower) {
            for borrow in borrows {
                if let Some(list) = self.borrowed_by.get_mut(&borrow.borrowed) {
                    list.retain(|b| b.borrower != borrower);
                }
            }
        }
    }

    /// Check whether `name` has any active borrows.
    pub fn is_borrowed(&self, name: &str) -> bool {
        self.borrowed_by
            .get(name)
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }

    /// Check whether `name` has an active mutable borrow.
    pub fn is_mutably_borrowed(&self, name: &str) -> bool {
        self.borrowed_by
            .get(name)
            .map(|borrows| borrows.iter().any(|b| b.mutable))
            .unwrap_or(false)
    }

    /// Check whether `name` has an active immutable borrow.
    pub fn is_immutably_borrowed(&self, name: &str) -> bool {
        self.borrowed_by
            .get(name)
            .map(|borrows| borrows.iter().any(|b| !b.mutable))
            .unwrap_or(false)
    }

    /// Check whether `name` has any mutable borrow.
    ///
    /// (Alias for [`is_mutably_borrowed`](Self::is_mutably_borrowed) —
    /// exists for semantic clarity in call sites.)
    pub fn has_mutable_borrows(&self, name: &str) -> bool {
        self.borrowed_by
            .get(name)
            .map(|borrows| borrows.iter().any(|b| b.mutable))
            .unwrap_or(false)
    }

    /// Check whether `name` has any immutable borrow.
    pub fn has_immutable_borrows(&self, name: &str) -> bool {
        self.borrowed_by
            .get(name)
            .map(|borrows| borrows.iter().any(|b| !b.mutable))
            .unwrap_or(false)
    }

    /// Collect all borrows of `name` (mutable and immutable).
    pub fn get_borrows_of(&self, name: &str) -> Vec<&Borrow> {
        self.borrowed_by
            .get(name)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Mark `name` as moved.
    pub fn record_move(&mut self, name: &str) {
        self.moved.insert(name.to_string());
    }

    /// Check whether `name` has been moved.
    pub fn is_moved(&self, name: &str) -> bool {
        self.moved.contains(name)
    }

    /// Merge the borrows and moves from `other` into `self`.
    pub fn merge(&mut self, other: BorrowGraph) {
        for (borrower, borrows) in other.borrows {
            self.borrows.entry(borrower).or_default().extend(borrows);
        }
        for (borrowed, borrows) in other.borrowed_by {
            self.borrowed_by.entry(borrowed).or_default().extend(borrows);
        }
        self.moved.extend(other.moved);
    }
}
