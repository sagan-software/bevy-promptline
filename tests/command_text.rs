//! Command text profile at the public parsing boundary.
use bevy_promptline::{TokenizeError, tokenize};

#[test]
fn empty_quotes_and_unicode_preserve_decoded_arguments_and_source_ranges() {
    let input = "echo '' \"héllo world\"";
    let tokens = tokenize(input).unwrap();
    assert_eq!(
        tokens
            .iter()
            .map(|token| token.value.as_str())
            .collect::<Vec<_>>(),
        ["echo", "", "héllo world"]
    );
    for token in tokens {
        assert!(input.get(token.range).is_some());
    }
}

#[test]
fn malformed_submission_has_distinct_errors() {
    assert_eq!(tokenize("echo '"), Err(TokenizeError::UnclosedQuote));
    assert_eq!(tokenize("echo \\"), Err(TokenizeError::TrailingEscape));
    assert_eq!(tokenize("echo\n"), Err(TokenizeError::LineBreak));
    assert_eq!(tokenize("echo\0"), Err(TokenizeError::Nul));
}

#[test]
fn profile_edges_and_exact_replacement_round_trips() {
    let inputs = [
        "",
        " \t",
        "echo -- -2",
        "echo a\\ b",
        "echo a;b",
        "echo \"a\\q\"",
        "echo 'a\\b'",
        "echo \"a\\\"b\"",
        "e\u{301} 😀",
    ];
    let expected: &[&[&str]] = &[
        &[],
        &[],
        &["echo", "--", "-2"],
        &["echo", "a b"],
        &["echo", "a;b"],
        &["echo", "a\\q"],
        &["echo", "a\\b"],
        &["echo", "a\"b"],
        &["e\u{301}", "😀"],
    ];
    for (input, expected) in inputs.into_iter().zip(expected) {
        let tokens = tokenize(input).unwrap();
        assert_eq!(
            tokens
                .iter()
                .map(|token| token.value.as_str())
                .collect::<Vec<_>>(),
            *expected
        );
    }
    for value in ["", "a b", "'", "\"", "\\", "😀", "plain", "e\u{301}"] {
        let encoded = bevy_promptline::quote_argument(value);
        let decoded = tokenize(&encoded).unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded.first().unwrap().value, value);
        assert_eq!(decoded.first().unwrap().range, 0..encoded.len());
    }
    assert_eq!(tokenize("echo\r"), Err(TokenizeError::LineBreak));
}

#[test]
fn identity_and_limits_validate_external_construction() {
    use bevy_promptline::{CommandName, Limit};
    for invalid in ["", "1abc", "a b", "é", "a.b", "a/b", "a\n"] {
        assert!(invalid.parse::<CommandName>().is_err());
    }
    for valid in ["a", "Echo", "scene-reset", "a_2"] {
        assert_eq!(valid.parse::<CommandName>().unwrap().as_str(), valid);
    }
    assert!(Limit::try_from(0).is_err());
    assert_eq!(Limit::try_from(1).unwrap().get(), 1);
}

#[test]
fn draft_analysis_tolerates_incomplete_quotes_but_rejects_forbidden_content() {
    let tokens = bevy_promptline::command::analyze_tokens("echo 'unfinished").unwrap();
    assert_eq!(tokens.last().unwrap().value, "unfinished");
    assert_eq!(
        bevy_promptline::command::analyze_tokens("echo\n"),
        Err(TokenizeError::LineBreak)
    );
    assert_eq!(
        bevy_promptline::command::analyze_tokens("echo\0"),
        Err(TokenizeError::Nul)
    );
}

#[test]
fn standard_borrowed_name_view_preserves_its_validated_spelling() {
    let name = "Scene-reset"
        .parse::<bevy_promptline::CommandName>()
        .unwrap();
    let spelling: &str = name.as_ref();
    assert_eq!(spelling, "Scene-reset");
}
