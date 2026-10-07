//! Command discovery and explicit draft replacement without parser execution.
use crate::command::{SubmissionSource, SubmittedText};
use crate::metadata::{Candidate, ContextDescription};
use bevy::prelude::World;
use std::ops::Range;
use thiserror::Error;
mod highlight;

/// An immutable analysis snapshot of a draft and its permitted command
/// candidates. Analysis retains the original text privately so acceptance can
/// reject stale input.
#[derive(Debug)]
pub struct Analysis {
    /// Original text whose debug output remains redacted.
    source: SubmittedText,
    /// Validated UTF-8 token range replaced on acceptance.
    range: Range<usize>,
    /// Permitted candidates in lexical spelling order.
    candidates: Vec<Candidate>,
    /// Non-authoritative suffix matching the next forward completion action.
    ghost: Option<String>,
    /// Contextual metadata help for the current command or argument.
    help: String,
    /// Source spans share token coordinates with completion and dispatch.
    highlights: Vec<crate::highlighting::HighlightSpan>,
    /// Local editing state remains separate from submission rejection.
    diagnostic: Option<crate::highlighting::EditDiagnostic>,
}

/// Editor activity controlling whether a ghost suffix can appear. Selection and
/// composition suppress the preview while leaving candidates available for
/// presentations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditingMode {
    /// The editor has a collapsed caret and no composition. A nonempty encoded
    /// suffix may appear as a preview without becoming authoritative command
    /// input.
    Caret,
    /// The editor has an active selection. Ghost text is withheld so a preview
    /// does not imply that it replaces selected input before explicit
    /// acceptance.
    Selection,
    /// The editor is composing text through an input method. Ghost text is
    /// withheld so completion does not compete with the composition's own
    /// displayed preview.
    Composition,
}

/// A completed draft and the source-byte caret position after replacement. The
/// accepted text preserves content outside the selected token and becomes
/// authoritative only when applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedCompletion {
    /// The draft after explicit replacement. Bytes outside the selected token
    /// remain unchanged, and the candidate uses the shared command text
    /// encoding profile.
    pub text: String,
    /// The UTF-8 byte position following the inserted token. Presentations use
    /// this position when moving their editor caret after applying the
    /// completed draft.
    pub caret: usize,
}

/// A failed analysis or completion acceptance without draft mutation. These
/// failures remain separate from submission rejection because typing does not
/// execute a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum AnalysisError {
    /// The requested caret is outside the draft or splits a UTF-8 scalar.
    /// Analysis requires source-byte coordinates on boundaries rather than
    /// guessed character widths.
    #[error("caret must be on a UTF-8 boundary within the draft")]
    InvalidCaret,
    /// The draft contains forbidden content. Incomplete quotes and escapes
    /// remain valid during analysis, but NUL and line breaks still violate the
    /// text profile.
    #[error(transparent)]
    Text(#[from] crate::TokenizeError),
    /// The draft changed after analysis. Acceptance refuses the replacement so
    /// stale candidate ranges cannot modify text belonging to a newer editor
    /// state.
    #[error("the draft changed; analyze it again before completing")]
    StaleInput,
    /// The requested candidate index is absent from this snapshot. Acceptance
    /// returns this error before constructing completed text or changing an
    /// editor caret.
    #[error("no completion candidate at that index")]
    MissingCandidate,
    /// The application has no command registry available for discovery.
    /// Analysis cannot infer registrations or permissions until the runtime
    /// installs that required resource.
    #[error("command registry is unavailable")]
    Unavailable,
}

