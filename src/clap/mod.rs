//! Optional adapters for existing Clap derive and builder commands.
//!
//! Final parsing uses Clap's fallible API. Discovery uses reusable metadata and
//! never calls an argument validator. Console text remains UTF-8 and
//! single-line; host-dependent parsers retain their own native or browser
//! limitations.
use crate::{CommandOutcome, RegistrationError};
use bevy::prelude::*;
mod syntax;
use clap::{ArgMatches, Command, CommandFactory, FromArgMatches};

/// Registers existing Clap command shapes with the same typed execution owner
/// as core commands. Applications import this extension trait by name and keep
/// their original derive or builder definition.
pub trait ClapAppExt {
    /// Registers a derive command through `CommandFactory` and `FromArgMatches`
    /// without a Promptline derive. Help, version, and malformed input do not
    /// execute the registered game handler.
    ///
    /// # Errors
    ///
    /// Returns an identity or collision error before mutating the command
    /// registry.
    fn register_clap<C: CommandFactory + FromArgMatches + Send + 'static, M: 'static>(
        &mut self,
        system: impl IntoSystem<In<C>, CommandOutcome, M> + 'static,
    ) -> Result<&mut Self, RegistrationError>;

    /// Registers a builder command with an ordinary Bevy system receiving its
    /// `ArgMatches`. The adapter retains nested commands, defaults, validators,
    /// conflicts, and argument-value behavior.
    ///
    /// # Errors
    ///
    /// Returns an identity or collision error before mutating the command
    /// registry.
    fn register_clap_builder<M: 'static>(
        &mut self,
        command: Command,
        system: impl IntoSystem<In<ArgMatches>, CommandOutcome, M> + 'static,
    ) -> Result<&mut Self, RegistrationError>;
}

impl ClapAppExt for App {
    fn register_clap<C: CommandFactory + FromArgMatches + Send + 'static, M: 'static>(
        &mut self,
        system: impl IntoSystem<In<C>, CommandOutcome, M> + 'static,
    ) -> Result<&mut Self, RegistrationError> {
        let mut command = C::command();
        // Build metadata once before attaching its immutable analysis adapter.
        command.build();
        let metadata = command_metadata(&command)?;
        // Metadata analysis remains independent of typed parsing and execution.
        let description_command = command.clone();
        let describe: Box<crate::registry::Describe> = Box::new(move |tokens, position| {
            describe_command(&description_command, tokens, position)
        });
        crate::registry::register_parser(
            self,
            metadata,
            move |arguments| match parse_matches(&command, arguments)
                .map_err(|error| error.to_string())?
            {
                crate::registry::ParseDecision::Run(matches) => C::from_arg_matches(&matches)
                    .map(crate::registry::ParseDecision::Run)
                    .map_err(|error| error.to_string()),
                crate::registry::ParseDecision::Information(text) => {
                    Ok(crate::registry::ParseDecision::Information(text))
                }
            },
            Some(describe),
            system,
        )
    }
    fn register_clap_builder<M: 'static>(
        &mut self,
        mut command: Command,
        system: impl IntoSystem<In<ArgMatches>, CommandOutcome, M> + 'static,
    ) -> Result<&mut Self, RegistrationError> {
        command.build();
        let metadata = command_metadata(&command)?;
        // Metadata analysis remains independent of typed parsing and execution.
        let description_command = command.clone();
        let describe: Box<crate::registry::Describe> = Box::new(move |tokens, position| {
            describe_command(&description_command, tokens, position)
        });
        crate::registry::register_parser(
            self,
            metadata,
            move |arguments| parse_matches(&command, arguments).map_err(|error| error.to_string()),
            Some(describe),
            system,
        )
    }
}

/// Converts reusable command descriptions without running any value validator.
fn command_metadata(command: &Command) -> Result<crate::CommandMetadata, RegistrationError> {
    let mut metadata = crate::CommandMetadata {
        name: crate::CommandName::from_clap(command.get_name())?,
        aliases: command
            .get_all_aliases()
            .map(crate::CommandName::from_clap)
            .collect::<Result<_, _>>()?,
        summary: command
            .get_about()
            .map_or_else(String::new, ToString::to_string),
        usage: String::new(),
        arguments: Vec::new(),
        sensitivity: crate::command::Sensitivity::Public,
    };
    // Usage rendering can mutate Clap caches; preserve the retained descriptor.
    let mut command = command.clone();
    metadata.usage = command.render_usage().to_string();
    Ok(metadata)
}

