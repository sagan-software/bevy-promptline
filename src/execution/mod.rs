//! Ordered invocation delivery and lifecycle notifications.
mod pending;
use bevy::prelude::*;

use crate::command::{
    CommandNotification, InvocationId, InvocationStatus, SubmissionSource, SubmitCommand, notify,
};

/// Installs command registration, bounded output, input limits, and ordered
/// dispatch. It creates no camera or window, so headless applications can use
/// the command runtime.
#[derive(Debug, Clone, Copy, Default)]
pub struct PromptlineRuntimePlugin;
impl Plugin for PromptlineRuntimePlugin {
    fn build(&self, app: &mut App) {
        pending::install(app);
        app.init_resource::<crate::CommandRegistry>()
            .init_resource::<crate::Output>()
            .init_resource::<RuntimeLimits>()
            .init_resource::<crate::command::CommandHistory>()
            .init_resource::<InvocationSequence>()
            .add_message::<SubmitCommand>()
            .add_message::<CommandNotification>()
            .add_systems(
                PostUpdate,
                dispatch.in_set(crate::sets::PromptlineSystems::Dispatch),
            );
    }
}

/// Configures positive command input and acceptance limits. Input bytes are
/// checked before tokenization, and submissions exceeding the per-frame
/// acceptance count receive visible rejection.
#[derive(Debug, Clone, Copy, Resource)]
pub struct RuntimeLimits {
    /// The maximum UTF-8 bytes accepted in one submitted command. The runtime
    /// checks this positive limit before decoding tokens or invoking any
    /// command parser.
    pub input_bytes: crate::Limit,
    /// The maximum number of submissions considered for execution in one frame.
    /// Later submissions receive rejection notifications and diagnostics
    /// instead of silently executing or disappearing.
    pub queue_entries: crate::Limit,
}
impl Default for RuntimeLimits {
    fn default() -> Self {
        Self {
            input_bytes: crate::Limit::try_from(4096).expect("positive constant"),
            queue_entries: crate::Limit::try_from(64).expect("positive constant"),
        }
    }
}

/// Last assigned invocation number, owned by the runtime.
#[derive(Debug, Default, Resource)]
struct InvocationSequence(u64);

