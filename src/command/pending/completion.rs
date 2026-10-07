//! One consuming completion endpoint sends one terminal outcome.
use super::CommandCancellation;
use crate::CommandOutcome;
use std::sync::mpsc::{SendError, Sender};

/// Worker-owned endpoint with one consuming terminal send and no cloning
/// capability. Dropping this endpoint without finishing makes the runtime emit
/// a disconnected-completion failure on its next poll.
#[derive(Debug)]
pub struct CommandCompletion {
    /// Private ownership prevents more than one queued terminal outcome.
    pub(super) sender: Sender<CommandOutcome>,
    /// Worker-visible cancellation request cannot directly cancel runtime
    /// state.
    pub(super) cancellation: CommandCancellation,
}
impl CommandCompletion {
    /// Returns a cloned read-only cancellation token that can outlive this
    /// endpoint.
    /// A request does not force a particular terminal outcome or undo completed
    /// side effects.
    #[must_use]
    pub fn cancellation(&self) -> CommandCancellation {
        self.cancellation.clone()
    }
    /// Sends a terminal outcome once and consumes this completion capability.
    /// A second send cannot compile:
    ///
    /// ```compile_fail
    /// use bevy_promptline::{CommandOutcome, command::pending::PendingCommand};
    /// let (_pending, completion) = PendingCommand::channel();
    /// completion.finish(CommandOutcome::Success).unwrap();
    /// completion.finish(CommandOutcome::Success).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the unsent outcome when the runtime has dropped its receiver.
    pub fn finish(self, outcome: CommandOutcome) -> Result<(), SendError<CommandOutcome>> {
        self.sender.send(outcome)
    }
}

impl AsRef<CommandCancellation> for CommandCompletion {
    fn as_ref(&self) -> &CommandCancellation {
        &self.cancellation
    }
}
