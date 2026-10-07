//! Public runtime acceptance tests, independent of the default UI.
use bevy::prelude::*;
use bevy_promptline::command::InvocationStatus;
use bevy_promptline::{
    CommandMetadata, CommandOutcome, ConsoleCommand, Output, PromptlineAppExt,
    PromptlineRuntimePlugin, SubmitCommand,
};

/// Typed command used by the minimal guide.
#[derive(Debug)]
struct Echo(String);

impl ConsoleCommand for Echo {
    fn metadata() -> Result<CommandMetadata, bevy_promptline::InvalidCommandName> {
        CommandMetadata::new("echo", "Print text")
    }
    fn parse(arguments: &[String]) -> Result<Self, String> {
        Ok(Self(arguments.join(" ")))
    }
}

/// Dispatches typed arguments through an ordinary Bevy system.
fn echo(In(command): In<Echo>, mut output: ResMut<'_, Output>) -> CommandOutcome {
    output.write(command.0);
    CommandOutcome::Success
}

#[test]
fn public_command_reaches_system_once_in_queue_order() {
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<Echo, _>(echo).unwrap();
    app.world_mut()
        .write_message(SubmitCommand::new("echo first"));
    app.world_mut()
        .write_message(SubmitCommand::new("echo second"));
    app.update();
    assert_eq!(
        app.world().resource::<Output>().texts().collect::<Vec<_>>(),
        ["first", "second"]
    );
    app.update();
    assert_eq!(app.world().resource::<Output>().texts().count(), 2);
}

/// A parser rejection cannot reach its handler.
#[derive(Debug)]
struct Reject;
impl ConsoleCommand for Reject {
    fn metadata() -> Result<CommandMetadata, bevy_promptline::InvalidCommandName> {
        CommandMetadata::new("reject", "Always reject input")
    }
    fn parse(_: &[String]) -> Result<Self, String> {
        Err("bad arguments".to_owned())
    }
}

#[test]
fn parser_policy_and_input_rejections_never_reach_a_handler() {
    use bevy_promptline::{CommandNotification, CommandRegistry, Limit, RuntimeLimits};
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<Echo, _>(echo).unwrap();
    app.register_command::<Reject, _>(|_: In<Reject>| -> CommandOutcome {
        panic!("rejection reached handler")
    })
    .unwrap();
    app.world_mut().resource_mut::<RuntimeLimits>().input_bytes = Limit::try_from(32).unwrap();
    for input in [
        "",
        "missing",
        "echo '",
        "reject",
        "echo aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ] {
        app.world_mut().write_message(SubmitCommand::new(input));
    }
    app.update();
    let notifications: Vec<_> = app
        .world_mut()
        .resource_mut::<Messages<CommandNotification>>()
        .drain()
        .collect();
    assert_eq!(notifications.len(), 10);
    assert!(notifications.iter().all(|message| matches!(
        message.status,
        InvocationStatus::Submitted | InvocationStatus::Rejected(_)
    )));
    app.world_mut()
        .resource_mut::<CommandRegistry>()
        .set_policy(|_, _, _| false);
    for source in [
        bevy_promptline::command::SubmissionSource::Ui,
        bevy_promptline::command::SubmissionSource::Programmatic,
    ] {
        let mut submission = SubmitCommand::new("echo forbidden");
        submission.source = source;
        app.world_mut().write_message(submission);
    }
    app.update();
    assert_eq!(
        app.world()
            .resource::<Output>()
            .texts()
            .filter(|text| *text == "command is not permitted")
            .count(),
        2
    );
    assert!(
        !app.world()
            .resource::<Output>()
            .texts()
            .any(|text| text == "forbidden")
    );
    assert_eq!(
        app.world()
            .resource::<CommandRegistry>()
            .visible(app.world(), bevy_promptline::command::SubmissionSource::Ui)
            .count(),
        0
    );
}

#[test]
fn collisions_are_atomic_and_aliases_resolve_to_one_owner() {
    use bevy_promptline::RegistrationError;
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<Echo, _>(echo).unwrap();
    assert_eq!(
        match app.register_command::<Echo, _>(echo) {
            Err(error) => error,
            Ok(_) => panic!("duplicate registered"),
        },
        RegistrationError::Collision("echo".to_owned())
    );
    app.register_command::<Reject, _>(|_: In<Reject>| CommandOutcome::Success)
        .unwrap();
    assert_eq!(
        app.world()
            .resource::<bevy_promptline::CommandRegistry>()
            .revision(),
        2
    );
}

