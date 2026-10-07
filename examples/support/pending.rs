//! Host-owned scheduled work demonstrates pending commands on native and Wasm.
use bevy::prelude::*;
use bevy_promptline::command::pending::{
    CancelCommand, CommandCompletion, CommandExecution, PendingAppExt, PendingCommand,
};
use bevy_promptline::command::{InvocationId, InvocationStatus};
use bevy_promptline::input::{CommandBuilder, ParseDecision, PromptlineSystems};
use bevy_promptline::{
    CommandMetadata, CommandNotification, CommandOutcome, ConsoleCommand, InvalidCommandName,
    Output, PromptlineAppExt, RegistrationError,
};
use std::time::Duration;

/// Five-second work has no host-dependent thread or filesystem requirement.
#[derive(Debug)]
struct Wait;
impl ConsoleCommand for Wait {
    fn metadata() -> Result<CommandMetadata, InvalidCommandName> {
        CommandMetadata::new("wait", "Start a cancellable five-second task")
    }
    fn parse(arguments: &[String]) -> Result<Self, String> {
        if arguments.is_empty() {
            Ok(Self)
        } else {
            Err("Usage: wait".into())
        }
    }
}
/// Terminal task state retains no completion capability while awaiting despawn.
#[derive(Debug, Component)]
enum PendingTask {
    /// Scheduled work owns exactly one terminal endpoint until completion.
    Running {
        /// Remaining game time is measured as a Duration through Bevy's Timer.
        timer: Timer,
        /// Endpoint sends a result or acknowledges cooperative cancellation.
        completion: CommandCompletion,
    },
    /// Completed work has relinquished its endpoint and cannot finish twice.
    Finished,
}
impl PendingTask {
    /// Moves the endpoint out only when work completes or cancellation is acknowledged.
    fn advance(&mut self, delta: Duration) -> Option<(CommandCompletion, CommandOutcome)> {
        // Each temporary terminal state either releases ownership or restores the running payload.
        match std::mem::replace(self, Self::Finished) {
            Self::Running {
                mut timer,
                completion,
            } => {
                if completion.as_ref().is_requested() {
                    return Some((completion, CommandOutcome::Cancelled));
                }
                // Without cancellation, advance game time before checking the deadline.
                timer.tick(delta);
                if timer.is_finished() {
                    Some((completion, CommandOutcome::Success))
                } else {
                    *self = Self::Running { timer, completion };
                    None
                }
            }
            Self::Finished => None,
        }
    }
}
/// Lifecycle observation tracks running identities without becoming another execution owner.
#[derive(Debug, Default, Resource)]
struct Jobs(Vec<InvocationId>);

/// Starts fixed scheduled work while returning immediately to ordinary command dispatch.
fn start(
    _: In<Wait>,
    mut commands: Commands<'_, '_>,
    mut output: ResMut<'_, Output>,
) -> CommandExecution {
    let (pending, completion) = PendingCommand::channel();
    commands.spawn(PendingTask::Running {
        timer: Timer::new(Duration::from_secs(5), TimerMode::Once),
        completion,
    });
    output.write("Task started. Use cancel to request cancellation.");
    CommandExecution::Pending(pending)
}
/// Sends requests only for identities that still lack a terminal observation.
fn cancel(
    _: In<()>,
    jobs: Res<'_, Jobs>,
    mut requests: MessageWriter<'_, CancelCommand>,
    mut output: ResMut<'_, Output>,
) -> CommandOutcome {
    for invocation in &jobs.0 {
        requests.write(CancelCommand {
            invocation: *invocation,
        });
    }
    if jobs.0.is_empty() {
        output.write("No pending tasks");
    } else {
        output.write("Cancellation requested");
    }
    CommandOutcome::Success
}
/// Returns endpoints after advancing game time, then despawns resource-free terminal tasks.
fn advance_tasks(
    time: Res<'_, Time>,
    mut commands: Commands<'_, '_>,
    mut tasks: Query<'_, '_, (Entity, &mut PendingTask)>,
    mut output: ResMut<'_, Output>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some((completion, outcome)) = task.advance(time.delta()) {
            match &outcome {
                CommandOutcome::Success => output.write("Task completed"),
                CommandOutcome::Cancelled => output.write("Task cancelled"),
                CommandOutcome::Failure(diagnostic) => output.write(diagnostic),
            }
            if completion.finish(outcome).is_err() {
                output.write("Task completion destination is unavailable");
            }
            commands.entity(entity).despawn();
        }
    }
}
/// Maintains host identity bookkeeping from the same buffered lifecycle stream.
fn observe(mut events: MessageReader<'_, '_, CommandNotification>, mut jobs: ResMut<'_, Jobs>) {
    for event in events.read() {
        match event.status {
            InvocationStatus::Started => jobs.0.push(event.invocation),
            InvocationStatus::Finished(_) => {
                jobs.0.retain(|invocation| *invocation != event.invocation);
            }
            InvocationStatus::Submitted
            | InvocationStatus::Rejected(_)
            | InvocationStatus::Accepted => {}
        }
    }
}
/// Installs the pending demonstration through public registration and notification contracts.
pub(crate) fn install(app: &mut App) -> Result<(), RegistrationError> {
    app.init_resource::<Jobs>()
        .add_systems(Update, advance_tasks)
        .add_systems(PostUpdate, observe.after(PromptlineSystems::Output));
    app.register_pending_command::<Wait, _>(start)?;
    app.register_builder(
        CommandBuilder::new(
            CommandMetadata::new("cancel", "Request cancellation of pending tasks")?,
            |arguments| {
                if arguments.is_empty() {
                    Ok(ParseDecision::Run(()))
                } else {
                    Err("Usage: cancel".into())
                }
            },
        ),
        cancel,
    )?;
    Ok(())
}