/// Respects binary-token configuration and never invokes Clap's exiting API.
fn parse_matches(
    command: &Command,
    arguments: &[String],
) -> Result<crate::registry::ParseDecision<ArgMatches>, clap::Error> {
    let command = command.clone().color(clap::ColorChoice::Never);
    // Preserve binary-token configuration while retaining Clap's concrete error source.
    let result = if command.is_no_binary_name_set() {
        command.try_get_matches_from(arguments)
    } else {
        let name = command.get_name().to_owned();
        command.try_get_matches_from(
            std::iter::once(name.as_str()).chain(arguments.iter().map(String::as_str)),
        )
    };
    // Help and version are successful information, not parse rejection or game execution.
    match result {
        Ok(matches) => Ok(crate::registry::ParseDecision::Run(matches)),
        Err(error)
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            Ok(crate::registry::ParseDecision::Information(
                error.to_string(),
            ))
        }
        Err(error) => Err(error),
    }
}

/// Inspects nested command metadata without running argument parsing or
/// validators.
fn describe_command(
    command: &Command,
    tokens: &[crate::Token],
    position: usize,
) -> crate::metadata::ContextDescription {
    let mut context = command_context(command, tokens, position);
    // Syntax uses the same immutable command metadata without invoking value validators.
    context.roles = syntax::roles(command, tokens);
    context
}

/// Selects contextual help and candidates at the requested caret slot.
fn command_context(
    command: &Command,
    tokens: &[crate::Token],
    position: usize,
) -> crate::metadata::ContextDescription {
    let mut cursor = syntax::Syntax::new(command);
    // Completion and syntax traverse the same immutable metadata before the caret.
    for token in tokens.iter().skip(1).take(position.saturating_sub(1)) {
        let _role = cursor.classify(token);
    }
    if let Some(argument) = cursor.pending_argument() {
        return describe_argument(argument);
    }
    // Positional help remains available after named-token discovery ends.
    let mut context = positional_context(cursor.command, cursor.positional);
    if cursor.flags == FlagContext::Options {
        context.candidates.extend(named_candidates(cursor.command));
    }
    context
}

/// Supplies positional help independently of option and subcommand suggestions.
fn positional_context(command: &Command, position: usize) -> crate::metadata::ContextDescription {
    if let Some(argument) = command
        .get_arguments()
        .find(|argument| argument.get_index() == Some(position))
    {
        return describe_argument(argument);
    }
    crate::metadata::ContextDescription {
        help: command
            .get_about()
            .map_or_else(String::new, ToString::to_string),
        candidates: Vec::new(),
        roles: Vec::new(),
    }
}

/// Streams named metadata choices without running value validators.
fn named_candidates(command: &Command) -> impl Iterator<Item = crate::metadata::Candidate> + '_ {
    // Hidden names remain available to explicit parsing but not suggestion discovery.
    let subcommands = command
        .get_subcommands()
        .filter(|command| !command.is_hide_set())
        .map(|command| crate::metadata::Candidate {
            value: command.get_name().to_owned(),
            description: command
                .get_about()
                .map_or_else(String::new, ToString::to_string),
        });
    let arguments = command
        .get_arguments()
        .filter(|argument| !argument.is_hide_set())
        .flat_map(argument_candidates);
    subcommands.chain(arguments)
}

/// Produces each visible spelling directly into its retained candidate owner.
fn argument_candidates(
    argument: &clap::Arg,
) -> impl Iterator<Item = crate::metadata::Candidate> + '_ {
    // Separate optional iterators avoid an intermediate vector for two spellings.
    let long = argument.get_long().map(|long| crate::metadata::Candidate {
        value: format!("--{long}"),
        description: argument
            .get_help()
            .map_or_else(String::new, ToString::to_string),
    });
    let short = argument
        .get_short()
        .map(|short| crate::metadata::Candidate {
            value: format!("-{short}"),
            description: argument
                .get_help()
                .map_or_else(String::new, ToString::to_string),
        });
    long.into_iter().chain(short)
}

/// Whether completed input still permits options and subcommand discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlagContext {
    /// Named options remain available before the delimiter.
    Options,
    /// All following input belongs to positional values.
    Positionals,
}

/// Matches exact long, short, or configured alias spellings without parsing a
/// value.
fn matches_flag(argument: &clap::Arg, value: &str) -> bool {
    value.strip_prefix("--").map_or_else(
        || {
            value.strip_prefix('-').is_some_and(|short| {
                argument
                    .get_short()
                    .is_some_and(|expected| short.chars().eq(std::iter::once(expected)))
            })
        },
        |long| {
            argument.get_long() == Some(long)
                || argument
                    .get_all_aliases()
                    .is_some_and(|aliases| aliases.contains(&long))
        },
    )
}

/// Enumerates stable possible-value metadata without calling the parser
/// implementation.
fn describe_argument(argument: &clap::Arg) -> crate::metadata::ContextDescription {
    let help = argument
        .get_help()
        .map_or_else(String::new, ToString::to_string);
    let candidates = argument
        .get_possible_values()
        .into_iter()
        .filter(|value| !value.is_hide_set())
        .take(64)
        .map(|value| crate::metadata::Candidate {
            value: value.get_name().to_owned(),
            description: help.clone(),
        })
        .collect();
    crate::metadata::ContextDescription {
        candidates,
        help,
        roles: Vec::new(),
    }
}
