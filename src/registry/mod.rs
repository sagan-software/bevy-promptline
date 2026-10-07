//! Typed system registration and permission-aware command discovery.
pub use crate::identity::RegistrationError;
use crate::{command::CommandOutcome, metadata::ConsoleCommand};
use bevy::prelude::*;
mod builder;
mod matching;
mod pending;
mod providers;
pub use self::builder::CommandBuilder;
pub use self::matching::NameMatching;
pub use self::providers::{
    CompletionFailure, CompletionProvider, CompletionRegistrationError, CompletionRequest,
};

/// Stores registered metadata, aliases, and typed dispatch adapters.
/// Authorization governs discovery and every submission source; parser
/// callbacks receive arguments without mutable access to the World.
#[derive(Resource)]
pub struct CommandRegistry {
    /// Primary names and aliases resolve to the same command.
    commands: std::collections::BTreeMap<crate::CommandName, std::sync::Arc<RegisteredCommand>>,
    /// App-supplied authorization policy, defaulting to explicit registrations.
    authorize: Box<Authorize>,
    /// Metadata revision used to invalidate completion state.
    revision: u64,
    /// Live providers are keyed by canonical command ownership, including alias
    /// registration.
    /// Explicit name matching profile, independent of argument spelling and
    /// actor policy.
    matching: NameMatching,
    /// Live game metadata providers share canonical command ownership.
    providers:
        std::collections::BTreeMap<crate::CommandName, std::sync::Arc<dyn CompletionProvider>>,
}

/// Extends a Bevy app with typed command registration. The parser creates
/// accepted input, and one registered system owns execution for that command
/// and its aliases.
pub trait PromptlineAppExt {
    /// Registers typed parsing with one Bevy system as execution owner.
    /// Metadata validation and collision checks precede system registration.
    ///
    /// # Errors
    ///
    /// Rejects primary-name and alias collisions before registration.
    fn register_command<C: ConsoleCommand, M: 'static>(
        &mut self,
        system: impl IntoSystem<In<C>, CommandOutcome, M> + 'static,
    ) -> Result<&mut Self, RegistrationError>;
    /// Registers author-supplied typed parsing with one Bevy system as
    /// execution owner.
    /// This method needs no metadata trait and preserves the runtime's
    /// validation and permission boundaries.
    ///
    /// # Errors
    ///
    /// Rejects primary-name and alias collisions before adding a handler
    /// system.
    fn register_builder<C: Send + 'static, M: 'static>(
        &mut self,
        builder: CommandBuilder<C>,
        system: impl IntoSystem<In<C>, CommandOutcome, M> + 'static,
    ) -> Result<&mut Self, RegistrationError>;
    /// Registers a read-only Bevy system that supplies live values for an
    /// existing command.
    /// Authorization precedes every provider call, and the adapter never
    /// invokes the command parser.
    ///
    /// # Errors
    ///
    /// Rejects an unknown owner or deferred mutation parameters before provider
    /// installation.
    ///
    /// Mutable resource access cannot satisfy the provider's capability bound:
    ///
    /// ```compile_fail
    /// use bevy::prelude::*;
    /// use bevy_promptline::PromptlineAppExt;
    /// use bevy_promptline::input::{Candidate, CompletionRequest};
    /// #[derive(Resource)]
    /// struct Models(Vec<String>);
    /// let mut app = App::new();
    /// app.register_completion_system("scene",
    ///     |_: InRef<'_, CompletionRequest>, mut models: ResMut<'_, Models>| {
    ///         models.0.clear();
    ///         Vec::<Candidate>::new()
    ///     });
    /// ```
    fn register_completion_system<S, M>(
        &mut self,
        name: &str,
        system: S,
    ) -> Result<&mut Self, CompletionRegistrationError>
    where
        S: IntoSystem<InRef<'static, CompletionRequest>, Vec<crate::metadata::Candidate>, M>
            + 'static,
        S::System: ReadOnlySystem,
        M: 'static;
}
impl PromptlineAppExt for App {
    fn register_command<C: ConsoleCommand, M: 'static>(
        &mut self,
        system: impl IntoSystem<In<C>, CommandOutcome, M> + 'static,
    ) -> Result<&mut Self, RegistrationError> {
        register_parser(
            self,
            C::metadata()?,
            |arguments| C::parse(arguments).map(ParseDecision::Run),
            None,
            system,
        )
    }
    fn register_builder<C: Send + 'static, M: 'static>(
        &mut self,
        builder: CommandBuilder<C>,
        system: impl IntoSystem<In<C>, CommandOutcome, M> + 'static,
    ) -> Result<&mut Self, RegistrationError> {
        // The same registration seam preserves collision atomicity and handler ownership.
        register_parser(self, builder.metadata, builder.parse, None, system)
    }
    fn register_completion_system<S, M>(
        &mut self,
        name: &str,
        system: S,
    ) -> Result<&mut Self, CompletionRegistrationError>
    where
        S: IntoSystem<InRef<'static, CompletionRequest>, Vec<crate::metadata::Candidate>, M>
            + 'static,
        S::System: ReadOnlySystem,
        M: 'static,
    {
        providers::install(self, name, system)
    }
}

