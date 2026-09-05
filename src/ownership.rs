use crate::span::Span;
use indexmap::IndexMap;
use std::collections::HashSet;

/// The ownership state of a single variable at a program point.
///
/// Transitions happen as the checker encounters moves, borrows,
/// and block boundaries.
#[derive(Clone, Debug, PartialEq)]
pub enum OwnershipState {
    /// Value is owned and can be moved or borrowed.
    Owned,
    /// Value has been moved and is no longer usable.
    Moved {
        /// Span of the expression that moved the value.
        moved_at: Span,
    },
    /// Value is mutably borrowed (exclusive, write-capable reference).
    MutablyBorrowed {
        /// Span of the `&mut` expression.
        borrow_span: Span,
    },
    /// Value is immutably borrowed (shared, read-only references).
    ImmutablyBorrowed {
        /// Spans of all active `&` borrows.
        borrow_spans: Vec<Span>,
    },
    /// Some fields of the value have been moved out of.
    PartiallyMoved {
        /// Names of moved fields.
        moved_fields: HashSet<String>,
    },
}

/// Tracks the ownership state of every variable in a scope.
///
/// Scopes form a parent chain: child scopes fall through to their
/// parent when a variable is not found locally.
#[derive(Clone, Debug, Default)]
pub struct Scope {
    /// Variable name → ownership state (preserves insertion order).
    pub variables: IndexMap<String, OwnershipState>,
    /// Parent scope to delegate lookups to.
    pub parent: Option<Box<Scope>>,
}

impl Scope {
    /// Create a new empty scope.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a child scope with a given parent.
    pub fn with_parent(parent: Scope) -> Self {
        Self {
            variables: IndexMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    /// Insert (or overwrite) a variable declaration.
    pub fn declare(&mut self, name: String, state: OwnershipState) {
        self.variables.insert(name, state);
    }

    /// Look up a variable, falling through to the parent scope if needed.
    pub fn get(&self, name: &str) -> Option<&OwnershipState> {
        self.variables
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.get(name)))
    }

    /// Mutably borrow the state of a variable, falling through to parent.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut OwnershipState> {
        if self.variables.contains_key(name) {
            self.variables.get_mut(name)
        } else {
            self.parent.as_mut().and_then(|p| p.get_mut(name))
        }
    }

    /// Check whether a variable is in the `Owned` state.
    pub fn is_owned(&self, name: &str) -> bool {
        matches!(self.get(name), Some(OwnershipState::Owned))
    }

    /// Check whether a variable has been moved.
    pub fn is_moved(&self, name: &str) -> bool {
        matches!(self.get(name), Some(OwnershipState::Moved { .. }))
    }

    /// Check whether a variable is currently borrowed (mutably or immutably).
    pub fn is_borrowed(&self, name: &str) -> bool {
        matches!(
            self.get(name),
            Some(OwnershipState::MutablyBorrowed { .. })
                | Some(OwnershipState::ImmutablyBorrowed { .. })
        )
    }

    /// Check whether a variable is currently mutably borrowed.
    pub fn is_mutably_borrowed(&self, name: &str) -> bool {
        matches!(self.get(name), Some(OwnershipState::MutablyBorrowed { .. }))
    }

    /// Check whether a variable is currently immutably borrowed.
    pub fn is_immutably_borrowed(&self, name: &str) -> bool {
        matches!(
            self.get(name),
            Some(OwnershipState::ImmutablyBorrowed { .. })
        )
    }

    /// Merge two parallel scopes (used for if/else branch convergence).
    ///
    /// A variable is only valid after the merge if it has the **same**
    /// ownership state (or compatible states) in **both** branches.
    /// Borrowed-in-both-branches → Owned (borrows released at end of branch).
    /// Mixed states → Moved (conservatively invalid).
    pub fn merge(mut self, other: Scope) -> Self {
        for (name, other_state) in other.variables {
            match (self.variables.get(&name), other_state) {
                (Some(OwnershipState::Moved { .. }), OwnershipState::Moved { .. }) => {}
                (Some(OwnershipState::Owned), OwnershipState::Owned) => {}
                (
                    Some(OwnershipState::ImmutablyBorrowed { .. }),
                    OwnershipState::ImmutablyBorrowed { .. },
                ) => {
                    self.variables.insert(name, OwnershipState::Owned);
                }
                _ => {
                    self.variables.insert(
                        name,
                        OwnershipState::Moved {
                            moved_at: Span::new(0, 0),
                        },
                    );
                }
            }
        }
        self
    }
}
