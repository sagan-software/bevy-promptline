//! A Bevy UI command console with typed game-system delivery.
//!
//! Register a `ConsoleCommand` through `PromptlineAppExt`, then submit input
//! through `SubmitCommand`. The runtime validates input and authorization
//! before invoking its registered system. Output remains bounded, and lifecycle
//! messages expose invocation status without exposing raw submitted arguments.
//! The runtime owns no camera or window. Applications choose their rendering
//! and platform plugins.
pub mod command;
mod completion;
mod execution;
mod highlighting;
mod identity;
mod metadata;
mod output;
mod registry;
mod sets;
mod tokenization;
mod ui;
pub use self::command::{CommandNotification, CommandOutcome, SubmitCommand};
pub use self::execution::{PromptlineRuntimePlugin, RuntimeLimits};
pub use self::identity::{CommandName, InvalidCommandName, Limit, ZeroLimit};
pub use self::metadata::{ArgumentMetadata, CommandMetadata, ConsoleCommand};
pub use self::output::{Output, OutputEntry, OutputLimits, Severity};
pub use self::registry::{CommandRegistry, PromptlineAppExt, RegistrationError};
pub use self::tokenization::{Token, TokenizeError, quote_argument, tokenize};

/// Input parsing, registration, analysis, and editor integration for console
/// presentations.
///
/// Analysis inspects draft text and permitted metadata without executing a
/// command parser. Presentations explicitly accept candidates while ghost text
/// remains separate from authoritative input. Command builders attach typed
/// parsing to the shared runtime; their parsers run only after submission
/// validation and authorization. Accepting an independently changed analysis
/// snapshot returns an error.
pub mod input {
    /// Typed syntax roles and local editing diagnostics for custom
    /// presentations.
    ///
    /// `Analysis::highlights` supplies original UTF-8 source
    /// ranges, while `Analysis::diagnostic` supplies non-color guidance. Shared
    /// token decoding preserves quote and escape markers, and permitted
    /// metadata identifies commands and subcommands. Styling never invokes
    /// validators or authorizes execution.
    pub mod highlight {
        pub use crate::highlighting::{EditDiagnostic, HighlightRole, HighlightSpan};
        pub use crate::ui::{HighlightColors, SeverityColors};
    }
    pub use crate::completion::{
        Analysis, AnalysisError, AppliedCompletion, CompletionCycle, CompletionDirection,
        EditingMode, analyze,
    };
    pub use crate::metadata::Candidate;
    pub use crate::registry::{
        CommandBuilder, CompletionFailure, CompletionProvider, CompletionRegistrationError,
        CompletionRequest, NameMatching, ParseDecision,
    };
    pub use crate::sets::PromptlineSystems;
    pub use crate::ui::{
        ConsoleBindings, ConsoleMode, ConsoleRole, ConsoleState, ConsoleTheme, PromptlineUiPlugin,
        gameplay_input_allowed,
    };
}

#[cfg(feature = "clap")]
pub mod clap;

#[cfg(feature = "log")]
pub mod logging;
