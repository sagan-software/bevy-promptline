//! Command input and lifecycle contracts shared by registration and dispatch.
//!
//! Submissions preserve raw input privately until validation and parsing select
//! an execution owner. Lifecycle notifications expose identities, origins, and
//! outcomes independently of raw arguments. Draft token analysis uses the same
//! text profile as submission while tolerating unfinished quotes and escapes
//! during typing.
mod history;
pub mod pending;
pub use self::history::CommandHistory;
pub use crate::metadata::Sensitivity;
use bevy::prelude::{Message, Messages, World};

/// Identifies an invocation assigned by the runtime at submission.
/// Notifications retain this value across validation, execution, and the
/// terminal outcome for that invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InvocationId(pub(crate) u64);

/// Identifies the single interactive console supported by this runtime.
/// Notifications include it so their shape can retain console identity
/// independently of invocation identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConsoleId;

/// Identifies the command input origin for authorization and lifecycle
/// consumers. Both variants pass through the same validation and execution
/// boundary before a handler runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SubmissionSource {
    /// Input originated in the console editor. The registry evaluates this
    /// source during authorization; opening the editor does not independently
    /// grant command execution permission.
    Ui,
    /// Input originated in game code rather than the editor. It still passes
    /// input limits, tokenization, authorization, and typed parsing before the
    /// registered handler runs.
    #[default]
    Programmatic,
}

/// Carries one command into runtime validation. Raw text remains private and
/// debug output redacts it before command metadata or argument sensitivity is
/// known.
#[derive(Debug, Clone, Message)]
pub struct SubmitCommand {
    /// Original UTF-8 command, private until sensitivity policy is known.
    pub(crate) input: SubmittedText,
    /// The input origin supplied for policy evaluation. Changing this value
    /// selects the actor policy branch but does not bypass tokenization or
    /// command parsing.
    pub source: SubmissionSource,
}
impl SubmitCommand {
    /// Creates programmatic command input while retaining its original text
    /// privately. Sending this message does not execute a handler until runtime
    /// validation and authorization succeed.
    #[must_use]
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: SubmittedText(input.into()),
            source: SubmissionSource::Programmatic,
        }
    }
}

/// Submitted text whose diagnostics never expose command arguments.
#[derive(Clone)]
pub(crate) struct SubmittedText(pub(crate) String);
impl std::fmt::Debug for SubmittedText {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("<redacted command input>")
    }
}

/// Describes the terminal outcome returned by a typed game handler. Handler
/// failures occur after acceptance and remain distinct from validation or
/// authorization rejection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandOutcome {
    /// The accepted command handler reported successful completion. This
    /// outcome produces a finished notification after the started transition
    /// and does not independently write console output.
    Success,
    /// The accepted command handler reported a failure diagnostic. The finished
    /// notification retains that diagnostic; this outcome remains distinct from
    /// rejection before handler execution.
    Failure(String),
    /// The accepted command handler reported cancellation. The finished
    /// notification records this terminal result; it does not undo side effects
    /// the handler already performed.
    Cancelled,
}

/// Describes one observable invocation transition. Submitted commands are
/// rejected or accepted; accepted commands start execution and receive one
/// finished notification carrying their outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvocationStatus {
    /// The runtime assigned an invocation identity but has not validated input.
    /// This notification precedes input limits, tokenization, authorization,
    /// and parsing for that submission.
    Submitted,
    /// Validation, authorization, parsing, or the acceptance limit rejected the
    /// input. The diagnostic explains the rejection, and the registered game
    /// handler has not executed.
    Rejected(String),
    /// The input passed limits, tokenization, authorization, and typed parsing.
    /// The runtime emits this transition before transferring the parsed value
    /// to its registered execution owner.
    Accepted,
    /// The runtime is transferring accepted typed input to its registered
    /// handler. Observers receive this transition without becoming additional
    /// command execution owners for the invocation.
    Started,
    /// The registered handler returned its terminal outcome. This transition
    /// closes an accepted invocation and retains success, handler failure, or
    /// cooperative cancellation as separate results.
    Finished(CommandOutcome),
}

/// Carries a lifecycle transition for observation by game systems. Invocation
/// identity, console identity, and input origin remain available without
/// exposing raw submitted argument text.
#[derive(Debug, Clone, Message, PartialEq, Eq)]
pub struct CommandNotification {
    /// The invocation identity assigned at submission. Every transition for the
    /// same invocation retains this value, allowing observers to associate
    /// acceptance and completion with it.
    pub invocation: InvocationId,
    /// The identity of the single supported interactive console. It remains
    /// separate from invocation identity, so consumers do not treat command
    /// order as console identity.
    pub console: ConsoleId,
    /// The origin supplied with the submission. It remains unchanged across the
    /// invocation transitions, allowing observers to distinguish editor input
    /// from programmatic command input.
    pub source: SubmissionSource,
    /// The transition emitted for this invocation. It reports validation and
    /// execution progress without carrying the original command text or
    /// creating another command execution owner.
    pub status: InvocationStatus,
}

/// Decodes a draft for metadata analysis without requiring closing quotes or
/// complete escapes. Forbidden NUL and line breaks remain errors, and decoded
/// values preserve their source ranges.
///
/// # Errors
///
/// Returns `TokenizeError` for NUL or line breaks. An unfinished quote or
/// escape is valid while typing.
pub fn analyze_tokens(input: &str) -> Result<Vec<crate::Token>, crate::TokenizeError> {
    crate::tokenization::decode(input, crate::tokenization::DecodeMode::Incomplete)
}

/// Emits observations without raw arguments, tolerating application-owned
/// resource removal.
pub(crate) fn notify(
    world: &mut World,
    invocation: InvocationId,
    source: SubmissionSource,
    status: InvocationStatus,
) {
    if let Some(mut notifications) = world.get_resource_mut::<Messages<CommandNotification>>() {
        notifications.write(CommandNotification {
            invocation,
            console: ConsoleId,
            source,
            status,
        });
    }
}

/// Output presentation controls preserve reading position independently from
/// command execution.
///
/// A viewport can follow new output or retain a review
/// baseline while arrivals continue. Entry counts remain distinct from wrapped
/// line counts, and clearing or replacing output resets stale review state.
pub mod output {
    pub use crate::output::OutputView;
}
