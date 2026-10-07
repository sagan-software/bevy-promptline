//! Bounded accepted-command history with draft restoration and sensitive-input exclusion.
use bevy_promptline::command::{CommandHistory, Sensitivity};

#[test]
fn history_browsing_restores_the_draft_and_never_retains_sensitive_commands() {
    let mut history = CommandHistory::default();
    history.record("echo first", Sensitivity::Public);
    history.record("password secret", Sensitivity::Redacted);
    history.record("echo second", Sensitivity::Public);
    assert_eq!(
        history.entries().collect::<Vec<_>>(),
        ["echo first", "echo second"]
    );
    assert_eq!(history.previous("unfinished"), Some("echo second"));
    assert_eq!(history.previous("echo second"), Some("echo first"));
    assert_eq!(history.next(), Some("echo second"));
    assert_eq!(history.next(), Some("unfinished"));
    assert_eq!(history.next(), None);
}

#[test]
fn history_enforces_retention_limits_and_handles_empty_or_oldest_browsing() {
    let mut history = CommandHistory::default();
    assert_eq!(history.previous("draft"), None);
    assert_eq!(history.next(), None);
    for index in 0..205 {
        history.record(&format!("echo {index}"), Sensitivity::Public);
    }
    assert_eq!(history.entries().count(), 200);
    assert_eq!(history.entries().next(), Some("echo 5"));
    for _ in 0..210 {
        history.previous("draft");
    }
    assert_eq!(history.previous("draft"), Some("echo 5"));
    history.record(&"x".repeat(512 * 1024 + 1), Sensitivity::Public);
    assert_eq!(history.entries().count(), 200);
}
