//! Highlighting shares draft token ranges without parsing or changing input.
use bevy::prelude::*;
use bevy_promptline::input::{
    analyze,
    highlight::{EditDiagnostic, HighlightRole, HighlightSpan},
};
use bevy_promptline::{
    CommandMetadata, CommandOutcome, ConsoleCommand, PromptlineAppExt, PromptlineRuntimePlugin,
};

/// This parser must remain untouched during draft analysis.
#[derive(Debug)]
struct Echo;
impl ConsoleCommand for Echo {
    fn metadata() -> Result<CommandMetadata, bevy_promptline::InvalidCommandName> {
        CommandMetadata::new("echo", "Echo text")
    }
    fn parse(_: &[String]) -> Result<Self, String> {
        panic!("typing must not parse")
    }
}

/// Shared grammar keeps incomplete syntax local and leaves delimiter values literal.
#[test]
fn grammar_edges_and_unknown_commands_keep_exact_roles_and_diagnostics() {
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<Echo, _>(|_: In<Echo>| CommandOutcome::Success)
        .unwrap();
    let cases = [
        ("", vec![], None),
        (" \t", vec![], None),
        ("ec", vec![HighlightRole::Command], None),
        (
            "other",
            vec![HighlightRole::Error],
            Some(EditDiagnostic::UnknownCommand),
        ),
        (
            "other 'open",
            vec![HighlightRole::Error, HighlightRole::Incomplete],
            Some(EditDiagnostic::UnclosedQuote),
        ),
        (
            "echo -- --flag -2",
            vec![
                HighlightRole::Command,
                HighlightRole::Option,
                HighlightRole::Value,
                HighlightRole::Value,
            ],
            None,
        ),
        (
            "echo a\\'b",
            vec![HighlightRole::Command, HighlightRole::Value],
            None,
        ),
        (
            "echo \"a\\q\"",
            vec![HighlightRole::Command, HighlightRole::Quoted],
            None,
        ),
        (
            "echo \"tail\\",
            vec![HighlightRole::Command, HighlightRole::Incomplete],
            Some(EditDiagnostic::TrailingEscape),
        ),
        (
            "''",
            vec![HighlightRole::Error],
            Some(EditDiagnostic::UnknownCommand),
        ),
    ];
    for (input, roles, diagnostic) in cases {
        let analysis = analyze(app.world(), input, input.len()).unwrap();
        assert_eq!(
            analysis
                .highlights()
                .iter()
                .map(HighlightSpan::role)
                .collect::<Vec<_>>(),
            roles,
            "{input}"
        );
        assert_eq!(analysis.diagnostic(), diagnostic, "{input}");
    }
}

/// Denied command metadata produces the same style and message as missing metadata.
#[test]
fn permission_denial_does_not_reveal_command_membership() {
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<Echo, _>(|_: In<Echo>| CommandOutcome::Success)
        .unwrap();
    app.world_mut()
        .resource_mut::<bevy_promptline::CommandRegistry>()
        .set_policy(|_, _, _| false);
    for input in ["echo", "missing"] {
        let analysis = analyze(app.world(), input, input.len()).unwrap();
        assert_eq!(
            analysis.highlights().first().unwrap().role(),
            HighlightRole::Error
        );
        assert_eq!(analysis.diagnostic(), Some(EditDiagnostic::UnknownCommand));
        assert!(analysis.candidates().is_empty());
    }
    assert_eq!(
        EditDiagnostic::UnknownCommand.to_string(),
        "Unknown command"
    );
    assert_eq!(
        EditDiagnostic::UnclosedQuote.to_string(),
        "Finish the quoted argument"
    );
    assert_eq!(
        EditDiagnostic::TrailingEscape.to_string(),
        "Enter the character after the backslash"
    );
}

