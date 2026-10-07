//! Source spans retain UTF-8 byte coordinates from the shared tokenizer.
use super::HighlightRole;
use std::ops::Range;
/// A source token range and its non-authoritative presentation role.
///
/// Shared analysis constructs spans from validated UTF-8 boundaries, including
/// original quote and escape markers.
/// Callers cannot construct unchecked source ranges:
///
/// ```compile_fail
/// use bevy_promptline::input::highlight::{HighlightRole, HighlightSpan};
/// let span = HighlightSpan::new(1..2, HighlightRole::Value);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    /// Original UTF-8 range includes quote and escape markers.
    source: Range<usize>,
    /// Semantic role derived without executing a validator or handler.
    role: HighlightRole,
}
impl HighlightSpan {
    /// Borrows source coordinates without copying or normalizing input. Both
    /// endpoints are UTF-8 boundaries within the original draft retained by
    /// this analysis snapshot.
    #[must_use]
    pub const fn source_range(&self) -> &Range<usize> {
        &self.source
    }
    /// Returns the semantic role used by a configurable presentation. This
    /// copied value describes styling and does not grant permission or assert
    /// parser acceptance.
    #[must_use]
    pub const fn role(&self) -> HighlightRole {
        self.role
    }
    /// Shared analysis constructs spans from validated tokenizer ranges.
    pub(crate) const fn new(source: Range<usize>, role: HighlightRole) -> Self {
        Self { source, role }
    }
    /// Adapter metadata refines roles without changing validated source coordinates.
    pub(crate) const fn set_role(&mut self, role: HighlightRole) {
        self.role = role;
    }
}
