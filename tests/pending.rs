//! Pending invocation completion and cooperative cancellation at the public seam.
use bevy::prelude::*;
use bevy_promptline::command::pending::{
    CancelCommand, CommandCompletion, CommandExecution, PendingAppExt, PendingCommand,
    PendingLimits,
};
use bevy_promptline::command::{InvocationId, InvocationStatus};
use bevy_promptline::{
    CommandMetadata, CommandNotification, CommandOutcome, ConsoleCommand, InvalidCommandName,
    Output, PromptlineAppExt, PromptlineRuntimePlugin, SubmitCommand,
};

/// Work remains stalled until the fixture explicitly completes its endpoint.
#[derive(Default, Resource)]
struct Work(Vec<CommandCompletion>);
/// Stalled work uses the same metadata and parser policy as synchronous commands.
struct Wait;
impl ConsoleCommand for Wait {
    fn metadata() -> Result<CommandMetadata, InvalidCommandName> {
        CommandMetadata::new("wait", "Start controlled pending work")
    }
    fn parse(_: &[String]) -> Result<Self, String> {
        Ok(Self)
    }
}
/// Starts one owned completion channel without retaining a World borrow.
fn wait(_: In<Wait>, mut work: ResMut<'_, Work>) -> CommandExecution {
    let (pending, completion) = PendingCommand::channel();
    work.0.push(completion);
    CommandExecution::Pending(pending)
}
/// Constructs a host-owned headless runtime with one pending registration.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin)
        .init_resource::<Work>();
    app.register_pending_command::<Wait, _>(wait).unwrap();
    app
}
/// Collects only newly emitted transitions without relying on message retention.
fn observations(app: &mut App) -> Vec<CommandNotification> {
    app.world_mut()
        .resource_mut::<Messages<CommandNotification>>()
        .drain()
        .collect()
}
/// Starts a stalled invocation and verifies its exact pre-terminal lifecycle.
fn start(app: &mut App) -> InvocationId {
    app.world_mut().write_message(SubmitCommand::new("wait"));
    app.update();
    let events = observations(app);
    assert_eq!(
        events.iter().map(|event| &event.status).collect::<Vec<_>>(),
        [
            &InvocationStatus::Submitted,
            &InvocationStatus::Accepted,
            &InvocationStatus::Started
        ]
    );
    events.first().unwrap().invocation
}

#[test]
fn stalled_work_allows_dispatch_and_analysis_then_finishes_once() {
    let mut app = app();
    let invocation = start(&mut app);
    app.register_builder(
        bevy_promptline::input::CommandBuilder::new(
            CommandMetadata::new("echo", "Print").unwrap(),
            |_| Ok(bevy_promptline::input::ParseDecision::Run(())),
        ),
        |_: In<()>, mut output: ResMut<'_, Output>| {
            output.write("concurrent command");
            CommandOutcome::Success
        },
    )
    .unwrap();
    app.world_mut().write_message(SubmitCommand::new("echo"));
    app.update();
    assert_eq!(
        app.world().resource::<Output>().texts().collect::<Vec<_>>(),
        ["concurrent command"]
    );
    assert_eq!(
        bevy_promptline::input::analyze(app.world(), "ec", 2)
            .unwrap()
            .ghost(bevy_promptline::input::EditingMode::Caret),
        Some("ho")
    );
    assert!(
        observations(&mut app)
            .iter()
            .all(|event| event.invocation != invocation)
    );
    app.world_mut()
        .resource_mut::<Work>()
        .0
        .pop()
        .unwrap()
        .finish(CommandOutcome::Success)
        .unwrap();
    app.update();
    let events = observations(&mut app);
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().invocation, invocation);
    assert_eq!(
        events.first().unwrap().status,
        InvocationStatus::Finished(CommandOutcome::Success)
    );
    app.update();
    assert!(observations(&mut app).is_empty());
}

#[test]
fn cancellation_waits_for_worker_acknowledgement_and_ignores_finished_ids() {
    let mut app = app();
    let invocation = start(&mut app);
    let token = app
        .world()
        .resource::<Work>()
        .0
        .first()
        .unwrap()
        .cancellation();
    assert!(!token.is_requested());
    app.world_mut().write_message(CancelCommand { invocation });
    app.update();
    assert!(token.is_requested());
    assert!(observations(&mut app).is_empty());
    app.world_mut()
        .resource_mut::<Work>()
        .0
        .pop()
        .unwrap()
        .finish(CommandOutcome::Cancelled)
        .unwrap();
    app.update();
    assert_eq!(
        observations(&mut app).first().unwrap().status,
        InvocationStatus::Finished(CommandOutcome::Cancelled)
    );
    app.world_mut().write_message(CancelCommand { invocation });
    app.update();
    assert!(observations(&mut app).is_empty());
}

