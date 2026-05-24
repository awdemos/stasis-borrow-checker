use crate::span::Span;
use indexmap::IndexMap;
use std::collections::HashSet;

/// The ownership state of a variable at a point in the program
#[derive(Clone, Debug, PartialEq)]
pub enum OwnershipState {
    /// Value is owned and can be moved or borrowed
    Owned,
    /// Value has been moved and is no longer valid
    Moved { moved_at: Span },
    /// Value is mutably borrowed (exclusive access)
    MutablyBorrowed { borrow_span: Span },
    /// Value is immutably borrowed (shared access)
    ImmutablyBorrowed { borrow_spans: Vec<Span> },
    /// Value is partially moved (some fields moved)
    PartiallyMoved { moved_fields: HashSet<String> },
}

/// Tracks the ownership state of all variables in a scope
#[derive(Clone, Debug, Default)]
pub struct Scope {
    /// Variable name -> ownership state
    pub variables: IndexMap<String, OwnershipState>,
    /// Parent scope (if any)
    pub parent: Option<Box<Scope>>,
}

impl Scope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_parent(parent: Scope) -> Self {
        Self {
            variables: IndexMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn declare(&mut self, name: String, state: OwnershipState) {
        self.variables.insert(name, state);
    }

    pub fn get(&self, name: &str) -> Option<&OwnershipState> {
        self.variables.get(name).or_else(|| {
            self.parent.as_ref().and_then(|p| p.get(name))
        })
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut OwnershipState> {
        if self.variables.contains_key(name) {
            self.variables.get_mut(name)
        } else {
            self.parent.as_mut().and_then(|p| p.get_mut(name))
        }
    }

    pub fn is_owned(&self, name: &str) -> bool {
        matches!(self.get(name), Some(OwnershipState::Owned))
    }

    pub fn is_moved(&self, name: &str) -> bool {
        matches!(self.get(name), Some(OwnershipState::Moved { .. }))
    }

    pub fn is_borrowed(&self, name: &str) -> bool {
        matches!(
            self.get(name),
            Some(OwnershipState::MutablyBorrowed { .. })
                | Some(OwnershipState::ImmutablyBorrowed { .. })
        )
    }

    pub fn is_mutably_borrowed(&self, name: &str) -> bool {
        matches!(self.get(name), Some(OwnershipState::MutablyBorrowed { .. }))
    }

    pub fn is_immutably_borrowed(&self, name: &str) -> bool {
        matches!(
            self.get(name),
            Some(OwnershipState::ImmutablyBorrowed { .. })
        )
    }

    /// Merge two scopes (used for if/else branches)
    /// A variable is only valid after the merge if it's valid in BOTH branches
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