/// Typed runtime rejection categories preserve tokenizer sources separately
/// from parser text.
#[derive(Debug, thiserror::Error)]
enum PreparationFailure {
    /// Required runtime state was removed by the application.
    #[error("command runtime is unavailable")]
    Unavailable,
    /// The submitted text exceeded its configured byte limit.
    #[error("command exceeds the input byte limit")]
    InputLimit,
    /// No command identity was submitted.
    #[error("enter a command")]
    Empty,
    /// No registered primary name or alias matched.
    #[error("unknown command")]
    Unknown,
    /// Actor policy denied execution before argument parsing.
    #[error("command is not permitted")]
    Unauthorized,
    /// The single-line command grammar rejected input.
    #[error(transparent)]
    Tokenization(#[from] crate::TokenizeError),
    /// The typed command parser returned its own diagnostic.
    #[error("{0}")]
    Arguments(String),
}

/// Drains submissions in arrival order without building an intermediate owned
/// collection.
fn dispatch(world: &mut World) {
    let Some(limits) = world.get_resource::<RuntimeLimits>().copied() else {
        return;
    };
    let Some(mut inbox) = world.get_resource_mut::<Messages<SubmitCommand>>() else {
        return;
    };
    // Move existing submissions out so handler submissions enter the next
    // frame's inbox.
    let mut submissions = std::mem::take(inbox.as_mut());
    for (index, submission) in submissions.drain().enumerate() {
        let Some(id) = next_invocation(world) else {
            diagnostic(world, "command invocation identities are unavailable");
            break;
        };
        notify(world, id, submission.source, InvocationStatus::Submitted);
        // Overflow remains observable and cannot reach a parser or handler.
        if index >= limits.queue_entries.get() {
            reject(
                world,
                id,
                submission.source,
                "command queue is full".to_owned(),
            );
        } else {
            execute(world, id, submission);
        }
    }
}

/// Assigns a new identity without wrapping or reusing previous invocation
/// numbers.
fn next_invocation(world: &mut World) -> Option<InvocationId> {
    let mut sequence = world.get_resource_mut::<InvocationSequence>()?;
    sequence.0 = sequence.0.checked_add(1)?;
    Some(InvocationId(sequence.0))
}

/// Emits acceptance and execution transitions around the sole handler
/// invocation.
fn execute(world: &mut World, id: InvocationId, submission: SubmitCommand) {
    match prepare(world, &submission) {
        Ok((prepared, sensitivity)) => {
            if let Some(mut history) = world.get_resource_mut::<crate::command::CommandHistory>() {
                history.record(&submission.input.0, sensitivity);
            }
            // Acceptance precedes every handler call, including a failed
            // registered system.
            notify(world, id, submission.source, InvocationStatus::Accepted);
            // Information completes without producing a started-handler observation.
            let outcome = match prepared {
                crate::registry::PreparedCommand::Run(run) => {
                    notify(world, id, submission.source, InvocationStatus::Started);
                    run(world)
                }
                crate::registry::PreparedCommand::Information(text) => {
                    if let Some(mut output) = world.get_resource_mut::<crate::Output>() {
                        output.write(text);
                    }
                    crate::command::pending::CommandExecution::Finished(
                        crate::CommandOutcome::Success,
                    )
                }
            };
            pending::finish_or_retain(world, id, submission.source, sensitivity, outcome);
        }
        Err(error) => reject(world, id, submission.source, error.to_string()),
    }
}

/// Validates input length before decoding any command arguments.
fn validate_input(
    world: &World,
    submission: &SubmitCommand,
) -> Result<Vec<crate::Token>, PreparationFailure> {
    // A removed runtime resource cannot authorize parsing.
    let limits = world
        .get_resource::<RuntimeLimits>()
        .ok_or(PreparationFailure::Unavailable)?;
    let submitted_bytes = submission.input.0.len();
    let maximum_bytes = limits.input_bytes.get();
    let is_overlong = submitted_bytes > maximum_bytes;
    // Limits precede grammar validation and arbitrary parser callbacks.
    if is_overlong {
        return Err(PreparationFailure::InputLimit);
    }
    Ok(crate::tokenize(&submission.input.0)?)
}

/// Resolves and authorizes metadata before the parser can observe arguments.
fn authorize(
    world: &World,
    name: &str,
    source: SubmissionSource,
) -> Result<std::sync::Arc<crate::registry::RegisteredCommand>, PreparationFailure> {
    let registry = world
        .get_resource::<crate::CommandRegistry>()
        .ok_or(PreparationFailure::Unavailable)?;
    // Resolve aliases before evaluating the shared actor policy.
    let command = registry.resolve(name).ok_or(PreparationFailure::Unknown)?;
    // The same actor policy governs discovery and all submission sources.
    let is_permitted = registry.is_permitted(world, &command, source);
    if !is_permitted {
        return Err(PreparationFailure::Unauthorized);
    }
    Ok(command)
}

/// Creates a deferred handler only after input validation and actor
/// authorization.
fn prepare(
    world: &World,
    submission: &SubmitCommand,
) -> Result<
    (
        crate::registry::PreparedCommand,
        crate::command::Sensitivity,
    ),
    PreparationFailure,
> {
    let tokens = validate_input(world, submission)?;
    let first = tokens.first().ok_or(PreparationFailure::Empty)?;
    let command = authorize(world, &first.value, submission.source)?;
    // Move decoded values into the typed parser without cloning token strings.
    let arguments: Vec<_> = tokens
        .into_iter()
        .skip(1)
        .map(|token| token.value)
        .collect();
    // Sensitive parser diagnostics cannot enter scrollback or lifecycle observations.
    (command.prepare)(&arguments)
        .map(|prepared| (prepared, command.metadata.sensitivity))
        .map_err(|diagnostic| parser_rejection(command.metadata.sensitivity, diagnostic))
}

/// Writes a runtime diagnostic without invoking tracing or recursively
/// capturing output.
fn diagnostic(world: &mut World, text: &str) {
    if let Some(mut output) = world.get_resource_mut::<crate::Output>() {
        output.push(crate::Severity::Error, "promptline", text.to_owned());
    }
}

/// Records rejection before returning execution ownership to the runtime.
fn reject(world: &mut World, id: InvocationId, source: SubmissionSource, error: String) {
    diagnostic(world, &error);
    notify(world, id, source, InvocationStatus::Rejected(error));
}

/// Keeps sensitive parser input out of all shared diagnostics.
fn parser_rejection(
    sensitivity: crate::command::Sensitivity,
    diagnostic: String,
) -> PreparationFailure {
    let diagnostic = match sensitivity {
        crate::command::Sensitivity::Public => diagnostic,
        crate::command::Sensitivity::Redacted => {
            "arguments rejected for a sensitive command".to_owned()
        }
    };
    PreparationFailure::Arguments(diagnostic)
}

#[cfg(test)]
mod tests {
    use super::{
        InvocationSequence, InvocationStatus, PreparationFailure, RuntimeLimits, SubmissionSource,
        SubmitCommand, authorize, diagnostic, dispatch, next_invocation, notify, validate_input,
    };
    use bevy::prelude::*;

    #[test]
    fn missing_runtime_resources_do_not_execute_or_panic() {
        let mut world = World::new();
        let submission = SubmitCommand::new("echo private");
        assert!(matches!(
            validate_input(&world, &submission),
            Err(PreparationFailure::Unavailable)
        ));
        assert!(matches!(
            authorize(&world, "echo", SubmissionSource::Ui),
            Err(PreparationFailure::Unavailable)
        ));
        dispatch(&mut world);
        world.insert_resource(RuntimeLimits::default());
        dispatch(&mut world);
        diagnostic(&mut world, "unavailable");
        notify(
            &mut world,
            super::InvocationId(1),
            SubmissionSource::Ui,
            InvocationStatus::Submitted,
        );
    }

    #[test]
    fn missing_and_exhausted_identity_sequences_reject_without_reuse() {
        let mut world = World::new();
        world.insert_resource(RuntimeLimits::default());
        world.init_resource::<Messages<SubmitCommand>>();
        world.init_resource::<crate::Output>();
        world.write_message(SubmitCommand::new("echo private"));
        dispatch(&mut world);
        assert_eq!(
            world.resource::<crate::Output>().texts().next(),
            Some("command invocation identities are unavailable")
        );
        world.insert_resource(InvocationSequence(u64::MAX));
        assert_eq!(next_invocation(&mut world), None);
        assert_eq!(world.resource::<InvocationSequence>().0, u64::MAX);
        world.write_message(SubmitCommand::new("echo private"));
        dispatch(&mut world);
        assert_eq!(world.resource::<crate::Output>().texts().count(), 2);
    }
}
