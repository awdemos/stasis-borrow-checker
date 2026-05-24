use std::fmt;

/// A source location span from `start` to `end` byte offset.
///
/// Used throughout the library to point at the relevant source text
/// when reporting errors or attaching position information to AST nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    /// Byte offset of the start (inclusive).
    pub start: usize,
    /// Byte offset of the end (exclusive).
    pub end: usize,
}

impl Span {
    /// Create a new span covering `[start, end)`.
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Return a span that covers both `self` and `other`.
    pub fn merge(self, other: Span) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}..{}]", self.start, self.end)
    }
}
