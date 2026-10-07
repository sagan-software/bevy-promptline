//! Existing extension-trait implementors retain their original required methods.
use bevy::prelude::*;
use bevy_promptline::input::{
    Candidate, CommandBuilder, CompletionRegistrationError, CompletionRequest,
};
use bevy_promptline::{CommandOutcome, ConsoleCommand, PromptlineAppExt, RegistrationError};

/// Host wrapper delegates the original synchronous extension contract.
struct Host(App);
impl PromptlineAppExt for Host {
    fn register_command<C: ConsoleCommand, M: 'static>(
        &mut self,
        system: impl IntoSystem<In<C>, CommandOutcome, M> + 'static,
    ) -> Result<&mut Self, RegistrationError> {
        self.0.register_command(system)?;
        Ok(self)
    }
    fn register_builder<C: Send + 'static, M: 'static>(
        &mut self,
        builder: CommandBuilder<C>,
        system: impl IntoSystem<In<C>, CommandOutcome, M> + 'static,
    ) -> Result<&mut Self, RegistrationError> {
        self.0.register_builder(builder, system)?;
        Ok(self)
    }
    fn register_completion_system<S, M>(
        &mut self,
        name: &str,
        system: S,
    ) -> Result<&mut Self, CompletionRegistrationError>
    where
        S: IntoSystem<InRef<'static, CompletionRequest>, Vec<Candidate>, M> + 'static,
        S::System: ReadOnlySystem,
        M: 'static,
    {
        self.0.register_completion_system(name, system)?;
        Ok(self)
    }
}

#[test]
fn existing_implementor_requires_no_pending_method() {
    let mut host = Host(App::new());
    host.0.add_plugins(bevy_promptline::PromptlineRuntimePlugin);
    host.register_builder(
        CommandBuilder::new(
            bevy_promptline::CommandMetadata::new("ready", "Existing host command").unwrap(),
            |_| Ok(bevy_promptline::input::ParseDecision::Run(())),
        ),
        |_: In<()>| CommandOutcome::Success,
    )
    .unwrap();
    host.0
        .world_mut()
        .write_message(bevy_promptline::SubmitCommand::new("ready"));
    host.0.update();
    let terminal = host
        .0
        .world_mut()
        .resource_mut::<Messages<bevy_promptline::CommandNotification>>()
        .drain()
        .last()
        .unwrap();
    assert_eq!(
        terminal.status,
        bevy_promptline::command::InvocationStatus::Finished(CommandOutcome::Success)
    );
}