#[test]
fn lifecycle_order_and_queue_overflow_are_observable() {
    use bevy_promptline::{CommandNotification, Limit, RuntimeLimits};
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<Echo, _>(echo).unwrap();
    app.world_mut()
        .resource_mut::<RuntimeLimits>()
        .queue_entries = Limit::try_from(1).unwrap();
    app.world_mut()
        .write_message(SubmitCommand::new("echo first"));
    app.world_mut()
        .write_message(SubmitCommand::new("echo second"));
    app.update();
    let notifications: Vec<_> = app
        .world_mut()
        .resource_mut::<Messages<CommandNotification>>()
        .drain()
        .collect();
    assert_eq!(
        notifications
            .iter()
            .map(|message| message.status.clone())
            .collect::<Vec<_>>(),
        [
            InvocationStatus::Submitted,
            InvocationStatus::Accepted,
            InvocationStatus::Started,
            InvocationStatus::Finished(CommandOutcome::Success),
            InvocationStatus::Submitted,
            InvocationStatus::Rejected("command queue is full".to_owned())
        ]
    );
    assert_eq!(
        notifications.first().unwrap().invocation,
        notifications.get(3).unwrap().invocation
    );
    assert_ne!(
        notifications.first().unwrap().invocation,
        notifications.last().unwrap().invocation
    );
    assert_eq!(
        app.world().resource::<Output>().texts().collect::<Vec<_>>(),
        ["first", "command queue is full"]
    );
}

/// Command with one alias to exercise shared identity ownership.
#[derive(Debug)]
struct Aliased;
impl ConsoleCommand for Aliased {
    fn metadata() -> Result<CommandMetadata, bevy_promptline::InvalidCommandName> {
        let mut metadata = CommandMetadata::new("aliased", "An alias fixture")?;
        metadata.aliases.push("echo".parse()?);
        Ok(metadata)
    }
    fn parse(_: &[String]) -> Result<Self, String> {
        Ok(Self)
    }
}

/// Invalid metadata fails before a handler system is registered.
#[derive(Debug)]
struct Invalid;
impl ConsoleCommand for Invalid {
    fn metadata() -> Result<CommandMetadata, bevy_promptline::InvalidCommandName> {
        CommandMetadata::new("bad name", "Invalid")
    }
    fn parse(_: &[String]) -> Result<Self, String> {
        panic!("invalid metadata reached parsing")
    }
}

#[test]
fn alias_collisions_and_invalid_metadata_leave_the_registry_unchanged() {
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<Echo, _>(echo).unwrap();
    assert!(
        matches!(app.register_command::<Aliased, _>(|_: In<Aliased>| CommandOutcome::Success), Err(bevy_promptline::RegistrationError::Collision(name)) if name == "echo")
    );
    assert!(matches!(
        app.register_command::<Invalid, _>(|_: In<Invalid>| CommandOutcome::Success),
        Err(bevy_promptline::RegistrationError::InvalidIdentity(_))
    ));
    assert_eq!(
        app.world()
            .resource::<bevy_promptline::CommandRegistry>()
            .revision(),
        1
    );
    let mut standalone = App::new();
    standalone.add_plugins(PromptlineRuntimePlugin);
    standalone
        .register_command::<Aliased, _>(|_: In<Aliased>, mut output: ResMut<'_, Output>| {
            output.write("alias invoked");
            CommandOutcome::Success
        })
        .unwrap();
    standalone
        .world_mut()
        .write_message(SubmitCommand::new("echo"));
    standalone.update();
    assert_eq!(
        standalone
            .world()
            .resource::<Output>()
            .texts()
            .collect::<Vec<_>>(),
        ["alias invoked"]
    );
    let registry = standalone
        .world()
        .resource::<bevy_promptline::CommandRegistry>();
    assert_eq!(
        registry
            .visible(
                standalone.world(),
                bevy_promptline::command::SubmissionSource::Ui
            )
            .count(),
        1
    );
}

#[test]
fn handler_failures_cancellation_and_removed_systems_have_terminal_notifications() {
    use bevy_promptline::CommandNotification;
    for outcome in [
        CommandOutcome::Failure("game error".to_owned()),
        CommandOutcome::Cancelled,
    ] {
        let mut app = App::new();
        app.add_plugins(PromptlineRuntimePlugin);
        let expected = outcome.clone();
        app.register_command::<Echo, _>(move |_: In<Echo>| outcome.clone())
            .unwrap();
        app.world_mut().write_message(SubmitCommand::new("echo"));
        app.update();
        let statuses: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<CommandNotification>>()
            .drain()
            .map(|message| message.status)
            .collect();
        assert_eq!(statuses.last(), Some(&InvocationStatus::Finished(expected)));
    }
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<Echo, _>(echo).unwrap();
    // Applications own the World and may remove a registered system entity.
    let entities: Vec<_> = app
        .world_mut()
        .query_filtered::<Entity, With<bevy::ecs::system::SystemIdMarker>>()
        .iter(app.world())
        .collect();
    for entity in entities {
        app.world_mut().despawn(entity);
    }
    app.world_mut()
        .write_message(SubmitCommand::new("echo unreachable"));
    app.update();
    assert_eq!(app.world().resource::<Output>().texts().count(), 0);
    let statuses: Vec<_> = app
        .world_mut()
        .resource_mut::<Messages<CommandNotification>>()
        .drain()
        .map(|message| message.status)
        .collect();
    assert!(matches!(
        statuses.last(),
        Some(InvocationStatus::Finished(CommandOutcome::Failure(_)))
    ));
}

