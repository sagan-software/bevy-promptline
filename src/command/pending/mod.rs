//! Pending-capable handlers return either an immediate terminal outcome or one
//! receiving endpoint.
//!
//! Workers retain a separate consuming completion endpoint
//! without retaining the handler's World borrow. Import `PendingAppExt` by name
//! to register this opt-in behavior alongside existing synchronous commands.
//!
//! Cancellation requests identify invocations within the receiving App. They
//! set a read-only worker signal without forcing completion or undoing earlier
//! side effects. Runtime polling never waits for worker data and retains at
//! most
//! the positive count configured by `PendingLimits`, defaulting to 64 handles.
mod cancellation;
mod command;
mod completion;
mod execution;
mod registration;
pub use self::cancellation::{CancelCommand, CommandCancellation};
pub use self::command::PendingCommand;
pub use self::completion::CommandCompletion;
pub use self::execution::CommandExecution;
pub use self::registration::PendingAppExt;
use bevy::prelude::Resource;

/// Configures the positive maximum number of receiving endpoints retained by
/// this App. The default permits 64 pending invocations independently of the
/// per-frame submission acceptance limit.
#[derive(Debug, Clone, Copy, Resource)]
pub struct PendingLimits {
    /// Positive maximum count of retained endpoints. When a started handler
    /// exceeds
    /// this count, the runtime requests cancellation, drops its receiver, and
    /// emits
    /// a terminal capacity failure without undoing prior side effects.
    pub invocations: crate::Limit,
}
impl Default for PendingLimits {
    fn default() -> Self {
        Self {
            invocations: crate::Limit::try_from(64).expect("positive constant"),
        }
    }
}
