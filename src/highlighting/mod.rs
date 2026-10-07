//! Typed draft presentation remains separate from parsing and execution.
mod diagnostic;
mod role;
mod span;
pub use self::diagnostic::EditDiagnostic;
pub use self::role::HighlightRole;
pub use self::span::HighlightSpan;
