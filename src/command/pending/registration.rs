//! Opt-in registration preserves the existing synchronous extension trait.
use crate::identity::RegistrationError;
use crate::metadata::ConsoleCommand;
use bevy::prelude::*;

/// Registers pending-capable handlers without adding required methods to
/// existing synchronous extension-trait implementations.
///
/// Metadata, parsing, permissions,
/// and lifecycle ordering use the same runtime owner as synchronous commands.
pub trait PendingAppExt {
    /// Registers a handler that can transfer pending completion ownership.
    /// Existing synchronous registration and terminal outcome types stay
    /// intact.
    ///
    /// # Errors
    ///
    /// Rejects invalid metadata and collisions before registering a system.
    fn register_pending_command<C: ConsoleCommand, M: 'static>(
        &mut self,
        system: impl IntoSystem<In<C>, super::CommandExecution, M> + 'static,
    ) -> Result<&mut Self, RegistrationError>;
}
