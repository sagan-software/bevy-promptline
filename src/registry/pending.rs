//! Pending registration adapts the shared parser and collision boundary.
use super::{ParseDecision, RegistrationError, register_execution};
use crate::command::pending::{CommandExecution, PendingAppExt};
use crate::metadata::ConsoleCommand;
use bevy::prelude::*;
impl PendingAppExt for App {
    fn register_pending_command<C: ConsoleCommand, M: 'static>(
        &mut self,
        system: impl IntoSystem<In<C>, CommandExecution, M> + 'static,
    ) -> Result<&mut Self, RegistrationError> {
        register_execution(
            self,
            C::metadata()?,
            |arguments| C::parse(arguments).map(ParseDecision::Run),
            None,
            system,
        )
    }
}
