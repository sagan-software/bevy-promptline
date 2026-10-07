//! Shared command discovery and completion through the public analysis boundary.
use bevy::prelude::*;
use bevy_promptline::{
    CommandMetadata, CommandOutcome, ConsoleCommand, PromptlineAppExt, PromptlineRuntimePlugin,
};

/// A parser that must remain untouched while typing.
#[derive(Debug)]
struct Echo;
impl ConsoleCommand for Echo {
    fn metadata() -> Result<CommandMetadata, bevy_promptline::InvalidCommandName> {
        CommandMetadata::new("echo", "Print text")
    }
    fn parse(_: &[String]) -> Result<Self, String> {
        panic!("analysis invoked parsing")
    }
}

#[test]
fn command_completion_previews_without_mutating_input_or_running_a_parser() {
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<Echo, _>(|_: In<Echo>| CommandOutcome::Success)
        .unwrap();
    let input = "ec";
    let analysis = bevy_promptline::input::analyze(app.world(), input, 2).unwrap();
    assert_eq!(
        analysis
            .candidates()
            .iter()
            .map(|candidate| candidate.value.as_str())
            .collect::<Vec<_>>(),
        ["echo"]
    );
    assert_eq!(
        analysis.ghost(bevy_promptline::input::EditingMode::Caret),
        Some("ho")
    );
    assert_eq!(
        analysis.ghost(bevy_promptline::input::EditingMode::Selection),
        None
    );
    assert_eq!(
        analysis.ghost(bevy_promptline::input::EditingMode::Composition),
        None
    );
    assert_eq!(input, "ec");
    let completed = analysis.accept(input, 0).unwrap();
    assert_eq!(completed.text, "echo");
    assert_eq!(completed.caret, 4);
}

#[test]
fn completion_validates_caret_snapshot_and_candidate_before_replacement() {
    use bevy_promptline::input::{AnalysisError, EditingMode, analyze};
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<Echo, _>(|_: In<Echo>| CommandOutcome::Success)
        .unwrap();
    assert!(matches!(
        analyze(app.world(), "😀", 1),
        Err(AnalysisError::InvalidCaret)
    ));
    assert!(matches!(
        analyze(app.world(), "ec", 3),
        Err(AnalysisError::InvalidCaret)
    ));
    let analysis = analyze(app.world(), "ec", 2).unwrap();
    assert!(matches!(
        analysis.accept("changed", 0),
        Err(AnalysisError::StaleInput)
    ));
    assert!(matches!(
        analysis.accept("ec", 9),
        Err(AnalysisError::MissingCandidate)
    ));
    let middle = analyze(app.world(), "ecxx tail", 2).unwrap();
    assert_eq!(middle.ghost(EditingMode::Caret), None);
    assert_eq!(middle.accept("ecxx tail", 0).unwrap().text, "echo tail");
    let quoted = analyze(app.world(), "'ec", 3).unwrap();
    assert_eq!(quoted.ghost(EditingMode::Caret), None);
    assert_eq!(quoted.accept("'ec", 0).unwrap().text, "echo");
}

#[test]
fn denied_commands_are_absent_from_completion_and_no_match_has_no_ghost() {
    use bevy_promptline::{
        CommandRegistry,
        input::{EditingMode, analyze},
    };
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<Echo, _>(|_: In<Echo>| CommandOutcome::Success)
        .unwrap();
    assert_eq!(
        analyze(app.world(), "none", 4)
            .unwrap()
            .ghost(EditingMode::Caret),
        None
    );
    app.world_mut()
        .resource_mut::<CommandRegistry>()
        .set_policy(|_, _, _| false);
    assert!(
        analyze(app.world(), "ec", 2)
            .unwrap()
            .candidates()
            .is_empty()
    );
}

/// A second command sharing a prefix with scene.
#[derive(Debug)]
struct Scope;
impl ConsoleCommand for Scope {
    fn metadata() -> Result<CommandMetadata, bevy_promptline::InvalidCommandName> {
        CommandMetadata::new("scope", "Scope fixture")
    }
    fn parse(_: &[String]) -> Result<Self, String> {
        Ok(Self)
    }
}
/// Scene command for common-prefix and cycle behavior.
#[derive(Debug)]
struct Scene;
impl ConsoleCommand for Scene {
    fn metadata() -> Result<CommandMetadata, bevy_promptline::InvalidCommandName> {
        CommandMetadata::new("scene", "Scene fixture")
    }
    fn parse(_: &[String]) -> Result<Self, String> {
        Ok(Self)
    }
}

#[test]
fn tab_expands_the_common_prefix_then_cycles_forward_and_backward() {
    use bevy_promptline::input::{CompletionDirection, analyze};
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<Scope, _>(|_: In<Scope>| CommandOutcome::Success)
        .unwrap();
    app.register_command::<Scene, _>(|_: In<Scene>| CommandOutcome::Success)
        .unwrap();
    let preview = analyze(app.world(), "s tail", 1).unwrap();
    assert_eq!(
        preview.ghost(bevy_promptline::input::EditingMode::Caret),
        Some("c")
    );
    let mut session = preview.into_cycle();
    let first = session
        .advance("s tail", CompletionDirection::Forward)
        .unwrap();
    assert_eq!(first.text, "sc tail");
    let second = session
        .advance(&first.text, CompletionDirection::Forward)
        .unwrap();
    assert_eq!(second.text, "scene tail");
    let third = session
        .advance(&second.text, CompletionDirection::Forward)
        .unwrap();
    assert_eq!(third.text, "scope tail");
    let fourth = session
        .advance(&third.text, CompletionDirection::Forward)
        .unwrap();
    assert_eq!(fourth.text, "scene tail");
    let reverse = session
        .advance(&fourth.text, CompletionDirection::Backward)
        .unwrap();
    assert_eq!(reverse.text, "scope tail");
}
