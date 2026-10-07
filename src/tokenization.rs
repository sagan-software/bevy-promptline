//! Command text decoding with original UTF-8 source ranges.
use std::ops::Range;
use thiserror::Error;

/// Stores decoded command text with its original UTF-8 source range. Quote and
/// escape markers remain represented by the range while the value contains
/// decoded content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The decoded token value without quote delimiters or consumed escape
    /// markers. Unicode spelling remains unchanged, and empty quoted arguments
    /// remain present as empty values.
    pub value: String,
    /// The byte range in the original input that produced this token. It
    /// includes quote delimiters and escape markers, with both endpoints on
    /// UTF-8 character boundaries.
    pub range: Range<usize>,
}

/// Identifies malformed input rejected by the single-line command text profile.
/// Each variant describes a separate restriction checked before command parsing
/// or game handler execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TokenizeError {
    /// A single or double quote remained open at submission. Typing analysis
    /// may tolerate this state, but complete command decoding requires its
    /// matching closing delimiter.
    #[error("close the quoted argument before submitting")]
    UnclosedQuote,
    /// An escape outside single quotes had no following character at
    /// submission. Incomplete typing analysis may retain the trailing backslash
    /// until another character is entered.
    #[error("a trailing backslash needs a following character")]
    TrailingEscape,
    /// The input contained a newline or carriage return. The single-line
    /// profile rejects the entire input before decoding any argument or
    /// invoking a command parser.
    #[error("commands must contain one line")]
    LineBreak,
    /// The input contained a NUL character. The command text profile rejects it
    /// before decoding tokens, even when the character occurs inside quotes or
    /// an escape.
    #[error("commands cannot contain NUL")]
    Nul,
}

/// Decodes one UTF-8 command while preserving source ranges and Unicode
/// spelling. Quotes and escapes group arguments; shell expansion, chaining, and
/// redirection have no special semantics.
///
/// # Errors
///
/// Returns a distinct error for unmatched quotes, trailing escapes, NUL, or
/// line breaks.
pub fn tokenize(input: &str) -> Result<Vec<Token>, TokenizeError> {
    decode(input, DecodeMode::Complete)
}

/// Whether decoding requires a complete submitted command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DecodeMode {
    /// Submission requires closing quotes and complete escapes.
    Complete,
    /// Typing analysis retains unfinished quotes and escapes.
    Incomplete,
}

/// Current quote delimiter while decoding one token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Quote {
    /// Whitespace separates tokens and backslashes escape any character.
    Unquoted,
    /// Single quotes preserve literal content.
    Single,
    /// Double quotes permit quote and backslash escapes.
    Double,
}

/// Presentation facts recorded by the same decoding transitions as submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenForm {
    /// No quote delimiters occurred in this token.
    Bare,
    /// Quote delimiters occurred and all syntax is complete.
    Quoted,
    /// A quote remains open while typing.
    UnclosedQuote,
    /// An escape has no following character while typing.
    TrailingEscape,
}

/// Rejects forbidden characters before allocating decoded token values.
fn validate(input: &str) -> Result<(), TokenizeError> {
    // NUL rejection precedes the independent single-line restriction.
    if input.contains('\0') {
        return Err(TokenizeError::Nul);
    }
    if input.contains(['\n', '\r']) {
        return Err(TokenizeError::LineBreak);
    }
    Ok(())
}

/// Decodes shared token structure while preserving original source coordinates.
pub(crate) fn decode(input: &str, mode: DecodeMode) -> Result<Vec<Token>, TokenizeError> {
    let mut tokens = Vec::new();
    visit_tokens(input, mode, |token, _| tokens.push(token))?;
    Ok(tokens)
}

/// Emits tokens and their forms in one pass without a second decoded collection.
pub(crate) fn visit_tokens(
    input: &str,
    mode: DecodeMode,
    mut visit: impl FnMut(Token, TokenForm),
) -> Result<(), TokenizeError> {
    validate(input)?;
    let mut characters = input.char_indices().peekable();
    // Skip separators only outside tokens, preserving quoted whitespace as
    // content.
    while let Some(first) = characters.next() {
        if first.1.is_whitespace() {
            continue;
        }
        let (token, form) = decode_token(first, &mut characters, mode)?;
        visit(token, form);
    }
    Ok(())
}

/// Decodes one token and records every consumed quote or escape in its source
/// range.
fn decode_token(
    first: (usize, char),
    characters: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    mode: DecodeMode,
) -> Result<(Token, TokenForm), TokenizeError> {
    // A source range starts at the first non-separator character.
    let mut token = Token {
        value: String::new(),
        range: first.0..first.0,
    };
    let mut quote = Quote::Unquoted;
    let mut form = TokenForm::Bare;
    let mut current = Some(first);
    // Source offsets advance independently of decoded character count.
    while let Some((offset, character)) = current {
        token.range.end = decode_character(
            character,
            offset,
            &mut quote,
            characters,
            mode,
            &mut token.value,
            &mut form,
        )?;
        let is_separator = characters
            .peek()
            .is_some_and(|(_, next)| next.is_whitespace());
        let is_boundary = quote == Quote::Unquoted && is_separator;
        if is_boundary {
            break;
        }
        current = characters.next();
    }
    // Only submission requires a matching closing quote.
    if quote != Quote::Unquoted && mode == DecodeMode::Complete {
        return Err(TokenizeError::UnclosedQuote);
    }
    // Escape failure precedes the final unmatched-quote check in both decoding modes.
    if quote != Quote::Unquoted && form != TokenForm::TrailingEscape {
        form = TokenForm::UnclosedQuote;
    }
    Ok((token, form))
}

