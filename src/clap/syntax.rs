//! Metadata traversal identifies subcommands without running argument validators.
use crate::highlighting::HighlightRole;
use clap::{Arg, Command};

/// Cursor keeps metadata ownership and pending value consumption together.
pub(super) struct Syntax<'a> {
    /// Current command is borrowed from the immutable descriptor tree.
    pub(super) command: &'a Command,
    /// An option or positional argument can own subsequent values.
    pending: Option<PendingValues<'a>>,
    /// The delimiter disables named-token discovery.
    pub(super) flags: super::FlagContext,
    /// Next positional slot derives from consumed argument starts.
    pub(super) positional: usize,
}
/// Values remain attached to their option until its declared arity is filled.
struct PendingValues<'a> {
    /// Immutable argument metadata owns its value-count profile.
    argument: &'a Arg,
    /// Values already encountered, including an inline assignment.
    received: usize,
}
impl<'a> Syntax<'a> {
    /// Starts traversal after the command token with option discovery enabled.
    pub(super) const fn new(command: &'a Command) -> Self {
        Self {
            command,
            pending: None,
            flags: super::FlagContext::Options,
            positional: 1,
        }
    }
    /// Returns a pending argument only while its declared arity still permits values.
    pub(super) fn pending_argument(&self) -> Option<&'a Arg> {
        let values = self.pending.as_ref()?;
        let arity = values.argument.get_num_args().unwrap_or_else(|| 1.into());
        (values.received < arity.max_values()).then_some(values.argument)
    }
    /// Classifies one token and applies its metadata transition without validation.
    pub(super) fn classify(&mut self, token: &crate::Token) -> Option<HighlightRole> {
        // Assignments account for the first value without splitting original source bytes.
        let (spelling, inline) = token
            .value
            .split_once('=')
            .map_or((token.value.as_str(), 0), |(name, _)| (name, 1));
        let flag = self
            .command
            .get_arguments()
            .find(|argument| super::matches_flag(argument, spelling));
        if let Some(role) = self.consume_pending(flag) {
            return Some(role);
        }
        // The delimiter ends named discovery for subsequent tokens.
        if self.flags == super::FlagContext::Options && token.value == "--" {
            self.flags = super::FlagContext::Positionals;
            return Some(HighlightRole::Option);
        }
        if self.flags == super::FlagContext::Options
            && let Some(role) = self.named_token(token, flag, inline)
        {
            return Some(role);
        }
        // Positional metadata can retain additional values until its arity is filled.
        self.start_positional();
        None
    }
    /// Consumes values before matching subcommands with the same spelling.
    fn consume_pending(&mut self, flag: Option<&Arg>) -> Option<HighlightRole> {
        let values = self.pending.as_mut()?;
        let arity = values.argument.get_num_args().unwrap_or_else(|| 1.into());
        let is_known_flag = flag.is_some();
        let is_hyphen_allowed = values.argument.is_allow_hyphen_values_set();
        // A recognized flag starts a new argument unless this value accepts hyphens.
        let is_new_flag = is_known_flag && !is_hyphen_allowed;
        if !is_new_flag && values.received < arity.max_values() {
            values.received += 1;
            return Some(HighlightRole::Value);
        }
        // Completed value ownership is released before the next metadata transition.
        self.pending = None;
        None
    }
    /// Named options and subcommands refine lexical styling under the active command.
    fn named_token(
        &mut self,
        token: &crate::Token,
        flag: Option<&'a Arg>,
        inline: usize,
    ) -> Option<HighlightRole> {
        // Options retain their values only when separate value tokens are permitted.
        if let Some(argument) = flag {
            let is_value_action = argument.get_action().takes_values();
            let is_equals_required = argument.is_require_equals_set();
            if is_value_action && !is_equals_required {
                self.pending = Some(PendingValues {
                    argument,
                    received: inline,
                });
            }
            return Some(HighlightRole::Option);
        }
        // Subcommand aliases enter the same immutable descriptor and restart positional slots.
        let subcommand = self.command.get_subcommands().find(|subcommand| {
            subcommand.get_name() == token.value
                || subcommand
                    .get_all_aliases()
                    .any(|alias| alias == token.value)
        })?;
        self.command = subcommand;
        self.positional = 1;
        Some(HighlightRole::Subcommand)
    }
    /// Starts a positional argument without assigning parser acceptance to its value.
    fn start_positional(&mut self) {
        // Metadata arity decides whether later tokens still belong to this positional argument.
        if let Some(argument) = self
            .command
            .get_arguments()
            .find(|argument| argument.get_index() == Some(self.positional))
        {
            self.pending = Some(PendingValues {
                argument,
                received: 1,
            });
        }
        self.positional += 1;
    }
}

/// Returns one metadata override per decoded source token.
pub(super) fn roles(command: &Command, tokens: &[crate::Token]) -> Vec<Option<HighlightRole>> {
    let mut result = Vec::with_capacity(tokens.len());
    let mut cursor = Syntax::new(command);
    // Empty input has no command span; remaining input begins with its permitted owner.
    if !tokens.is_empty() {
        result.push(Some(HighlightRole::Command));
    }
    // Later tokens share one cursor so values cannot become independent commands.
    result.extend(tokens.iter().skip(1).map(|token| cursor.classify(token)));
    result
}
