//! Semantic color roles for original command source bytes.
/// Identifies a source token's non-authoritative presentation role. Command and
/// subcommand roles use permitted metadata; quoted and incomplete roles come
/// from shared decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightRole {
    /// Registered command spelling or a permitted command prefix. Quote
    /// delimiters remain in its source range, and styling does not authorize
    /// submission.
    Command,
    /// A subcommand identified by adapter metadata after resolving its parent
    /// command. A word consumed as an option value cannot receive this role.
    Subcommand,
    /// An option spelling before the end-of-options delimiter. Styling
    /// describes syntax without validating its value, conflicts, required
    /// arguments, or final parser acceptance.
    Option,
    /// An ordinary argument value, including negative numbers and tokens after
    /// the delimiter. Value styling does not imply that a command validator
    /// accepts it.
    Value,
    /// Quoted argument text includes its original delimiters and escapes. The
    /// span retains raw source bytes while the shared tokenizer separately
    /// decodes argument values.
    Quoted,
    /// A quote or escape is still being entered. Presentations show local
    /// guidance without clearing the draft, submitting input, or emitting a
    /// rejection.
    Incomplete,
    /// A command spelling has no permitted match. Missing and denied commands
    /// receive the same role, so coloring cannot reveal protected command
    /// membership.
    Error,
}