#[test]
fn submission_debug_does_not_disclose_sensitive_arguments() {
    let submission = SubmitCommand::new("password secret-value");
    let diagnostic = format!("{submission:?}");
    assert!(!diagnostic.contains("secret-value"));
    assert!(diagnostic.contains("Programmatic"));
}

/// A rejected command whose parser diagnostic includes sensitive input.
#[derive(Debug)]
struct Sensitive;
impl ConsoleCommand for Sensitive {
    fn metadata() -> Result<CommandMetadata, bevy_promptline::InvalidCommandName> {
        let mut metadata = CommandMetadata::new("sensitive", "Sensitive rejection fixture")?;
        metadata.sensitivity = bevy_promptline::command::Sensitivity::Redacted;
        Ok(metadata)
    }
    fn parse(arguments: &[String]) -> Result<Self, String> {
        let supplied = arguments.join(" ");
        Err(format!("invalid secret {supplied}"))
    }
}

#[test]
fn sensitive_rejection_does_not_expose_parser_arguments_in_output_or_notifications() {
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<Sensitive, _>(|_: In<Sensitive>| -> CommandOutcome {
        panic!("rejected secret reached handler")
    })
    .unwrap();
    app.world_mut()
        .write_message(SubmitCommand::new("sensitive secret-value"));
    app.update();
    assert!(
        !app.world()
            .resource::<Output>()
            .texts()
            .any(|text| text.contains("secret-value"))
    );
    let notifications: Vec<_> = app
        .world_mut()
        .resource_mut::<Messages<bevy_promptline::CommandNotification>>()
        .drain()
        .collect();
    assert!(!format!("{notifications:?}").contains("secret-value"));
}

/// Repeated aliases within one descriptor must reject the whole registration.
#[derive(Debug)]
struct RepeatedAlias;
impl ConsoleCommand for RepeatedAlias {
    fn metadata() -> Result<CommandMetadata, bevy_promptline::InvalidCommandName> {
        let mut metadata = CommandMetadata::new("repeat", "Repeated alias fixture")?;
        metadata.aliases.push("repeat".parse()?);
        Ok(metadata)
    }
    fn parse(_: &[String]) -> Result<Self, String> {
        Ok(Self)
    }
}

#[test]
fn repeated_aliases_are_rejected_and_registry_debug_omits_callbacks() {
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<Echo, _>(echo).unwrap();
    assert!(
        matches!(app.register_command::<RepeatedAlias, _>(|_: In<RepeatedAlias>| CommandOutcome::Success), Err(bevy_promptline::RegistrationError::Collision(name)) if name == "repeat")
    );
    let registry = app.world().resource::<bevy_promptline::CommandRegistry>();
    let diagnostic = format!("{registry:?}");
    assert!(diagnostic.contains("echo"));
    assert!(diagnostic.contains("<redacted callback>"));
}

#[test]
fn explicit_ascii_case_matching_preserves_default_and_rejects_ambiguous_transition() {
    use bevy_promptline::input::NameMatching;
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<Echo, _>(|In(command): In<Echo>, mut output: ResMut<'_, Output>| {
        output.write(command.0);
        CommandOutcome::Success
    })
    .unwrap();
    app.world_mut()
        .write_message(SubmitCommand::new("ECHO Original"));
    app.update();
    assert!(
        !app.world()
            .resource::<Output>()
            .texts()
            .any(|text| text == "Original")
    );
    app.world_mut()
        .resource_mut::<bevy_promptline::CommandRegistry>()
        .set_name_matching(NameMatching::AsciiInsensitive)
        .unwrap();
    app.world_mut()
        .write_message(SubmitCommand::new("eChO Original"));
    app.update();
    assert!(
        app.world()
            .resource::<Output>()
            .texts()
            .any(|text| text == "Original")
    );
}

#[test]
fn folded_collision_keeps_exact_registration_and_lookup_unchanged() {
    use bevy_promptline::input::{CommandBuilder, NameMatching, ParseDecision};
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<Echo, _>(echo).unwrap();
    let upper = CommandBuilder::new(
        CommandMetadata::new("ECHO", "Uppercase owner").unwrap(),
        |_| Ok(ParseDecision::Run(())),
    );
    app.register_builder(upper, |_: In<()>, mut output: ResMut<'_, Output>| {
        output.write("uppercase owner");
        CommandOutcome::Success
    })
    .unwrap();
    let revision = app
        .world()
        .resource::<bevy_promptline::CommandRegistry>()
        .revision();
    assert!(
        app.world_mut()
            .resource_mut::<bevy_promptline::CommandRegistry>()
            .set_name_matching(NameMatching::AsciiInsensitive)
            .is_err()
    );
    assert_eq!(
        app.world()
            .resource::<bevy_promptline::CommandRegistry>()
            .revision(),
        revision
    );
    app.world_mut()
        .write_message(SubmitCommand::new("echo lowercase"));
    app.world_mut().write_message(SubmitCommand::new("ECHO"));
    app.update();
    assert_eq!(
        app.world().resource::<Output>().texts().collect::<Vec<_>>(),
        ["lowercase", "uppercase owner"]
    );
}