impl Analysis {
    /// Borrows the analyzed source for presentation without exposing diagnostic arguments.
    pub(crate) const fn source_text(&self) -> &str {
        self.source.0.as_str()
    }
    /// Borrows typed source spans without modifying authoritative input.
    /// Spans preserve original UTF-8 coordinates, and adapters can refine
    /// semantic roles without running validators.
    #[must_use]
    pub fn highlights(&self) -> &[crate::highlighting::HighlightSpan] {
        &self.highlights
    }
    /// Returns local editing guidance independently of submission outcomes.
    /// Presentations can display its message without clearing input or creating
    /// a lifecycle rejection notification.
    #[must_use]
    pub const fn diagnostic(&self) -> Option<crate::highlighting::EditDiagnostic> {
        self.diagnostic
    }
    /// Borrows permitted candidates in lexical order without copying the
    /// collection. Candidate discovery has already evaluated policy and has not
    /// executed a command parser.
    #[must_use]
    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }

    /// Borrows a nonempty completion suffix only for ordinary caret editing.
    /// The suffix stays outside input, submission, history, and copying until
    /// explicit replacement.
    #[must_use]
    pub fn ghost(&self, mode: EditingMode) -> Option<&str> {
        match mode {
            EditingMode::Caret => self.ghost.as_deref(),
            EditingMode::Selection | EditingMode::Composition => None,
        }
    }

    /// Borrows contextual command or argument help from the same metadata
    /// snapshot. This description is independent of input text and never comes
    /// from running a validator.
    #[must_use]
    pub fn help(&self) -> &str {
        &self.help
    }

    /// Replaces the selected token with one explicitly chosen candidate.
    /// Acceptance verifies the original draft and preserves bytes before and
    /// after the token.
    ///
    /// # Errors
    ///
    /// Returns `StaleInput` for a changed draft or `MissingCandidate` for an
    /// absent candidate.
    pub fn accept(&self, input: &str, index: usize) -> Result<AppliedCompletion, AnalysisError> {
        // Reject stale draft coordinates before selecting a candidate.
        if input != self.source.0 {
            return Err(AnalysisError::StaleInput);
        }
        let candidate = self
            .candidates
            .get(index)
            .ok_or(AnalysisError::MissingCandidate)?;
        // Encoding and source preservation are shared with common-prefix replacement.
        replace_range(input, &self.range, &candidate.value)
    }
}

/// Analyzes the command token at a validated source-byte caret using permitted
/// metadata. Draft inspection tolerates unfinished quotes and escapes and never
/// invokes a registered parser.
///
/// # Errors
///
/// Returns an invalid-caret, forbidden-text, or unavailable-registry error
/// without mutating input.
pub fn analyze(world: &World, input: &str, caret: usize) -> Result<Analysis, AnalysisError> {
    // Validate and decode coordinates before any registry lookup or metadata exposure.
    let (mut draft, position, range, prefix) = draft_context(input, caret)?;
    let registry = world
        .get_resource::<crate::CommandRegistry>()
        .ok_or(AnalysisError::Unavailable)?;
    highlight::mark_command(&mut draft, registry, world);
    // Move decoded tokens into the provider request rather than cloning their strings.
    let request = crate::registry::CompletionRequest {
        tokens: draft.tokens,
        position,
        prefix,
    };
    // Permission-filtered metadata describes draft text without running a parser.
    let (candidates, help, roles) = if request.position == 0 {
        let candidates = command_candidates(registry, world, &request.prefix);
        let help = candidates
            .first()
            .map_or_else(String::new, |candidate| candidate.description.clone());
        let roles = highlight::adapter_roles(registry, world, &request.tokens);
        (candidates, help, roles)
    } else {
        argument_context(registry, world, &request)
    };
    highlight::apply_adapter_roles(&mut draft.spans, &roles);
    // A ghost suffix previews exactly the encoded replacement and only at token end.
    let replacement = forward_preview(&candidates, &request.prefix);
    let ghost = ghost_suffix(input, &range, caret, replacement);
    Ok(Analysis {
        source: SubmittedText(input.to_owned()),
        range,
        candidates,
        ghost,
        help,
        highlights: draft.spans,
        diagnostic: draft.diagnostic,
    })
}

/// Decodes one draft into validated token coordinates and its owned typed
/// prefix.
fn draft_context(
    input: &str,
    caret: usize,
) -> Result<(highlight::DraftTokens, usize, Range<usize>, String), AnalysisError> {
    let is_valid_caret = caret <= input.len() && input.is_char_boundary(caret);
    if !is_valid_caret {
        return Err(AnalysisError::InvalidCaret);
    }
    // Token ranges determine the whole replacement span even when the caret is inside it.
    let draft = highlight::draft_tokens(input)?;
    let tokens = &draft.tokens;
    let selected = tokens
        .iter()
        .enumerate()
        .find(|(_, token)| token.range.contains(&caret) || token.range.end == caret);
    let position = selected.map_or_else(
        || {
            tokens
                .iter()
                .take_while(|token| token.range.end < caret)
                .count()
        },
        |(index, _)| index,
    );
    let range = selected.map_or(caret..caret, |(_, token)| token.range.clone());
    // Decode the typed source prefix with the same unfinished-input rules as the draft.
    let typed = input
        .get(range.start..caret)
        .ok_or(AnalysisError::InvalidCaret)?;
    let prefix_tokens = crate::command::analyze_tokens(typed)?;
    let prefix = prefix_tokens
        .into_iter()
        .next()
        .map_or_else(String::new, |token| token.value);
    // Move the decoded prefix instead of cloning an allocation-backed token value.
    Ok((draft, position, range, prefix))
}

