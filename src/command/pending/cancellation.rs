//! Cancellation requests remain cooperative and cannot undo side effects.
use crate::command::InvocationId;
use bevy::prelude::Message;
use std::sync::{Arc, atomic::AtomicBool};

/// Observes the runtime's cancellation request without granting mutation access
/// to the signal. Clones share the same request, while workers remain
/// responsible
/// for reporting their final outcome.
#[derive(Debug, Clone)]
pub struct CommandCancellation(pub(super) Arc<AtomicBool>);
impl CommandCancellation {
    /// Reports whether the runtime requested cancellation for this work. The
    /// worker chooses when and how to finish; a true value does not prove
    /// terminal
    /// cancellation.
    #[must_use]
    pub fn is_requested(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::Acquire)
    }
}

/// Requests cooperative cancellation of an existing pending invocation in this
/// App.
///
/// Unknown or already finished identities have no effect, and a request
/// does not produce a terminal notification until the worker finishes.
#[derive(Debug, Clone, Copy, Message)]
pub struct CancelCommand {
    /// Identity supplied by this invocation's lifecycle notification in the
    /// receiving App. It selects an existing pending endpoint rather than a
    /// registration name or command input.
    pub invocation: InvocationId,
}

impl CommandCancellation {
    /// Publishes the runtime's request without forcing worker completion.
    pub(crate) fn request(&self) {
        self.0.store(true, std::sync::atomic::Ordering::Release);
    }
}
