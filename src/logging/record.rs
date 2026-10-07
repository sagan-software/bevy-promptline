//! Owned bounded tracing values after their callback borrows expire.
use std::collections::BTreeMap;
use std::time::Duration;

/// Owns one event after capture so its callback borrows cannot escape. Elapsed
/// time is measured from capture initialization and does not imply wall-clock
/// time.
#[derive(Debug, Clone, PartialEq)]
pub struct LogRecord {
    /// Capture sequence orders successfully retained events independently of
    /// their elapsed times. Concurrent writers receive sequence identities only
    /// while holding the queue insertion guard.
    pub sequence: CaptureSequence,
    /// Elapsed capture time uses a duration rather than a textual timestamp. It
    /// remains available for formatting with caller-selected visibility and
    /// precision.
    pub elapsed: Duration,
    /// Event importance remains distinct from target, message, and presentation
    /// colors. Filtering can inspect this typed value without formatting the
    /// retained event.
    pub severity: crate::Severity,
    /// Tracing target identifies the event origin independently of its message
    /// text. Capture bounds this owned string before retaining it in the
    /// cross-thread queue.
    pub target: String,
    /// Event message is retained separately from other recorded fields. Missing
    /// message fields produce an empty string rather than an invented
    /// diagnostic.
    pub message: String,
    /// Other event fields retain their recorded types in lexical name order.
    /// Redacted values are replaced before storage and cannot be recovered by
    /// formatting.
    pub fields: BTreeMap<String, FieldValue>,
    /// Source filename remains absent when tracing supplied no file metadata.
    /// This optional value never substitutes the event target or guesses a
    /// module path.
    pub file: Option<String>,
    /// Source line remains absent when tracing supplied no line metadata. The
    /// number describes the captured callsite and does not establish execution
    /// order.
    pub line: Option<u32>,
    /// Available span context is retained from root toward the active leaf.
    /// Capture limits this list to eight scopes and bounds each scope's owned
    /// field payload.
    pub spans: Vec<SpanRecord>,
    /// Available native thread context identifies the capturing writer without
    /// implying causal order. Browser capture leaves this absent because native
    /// thread metadata is unavailable there.
    pub thread: Option<ThreadRecord>,
}

/// Preserves the type supplied by tracing's visitor rather than parsing
/// formatted text. Diagnostic values remain diagnostic text because their
/// original semantic type is unavailable.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    /// Boolean fields retain their original truth value without textual
    /// parsing. Formatters choose its spelling while display selection can
    /// inspect the typed value directly.
    Boolean(bool),
    /// Signed integer fields retain their original numeric value without
    /// rounding. Their type remains distinct from unsigned values even when
    /// both have equal magnitudes.
    Signed(i64),
    /// Signed 128-bit fields retain values beyond the narrower integer
    /// visitor's range.
    /// Their recorded width remains distinct instead of converting them into
    /// diagnostic strings.
    Signed128(i128),
    /// Unsigned integer fields retain their original numeric value without
    /// narrowing. They remain distinct from signed fields and sequence
    /// identities assigned by the capture queue.
    Unsigned(u64),
    /// Unsigned 128-bit fields retain their full range without narrowing or
    /// rounding.
    /// Their recorded width remains distinct from ordinary unsigned values and
    /// capture sequence identities.
    Unsigned128(u128),
    /// Floating-point fields retain the value recorded by tracing without
    /// integer conversion. Nonfinite values remain floating-point values rather
    /// than becoming invalid JSON or invented strings.
    Float(f64),
    /// String fields retain bounded literal text without interpreting command
    /// or terminal escapes. Their recorded type remains distinct from
    /// diagnostic text supplied through a Debug callback.
    String(String),
    /// Diagnostic fields retain bounded Debug output without inferring the
    /// original value's type. Capture uses a bounded formatting writer so a
    /// large diagnostic cannot allocate unlimited text.
    Diagnostic(String),
    /// A configured sensitive field was replaced before storage or formatting.
    /// No original value remains in this record, regardless of the selected
    /// formatter.
    Redacted,
}

impl std::fmt::Display for FieldValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Numeric fields use their original widths and standard display spelling.
        match self {
            Self::Boolean(value) => std::fmt::Display::fmt(value, formatter),
            Self::Signed(value) => std::fmt::Display::fmt(value, formatter),
            Self::Signed128(value) => std::fmt::Display::fmt(value, formatter),
            Self::Unsigned(value) => std::fmt::Display::fmt(value, formatter),
            Self::Unsigned128(value) => std::fmt::Display::fmt(value, formatter),
            Self::Float(value) => std::fmt::Display::fmt(value, formatter),
            // Literal and diagnostic text keep their existing bytes and redaction markers.
            Self::String(value) | Self::Diagnostic(value) => formatter.write_str(value),
            Self::Redacted => formatter.write_str("<redacted>"),
        }
    }
}

/// Owns bounded context for one tracing span without retaining subscriber
/// guards. Event capture copies its current fields so later span changes cannot
/// alter retained records.
#[derive(Debug, Clone, PartialEq)]
pub struct SpanRecord {
    /// The declared span name remains separate from its target and recorded
    /// fields. Capture preserves its spelling within a bounded UTF-8 text
    /// budget.
    pub name: String,
    /// Span target identifies context origin independently of the captured
    /// event's target. The formatter can inspect both values without
    /// substituting one for the other.
    pub target: String,
    /// Current span field values use the same typed and redacted profile as
    /// events. Subsequent span updates affect future captures while retained
    /// records remain unchanged.
    pub fields: BTreeMap<String, FieldValue>,
}

/// Owns native thread metadata when the platform supplies a current thread
/// identity. This context does not imply a stable process identity or ordering
/// between different writers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadRecord {
    /// Native thread identity uses the standard library's diagnostic spelling
    /// without parsing. It identifies the captured writer within the process
    /// and is not a durable application identifier.
    pub id: String,
    /// Optional thread name remains absent when the application did not assign
    /// one. Capture bounds supplied names before retaining them with the
    /// structured event.
    pub name: Option<String>,
}

/// Identifies a successfully queued capture without permitting the reserved
/// zero value. It orders queue insertion independently of elapsed capture time
/// or native thread metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CaptureSequence(std::num::NonZeroU64);
impl From<std::num::NonZeroU64> for CaptureSequence {
    fn from(value: std::num::NonZeroU64) -> Self {
        Self(value)
    }
}
impl CaptureSequence {
    /// Returns the numeric spelling of this nonzero capture identity without
    /// changing it. Applications can compare identities or serialize them while
    /// retaining the typed value internally.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A failing caller-owned sink must receive the standard formatting error.
    #[test]
    fn field_display_propagates_sink_errors() {
        struct Reject;
        impl std::fmt::Write for Reject {
            fn write_str(&mut self, _: &str) -> std::fmt::Result {
                Err(std::fmt::Error)
            }
        }
        for value in [
            FieldValue::Boolean(true),
            FieldValue::Signed(-1),
            FieldValue::Signed128(i128::MIN),
            FieldValue::Unsigned(1),
            FieldValue::Unsigned128(u128::MAX),
            FieldValue::Float(0.125),
            FieldValue::String("literal".into()),
            FieldValue::Diagnostic("debug".into()),
            FieldValue::Redacted,
        ] {
            assert!(std::fmt::write(&mut Reject, format_args!("{value}")).is_err());
        }
    }
}
