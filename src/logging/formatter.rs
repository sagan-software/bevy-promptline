//! Formatting retained records without recapturing tracing events.
use super::record::LogRecord;
use std::fmt::Write as _;

/// Converts retained structured records into plain presentation text without
/// capture side effects. Applications can replace formatting while preserving
/// the record, capture filter, and host logging sinks.
pub trait LogFormatter: Send + Sync + 'static {
    /// Formats one retained record without generating events or changing its
    /// stored values. The returned text has no command execution or terminal
    /// escape interpretation semantics.
    fn format(&self, record: &LogRecord) -> String;
}

/// Produces level, target, message, and lexically ordered fields on one line.
/// It omits elapsed time and source location while preserving sensitive-field
/// redaction markers.
#[derive(Debug, Default, Clone, Copy)]
pub struct CompactFormatter;
impl LogFormatter for CompactFormatter {
    fn format(&self, record: &LogRecord) -> String {
        // Borrow retained labels while selecting the canonical severity spelling.
        let level = severity_label(record.severity);
        let target = &record.target;
        let message = &record.message;
        // Prefix order and punctuation remain part of the emitted row.
        let mut text = format!("{level} {target}: {message}");
        // Typed fields follow the message in lexical map order.
        append_fields(&mut text, record);
        text
    }
}

/// Displays elapsed seconds with three fractional digits before the compact
/// record text. The timestamp is a duration since capture initialization and
/// never claims to represent UTC.
#[derive(Debug, Default, Clone, Copy)]
pub struct ReadableFormatter;
impl LogFormatter for ReadableFormatter {
    fn format(&self, record: &LogRecord) -> String {
        // Seconds and the millisecond remainder preserve truncation rather than rounding.
        let seconds = record.elapsed.as_secs();
        let millis = record.elapsed.subsec_millis();
        // Reuse compact field order before adding the elapsed prefix.
        let body = CompactFormatter.format(record);
        format!("[{seconds}.{millis:03}] {body}")
    }
}

/// Shares a caller-selected formatter with the app-thread console presentation.
/// Replacing this resource changes retained row text without changing capture
/// or installing another subscriber.
#[derive(bevy::prelude::Resource)]
pub struct LogPresentation {
    /// Caller-owned formatting behavior shared without cloning retained
    /// records.
    formatter: std::sync::Arc<dyn LogFormatter>,
}
impl std::fmt::Debug for LogPresentation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LogPresentation")
            .field("formatter", &"<redacted callback>")
            .finish()
    }
}
impl Default for LogPresentation {
    fn default() -> Self {
        Self::new(ReadableFormatter)
    }
}
impl LogPresentation {
    /// Selects custom formatting for retained records without modifying capture
    /// configuration. The formatter is shared by presentation and remains
    /// independent of host terminal or browser sinks.
    pub fn new(formatter: impl LogFormatter) -> Self {
        Self {
            formatter: std::sync::Arc::new(formatter),
        }
    }
    /// Borrows the selected formatter without cloning retained structured
    /// records. Applications can render alternate views from this same behavior
    /// without duplicating captured events.
    #[must_use]
    pub fn formatter(&self) -> &dyn LogFormatter {
        self.formatter.as_ref()
    }
}

/// Uses one canonical uppercase spelling for each typed severity.
pub(super) const fn severity_label(severity: crate::Severity) -> &'static str {
    match severity {
        crate::Severity::Trace => "TRACE",
        crate::Severity::Debug => "DEBUG",
        crate::Severity::Info => "INFO",
        crate::Severity::Warn => "WARN",
        crate::Severity::Error => "ERROR",
    }
}

/// Appends typed fields in the record's lexical order while preserving
/// redaction markers.
pub(super) fn append_fields(text: &mut String, record: &LogRecord) {
    // Fields remain ordered by the record's map rather than callback order.
    for (name, value) in &record.fields {
        // Display streams captured values directly into the existing String buffer.
        text.write_fmt(format_args!(" {name}={value}"))
            .expect("writing captured fields to a String is infallible");
    }
}
