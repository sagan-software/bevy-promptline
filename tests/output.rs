//! Direct-output retention bounds and Unicode truncation.
use bevy_promptline::{Limit, Output, OutputLimits, Severity};

/// Constructs constrained scrollback through validated limits.
fn buffer(entries: usize, bytes: usize, entry_bytes: usize) -> Output {
    Output::new(OutputLimits {
        entries: Limit::try_from(entries).unwrap(),
        bytes: Limit::try_from(bytes).unwrap(),
        entry_bytes: Limit::try_from(entry_bytes).unwrap(),
    })
}

#[test]
fn oldest_entries_are_evicted_at_exact_entry_and_byte_limits() {
    let mut output = buffer(2, 6, 6);
    output.write("abc");
    output.write("def");
    assert_eq!(output.retained_bytes(), 6);
    assert_eq!(output.dropped(), 0);
    output.write("g");
    assert_eq!(output.texts().collect::<Vec<_>>(), ["def", "g"]);
    assert_eq!(output.dropped(), 1);
    output.push(Severity::Error, "parser", "abcdef".to_owned());
    assert_eq!(output.texts().collect::<Vec<_>>(), ["abcdef"]);
    assert_eq!(output.dropped(), 3);
    assert_eq!(output.entries().next().unwrap().severity, Severity::Error);
    assert_eq!(output.entries().next().unwrap().target, "parser");
    output.clear();
    assert_eq!(output.retained_bytes(), 0);
    output.write("");
    assert_eq!(output.entries().next().unwrap().sequence, 5);
}

#[test]
fn overlong_entries_truncate_at_unicode_boundaries_and_fit_tiny_limits() {
    let mut output = buffer(2, 8, 5);
    output.write("éééé");
    assert_eq!(output.texts().next(), Some("é…"));
    assert_eq!(output.retained_bytes(), 5);
    let mut tiny = buffer(1, 1, 8);
    tiny.write("😀");
    assert_eq!(tiny.texts().next(), Some("~"));
    let mut ascii = buffer(1, 2, 2);
    ascii.write("abcd");
    assert_eq!(ascii.texts().next(), Some("a~"));
}

#[test]
fn truncation_never_splits_a_multibyte_scalar_before_the_marker() {
    let mut output = buffer(1, 5, 5);
    output.write("😀😀");
    assert_eq!(output.texts().next(), Some("…"));
    assert_eq!(output.retained_bytes(), 3);
}