/// Deferred typed input owned by exactly one registered execution system.
pub(crate) enum PreparedCommand {
    /// Accepted typed input owned by one registered system.
    Run(Box<dyn FnOnce(&mut World) -> crate::command::pending::CommandExecution + Send>),
    /// Parser-generated help or version output without a handler invocation.
    Information(String),
}

/// Separates accepted typed handler input from an explicit informational
/// request.
/// Information prints through bounded output and finishes successfully without
/// invoking a registered game handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseDecision<C> {
    /// Transfers this typed value to the registered handler after runtime
    /// acceptance.
    /// Validation and permission checks have already succeeded before the
    /// execution owner receives it.
    Run(C),
    /// Prints author-supplied information without starting or invoking the
    /// registered handler.
    /// Help and version requests use this terminal path while still producing
    /// acceptance and finish observations.
    Information(String),
}
/// Parser that does not run a game handler until acceptance.
type Prepare = dyn Fn(&[String]) -> Result<PreparedCommand, String> + Send + Sync;
/// Metadata callback used without executing argument validators.
pub(crate) type Describe =
    dyn Fn(&[crate::Token], usize) -> crate::metadata::ContextDescription + Send + Sync;
/// Permission callback shared by discovery and submission.
type Authorize =
    dyn Fn(&World, &crate::CommandMetadata, crate::command::SubmissionSource) -> bool + Send + Sync;

/// A command descriptor and its typed dispatch adapter.
pub(crate) struct RegisteredCommand {
    /// Metadata reused by help and completion.
    pub(crate) metadata: crate::CommandMetadata,
    /// Decodes arguments into one deferred handler invocation.
    pub(crate) prepare: Box<Prepare>,
    /// Optional adapter context analysis, independent from parsing.
    pub(crate) describe: Option<Box<Describe>>,
}

impl std::fmt::Debug for CommandRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CommandRegistry")
            .field("commands", &self.commands.keys())
            .field("revision", &self.revision)
            .field("matching", &self.matching)
            .field("providers", &self.providers.keys())
            .field("authorize", &"<redacted callback>")
            .finish()
    }
}
impl Default for CommandRegistry {
    fn default() -> Self {
        Self {
            commands: std::collections::BTreeMap::new(),
            authorize: Box::new(|_, _, _| true),
            revision: 0,
            providers: std::collections::BTreeMap::new(),
            matching: NameMatching::Exact,
        }
    }
}
impl CommandRegistry {
    /// Replaces the actor policy used by discovery and submission. The registry
    /// increments its metadata revision so analysis can discard results from
    /// the previous policy.
    pub fn set_policy(
        &mut self,
        policy: impl Fn(&World, &crate::CommandMetadata, crate::command::SubmissionSource) -> bool
        + Send
        + Sync
        + 'static,
    ) {
        self.authorize = Box::new(policy);
        self.revision += 1;
    }
    /// Borrows primary command metadata permitted for the given World and input
    /// source. Aliases resolve during submission but do not create duplicate
    /// entries in this iterator.
    pub fn visible<'a>(
        &'a self,
        world: &'a World,
        source: crate::command::SubmissionSource,
    ) -> impl Iterator<Item = &'a crate::CommandMetadata> {
        self.commands
            .iter()
            .filter(move |(name, command)| {
                *name == &command.metadata.name && self.is_permitted(world, command, source)
            })
            .map(|(_, command)| &command.metadata)
    }
    /// Returns the current metadata revision without mutating the registry.
    /// Successful registration and policy replacement increment it, allowing
    /// analysis to identify results produced from earlier metadata.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }
    /// Changes name matching only after every registered identity passes
    /// collision validation.
    /// The ordinary exact profile remains the default; this setting never
    /// changes submitted arguments or history spelling.
    ///
    /// # Errors
    ///
    /// Rejects an ASCII-folded collision before changing lookup, discovery, or
    /// metadata revision.
    pub fn set_name_matching(&mut self, matching: NameMatching) -> Result<(), RegistrationError> {
        // Validate the complete collision domain before changing lookup policy.
        if matching == NameMatching::AsciiInsensitive {
            for (index, left) in self.commands.keys().enumerate() {
                let mut remaining = self.commands.keys().skip(index + 1);
                let is_collision = remaining.any(|right| matching.is_identity_match(left, right));
                if is_collision {
                    return Err(RegistrationError::Collision(left.as_ref().to_owned()));
                }
            }
        }
        // Discovery invalidates old candidates only after successful validation.
        self.matching = matching;
        self.revision += 1;
        Ok(())
    }
    /// Applies the same explicit profile to metadata prefixes and full command
    /// resolution.
    pub(crate) fn is_name_prefix_match(&self, name: &str, prefix: &str) -> bool {
        self.matching.is_prefix(name, prefix)
    }
    /// Installs a custom read-only provider for an existing primary identity or
    /// alias.
    /// Replacing a provider updates metadata revision without changing parser
    /// ownership, collision mappings, or actor policy.
    ///
    /// # Errors
    ///
    /// Returns `UnknownCommand` before changing provider ownership when the
    /// name does not resolve.
    pub fn set_completion_provider(
        &mut self,
        name: &str,
        provider: impl CompletionProvider,
    ) -> Result<(), CompletionRegistrationError> {
        let owner = self
            .resolve(name)
            .ok_or(CompletionRegistrationError::UnknownCommand)?;
        self.providers
            .insert(owner.metadata.name.clone(), std::sync::Arc::new(provider));
        self.revision += 1;
        Ok(())
    }
    /// Borrows a live provider by canonical command identity after permission
    /// validation.
    pub(crate) fn provider(&self, name: &crate::CommandName) -> Option<&dyn CompletionProvider> {
        self.providers.get(name).map(AsRef::as_ref)
    }
    /// Resolves a validated spelling to a shared descriptor.
    pub(crate) fn resolve(&self, name: &str) -> Option<std::sync::Arc<RegisteredCommand>> {
        // Borrowed lookup preserves labels validated by either supported registration profile.
        match self.matching {
            NameMatching::Exact => self.commands.get(name).cloned(),
            NameMatching::AsciiInsensitive => self
                .commands
                .iter()
                .find(|(candidate, _)| self.matching.is_match(candidate.as_ref(), name))
                .map(|(_, command)| command.clone()),
        }
    }
    /// Evaluates actor policy without exposing raw arguments.
    pub(crate) fn is_permitted(
        &self,
        world: &World,
        command: &RegisteredCommand,
        source: crate::command::SubmissionSource,
    ) -> bool {
        (self.authorize)(world, &command.metadata, source)
    }
    /// Checks the entire collision domain before registering a system.
    fn validate(&self, metadata: &crate::CommandMetadata) -> Result<(), RegistrationError> {
        let mut names: std::collections::BTreeSet<&crate::CommandName> =
            std::collections::BTreeSet::new();
        // One collision domain covers the primary identity and every alias.
        for name in std::iter::once(&metadata.name).chain(&metadata.aliases) {
            let is_repeated = names
                .iter()
                .any(|previous| self.matching.is_identity_match(previous, name));
            names.insert(name);
            let is_registered = self.resolve(name.as_ref()).is_some();
            // A failed identity prevents all registration mutation.
            if is_repeated || is_registered {
                return Err(RegistrationError::Collision(name.as_str().to_owned()));
            }
        }
        Ok(())
    }
    /// Inserts validated metadata and an already registered dispatch adapter.
    fn insert(&mut self, command: RegisteredCommand) {
        // Aliases share the descriptor and sole execution owner.
        let command = std::sync::Arc::new(command);
        for name in std::iter::once(&command.metadata.name).chain(&command.metadata.aliases) {
            self.commands.insert(name.clone(), command.clone());
        }
        self.revision += 1;
    }
}