/// Task ownership checks exercise time, cancellation, and terminal exhaustion directly.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_retains_completion_until_deadline_then_cannot_finish_twice() {
        let (pending, completion) = PendingCommand::channel();
        let mut task = PendingTask::Running {
            timer: Timer::new(Duration::from_secs(5), TimerMode::Once),
            completion,
        };
        assert!(task.advance(Duration::from_secs(4)).is_none());
        let (completion, outcome) = task.advance(Duration::from_secs(1)).unwrap();
        assert_eq!(outcome, CommandOutcome::Success);
        completion.finish(outcome).unwrap();
        assert!(matches!(task, PendingTask::Finished));
        assert!(task.advance(Duration::from_secs(1)).is_none());
        drop(pending);
    }
    #[test]
    fn host_cancellation_acknowledges_work_and_cleans_up_its_identity() {
        let mut app = App::new();
        app.add_plugins(bevy_promptline::PromptlineRuntimePlugin)
            .init_resource::<Time>();
        install(&mut app).unwrap();
        app.world_mut()
            .write_message(bevy_promptline::SubmitCommand::new("wait"));
        app.update();
        assert_eq!(app.world().resource::<Jobs>().0.len(), 1);
        app.world_mut()
            .write_message(bevy_promptline::SubmitCommand::new("cancel"));
        app.update();
        app.update();
        assert!(app.world().resource::<Jobs>().0.is_empty());
        assert_eq!(
            app.world_mut()
                .query::<&PendingTask>()
                .iter(app.world())
                .count(),
            0
        );
        assert!(
            app.world()
                .resource::<Output>()
                .texts()
                .any(|text| text == "Task cancelled")
        );
        app.world_mut()
            .write_message(bevy_promptline::SubmitCommand::new("cancel"));
        app.update();
        assert!(
            app.world()
                .resource::<Output>()
                .texts()
                .any(|text| text == "No pending tasks")
        );
    }

    #[test]
    fn excess_host_work_releases_its_completion_and_reports_destination_loss() {
        let mut app = App::new();
        app.add_plugins(bevy_promptline::PromptlineRuntimePlugin)
            .init_resource::<Time>();
        app.insert_resource(bevy_promptline::command::pending::PendingLimits {
            invocations: bevy_promptline::Limit::try_from(1).unwrap(),
        });
        install(&mut app).unwrap();
        for _ in 0..2 {
            app.world_mut()
                .write_message(bevy_promptline::SubmitCommand::new("wait"));
        }
        app.update();
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&PendingTask>()
                .iter(app.world())
                .count(),
            1
        );
        assert_eq!(app.world().resource::<Jobs>().0.len(), 1);
        assert!(
            app.world()
                .resource::<Output>()
                .texts()
                .any(|text| text == "Task completion destination is unavailable")
        );
    }
    #[test]
    fn invalid_wait_and_cancel_arguments_cannot_create_or_cancel_work() {
        let mut app = App::new();
        app.add_plugins(bevy_promptline::PromptlineRuntimePlugin)
            .init_resource::<Time>();
        install(&mut app).unwrap();
        for (input, diagnostic) in [
            ("wait extra", "Usage: wait"),
            ("cancel extra", "Usage: cancel"),
        ] {
            app.world_mut()
                .write_message(bevy_promptline::SubmitCommand::new(input));
            app.update();
            let events: Vec<_> = app
                .world_mut()
                .resource_mut::<Messages<CommandNotification>>()
                .drain()
                .collect();
            assert_eq!(events.len(), 2);
            assert_eq!(
                events.last().unwrap().status,
                InvocationStatus::Rejected(diagnostic.into())
            );
            assert!(app.world().resource::<Jobs>().0.is_empty());
            assert_eq!(
                app.world_mut()
                    .query::<&PendingTask>()
                    .iter(app.world())
                    .count(),
                0
            );
        }
    }
}
