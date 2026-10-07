//! Existing Clap parsers delivered through the runtime without process exit.
#![cfg(feature = "clap")]
use bevy::prelude::*;
use bevy_promptline::clap::ClapAppExt;
use bevy_promptline::{CommandOutcome, Output, PromptlineRuntimePlugin, SubmitCommand};
use clap::{Parser, Subcommand};

/// Existing derive parser with nested commands and negative values.
#[derive(Debug, Parser)]
#[command(name = "scene", version = "1.0")]
struct Scene {
    #[command(subcommand)]
    action: Action,
}
/// Closed scene actions for conformance fixtures.
#[derive(Debug, Subcommand)]
enum Action {
    Rotate {
        #[arg(long, allow_negative_numbers = true)]
        speed: f32,
    },
}

#[test]
fn derive_dispatches_typed_nested_input_and_never_dispatches_help_or_errors() {
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_clap::<Scene, _>(|In(command): In<Scene>, mut output: ResMut<'_, Output>| {
        let Action::Rotate { speed } = command.action;
        output.write(format!("speed={speed}"));
        CommandOutcome::Success
    })
    .unwrap();
    for text in [
        "scene rotate --speed -2",
        "scene --help",
        "scene --version",
        "scene rotate",
        "scene bad",
    ] {
        app.world_mut().write_message(SubmitCommand::new(text));
    }
    app.update();
    let output: Vec<_> = app.world().resource::<Output>().texts().collect();
    assert_eq!(
        output
            .iter()
            .filter(|line| line.starts_with("speed="))
            .count(),
        1
    );
    assert_eq!(output.first(), Some(&"speed=-2"));
    assert!(output.iter().any(|line| line.contains("Usage:")));
    assert!(output.iter().any(|line| line.contains("1.0")));
}

#[test]
fn builder_preserves_empty_values_defaults_conflicts_and_end_of_options() {
    let command = clap::Command::new("values")
        .arg(
            clap::Arg::new("input")
                .num_args(1..)
                .allow_hyphen_values(true),
        )
        .arg(clap::Arg::new("mode").long("mode").default_value("normal"));
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_clap_builder(
        command,
        |In(matches): In<clap::ArgMatches>, mut output: ResMut<'_, Output>| {
            let values = matches
                .get_many::<String>("input")
                .unwrap()
                .cloned()
                .collect::<Vec<_>>();
            output.write(format!("{values:?}"));
            CommandOutcome::Success
        },
    )
    .unwrap();
    app.world_mut()
        .write_message(SubmitCommand::new("values -- '' -2"));
    app.update();
    assert_eq!(
        app.world().resource::<Output>().texts().next(),
        Some("[\"\", \"-2\"]")
    );
}

#[test]
fn no_binary_name_multicall_external_subcommands_and_aliases_keep_clap_semantics() {
    let cases = [
        (
            clap::Command::new("nobin")
                .no_binary_name(true)
                .arg(clap::Arg::new("value").required(true)),
            "nobin text",
            "value",
        ),
        (
            clap::Command::new("busybox")
                .multicall(true)
                .subcommand(clap::Command::new("busybox").subcommand(clap::Command::new("true")))
                .subcommand(clap::Command::new("true")),
            "busybox true",
            "subcommand",
        ),
        (
            clap::Command::new("external").allow_external_subcommands(true),
            "external other one two",
            "subcommand",
        ),
        (
            clap::Command::new("aliased")
                .alias("alias")
                .arg(clap::Arg::new("value")),
            "alias text",
            "value",
        ),
    ];
    for (command, text, kind) in cases {
        let mut app = App::new();
        app.add_plugins(PromptlineRuntimePlugin);
        app.register_clap_builder(
            command,
            move |In(matches): In<clap::ArgMatches>, mut output: ResMut<'_, Output>| {
                if kind == "value" {
                    output.write(matches.get_one::<String>("value").unwrap().clone());
                } else {
                    output.write(matches.subcommand_name().unwrap());
                }
                CommandOutcome::Success
            },
        )
        .unwrap();
        app.world_mut().write_message(SubmitCommand::new(text));
        app.update();
        assert_eq!(app.world().resource::<Output>().texts().count(), 1);
        assert!(
            app.world()
                .resource::<Output>()
                .texts()
                .next()
                .is_some_and(|value| ["text", "busybox", "other"].contains(&value)),
            "case {text}: {:?}",
            app.world().resource::<Output>().texts().collect::<Vec<_>>()
        );
    }
}

