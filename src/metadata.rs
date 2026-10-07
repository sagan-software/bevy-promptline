//! Command descriptions and parser contracts.
use crate::{CommandName, InvalidCommandName};

/// Describes a registered command for discovery and parsing. Identity
/// validation happens during construction; live analysis reads these
/// descriptions without executing the parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandMetadata {
    /// The primary command identity validated during construction. Registration
    /// checks this name against every existing primary name and alias before
    /// creating the handler system.
    pub name: CommandName,
    /// Alternative names resolving to the same command and handler.
    /// Registration rejects collisions with other commands and duplicate
    /// identities within this metadata record.
    pub aliases: Vec<CommandName>,
    /// The command purpose supplied by its author. This text describes the
    /// command independently of parsing, so discovery can read it without
    /// running validators.
    pub summary: String,
    /// The argument syntax supplied by the command author. Construction
    /// initially uses the command name; applications can replace it with their
    /// complete usage text.
    pub usage: String,
    /// Argument and subcommand metadata supplied by the command author. These
    /// records describe help and candidate values without invoking the command
    /// parser or handler.
    pub arguments: Vec<ArgumentMetadata>,
    /// Identifies whether command arguments may appear in history and
    /// observations. This metadata does not grant permission; the registry
    /// evaluates execution authorization separately.
    pub sensitivity: Sensitivity,
}

/// Describes argument disclosure separately from execution authorization.
/// Applications select the policy in command metadata; sensitive arguments must
/// remain private to parsing and execution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Sensitivity {
    /// The command author permits argument disclosure in history and
    /// observations. This value does not make the command available to an actor
    /// denied by authorization.
    #[default]
    Public,
    /// The command author requires arguments to remain private to parsing and
    /// execution. Submission debug output already hides raw input before this
    /// policy is known.
    Redacted,
}

/// Describes one argument, option, or subcommand for live help. Candidate
/// values are metadata, and inspecting them must not invoke arbitrary custom
/// value validators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentMetadata {
    /// The argument identity, flag spelling, or subcommand name supplied by its
    /// author. This metadata describes parsing syntax without granting
    /// execution permission to an actor.
    pub name: String,
    /// The explanation supplied by the command author. Include units and valid
    /// ranges when an argument has them, so help can explain accepted values.
    pub help: String,
    /// Enumerated values available for metadata-driven completion. An empty
    /// list means no values were supplied; it does not mean the parser rejects
    /// every value.
    pub values: Vec<String>,
}

impl CommandMetadata {
    /// Validates a command identity and stores its summary. Aliases and
    /// arguments initially remain empty, usage contains the name, and argument
    /// sensitivity starts public.
    ///
    /// # Errors
    ///
    /// Returns `InvalidCommandName` for a malformed identity.
    pub fn new(name: &str, summary: impl Into<String>) -> Result<Self, InvalidCommandName> {
        Ok(Self {
            name: name.parse()?,
            aliases: Vec::new(),
            summary: summary.into(),
            usage: name.to_owned(),
            arguments: Vec::new(),
            sensitivity: Sensitivity::Public,
        })
    }
}

/// Defines metadata and typed parsing for a game command. Registration connects
/// accepted values to one Bevy system; rejected input cannot reach that
/// handler.
pub trait ConsoleCommand: Send + 'static {
    /// Constructs metadata without parsing input or mutating game state.
    /// Registration uses the returned descriptions and validates collisions
    /// before creating the command handler system.
    ///
    /// # Errors
    ///
    /// Returns an identity validation error before registration mutates the
    /// app.
    fn metadata() -> Result<CommandMetadata, InvalidCommandName>;
    /// Parses decoded arguments without terminating the process or mutating
    /// game state. Return a diagnostic for rejection; only an accepted value
    /// reaches the handler.
    ///
    /// # Errors
    ///
    /// Returns a readable rejection diagnostic. Rejected input cannot reach the
    /// handler.
    fn parse(arguments: &[String]) -> Result<Self, String>
    where
        Self: Sized;
}

/// One permitted command spelling and its author-supplied description. It is
/// metadata for discovery, and selecting it does not invoke a command parser or
/// handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// The spelling accepted through completion. It identifies a registered
    /// primary name or alias and remains separate from the description supplied
    /// for discovery.
    pub value: String,
    /// The purpose supplied by the command author. Analysis reads it as
    /// metadata without validating arguments or transferring execution
    /// ownership to a registered game system.
    pub description: String,
}

/// Owned metadata description returned by adapter-specific context analysis.
#[derive(Debug, Default)]
pub(crate) struct ContextDescription {
    /// Candidate values before prefix filtering.
    pub(crate) candidates: Vec<Candidate>,
    /// Current command or argument description.
    pub(crate) help: String,
    /// Adapter overrides follow token order; absence preserves the lexical role.
    pub(crate) roles: Vec<Option<crate::highlighting::HighlightRole>>,
}
