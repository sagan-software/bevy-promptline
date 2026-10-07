//! Local editing diagnostics do not emit submission rejection events.
/// Describes incomplete or unknown draft input independently of execution.
/// Display supplies local editing guidance, and these values never represent
/// submitted command outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditDiagnostic {
    /// The current token has an unfinished quote. The editor retains its draft
    /// and provides guidance while normal submission still requires the
    /// matching delimiter.
    UnclosedQuote,
    /// The current token ends with an unfinished escape. The next character
    /// completes that escape; no parser or game handler executes during this
    /// state.
    TrailingEscape,
    /// No permitted command spelling or prefix matches the draft. Missing and
    /// denied commands share this diagnostic so editing guidance does not
    /// disclose registry membership.
    UnknownCommand,
}
impl std::fmt::Display for EditDiagnostic {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Diagnostics describe editing work without claiming a submitted command failed.
        let message = match self {
            Self::UnclosedQuote => "Finish the quoted argument",
            Self::TrailingEscape => "Enter the character after the backslash",
            Self::UnknownCommand => "Unknown command",
        };
        formatter.write_str(message)
    }
}