#[test]
fn draft_spans_preserve_source_and_distinguish_options_values_and_quotes() {
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<Echo, _>(|_: In<Echo>| CommandOutcome::Success)
        .unwrap();
    let input = "echo --flag -2 '日本語 text'";
    let analysis = analyze(app.world(), input, input.len()).unwrap();
    let roles: Vec<_> = analysis
        .highlights()
        .iter()
        .map(HighlightSpan::role)
        .collect();
    assert_eq!(
        roles,
        [
            HighlightRole::Command,
            HighlightRole::Option,
            HighlightRole::Value,
            HighlightRole::Quoted
        ]
    );
    let source: Vec<_> = analysis
        .highlights()
        .iter()
        .map(|span| &input[span.source_range().clone()])
        .collect();
    assert_eq!(source, ["echo", "--flag", "-2", "'日本語 text'"]);
    assert_eq!(analysis.diagnostic(), None);
}

#[test]
fn incomplete_input_is_a_local_diagnostic_without_submission() {
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_command::<Echo, _>(|_: In<Echo>| CommandOutcome::Success)
        .unwrap();
    for (input, expected) in [
        ("echo 'unfinished", EditDiagnostic::UnclosedQuote),
        ("echo trailing\\", EditDiagnostic::TrailingEscape),
    ] {
        let analysis = analyze(app.world(), input, input.len()).unwrap();
        assert_eq!(analysis.diagnostic(), Some(expected));
        assert_eq!(
            analysis.highlights().last().unwrap().role(),
            HighlightRole::Incomplete
        );
    }
}

#[cfg(feature = "clap")]
#[test]
fn clap_metadata_distinguishes_subcommands_from_option_values_without_validation() {
    use bevy_promptline::clap::ClapAppExt;
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    let command = clap::Command::new("scene").subcommand(
        clap::Command::new("rotate").arg(clap::Arg::new("speed").long("speed").value_parser(
            |_: &str| -> Result<String, String> { panic!("typing must not validate") },
        )),
    );
    app.register_clap_builder(command, |_: In<clap::ArgMatches>| CommandOutcome::Success)
        .unwrap();
    let input = "scene rotate --speed rotate";
    let analysis = analyze(app.world(), input, input.len()).unwrap();
    assert_eq!(
        analysis
            .highlights()
            .iter()
            .map(HighlightSpan::role)
            .collect::<Vec<_>>(),
        [
            HighlightRole::Command,
            HighlightRole::Subcommand,
            HighlightRole::Option,
            HighlightRole::Value
        ]
    );
}

#[cfg(feature = "clap")]
#[test]
fn clap_arity_aliases_assignments_and_delimiters_preserve_token_roles() {
    use bevy_promptline::clap::ClapAppExt;
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    let command = clap::Command::new("scene")
        .arg(clap::Arg::new("pair").long("pair").num_args(2))
        .arg(
            clap::Arg::new("eq")
                .long("eq")
                .num_args(0..=1)
                .require_equals(true),
        )
        .subcommand(
            clap::Command::new("rotate")
                .alias("rot")
                .arg(clap::Arg::new("speed").long("speed")),
        );
    app.register_clap_builder(command, |_: In<clap::ArgMatches>| CommandOutcome::Success)
        .unwrap();
    for (input, roles) in [
        (
            "scene --pair rotate rotate rot",
            vec![
                HighlightRole::Command,
                HighlightRole::Option,
                HighlightRole::Value,
                HighlightRole::Value,
                HighlightRole::Subcommand,
            ],
        ),
        (
            "scene --pair=one two rotate",
            vec![
                HighlightRole::Command,
                HighlightRole::Option,
                HighlightRole::Value,
                HighlightRole::Subcommand,
            ],
        ),
        (
            "scene rotate --speed=2 --unknown -3",
            vec![
                HighlightRole::Command,
                HighlightRole::Subcommand,
                HighlightRole::Option,
                HighlightRole::Option,
                HighlightRole::Value,
            ],
        ),
        (
            "scene -- rotate",
            vec![
                HighlightRole::Command,
                HighlightRole::Option,
                HighlightRole::Value,
            ],
        ),
        (
            "scene --eq rot rotate",
            vec![
                HighlightRole::Command,
                HighlightRole::Option,
                HighlightRole::Subcommand,
                HighlightRole::Value,
            ],
        ),
    ] {
        for caret in [0, input.len()] {
            let analysis = analyze(app.world(), input, caret).unwrap();
            assert_eq!(
                analysis
                    .highlights()
                    .iter()
                    .map(HighlightSpan::role)
                    .collect::<Vec<_>>(),
                roles,
                "{input}"
            );
        }
    }
}