/// Applies one quote-state transition or appends one decoded character.
fn decode_character(
    character: char,
    offset: usize,
    quote: &mut Quote,
    characters: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    mode: DecodeMode,
    value: &mut String,
    form: &mut TokenForm,
) -> Result<usize, TokenizeError> {
    let end = offset + character.len_utf8();
    // Quote delimiters change decoding mode without entering the argument
    // value.
    match (*quote, character) {
        (Quote::Unquoted, '\'') => {
            *quote = Quote::Single;
            *form = TokenForm::Quoted;
        }
        (Quote::Unquoted, '"') => {
            *quote = Quote::Double;
            *form = TokenForm::Quoted;
        }
        (Quote::Single, '\'') | (Quote::Double, '"') => *quote = Quote::Unquoted,
        (Quote::Unquoted | Quote::Double, '\\') => {
            // Record unfinished escapes without treating them as submission rejection.
            if characters.peek().is_none() && mode == DecodeMode::Incomplete {
                *form = TokenForm::TrailingEscape;
            }
            let (end, escaped) = decode_escape(*quote, characters, mode, end)?;
            value.push(escaped);
            return Ok(end);
        }
        _ => value.push(character),
    }
    Ok(end)
}

/// Consumes an escape when permitted by the active quote profile.
fn decode_escape(
    quote: Quote,
    characters: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    mode: DecodeMode,
    end: usize,
) -> Result<(usize, char), TokenizeError> {
    // An unfinished draft preserves its final backslash until more input
    // arrives.
    let Some((offset, character)) = characters.peek().copied() else {
        return match mode {
            DecodeMode::Complete => Err(TokenizeError::TrailingEscape),
            DecodeMode::Incomplete => Ok((end, '\\')),
        };
    };
    let is_allowed = quote == Quote::Unquoted || matches!(character, '"' | '\\');
    // Unrecognized double-quote escapes preserve the backslash and following
    // character.
    if !is_allowed {
        return Ok((end, '\\'));
    }
    characters.next();
    Ok((offset + character.len_utf8(), character))
}

/// Encodes one argument using the same quote and escape profile as decoding.
/// Empty values and values containing whitespace, quotes, or backslashes
/// receive quoted encoding.
#[must_use]
pub fn quote_argument(value: &str) -> String {
    let is_empty = value.is_empty();
    let has_reserved_character = value
        .chars()
        .any(|character| character.is_whitespace() || matches!(character, '\'' | '"' | '\\'));
    let needs_quotes = is_empty || has_reserved_character;
    // Preserve plain spellings when quote delimiters would add no decoding
    // information.
    if !needs_quotes {
        return value.to_owned();
    }
    let mut encoded = String::with_capacity(value.len() + 2);
    encoded.push('"');
    // Escape individual quote and backslash characters without re-escaping
    // inserted markers.
    for character in value.chars() {
        if matches!(character, '"' | '\\') {
            encoded.push('\\');
        }
        encoded.push(character);
    }
    encoded.push('"');
    encoded
}

#[cfg(test)]
mod tests {
    use super::{DecodeMode, TokenizeError, decode, visit_tokens};

    /// Forbidden content is rejected before a streaming consumer observes any token.
    #[test]
    fn forbidden_content_prevents_all_token_visits() {
        for mode in [DecodeMode::Complete, DecodeMode::Incomplete] {
            for (input, expected) in [
                ("echo later\0", TokenizeError::Nul),
                ("echo later\n", TokenizeError::LineBreak),
                ("echo later\r", TokenizeError::LineBreak),
            ] {
                let mut visits = 0;
                let result = visit_tokens(input, mode, |_, _| visits += 1);
                assert_eq!(result, Err(expected));
                assert_eq!(visits, 0);
            }
        }
    }

    #[test]
    fn incomplete_analysis_retains_unfinished_quotes_and_escapes() {
        for (input, value) in [("'open", "open"), ("\"open", "open"), ("open\\", "open\\")] {
            let decoded = decode(input, DecodeMode::Incomplete).unwrap();
            assert_eq!(decoded.first().unwrap().value, value);
            assert_eq!(decoded.first().unwrap().range, 0..input.len());
        }
        assert_eq!(
            decode("\0", DecodeMode::Incomplete),
            Err(TokenizeError::Nul)
        );
        assert_eq!(
            decode("\n", DecodeMode::Incomplete),
            Err(TokenizeError::LineBreak)
        );
    }
}
