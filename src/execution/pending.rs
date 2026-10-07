//! Main-thread polling retains bounded handles and emits one terminal message.
use crate::command::pending::{CancelCommand, CommandExecution, PendingCommand, PendingLimits};
use crate::command::{InvocationId, InvocationStatus, Sensitivity, SubmissionSource};
use crate::sets::PromptlineSystems;
use bevy::prelude::*;

/// Runtime collection contains only invocations without a terminal observation.
#[derive(Debug, Default, Resource)]
struct PendingWork(Vec<PendingInvocation>);
/// Accepted identity and origin stay attached to their sole receiving endpoint.
#[derive(Debug)]
struct PendingInvocation {
    /// Identity assigned before validation and handler dispatch.
    invocation: InvocationId,
    /// Original source is preserved in the eventual terminal notification.
    source: SubmissionSource,
    /// Accepted metadata controls terminal privacy even after registration changes.
    sensitivity: Sensitivity,
    /// Receiving endpoint owns no World reference or worker thread.
    command: PendingCommand,
}

/// Installs polling after dispatch and before lifecycle output consumers.
pub(crate) fn install(app: &mut App) {
    app.init_resource::<PendingLimits>()
        .init_resource::<PendingWork>()
        .add_message::<CancelCommand>()
        .add_systems(
            PostUpdate,
            poll.after(PromptlineSystems::Dispatch)
                .before(PromptlineSystems::Output),
        );
}

/// Completes immediate results or retains a handle within the positive limit.
pub(crate) fn finish_or_retain(
    world: &mut World,
    invocation: InvocationId,
    source: SubmissionSource,
    sensitivity: Sensitivity,
    execution: CommandExecution,
) {
    // Immediate handlers retain no asynchronous state or completion slot.
    match execution {
        CommandExecution::Finished(outcome) => {
            finish(world, invocation, source, sensitivity, outcome);
        }
        CommandExecution::Pending(command) => {
            retain(world, invocation, source, sensitivity, command);
        }
    }
}

/// Excess work receives a cooperative request before its channel is dropped.
fn retain(
    world: &mut World,
    invocation: InvocationId,
    source: SubmissionSource,
    sensitivity: Sensitivity,
    command: PendingCommand,
) {
    // Missing runtime state cannot admit new pending ownership.
    let limits = world.get_resource::<PendingLimits>().copied();
    let Some(mut work) = world.get_resource_mut::<PendingWork>() else {
        discard(
            world,
            invocation,
            source,
            sensitivity,
            command,
            "pending command runtime unavailable",
        );
        return;
    };
    let failure = match limits {
        None => Some("pending command runtime unavailable"),
        Some(limits) if work.0.len() >= limits.invocations.get() => {
            Some("pending command limit reached")
        }
        Some(_) => None,
    };
    // The handler already ran; capacity failure cannot undo its earlier side effects.
    if let Some(failure) = failure {
        discard(world, invocation, source, sensitivity, command, failure);
    } else {
        work.0.push(PendingInvocation {
            invocation,
            source,
            sensitivity,
            command,
        });
    }
}

/// Applies requests before polling; workers remain responsible for
/// acknowledgement.
fn poll(world: &mut World) {
    // Moving this inbox keeps any later requests for the next frame.
    let mut requests = world
        .get_resource_mut::<Messages<CancelCommand>>()
        .map_or_else(Messages::default, |mut messages| {
            std::mem::take(messages.as_mut())
        });
    let Some(mut work) = world.remove_resource::<PendingWork>() else {
        return;
    };
    requests
        .drain()
        .filter_map(|request| {
            work.0
                .iter()
                .find(|entry| entry.invocation == request.invocation)
        })
        .for_each(|invocation| invocation.command.cancellation.request());
    // Stable retention drops finished endpoints before restoring the runtime collection.
    work.0.retain_mut(|entry| match entry.command.poll() {
        Some(outcome) => {
            finish(
                world,
                entry.invocation,
                entry.source,
                entry.sensitivity,
                outcome,
            );
            false
        }
        None => true,
    });
    world.insert_resource(work);
}

/// Requests cancellation before dropping work that cannot enter runtime storage.
fn discard(
    world: &mut World,
    invocation: InvocationId,
    source: SubmissionSource,
    sensitivity: Sensitivity,
    command: PendingCommand,
    diagnostic: &str,
) {
    command.cancellation.request();
    finish(
        world,
        invocation,
        source,
        sensitivity,
        crate::CommandOutcome::Failure(diagnostic.into()),
    );
}

/// Sends a terminal transition through the same notification owner as
/// synchronous work.
fn finish(
    world: &mut World,
    invocation: InvocationId,
    source: SubmissionSource,
    sensitivity: Sensitivity,
    outcome: crate::CommandOutcome,
) {
    // A redacted command cannot expose worker text in a shared lifecycle message.
    let outcome = match outcome {
        crate::CommandOutcome::Failure(diagnostic) => match sensitivity {
            Sensitivity::Public => crate::CommandOutcome::Failure(diagnostic),
            Sensitivity::Redacted => {
                crate::CommandOutcome::Failure("command failed for a sensitive command".into())
            }
        },
        crate::CommandOutcome::Success => crate::CommandOutcome::Success,
        crate::CommandOutcome::Cancelled => crate::CommandOutcome::Cancelled,
    };
    // Publish the selected privacy profile through the shared notification owner.
    crate::command::notify(
        world,
        invocation,
        source,
        InvocationStatus::Finished(outcome),
    );
}

/// Private retention invariant is independent from visible lifecycle
/// observations.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finished_execution_does_not_retain_a_pending_slot() {
        let mut app = App::new();
        app.add_plugins(crate::PromptlineRuntimePlugin);
        finish_or_retain(
            app.world_mut(),
            InvocationId(1),
            SubmissionSource::Programmatic,
            Sensitivity::Public,
            CommandExecution::Finished(crate::CommandOutcome::Success),
        );
        assert!(app.world().resource::<PendingWork>().0.is_empty());
    }
    #[test]
    fn removed_private_runtime_state_does_not_panic_or_retain_work() {
        let mut world = World::new();
        poll(&mut world);
        let mut app = App::new();
        app.add_plugins(crate::PromptlineRuntimePlugin);
        app.world_mut().remove_resource::<PendingWork>();
        let (pending, completion) = PendingCommand::channel();
        finish_or_retain(
            app.world_mut(),
            InvocationId(1),
            SubmissionSource::Programmatic,
            Sensitivity::Public,
            CommandExecution::Pending(pending),
        );
        assert!(completion.as_ref().is_requested());
        assert!(completion.finish(crate::CommandOutcome::Cancelled).is_err());
    }
}