/// Connects an adapter parser to the same registry and execution owner as core
/// commands.
pub(crate) fn register_parser<C: Send + 'static, M: 'static>(
    app: &mut App,
    metadata: crate::CommandMetadata,
    parse: impl Fn(&[String]) -> Result<ParseDecision<C>, String> + Send + Sync + 'static,
    describe: Option<Box<Describe>>,
    system: impl IntoSystem<In<C>, CommandOutcome, M> + 'static,
) -> Result<&mut App, RegistrationError> {
    // Synchronous handlers convert only their return value; their public signature stays intact.
    register_execution(
        app,
        metadata,
        parse,
        describe,
        system.map(crate::command::pending::CommandExecution::Finished),
    )
}

/// Installs either handler mode through the same validation and parsing owner.
pub(crate) fn register_execution<C: Send + 'static, M: 'static>(
    app: &mut App,
    metadata: crate::CommandMetadata,
    parse: impl Fn(&[String]) -> Result<ParseDecision<C>, String> + Send + Sync + 'static,
    describe: Option<Box<Describe>>,
    system: impl IntoSystem<In<C>, crate::command::pending::CommandExecution, M> + 'static,
) -> Result<&mut App, RegistrationError> {
    app.init_resource::<CommandRegistry>();
    // Validate the entire identity set before creating a handler system.
    app.world_mut()
        .resource_scope(|world, mut registry: Mut<'_, CommandRegistry>| {
            registry.validate(&metadata)?;
            let id = world.register_system(system);
            let prepare = Box::new(move |arguments: &[String]| match parse(arguments)? {
                ParseDecision::Run(typed) => Ok(PreparedCommand::Run(Box::new(move |world| {
                    match world.run_system_with(id, typed) {
                        Ok(outcome) => outcome,
                        Err(error) => crate::command::pending::CommandExecution::Finished(
                            CommandOutcome::Failure(error.to_string()),
                        ),
                    }
                }))),
                ParseDecision::Information(text) => Ok(PreparedCommand::Information(text)),
            });
            // All aliases share the parser and sole execution owner.
            registry.insert(RegisteredCommand {
                metadata,
                prepare,
                describe,
            });
            Ok::<_, RegistrationError>(())
        })?;
    Ok(app)
}
