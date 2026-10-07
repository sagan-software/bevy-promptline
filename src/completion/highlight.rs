//! One shared decoding pass builds completion tokens and original-source styles.
use crate::highlighting::{EditDiagnostic, HighlightRole, HighlightSpan};
use crate::tokenization::{DecodeMode, TokenForm};

/// Transient analysis owns decoded tokens and their paired presentation facts.
pub(super) struct DraftTokens {
    /// Decoded values retain the existing provider request contract.
    pub(super) tokens: Vec<crate::Token>,
    /// Source spans preserve all original quote and escape bytes.
    pub(super) spans: Vec<HighlightSpan>,
    /// Incomplete syntax remains local to editing.
    pub(super) diagnostic: Option<EditDiagnostic>,
}
/// The delimiter ends option styling without changing parser authority.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Options {
    /// Option spellings may receive their own role.
    Enabled,
    /// Remaining tokens are positional values.
    Ended,
}

/// Builds both projections while consuming each decoded token exactly once.
pub(super) fn draft_tokens(input: &str) -> Result<DraftTokens, crate::TokenizeError> {
    let mut result = DraftTokens {
        tokens: Vec::new(),
        spans: Vec::new(),
        diagnostic: None,
    };
    let mut options = Options::Enabled;
    // The shared decoder validates forbidden content before emitting any token.
    crate::tokenization::visit_tokens(input, DecodeMode::Incomplete, |token, form| {
        let (role, diagnostic) = lexical_role(&token, form, result.tokens.is_empty(), options);
        if diagnostic.is_some() {
            result.diagnostic = diagnostic;
        }
        // The delimiter affects following values, including hyphen-prefixed spellings.
        if token.value == "--" {
            options = Options::Ended;
        }
        result
            .spans
            .push(HighlightSpan::new(token.range.clone(), role));
        result.tokens.push(token);
    })?;
    Ok(result)
}

/// Syntax state precedes positional styling so incomplete input stays non-authoritative.
fn lexical_role(
    token: &crate::Token,
    form: TokenForm,
    is_first: bool,
    options: Options,
) -> (HighlightRole, Option<EditDiagnostic>) {
    // Incomplete forms preserve the shared decoder's error precedence.
    let diagnostic = match form {
        TokenForm::UnclosedQuote => Some(EditDiagnostic::UnclosedQuote),
        TokenForm::TrailingEscape => Some(EditDiagnostic::TrailingEscape),
        TokenForm::Bare | TokenForm::Quoted => None,
    };
    if diagnostic.is_some() {
        return (HighlightRole::Incomplete, diagnostic);
    }
    // Complete command names remain distinct from argument quoting and option syntax.
    if is_first {
        return (HighlightRole::Command, None);
    }
    if form == TokenForm::Quoted {
        return (HighlightRole::Quoted, None);
    }
    if options == Options::Enabled && is_option(&token.value) {
        return (HighlightRole::Option, None);
    }
    (HighlightRole::Value, None)
}

/// Distinguishes lexical option spellings from negative numeric values.
fn is_option(value: &str) -> bool {
    // Long spellings identify syntax without asserting parser acceptance.
    let is_long = value.starts_with("--");
    let Some(tail) = value.strip_prefix('-') else {
        return is_long;
    };
    // Alphabetic short spellings exclude negative numeric values.
    let first = tail.chars().next();
    let is_short = first.is_some_and(char::is_alphabetic);
    is_long || is_short
}

/// Permission-aware lookup styles missing and denied names identically.
pub(super) fn mark_command(
    draft: &mut DraftTokens,
    registry: &crate::CommandRegistry,
    world: &bevy::prelude::World,
) {
    let (Some(first), Some(span)) = (draft.tokens.first(), draft.spans.first_mut()) else {
        return;
    };
    // Incomplete syntax keeps its editing diagnostic ahead of name discovery.
    if span.role() == HighlightRole::Incomplete {
        return;
    }
    // Only permitted names participate in command-prefix discovery.
    let is_known = is_visible_prefix(registry, world, &first.value);
    if !is_known {
        span.set_role(HighlightRole::Error);
        draft
            .diagnostic
            .get_or_insert(EditDiagnostic::UnknownCommand);
    }
}

/// Empty names never trigger discovery; other names use the registry's matching profile.
fn is_visible_prefix(
    registry: &crate::CommandRegistry,
    world: &bevy::prelude::World,
    value: &str,
) -> bool {
    if value.is_empty() {
        return false;
    }
    registry
        .visible(world, crate::command::SubmissionSource::Ui)
        .any(|metadata| {
            std::iter::once(&metadata.name)
                .chain(&metadata.aliases)
                .any(|name| registry.is_name_prefix_match(name.as_ref(), value))
        })
}

/// Command-position caret analysis still obtains syntax from its permitted owner.
pub(super) fn adapter_roles(
    registry: &crate::CommandRegistry,
    world: &bevy::prelude::World,
    tokens: &[crate::Token],
) -> Vec<Option<HighlightRole>> {
    // Resolve ownership before calling adapter metadata code.
    let Some(first) = tokens.first() else {
        return Vec::new();
    };
    let Some(command) = registry.resolve(&first.value) else {
        return Vec::new();
    };
    if !registry.is_permitted(world, &command, crate::command::SubmissionSource::Ui) {
        return Vec::new();
    }
    // A command-position caret still needs styles for the entire existing draft.
    command
        .describe
        .as_ref()
        .map_or_else(Vec::new, |describe| describe(tokens, 0).roles)
}

/// Incomplete syntax and quoted values retain their lexical presentation.
pub(super) fn apply_adapter_roles(spans: &mut [HighlightSpan], roles: &[Option<HighlightRole>]) {
    // Missing adapter entries retain their lexical classification.
    for (span, role) in spans.iter_mut().zip(roles) {
        let Some(role) = role else {
            continue;
        };
        if matches!(
            span.role(),
            HighlightRole::Incomplete | HighlightRole::Error
        ) {
            continue;
        }
        // Metadata refines semantic ownership without erasing quoted value presentation.
        if span.role() == HighlightRole::Quoted && *role == HighlightRole::Value {
            continue;
        }
        span.set_role(*role);
    }
}
