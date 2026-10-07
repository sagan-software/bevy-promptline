//! Pending completion polling never waits for worker data.
use super::{CommandCancellation, CommandCompletion};
use crate::CommandOutcome;
use bevy::platform::cell::SyncCell;
use std::sync::mpsc::Receiver;

/// Runtime-owned receiving endpoint returned by a pending-capable handler.
/// Callers cannot clone or poll its channel, and it retains no World reference
/// while waiting for the worker's terminal outcome.
pub struct PendingCommand {
    /// Exclusive mutable access keeps the receiver Sync without runtime
    /// locking.
    pub(super) receiver: SyncCell<Receiver<CommandOutcome>>,
    /// Cancellation is a request shared with the worker's read-only token.
    pub(crate) cancellation: CommandCancellation,
}
impl std::fmt::Debug for PendingCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PendingCommand")
            .field("cancellation", &self.cancellation)
            .finish_non_exhaustive()
    }
}
impl PendingCommand {
    /// Creates one runtime endpoint and one consuming worker endpoint. The
    /// private
    /// sender permits one terminal value; this operation creates no thread,
    /// task,
    /// or retained World access.
    #[must_use]
    pub fn channel() -> (Self, CommandCompletion) {
        // The private, consuming sender bounds this channel to one terminal value.
        let (sender, receiver) = std::sync::mpsc::channel();
        let cancellation = CommandCancellation(std::sync::Arc::new(
            std::sync::atomic::AtomicBool::new(false),
        ));
        // Each endpoint shares only read-only cancellation observation with the worker.
        let completion = CommandCompletion {
            sender,
            cancellation: cancellation.clone(),
        };
        (
            Self {
                receiver: SyncCell::new(receiver),
                cancellation,
            },
            completion,
        )
    }
}

impl PendingCommand {
    /// Uses the standard nonblocking receive boundary on the app thread.
    /// <https://doc.rust-lang.org/std/sync/mpsc/struct.Receiver.html#method.try_recv>
    pub(crate) fn poll(&mut self) -> Option<CommandOutcome> {
        // Empty channels retain pending ownership without waiting for worker data.
        match self.receiver.get().try_recv() {
            Ok(outcome) => Some(outcome),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            // A dropped worker endpoint cannot leave an invocation running forever.
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Some(CommandOutcome::Failure(
                "pending command completion disconnected".into(),
            )),
        }
    }
}