/// Chooses the same encoded preview as the first forward completion step.
fn forward_preview<'a>(candidates: &'a [Candidate], prefix: &str) -> Option<&'a str> {
    // A longer common prefix takes precedence over selecting the first candidate.
    let common = common_prefix(candidates);
    let common_length = common.len();
    let typed_length = prefix.len();
    let is_longer_common = candidates.len() > 1 && common_length > typed_length;
    if is_longer_common {
        Some(common)
    } else {
        // Without a longer shared spelling, preview the first actual candidate.
        candidates.first().map(|candidate| candidate.value.as_str())
    }
}

/// Keeps at most 64 lexically ordered command spellings before cloning their
/// metadata.
fn command_candidates(
    registry: &crate::CommandRegistry,
    world: &World,
    prefix: &str,
) -> Vec<Candidate> {
    // Cap retained candidate state before cloning matching metadata.
    let mut candidates = Vec::with_capacity(64);
    // Permission is checked before exposing either a primary name or any of its aliases.
    for metadata in registry.visible(world, SubmissionSource::Ui) {
        for name in std::iter::once(&metadata.name).chain(&metadata.aliases) {
            let value = name.as_str();
            if !registry.is_name_prefix_match(value, prefix) {
                continue;
            }
            let position = candidates
                .binary_search_by(|candidate: &Candidate| candidate.value.as_str().cmp(value))
                .unwrap_or_else(|position| position);
            if position >= 64 {
                continue;
            }
            // Bound retained candidate state even when every registered name matches.
            if candidates.len() == 64 {
                candidates.pop();
            }
            candidates.insert(
                position,
                Candidate {
                    value: value.to_owned(),
                    description: metadata.summary.clone(),
                },
            );
        }
    }
    candidates
}

/// Computes a literal suffix only when replacement extends the current raw
/// token spelling.
fn ghost_suffix(
    input: &str,
    range: &Range<usize>,
    caret: usize,
    candidate: Option<&str>,
) -> Option<String> {
    if caret != range.end {
        return None;
    }
    // The suffix is visible only when encoded replacement extends the raw spelling.
    let candidate = candidate?;
    let typed = input.get(range.clone())?;
    let encoded = crate::quote_argument(candidate);
    // Quoted spellings that need replacement rather than append remain in the candidate list.
    encoded
        .strip_prefix(typed)
        .filter(|suffix| !suffix.is_empty())
        .map(str::to_owned)
}

/// Derives fallback candidates and help from reusable registered argument metadata.
fn metadata_argument_context(
    metadata: &crate::CommandMetadata,
    position: usize,
) -> ContextDescription {
    let argument = metadata.arguments.get(position.saturating_sub(1));
    // Enumerated values are bounded before cloning retained candidate text.
    let candidates = argument.map_or_else(Vec::new, |argument| {
        argument
            .values
            .iter()
            .take(64)
            .map(|value| Candidate {
                value: value.clone(),
                description: argument.help.clone(),
            })
            .collect()
    });
    // Missing argument metadata retains the active command summary.
    let help = argument.map_or_else(
        || metadata.summary.clone(),
        |argument| argument.help.clone(),
    );
    ContextDescription {
        candidates,
        help,
        roles: Vec::new(),
    }
}

/// Selects permitted argument metadata without invoking the registered parser.
fn argument_context(
    registry: &crate::CommandRegistry,
    world: &World,
    request: &crate::registry::CompletionRequest,
) -> (
    Vec<Candidate>,
    String,
    Vec<Option<crate::highlighting::HighlightRole>>,
) {
    let tokens = request.tokens();
    let position = request.position();
    let prefix = request.prefix();
    // Resolve command ownership before exposing any contextual argument metadata.
    let Some(first) = tokens.first() else {
        return (Vec::new(), String::new(), Vec::new());
    };
    let Some(command) = registry.resolve(&first.value) else {
        return (Vec::new(), String::new(), Vec::new());
    };
    if !registry.is_permitted(world, &command, SubmissionSource::Ui) {
        return (Vec::new(), String::new(), Vec::new());
    }
    // Adapter metadata supplies help and semantic roles within the same permitted context.
    let mut context = command.describe.as_ref().map_or_else(
        || metadata_argument_context(&command.metadata, position),
        |describe| describe(tokens, position),
    );
    // The same owner policy protects live game metadata before its read-only system runs.
    if let Some(provider) = registry.provider(&command.metadata.name) {
        match provider.complete(request, world) {
            Ok(candidates) => context.candidates.extend(candidates),
            Err(error) => context.help = error.to_string(),
        }
    }
    finish_candidates(&mut context.candidates, prefix);
    (context.candidates, context.help, context.roles)
}

