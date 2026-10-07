//! Live game values come from read-only Bevy systems without invoking parsing.
use bevy::prelude::*;
use bevy_promptline::input::{Candidate, CompletionRegistrationError, CompletionRequest, analyze};
use bevy_promptline::{
    CommandMetadata, CommandOutcome, ConsoleCommand, PromptlineAppExt, PromptlineRuntimePlugin,
};

/// Available game models change independently of the registered command parser.
#[derive(Resource)]
struct Models(Vec<&'static str>);
/// Command parsing must remain absent from every discovery operation.
struct ModelCommand;
impl ConsoleCommand for ModelCommand {
    fn metadata() -> Result<CommandMetadata, bevy_promptline::InvalidCommandName> {
        CommandMetadata::new("model", "Choose a model")
    }
    fn parse(_: &[String]) -> Result<Self, String> {
        panic!("completion invoked parsing")
    }
}

#[test]
fn read_only_completion_tracks_live_values_and_permission_before_execution() {
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin)
        .insert_resource(Models(vec!["cube", "cone"]));
    app.register_command::<ModelCommand, _>(|_: In<ModelCommand>| CommandOutcome::Success)
        .unwrap();
    app.register_completion_system(
        "model",
        |InRef(request): InRef<'_, CompletionRequest>, models: Res<'_, Models>| {
            assert_eq!(request.position(), 1);
            models
                .0
                .iter()
                .map(|name| Candidate {
                    value: (*name).into(),
                    description: "Game model".into(),
                })
                .collect::<Vec<_>>()
        },
    )
    .unwrap();
    let first = analyze(app.world(), "model c", 7).unwrap();
    assert_eq!(
        first
            .candidates()
            .iter()
            .map(|value| value.value.as_str())
            .collect::<Vec<_>>(),
        ["cone", "cube"]
    );
    app.world_mut().resource_mut::<Models>().0 = vec!["capsule"];
    assert_eq!(
        analyze(app.world(), "model c", 7).unwrap().candidates()[0].value,
        "capsule"
    );
    app.world_mut()
        .resource_mut::<bevy_promptline::CommandRegistry>()
        .set_policy(|_, _, _| false);
    app.world_mut().remove_resource::<Models>();
    assert!(
        analyze(app.world(), "model c", 7)
            .unwrap()
            .candidates()
            .is_empty()
    );
}

#[test]
fn missing_commands_and_deferred_mutations_are_rejected_before_provider_installation() {
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    assert!(matches!(
        app.register_completion_system("absent", |_: InRef<'_, CompletionRequest>| {
            Vec::<Candidate>::new()
        }),
        Err(CompletionRegistrationError::UnknownCommand)
    ));
    app.register_command::<ModelCommand, _>(|_: In<ModelCommand>| CommandOutcome::Success)
        .unwrap();
    assert!(matches!(
        app.register_completion_system(
            "model",
            |_: InRef<'_, CompletionRequest>, mut commands: Commands<'_, '_>| {
                commands.spawn_empty();
                Vec::<Candidate>::new()
            }
        ),
        Err(CompletionRegistrationError::DeferredMutation)
    ));
}

#[test]
fn provider_request_debug_does_not_disclose_draft_tokens_or_prefix() {
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<ModelCommand, _>(|_: In<ModelCommand>| CommandOutcome::Success)
        .unwrap();
    app.register_completion_system("model", |InRef(request): InRef<'_, CompletionRequest>| {
        let debug = format!("{request:?}");
        assert!(!debug.contains("secret-token"));
        Vec::<Candidate>::new()
    })
    .unwrap();
    analyze(app.world(), "model secret-token", 18).unwrap();
}