#[test]
fn defaults_conflicts_unknown_and_missing_values_are_enforced_before_dispatch() {
    let command = clap::Command::new("checked")
        .arg(
            clap::Arg::new("mode")
                .long("mode")
                .default_value("normal")
                .value_parser(["normal", "fast"]),
        )
        .arg(
            clap::Arg::new("left")
                .long("left")
                .action(clap::ArgAction::SetTrue)
                .conflicts_with("right"),
        )
        .arg(
            clap::Arg::new("right")
                .long("right")
                .action(clap::ArgAction::SetTrue),
        );
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_clap_builder(
        command,
        |In(matches): In<clap::ArgMatches>, mut output: ResMut<'_, Output>| {
            output.write(format!(
                "handler:{}",
                matches.get_one::<String>("mode").unwrap()
            ));
            CommandOutcome::Success
        },
    )
    .unwrap();
    for input in [
        "checked",
        "checked --mode fast",
        "checked --mode",
        "checked --mode bad",
        "checked --unknown",
        "checked --left --right",
    ] {
        app.world_mut().write_message(SubmitCommand::new(input));
    }
    app.update();
    let entries: Vec<_> = app.world().resource::<Output>().texts().collect();
    assert_eq!(
        entries
            .iter()
            .filter(|line| line.starts_with("handler:"))
            .copied()
            .collect::<Vec<_>>(),
        ["handler:normal", "handler:fast"]
    );
    assert_eq!(entries.len(), 6);
}

#[test]
fn help_and_version_have_terminal_information_without_started_handler_notifications() {
    use bevy_promptline::{CommandNotification, command::InvocationStatus};
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_clap_builder(
        clap::Command::new("info").version("2.0"),
        |_: In<clap::ArgMatches>| -> CommandOutcome {
            panic!("informational parsing executed handler")
        },
    )
    .unwrap();
    app.world_mut()
        .write_message(SubmitCommand::new("info --help"));
    app.world_mut()
        .write_message(SubmitCommand::new("info --version"));
    app.update();
    let messages: Vec<_> = app
        .world_mut()
        .resource_mut::<Messages<CommandNotification>>()
        .drain()
        .collect();
    assert_eq!(
        messages
            .iter()
            .filter(|message| matches!(
                message.status,
                InvocationStatus::Finished(CommandOutcome::Success)
            ))
            .count(),
        2
    );
    assert!(!messages.iter().any(|message| matches!(
        message.status,
        InvocationStatus::Started | InvocationStatus::Rejected(_)
    )));
}

#[test]
fn nested_metadata_completes_subcommands_flags_and_values_without_validation() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let validator_calls = calls.clone();
    let command = clap::Command::new("scene").subcommand(
        clap::Command::new("rotate")
            .arg(
                clap::Arg::new("axis")
                    .long("axis")
                    .help("Rotation axis")
                    .value_parser(["x", "y", "z"]),
            )
            .arg(clap::Arg::new("speed").long("speed").value_parser(
                clap::builder::ValueParser::new(move |value: &str| -> Result<String, String> {
                    validator_calls.fetch_add(1, Ordering::Relaxed);
                    Ok(value.to_owned())
                }),
            )),
    );
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin);
    app.register_clap_builder(command, |_: In<clap::ArgMatches>| CommandOutcome::Success)
        .unwrap();
    for (input, expected) in [
        ("scene ro", vec!["rotate"]),
        ("scene rotate --a", vec!["--axis"]),
        ("scene rotate --axis ", vec!["x", "y", "z"]),
    ] {
        let analysis = bevy_promptline::input::analyze(app.world(), input, input.len()).unwrap();
        assert_eq!(
            analysis
                .candidates()
                .iter()
                .map(|candidate| candidate.value.as_str())
                .collect::<Vec<_>>(),
            expected
        );
    }
    let help = bevy_promptline::input::analyze(app.world(), "scene rotate --axis ", 20).unwrap();
    assert!(help.help().contains("Rotation axis"));
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}

#[test]
fn clap_command_labels_preserve_unicode_and_quoted_spaces() {
    for name in ["42", "démo", "my tool"] {
        let command = clap::Command::new(name);
        let mut app = App::new();
        app.add_plugins(PromptlineRuntimePlugin);
        app.register_clap_builder(
            command,
            |_: In<clap::ArgMatches>, mut output: ResMut<'_, Output>| {
                output.write("label accepted");
                CommandOutcome::Success
            },
        )
        .unwrap();
        let input = bevy_promptline::quote_argument(name);
        app.world_mut().write_message(SubmitCommand::new(input));
        app.update();
        assert_eq!(
            app.world().resource::<Output>().texts().next(),
            Some("label accepted")
        );
    }
}