/// Bounds, filters, and orders candidate values before exposing replacements.
fn finish_candidates(candidates: &mut Vec<Candidate>, prefix: &str) {
    // Reject values outside the single-line text profile before exposing replacements.
    candidates.retain(|candidate| {
        candidate.value.starts_with(prefix) && !candidate.value.contains(['\0', '\n', '\r'])
    });
    // Stable ordering and deduplication preserve repeatable cycling across adapters.
    candidates.sort_by(|left, right| left.value.cmp(&right.value));
    candidates.dedup_by(|left, right| left.value == right.value);
    candidates.truncate(64);
}

/// Direction for candidate navigation and explicit completion acceptance.
/// Forward and backward steps use the same frozen candidate list until the
/// authoritative draft changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionDirection {
    /// Advance through candidates in lexical order and wrap at the end. An
    /// initial forward step first expands a longer common prefix when one
    /// exists.
    Forward,
    /// Move through candidates in reverse lexical order and wrap at the
    /// beginning. An initial backward step chooses the last candidate rather
    /// than expanding a prefix.
    Backward,
}

/// A completion session retaining one candidate list across its own
/// replacements. Changes outside this session reject stale ranges, while
/// forward and backward steps update the expected draft.
#[derive(Debug)]
pub struct CompletionCycle {
    /// Metadata and replacement range frozen for this session.
    analysis: Analysis,
    /// Last selection, or a common-prefix expansion before candidate selection.
    selection: CycleSelection,
}

/// Selection states distinguish prefix expansion from an accepted candidate.
#[derive(Debug, Clone, Copy)]
enum CycleSelection {
    /// No completion action has changed the draft.
    Initial,
    /// The common prefix was inserted; the next forward step selects candidate
    /// zero.
    Prefix,
    /// A validated candidate index selected by this cycle.
    Candidate(usize),
}

impl Analysis {
    /// Moves this snapshot into a cycle without copying its retained candidate
    /// list. Subsequent replacements preserve content outside the token and
    /// reject independently changed drafts.
    #[must_use]
    pub const fn into_cycle(self) -> CompletionCycle {
        CompletionCycle {
            analysis: self,
            selection: CycleSelection::Initial,
        }
    }
}

impl CompletionCycle {
    /// Checks whether editor movement or external text changes invalidated this
    /// cycle.
    pub(crate) fn is_current(&self, input: &str, caret: usize) -> bool {
        input == self.analysis.source.0 && caret == self.analysis.range.end
    }
    /// Accepts one completion step and updates the expected draft for further
    /// cycling. The first forward step expands a longer common prefix; later
    /// steps select candidates.
    ///
    /// # Errors
    ///
    /// Returns `StaleInput` for a changed draft or `MissingCandidate` when the
    /// list is empty.
    pub fn advance(
        &mut self,
        input: &str,
        direction: CompletionDirection,
    ) -> Result<AppliedCompletion, AnalysisError> {
        // Validate identity and candidate availability before selecting a replacement.
        self.validate_step(input)?;
        let (completion, selection) = self.prefix_step(input, direction)?.map_or_else(
            || self.candidate_step(input, direction),
            |completion| Ok((completion, CycleSelection::Prefix)),
        )?;
        // Rebase only after a successful step so errors leave the session unchanged.
        self.selection = selection;
        self.rebase(&completion);
        Ok(completion)
    }

    /// Validates the expected draft and nonempty candidate domain for every
    /// step.
    fn validate_step(&self, input: &str) -> Result<(), AnalysisError> {
        if input != self.analysis.source.0 {
            return Err(AnalysisError::StaleInput);
        }
        // Empty candidate domains cannot yield a wrapped selection index.
        if self.analysis.candidates.is_empty() {
            return Err(AnalysisError::MissingCandidate);
        }
        Ok(())
    }

