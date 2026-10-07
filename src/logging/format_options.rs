//! Typed elapsed precision and independent plain-text formatting choices.
use super::{LogFormatter, LogRecord};
use std::fmt::Write as _;

/// Chooses independent labels and retained details for the configured log
/// formatter. These settings change only plain-text presentation and cannot
/// widen capture or modify another subscriber sink.
#[expect(
    clippy::struct_excessive_bools,
    reason = "Level, target, fields, and source are independent display switches with no invalid combinations."
)]
#[derive(Debug, Clone, Copy)]
pub struct FormatOptions {
    /// Optional elapsed precision controls timestamp presence and exact
    /// fractional spelling. Absence omits the timestamp without changing the
    /// original duration in retained records.
    pub elapsed: Option<ElapsedPrecision>,
    /// Includes the uppercase severity label when enabled, independently of
    /// theme colors. Disabling this label does not remove severity from
    /// retained records or display filtering.
    pub level: bool,
    /// Includes the captured event target when enabled, independently of
    /// capture directives. Disabling its label leaves the original target
    /// available for display selection and inspection.
    pub target: bool,
    /// Includes event fields in lexical name order when enabled, preserving
    /// redaction markers. Disabling fields leaves typed retained values
    /// available for another formatter or inspection.
    pub fields: bool,
    /// Includes available filename and line metadata when enabled, without
    /// inventing missing values. Browser or stripped callsites can omit this
    /// context even when the option is enabled.
    pub source: bool,
}
impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            elapsed: Some(ElapsedPrecision::Milliseconds),
            level: true,
            target: true,
            fields: true,
            source: false,
        }
    }
}

/// Selects the exact fractional precision for elapsed durations in log rows.
/// Every spelling measures seconds from capture initialization and does not
/// represent UTC or an offset timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElapsedPrecision {
    /// Whole seconds omit fractional digits and truncate the duration's
    /// subsecond remainder. This presentation choice leaves the retained
    /// duration unchanged for another formatter.
    Seconds,
    /// Milliseconds emit exactly three fractional digits after whole elapsed
    /// seconds. The fractional value truncates smaller units and never rounds
    /// into the next second.
    Milliseconds,
    /// Microseconds emit exactly six fractional digits after whole elapsed
    /// seconds. The fractional value truncates nanoseconds while the original
    /// duration remains retained.
    Microseconds,
}

/// Formats retained log records with caller-selected labels, precision, fields,
/// and source context.
///
/// It keeps presentation choices separate from capture
/// filtering, display selection, and host sink formatting.
#[derive(Debug, Default, Clone, Copy)]
pub struct ConfiguredFormatter {
    /// Independent formatting choices retained as one presentation
    /// configuration.
    options: FormatOptions,
}
impl ConfiguredFormatter {
    /// Selects formatting behavior without changing retained records or
    /// subscriber configuration. The resulting formatter can replace
    /// presentation after capture without producing duplicate events.
    #[must_use]
    pub const fn new(options: FormatOptions) -> Self {
        Self { options }
    }
}
impl LogFormatter for ConfiguredFormatter {
    fn format(&self, record: &LogRecord) -> String {
        let mut text = String::new();
        // Elapsed formatting uses explicit units and preserves the configured fixed precision.
        if let Some(precision) = self.options.elapsed {
            append_elapsed(&mut text, record, precision);
        }
        // Severity labels remain independent of event selection and theme colors.
        if self.options.level {
            let label = super::formatter::severity_label(record.severity);
            text.write_fmt(format_args!("{label} "))
                .expect("writing log presentation to a String is infallible");
        }
        // Targets precede the message and retain their captured spelling.
        if self.options.target {
            let target = &record.target;
            text.write_fmt(format_args!("{target}: "))
                .expect("writing log presentation to a String is infallible");
        }
        // Messages retain literal bytes before any optional field suffix.
        text.push_str(&record.message);
        // Field and source switches affect presentation only; their records remain retained.
        if self.options.fields {
            super::formatter::append_fields(&mut text, record);
        }
        // Source context follows fields and omits unavailable metadata.
        if self.options.source {
            append_source(&mut text, record);
        }
        text
    }
}
/// Writes elapsed seconds; subsecond accessors already convert nanoseconds to
/// their named units.
fn append_elapsed(text: &mut String, record: &LogRecord, precision: ElapsedPrecision) {
    // Whole elapsed seconds are the integer operand in every emitted profile.
    let seconds = record.elapsed.as_secs();
    match precision {
        // Whole-second output omits the retained subsecond remainder.
        ElapsedPrecision::Seconds => {
            text.write_fmt(format_args!("[{seconds}] "))
                .expect("writing log presentation to a String is infallible");
        }
        // Millisecond access truncates nanoseconds; three digits preserve this unit.
        ElapsedPrecision::Milliseconds => {
            let millis = record.elapsed.subsec_millis();
            text.write_fmt(format_args!("[{seconds}.{millis:03}] "))
                .expect("writing log presentation to a String is infallible");
        }
        // Microsecond access truncates nanoseconds; six digits preserve this unit.
        ElapsedPrecision::Microseconds => {
            let micros = record.elapsed.subsec_micros();
            text.write_fmt(format_args!("[{seconds}.{micros:06}] "))
                .expect("writing log presentation to a String is infallible");
        }
    }
}
/// Writes only available source context rather than guessing a missing line or
/// filename.
fn append_source(text: &mut String, record: &LogRecord) {
    // A missing filename contributes no source suffix.
    if let Some(file) = &record.file {
        // A captured line is appended only beside its captured filename.
        if let Some(line) = record.line {
            text.write_fmt(format_args!(" ({file}:{line})"))
                .expect("writing log presentation to a String is infallible");
        } else {
            // Filename-only metadata remains useful without inventing a line.
            text.write_fmt(format_args!(" ({file})"))
                .expect("writing log presentation to a String is infallible");
        }
    }
}
