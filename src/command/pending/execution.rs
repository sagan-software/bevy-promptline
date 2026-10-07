//! A handler either finishes now or transfers one pending completion channel.
use super::PendingCommand;
use crate::CommandOutcome;

/// Result of an opt-in pending-capable handler using existing terminal outcomes.
///
/// Each variant owns either its terminal outcome or its receiving endpoint.
/// The runtime emits completion immediately for `Finished` and polls `Pending`
/// later, without retaining the handler's World borrow.
#[derive(Debug)]
pub enum CommandExecution {
    /// The handler has finished and owns no runtime completion handle. The
    /// runtime
    /// emits one terminal notification without retaining a pending slot or
    /// calling
    /// the handler again.
    Finished(CommandOutcome),
    /// Transfers receiving ownership to the runtime while the worker retains
    /// its
    /// separate completion endpoint. Work continues without retaining the
    /// handler's
    /// World borrow, and completion arrives through later polling.
    Pending(PendingCommand),
}
impl From<CommandOutcome> for CommandExecution {
    fn from(outcome: CommandOutcome) -> Self {
        Self::Finished(outcome)
    }
}