    /// Expands a longer common prefix only on the initial forward action.
    fn prefix_step(
        &self,
        input: &str,
        direction: CompletionDirection,
    ) -> Result<Option<AppliedCompletion>, AnalysisError> {
        // Initial forward completion is the only action eligible for prefix expansion.
        let is_initial = matches!(self.selection, CycleSelection::Initial);
        let is_forward = direction == CompletionDirection::Forward;
        if !(is_initial && is_forward && self.analysis.candidates.len() > 1) {
            return Ok(None);
        }
        // The same draft decoder determines whether the common prefix adds actual content.
        let raw = input
            .get(self.analysis.range.clone())
            .ok_or(AnalysisError::InvalidCaret)?;
        let tokens = crate::command::analyze_tokens(raw)?;
        let typed = tokens.first().map_or("", |token| token.value.as_str());
        // A shorter shared spelling must never erase already typed input.
        let common = common_prefix(&self.analysis.candidates);
        if common.len() > typed.len() {
            replace_range(input, &self.analysis.range, common).map(Some)
        } else {
            Ok(None)
        }
    }

    /// Selects a wrapped candidate after prefix expansion or an explicit
    /// reverse action.
    fn candidate_step(
        &self,
        input: &str,
        direction: CompletionDirection,
    ) -> Result<(AppliedCompletion, CycleSelection), AnalysisError> {
        let count = self.analysis.candidates.len();
        let index = match (self.selection, direction) {
            (CycleSelection::Initial | CycleSelection::Prefix, CompletionDirection::Forward) => 0,
            (CycleSelection::Initial | CycleSelection::Prefix, CompletionDirection::Backward) => {
                count - 1
            }
            (CycleSelection::Candidate(index), CompletionDirection::Forward) => (index + 1) % count,
            (CycleSelection::Candidate(index), CompletionDirection::Backward) => {
                index.checked_sub(1).unwrap_or(count - 1)
            }
        };
        // Candidate indices derive from the nonempty list validated by advance.
        self.analysis
            .accept(input, index)
            .map(|completion| (completion, CycleSelection::Candidate(index)))
    }

    /// Updates the range after this session's own replacement without changing
    /// its candidate list.
    fn rebase(&mut self, completion: &AppliedCompletion) {
        self.analysis.range.end = completion.caret;
        self.analysis.source = SubmittedText(completion.text.clone());
    }
}

/// Returns a UTF-8 boundary prefix shared by every candidate.
fn common_prefix(candidates: &[Candidate]) -> &str {
    // Compare Unicode scalars so the resulting prefix always ends at a UTF-8 boundary.
    let Some(first) = candidates.first() else {
        return "";
    };
    let mut prefix = first.value.as_str();
    // Each additional candidate can only shorten the current shared spelling.
    for candidate in candidates.iter().skip(1) {
        let mut bytes = 0;
        // Scalar comparison preserves boundaries without normalizing Unicode spelling.
        for (left, right) in prefix.chars().zip(candidate.value.chars()) {
            if left != right {
                break;
            }
            bytes += left.len_utf8();
        }
        prefix = prefix.get(..bytes).unwrap_or_default();
    }
    prefix
}

/// Encodes one replacement and preserves the source bytes outside its token
/// range.
fn replace_range(
    input: &str,
    range: &Range<usize>,
    value: &str,
) -> Result<AppliedCompletion, AnalysisError> {
    let prefix = input
        .get(..range.start)
        .ok_or(AnalysisError::InvalidCaret)?;
    let suffix = input.get(range.end..).ok_or(AnalysisError::InvalidCaret)?;
    // Encode one argument while preserving all source bytes outside its token.
    let replacement = crate::quote_argument(value);
    // Allocate the final replacement once and append its three source-preserving segments.
    let mut text = String::with_capacity(prefix.len() + replacement.len() + suffix.len());
    text.push_str(prefix);
    text.push_str(&replacement);
    text.push_str(suffix);
    Ok(AppliedCompletion {
        text,
        caret: prefix.len() + replacement.len(),
    })
}

/// Retains permitted command help while the caret moves through its arguments.
pub(crate) fn active_command_help(world: &World, text: &str) -> String {
    let Ok(tokens) = crate::command::analyze_tokens(text) else {
        return String::new();
    };
    // Resolve command ownership before exposing any contextual argument metadata.
    let Some(first) = tokens.first() else {
        return String::new();
    };
    let Some(registry) = world.get_resource::<crate::CommandRegistry>() else {
        return String::new();
    };
    // Discovery uses the same policy as completion and does not invoke a parser.
    registry
        .visible(world, SubmissionSource::Ui)
        .find(|metadata| {
            metadata.name.as_str() == first.value
                || metadata
                    .aliases
                    .iter()
                    .any(|alias| alias.as_str() == first.value)
        })
        .map_or_else(String::new, |metadata| metadata.summary.clone())
}
