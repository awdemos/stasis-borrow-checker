use crate::span::Span;
use indexmap::IndexMap;
use std::collections::HashSet;

/// A borrow in the borrow graph
#[derive(Clone, Debug, PartialEq)]
pub struct Borrow {
    pub borrower: String,
    pub borrowed: String,
    pub mutable: bool,
    pub span: Span,
}

/// Tracks all active borrows in the program
#[derive(Clone, Debug, Default)]
pub struct BorrowGraph {
    /// All active borrows keyed by borrower
    pub borrows: IndexMap<String, Vec<Borrow>>,
    /// Set of variables that have been moved
    pub moved: HashSet<String>,
    /// Map of variable -> what borrows it
    pub borrowed_by: IndexMap<String, Vec<Borrow>>,
}

impl BorrowGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new borrow
    pub fn add_borrow(&mut self, borrow: Borrow) {
        let borrowed = borrow.borrowed.clone();
        let borrower = borrow.borrower.clone();

        self.borrowed_by
            .entry(borrowed)
            .or_default()
            .push(borrow.clone());
        self.borrows.entry(borrower).or_default().push(borrow);
    }

    /// Remove all borrows by a given borrower (when it goes out of scope)
    pub fn remove_borrows_by(&mut self, borrower: &str) {
            if let Some(borrows) = self.borrows.swap_remove(borrower) {
            for borrow in borrows {
                if let Some(list) = self.borrowed_by.get_mut(&borrow.borrowed) {
                    list.retain(|b| b.borrower != borrower);
                }
            }
        }
    }

    /// Check if a variable is currently borrowed
    pub fn is_borrowed(&self, name: &str) -> bool {
        self.borrowed_by
            .get(name)
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }

    /// Check if a variable is mutably borrowed
    pub fn is_mutably_borrowed(&self, name: &str) -> bool {
        self.borrowed_by
            .get(name)
            .map(|borrows| borrows.iter().any(|b| b.mutable))
            .unwrap_or(false)
    }

    /// Check if a variable is immutably borrowed
    pub fn is_immutably_borrowed(&self, name: &str) -> bool {
        self.borrowed_by
            .get(name)
            .map(|borrows| borrows.iter().any(|b| !b.mutable))
            .unwrap_or(false)
    }

    /// Check if there are any mutable borrows of a variable
    pub fn has_mutable_borrows(&self, name: &str) -> bool {
        self.borrowed_by
            .get(name)
            .map(|borrows| borrows.iter().any(|b| b.mutable))
            .unwrap_or(false)
    }

    /// Check if there are any immutable borrows of a variable
    pub fn has_immutable_borrows(&self, name: &str) -> bool {
        self.borrowed_by
            .get(name)
            .map(|borrows| borrows.iter().any(|b| !b.mutable))
            .unwrap_or(false)
    }

    /// Get all borrows of a variable
    pub fn get_borrows_of(&self, name: &str) -> Vec<&Borrow> {
        self.borrowed_by
            .get(name)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Record that a variable has been moved
    pub fn record_move(&mut self, name: &str) {
        self.moved.insert(name.to_string());
    }

    /// Check if a variable has been moved
    pub fn is_moved(&self, name: &str) -> bool {
        self.moved.contains(name)
    }

    /// Merge another borrow graph into this one
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
