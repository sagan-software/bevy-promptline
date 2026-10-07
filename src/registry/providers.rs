//! Read-only system adapters supply live candidates after the registry's actor
//! policy succeeds.
use crate::metadata::Candidate;
use bevy::ecs::system::RunSystemError;
use bevy::prelude::*;

/// Owns validated draft tokens and their selected completion slot for one
/// analysis call.
/// Providers borrow this request while the runtime retains the authoritative
/// editor value separately.
pub struct CompletionRequest {
    /// Decoded tokens retain their original UTF-8 source ranges.
    pub(crate) tokens: Vec<crate::Token>,
    /// Selected token slot derived from validated draft coordinates.
    pub(crate) position: usize,
    /// Decoded prefix before the actual caret.
    pub(crate) prefix: String,
}
impl std::fmt::Debug for CompletionRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CompletionRequest")
            .field("position", &self.position)
            .field("input", &"<redacted draft>")
            .finish_non_exhaustive()
    }
}
impl CompletionRequest {
    /// Borrows decoded tokens and their original source ranges without copying
    /// the collection.
    /// Providers use these values for contextual discovery rather than invoking
    /// a command parser.
    #[must_use]
    pub fn tokens(&self) -> &[crate::Token] {
        &self.tokens
    }
    /// Returns the selected token slot, including an empty slot after a
    /// trailing separator.
    /// This value never exceeds the token count in the validated request.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }
    /// Borrows the decoded prefix preceding the validated caret within its
    /// selected token.
    /// This spelling preserves Unicode and incomplete-input decoding without
    /// executing an argument validator.
    #[must_use]
    pub fn prefix(&self) -> &str {
        &self.prefix
    }
}

/// Supplies live game candidates without mutable access to the ECS world.
/// System adapters enforce Bevy's read-only parameter capability and reject
/// deferred mutation before installation.
pub trait CompletionProvider: Send + Sync + 'static {
    /// Produces contextual candidate data after registry permission has
    /// succeeded for this command.
    /// Providers must keep work bounded and avoid parser execution or external
    /// side effects.
    ///
    /// # Errors
    ///
    /// Returns an unavailable, contended, or poisoned provider error without
    /// mutating the world.
    fn complete(
        &self,
        request: &CompletionRequest,
        world: &World,
    ) -> Result<Vec<Candidate>, CompletionFailure>;
}

/// Distinguishes read-only provider availability from command parsing or
/// execution failures.
/// Analysis can report this state in help while ordinary command submission
/// retains its own validation path.
#[derive(Debug)]
pub enum CompletionFailure {
    /// Another caller owns the provider's local system state during this
    /// analysis call.
    /// Discovery defers instead of blocking input or waiting for another
    /// app-thread drain.
    Contended,
    /// A prior provider panic poisoned its local system-state guard before this
    /// request.
    /// The adapter refuses reuse rather than executing with potentially
    /// inconsistent cached parameters.
    Poisoned,
    /// Bevy could not run the initialized read-only provider with current world
    /// parameters.
    /// The original system error remains available for typed inspection rather
    /// than being replaced by its formatted message.
    System(RunSystemError),
}

impl From<RunSystemError> for CompletionFailure {
    fn from(error: RunSystemError) -> Self {
        Self::System(error)
    }
}
impl std::fmt::Display for CompletionFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Contended => formatter.write_str("completion provider is busy"),
            Self::Poisoned => formatter.write_str("completion provider state is unavailable"),
            Self::System(error) => std::fmt::Display::fmt(error, formatter),
        }
    }
}
impl std::error::Error for CompletionFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::System(RunSystemError::Skipped(error)) => Some(error),
            Self::System(RunSystemError::Failed(_)) | Self::Poisoned | Self::Contended => None,
        }
    }
}

/// Rejects unavailable command ownership or a provider that can queue deferred
/// mutations.
/// Failed installation leaves existing provider ownership and the command
/// registry's identity mappings unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CompletionRegistrationError {
    /// The requested primary name or alias has no registered execution owner in
    /// this app.
    /// Provider initialization does not occur until an existing command
    /// identity resolves successfully.
    #[error("register the command before its completion provider")]
    UnknownCommand,
    /// Bevy's read-only marker permits deferred buffers, but console discovery
    /// never applies them.
    /// The adapter rejects these parameters so typing cannot accumulate
    /// deferred world mutation requests.
    #[error("completion systems cannot use deferred mutation parameters")]
    DeferredMutation,
}

/// Local read-only system state is synchronized without acquiring a mutable
/// World reference.
struct SystemProvider<S>(std::sync::Mutex<S>);
impl<S> CompletionProvider for SystemProvider<S>
where
    S: ReadOnlySystem<In = InRef<'static, CompletionRequest>, Out = Vec<Candidate>>,
{
    fn complete(
        &self,
        request: &CompletionRequest,
        world: &World,
    ) -> Result<Vec<Candidate>, CompletionFailure> {
        let mut system = self.0.try_lock().map_err(|error| match error {
            std::sync::TryLockError::WouldBlock => CompletionFailure::Contended,
            std::sync::TryLockError::Poisoned(_) => CompletionFailure::Poisoned,
        })?;
        // This safe Bevy boundary never applies deferred buffers or exposes mutable world access.
        Ok(system.run_readonly(request, world)?)
    }
}

/// Initializes a provider only after its owner resolves, then excludes deferred
/// state.
pub(super) fn install<'app, S, M>(
    app: &'app mut App,
    name: &str,
    system: S,
) -> Result<&'app mut App, CompletionRegistrationError>
where
    S: IntoSystem<InRef<'static, CompletionRequest>, Vec<Candidate>, M> + 'static,
    S::System: ReadOnlySystem,
    M: 'static,
{
    // Resolve the owner before initializing provider-local system state.
    let _owner = app
        .world()
        .get_resource::<super::CommandRegistry>()
        .and_then(|registry| registry.resolve(name))
        .ok_or(CompletionRegistrationError::UnknownCommand)?;
    let mut system = IntoSystem::into_system(system);
    system.initialize(app.world_mut());
    // Read-only systems may still buffer mutations; discovery excludes them.
    if system.has_deferred() {
        return Err(CompletionRegistrationError::DeferredMutation);
    }
    if let Some(mut registry) = app.world_mut().get_resource_mut::<super::CommandRegistry>() {
        registry.set_completion_provider(name, SystemProvider(std::sync::Mutex::new(system)))?;
    }
    Ok(app)
}