#[test]
fn disconnected_completion_finishes_with_a_diagnostic() {
    let mut app = app();
    start(&mut app);
    drop(app.world_mut().resource_mut::<Work>().0.pop().unwrap());
    app.update();
    assert_eq!(
        observations(&mut app).first().unwrap().status,
        InvocationStatus::Finished(CommandOutcome::Failure(
            "pending command completion disconnected".into()
        ))
    );
}

#[test]
fn capacity_cancels_excess_work_and_preserves_existing_pending_work() {
    let mut app = app();
    app.insert_resource(PendingLimits {
        invocations: bevy_promptline::Limit::try_from(1).unwrap(),
    });
    let first = start(&mut app);
    app.world_mut().write_message(SubmitCommand::new("wait"));
    app.update();
    let events = observations(&mut app);
    assert_eq!(events.len(), 4);
    assert_eq!(
        events.get(3).unwrap().status,
        InvocationStatus::Finished(CommandOutcome::Failure(
            "pending command limit reached".into()
        ))
    );
    let excess = app.world_mut().resource_mut::<Work>().0.pop().unwrap();
    assert!(excess.cancellation().is_requested());
    let error = excess.finish(CommandOutcome::Cancelled).unwrap_err();
    assert_eq!(error.0, CommandOutcome::Cancelled);
    app.world_mut()
        .resource_mut::<Work>()
        .0
        .pop()
        .unwrap()
        .finish(CommandOutcome::Success)
        .unwrap();
    app.update();
    assert_eq!(observations(&mut app).first().unwrap().invocation, first);
}

#[test]
fn completion_endpoint_reports_receiver_loss_without_requesting_cancellation() {
    let (pending, completion) = PendingCommand::channel();
    let token = completion.cancellation();
    drop(pending);
    let error = completion.finish(CommandOutcome::Success).unwrap_err();
    assert_eq!(error.0, CommandOutcome::Success);
    assert!(!token.is_requested());
}

#[test]
fn immediate_execution_preserves_ui_origin_and_terminal_failure() {
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_pending_command::<Wait, _>(|_: In<Wait>| -> CommandExecution {
        CommandOutcome::Failure("immediate failure".into()).into()
    })
    .unwrap();
    let mut submission = SubmitCommand::new("wait");
    submission.source = bevy_promptline::command::SubmissionSource::Ui;
    app.world_mut().write_message(submission);
    app.update();
    let events = observations(&mut app);
    assert_eq!(events.len(), 4);
    assert!(
        events
            .iter()
            .all(|event| event.source == bevy_promptline::command::SubmissionSource::Ui)
    );
    assert_eq!(
        events.last().unwrap().status,
        InvocationStatus::Finished(CommandOutcome::Failure("immediate failure".into()))
    );
}

#[test]
fn removed_pending_limits_cancel_new_work_with_a_terminal_failure() {
    let mut app = app();
    app.world_mut().remove_resource::<PendingLimits>();
    app.world_mut().write_message(SubmitCommand::new("wait"));
    app.update();
    assert_eq!(
        observations(&mut app).last().unwrap().status,
        InvocationStatus::Finished(CommandOutcome::Failure(
            "pending command runtime unavailable".into()
        ))
    );
    assert!(
        app.world()
            .resource::<Work>()
            .0
            .first()
            .unwrap()
            .cancellation()
            .is_requested()
    );
}

#[test]
fn pending_registration_collision_preserves_the_original_synchronous_handler() {
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin)
        .init_resource::<Work>();
    app.register_command::<Wait, _>(|_: In<Wait>, mut output: ResMut<'_, Output>| {
        output.write("original handler");
        CommandOutcome::Success
    })
    .unwrap();
    let error = app.register_pending_command::<Wait, _>(wait).unwrap_err();
    assert_eq!(
        error,
        bevy_promptline::RegistrationError::Collision("wait".into())
    );
    app.world_mut().write_message(SubmitCommand::new("wait"));
    app.update();
    assert_eq!(
        app.world().resource::<Output>().texts().collect::<Vec<_>>(),
        ["original handler"]
    );
    assert!(app.world().resource::<Work>().0.is_empty());
}

/// Invalid metadata cannot create a pending execution owner.
struct Invalid;
impl ConsoleCommand for Invalid {
    fn metadata() -> Result<CommandMetadata, InvalidCommandName> {
        CommandMetadata::new("", "")
    }
    fn parse(_: &[String]) -> Result<Self, String> {
        Ok(Self)
    }
}
#[test]
fn invalid_pending_metadata_and_denied_input_never_start_work() {
    let mut app = app();
    let error = app
        .register_pending_command::<Invalid, _>(|_: In<Invalid>| {
            CommandExecution::Finished(CommandOutcome::Success)
        })
        .unwrap_err();
    assert!(matches!(
        error,
        bevy_promptline::RegistrationError::InvalidIdentity(_)
    ));
    app.world_mut()
        .resource_mut::<bevy_promptline::CommandRegistry>()
        .set_policy(|_, _, _| false);
    app.world_mut().write_message(SubmitCommand::new("wait"));
    app.update();
    let events = observations(&mut app);
    assert_eq!(events.len(), 2);
    assert_eq!(
        events.last().unwrap().status,
        InvocationStatus::Rejected("command is not permitted".into())
    );
    assert!(app.world().resource::<Work>().0.is_empty());
}

#[test]
fn cancellation_is_selective_and_worker_outcomes_preserve_their_origin() {
    let mut app = app();
    let mut submission = SubmitCommand::new("wait");
    submission.source = bevy_promptline::command::SubmissionSource::Ui;
    app.world_mut().write_message(submission);
    app.update();
    let first = observations(&mut app).first().unwrap().invocation;
    let second = start(&mut app);
    let other = app
        .world()
        .resource::<Work>()
        .0
        .get(1)
        .unwrap()
        .cancellation();
    app.world_mut()
        .write_message(CancelCommand { invocation: first });
    app.update();
    assert!(!other.is_requested());
    app.world_mut()
        .resource_mut::<Work>()
        .0
        .remove(0)
        .finish(CommandOutcome::Success)
        .unwrap();
    app.update();
    let events = observations(&mut app);
    assert_eq!(events.first().unwrap().invocation, first);
    assert_eq!(
        events.first().unwrap().source,
        bevy_promptline::command::SubmissionSource::Ui
    );
    assert_eq!(
        events.first().unwrap().status,
        InvocationStatus::Finished(CommandOutcome::Success)
    );
    app.world_mut()
        .write_message(CancelCommand { invocation: first });
    app.update();
    assert!(!other.is_requested());
    assert!(observations(&mut app).is_empty());
    app.world_mut()
        .resource_mut::<Work>()
        .0
        .pop()
        .unwrap()
        .finish(CommandOutcome::Failure("worker failure".into()))
        .unwrap();
    app.update();
    let events = observations(&mut app);
    assert_eq!(events.first().unwrap().invocation, second);
    assert_eq!(
        events.first().unwrap().status,
        InvocationStatus::Finished(CommandOutcome::Failure("worker failure".into()))
    );
}

#[test]
fn pending_debug_omits_queued_worker_diagnostics() {
    let (pending, completion) = PendingCommand::channel();
    assert!(!completion.as_ref().is_requested());
    completion
        .finish(CommandOutcome::Failure("private diagnostic".into()))
        .unwrap();
    let diagnostic = format!("{pending:?}");
    assert!(diagnostic.starts_with("PendingCommand"));
    assert!(!diagnostic.contains("private diagnostic"));
}

/// Redacted arguments must not escape through runtime-owned terminal diagnostics.
struct Sensitive;
impl ConsoleCommand for Sensitive {
    fn metadata() -> Result<CommandMetadata, InvalidCommandName> {
        let mut metadata = CommandMetadata::new("sensitive", "Protected arguments")?;
        metadata.sensitivity = bevy_promptline::command::Sensitivity::Redacted;
        Ok(metadata)
    }
    fn parse(_: &[String]) -> Result<Self, String> {
        Ok(Self)
    }
}

#[test]
fn sensitive_terminal_diagnostics_are_redacted_for_immediate_and_pending_work() {
    for is_pending in [false, true] {
        let mut app = App::new();
        app.add_plugins(PromptlineRuntimePlugin)
            .init_resource::<Work>();
        if is_pending {
            app.register_pending_command::<Sensitive, _>(
                |_: In<Sensitive>, mut work: ResMut<'_, Work>| {
                    let (pending, completion) = PendingCommand::channel();
                    work.0.push(completion);
                    CommandExecution::Pending(pending)
                },
            )
            .unwrap();
        } else {
            app.register_command::<Sensitive, _>(|_: In<Sensitive>| {
                CommandOutcome::Failure("sensitive-value".into())
            })
            .unwrap();
        }
        app.world_mut()
            .write_message(SubmitCommand::new("sensitive sensitive-value"));
        app.update();
        if is_pending {
            observations(&mut app);
            app.world_mut()
                .resource_mut::<Work>()
                .0
                .pop()
                .unwrap()
                .finish(CommandOutcome::Failure("sensitive-value".into()))
                .unwrap();
            app.update();
        }
        let events = observations(&mut app);
        assert_eq!(
            events.last().unwrap().status,
            InvocationStatus::Finished(CommandOutcome::Failure(
                "command failed for a sensitive command".into()
            ))
        );
        assert!(!format!("{events:?}").contains("sensitive-value"));
    }
}
