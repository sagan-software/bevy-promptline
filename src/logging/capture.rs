//! Layer-local filtering and nonblocking bounded tracing capture.
use super::record::{CaptureSequence, FieldValue, LogRecord, SpanRecord};
use super::store::LogStore;
use bevy::platform::time::Instant;
use bevy::prelude::Resource;
use std::collections::{BTreeMap, VecDeque};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use tracing_subscriber::{EnvFilter, Layer, Registry, layer::Context, reload};

/// Chooses initial capture directives and field names redacted before
/// retention. Invalid directives fail layer construction instead of silently
/// changing the requested capture policy.
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// `EnvFilter` directives narrow events already allowed by the host
    /// subscriber. An empty string permits all host events and does not read
    /// process environment variables.
    pub directives: String,
    /// Exact sensitive field names replaced before any value reaches retained
    /// storage. Defaults protect password, secret, and token fields;
    /// applications can supply their own list.
    pub redacted_fields: Vec<String>,
}
impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            directives: String::new(),
            redacted_fields: vec!["password".into(), "secret".into(), "token".into()],
        }
    }
}

/// Owns the console capture queue and its independent live reload handle.
/// Cloning this resource shares one capture destination rather than installing
/// another global subscriber.
#[derive(Clone, Resource)]
pub struct LogCapture {
    /// Bounded destination shared with the installed layer.
    queue: Arc<CaptureQueue>,
    /// Layer-local filter reload never changes another sink's policy.
    filter: reload::Handle<EnvFilter, Registry>,
}
impl std::fmt::Debug for LogCapture {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LogCapture")
            .field("dropped", &self.dropped())
            .finish_non_exhaustive()
    }
}
impl LogCapture {
    /// Validates and applies new layer-local directives while preserving
    /// malformed-update state. This cannot recover upstream rejection or
    /// compile-time removal and never changes other attached sinks.
    ///
    /// # Errors
    ///
    /// Returns invalid-directive diagnostics or a disconnected-subscriber
    /// reload error.
    pub fn set_filter(&self, directives: &str) -> Result<(), CaptureError> {
        self.filter
            .reload(capture_filter(directives)?)
            .map_err(CaptureError::Reload)
    }
    /// Returns the installed layer-local filter without reading process
    /// environment variables. These directives cannot describe or widen
    /// upstream global filters or compile-time event removal.
    ///
    /// # Errors
    ///
    /// Returns a reload error when the capture subscriber is no longer
    /// available.
    pub fn filter(&self) -> Result<String, CaptureError> {
        self.filter
            .with_current(ToString::to_string)
            .map_err(CaptureError::Reload)
    }

    /// Reports events lost because the bounded queue was full or temporarily
    /// contended. Writers never wait for the app to drain, and overflow does
    /// not emit recursive diagnostics.
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.queue.dropped.load(Ordering::Relaxed)
    }
    /// Moves at most 64 queued records into app-thread retention per call. A
    /// busy writer leaves the queue for the next call rather than blocking the
    /// app thread.
    pub fn drain_into(&self, store: &mut LogStore) {
        // Contention defers draining rather than blocking the app thread.
        let Ok(mut pending) = self.queue.pending.try_lock() else {
            return;
        };
        // The per-call limit bounds work while preserving FIFO queue order.
        for _ in 0..64 {
            let Some(record) = pending.records.pop_front() else {
                break;
            };
            store.push(record);
        }
    }
}

/// Distinguishes invalid capture directives from an unavailable installed
/// subscriber. Both errors preserve their underlying diagnostic source and
/// leave the previous valid filter unchanged.
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    /// The requested `EnvFilter` grammar failed validation before construction
    /// or reload. No queue state, retained record, or existing subscriber
    /// filter changes on this path.
    #[error(transparent)]
    Directives(#[from] tracing_subscriber::filter::ParseError),
    /// The layer-local reload handle no longer reaches an installed subscriber.
    /// Ordinary command output remains usable even when this capture
    /// integration is unavailable.
    #[error(transparent)]
    Reload(reload::Error),
}

/// Creates a layer for Bevy's custom-layer hook or a caller-owned Registry
/// subscriber. Install it during subscriber construction; an existing global
/// subscriber cannot accept it afterward.
///
/// # Errors
///
/// Returns the `EnvFilter` parse error for malformed initial directives.
pub fn capture_layer(
    config: CaptureConfig,
) -> Result<(Box<dyn Layer<Registry> + Send + Sync>, LogCapture), CaptureError> {
    // Validate the initial policy before allocating its capture destination.
    let (filter, handle) = reload::Layer::new(capture_filter(&config.directives)?);
    let queue = Arc::new(CaptureQueue {
        pending: Mutex::new(Pending {
            records: VecDeque::with_capacity(1024),
            sequence: 0,
        }),
        dropped: AtomicU64::new(0),
        started: Instant::now(),
    });
    let capture = LogCapture {
        queue: Arc::clone(&queue),
        filter: handle,
    };
    let layer = CaptureLayer {
        queue,
        redacted_fields: config.redacted_fields,
    }
    .with_filter(filter);
    Ok((Box::new(layer), capture))
}

/// An empty profile explicitly permits every event the upstream subscriber
/// emits. This builder selects capture policy rather than the default
/// `EnvFilter::from_str` level.
fn capture_filter(directives: &str) -> Result<EnvFilter, tracing_subscriber::filter::ParseError> {
    EnvFilter::builder()
        .with_default_directive(tracing_subscriber::filter::LevelFilter::TRACE.into())
        .parse(directives)
}

/// Fixed-capacity queue keeps capture independent from app-thread drain
/// progress.
#[derive(Debug)]
struct CaptureQueue {
    /// Pending records and their insertion sequence share one short insertion
    /// guard.
    pending: Mutex<Pending>,
    /// Overflow and contention count without recursive logging.
    dropped: AtomicU64,
    /// Monotonic platform-compatible capture origin.
    started: Instant,
}
/// Queue sequence cannot disagree with retained insertion order.
#[derive(Debug)]
struct Pending {
    /// Oldest pending record comes first.
    records: VecDeque<LogRecord>,
    /// Last successfully assigned identity.
    sequence: u64,
}
/// Records events after the layer-local filter accepts them.
#[derive(Debug)]
struct CaptureLayer {
    /// Shared app-selected capture destination.
    queue: Arc<CaptureQueue>,
    /// Exact names whose values never enter storage.
    redacted_fields: Vec<String>,
}
impl Layer<Registry> for CaptureLayer {
    fn on_new_span(
        &self,
        attributes: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        context: Context<'_, Registry>,
    ) {
        // Span attributes receive their own bounded, redacted snapshot.
        let mut visitor = RecordVisitor::new(&self.redacted_fields, VisitorScope::Span);
        attributes.record(&mut visitor);
        if let Some(span) = context.span(id) {
            let metadata = attributes.metadata();
            span.extensions_mut().insert(SpanRecord {
                name: bounded(metadata.name(), 64),
                target: bounded(metadata.target(), 64),
                fields: visitor.fields,
            });
        }
    }
    fn on_record(
        &self,
        id: &tracing::span::Id,
        values: &tracing::span::Record<'_>,
        context: Context<'_, Registry>,
    ) {
        // Update only context owned by this capture layer.
        if let Some(span) = context.span(id) {
            let mut extensions = span.extensions_mut();
            if let Some(record) = extensions.get_mut::<SpanRecord>() {
                let mut visitor = RecordVisitor::new(&self.redacted_fields, VisitorScope::Span);
                visitor.fields = std::mem::take(&mut record.fields);
                visitor.bytes = visitor
                    .fields
                    .iter()
                    .map(|(name, value)| name.len() + value_bytes(value))
                    .sum();
                // Existing field bytes count against the same span budget during updates.
                values.record(&mut visitor);
                record.fields = visitor.fields;
            }
        }
    }
    fn on_event(&self, event: &tracing::Event<'_>, context: Context<'_, Registry>) {
        // Capture callback borrows end before the owned record enters the queue.
        let metadata = event.metadata();
        let mut visitor = RecordVisitor::new(&self.redacted_fields, VisitorScope::Event);
        event.record(&mut visitor);
        // Writers count contention without waiting or emitting recursive diagnostics.
        let Ok(mut pending) = self.queue.pending.try_lock() else {
            self.queue.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        };
        // Full or exhausted queues drop new records without waiting for drain.
        if pending.records.len() >= 1024 || pending.sequence == u64::MAX {
            self.queue.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        }
        // Assign arrival identity only inside the successful insertion guard.
        pending.sequence += 1;
        let record = LogRecord {
            sequence: CaptureSequence::from(
                std::num::NonZeroU64::new(pending.sequence)
                    .expect("sequence incremented from zero"),
            ),
            elapsed: self.queue.started.elapsed(),
            severity: severity(*metadata.level()),
            target: bounded(metadata.target(), 256),
            message: visitor.message,
            fields: visitor.fields,
            file: metadata.file().map(|file| bounded(file, 512)),
            line: metadata.line(),
            spans: context.event_scope(event).map_or_else(Vec::new, |scope| {
                scope
                    .from_root()
                    .take(8)
                    .filter_map(|span| span.extensions().get::<SpanRecord>().cloned())
                    .collect()
            }),
            thread: {
                #[cfg(target_arch = "wasm32")]
                {
                    None
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    Some(current_thread())
                }
            },
        };
        pending.records.push_back(record);
    }
}
/// Maps the closed tracing level vocabulary without parsing display text.
const fn severity(level: tracing::Level) -> crate::Severity {
    match level {
        tracing::Level::TRACE => crate::Severity::Trace,
        tracing::Level::DEBUG => crate::Severity::Debug,
        tracing::Level::INFO => crate::Severity::Info,
        tracing::Level::WARN => crate::Severity::Warn,
        tracing::Level::ERROR => crate::Severity::Error,
    }
}
/// Measures owned field payload without formatting numeric values.
const fn value_bytes(value: &FieldValue) -> usize {
    match value {
        FieldValue::String(text) | FieldValue::Diagnostic(text) => text.len(),
        FieldValue::Signed128(_) | FieldValue::Unsigned128(_) => 16,
        FieldValue::Boolean(_)
        | FieldValue::Signed(_)
        | FieldValue::Unsigned(_)
        | FieldValue::Float(_)
        | FieldValue::Redacted => 8,
    }
}

/// Captures available native thread context with bounded names.
#[cfg(not(target_arch = "wasm32"))]
fn current_thread() -> super::record::ThreadRecord {
    // Thread identity is captured independently of event ordering.
    let thread = std::thread::current();
    let identity = thread.id();
    // Bound optional names while preserving the standard thread ID spelling.
    super::record::ThreadRecord {
        id: format!("{identity:?}"),
        name: thread.name().map(|name| bounded(name, 128)),
    }
}

/// Truncates at a UTF-8 boundary without allocating the rejected suffix.
fn bounded(text: &str, maximum: usize) -> String {
    utf8_prefix(text, maximum).to_owned()
}

/// Borrows the accepted prefix so diagnostic writes need no temporary owned
/// string.
fn utf8_prefix(text: &str, maximum: usize) -> &str {
    // Backtrack only within the candidate prefix until it ends on a UTF-8 boundary.
    let mut end = text.len().min(maximum);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    // The rejected suffix is never copied into temporary storage.
    text.get(..end).unwrap_or_default()
}

/// Consumes an already owned field string instead of copying it during
/// truncation.
fn truncate_field(mut text: String, maximum: usize) -> String {
    let length = utf8_prefix(&text, maximum).len();
    text.truncate(length);
    text
}
/// Event messages split from fields while span fields preserve their declared
/// names.
#[derive(Debug, Clone, Copy)]
enum VisitorScope {
    /// Events retain a separate message with an 8192-byte field payload budget.
    Event,
    /// Span context retains every accepted named field within 512 payload
    /// bytes.
    Span,
}

/// Visitor bounds field count and total payload before retaining event values.
struct RecordVisitor<'a> {
    /// Lexically ordered non-message fields.
    fields: BTreeMap<String, FieldValue>,
    /// Message stays separate from named fields.
    message: String,
    /// Caller-selected sensitive field names.
    redacted: &'a [String],
    /// Total retained field text and names.
    bytes: usize,
    /// Maximum retained field payload for this event or scope.
    maximum: usize,
    /// Record scope controls the reserved event message field.
    scope: VisitorScope,
}
impl<'a> RecordVisitor<'a> {
    /// Creates an empty visitor with its scope-specific retention budget.
    const fn new(redacted: &'a [String], scope: VisitorScope) -> Self {
        Self {
            fields: BTreeMap::new(),
            message: String::new(),
            redacted,
            bytes: 0,
            maximum: match scope {
                VisitorScope::Event => 8192,
                VisitorScope::Span => 512,
            },
            scope,
        }
    }
    /// Applies redaction and payload limits before storing one recorded value.
    fn insert(&mut self, field: &tracing::field::Field, value: FieldValue) {
        let name = field.name();
        // Existing names may update a full map, but oversized names cannot enter retention.
        let is_field_limit_reached = self.fields.len() >= 32;
        let is_full = is_field_limit_reached && !self.fields.contains_key(name);
        if is_full || name.len() > 128 {
            return;
        }
        // Redaction precedes both payload measurement and value storage.
        let is_redacted = self.redacted.iter().any(|redacted| redacted == name);
        let value = if is_redacted {
            FieldValue::Redacted
        } else {
            value
        };
        let (current_bytes, remaining) = self.remaining_budget(name);
        // Every scalar needs at least eight bytes; failed updates discard obsolete span values.
        if remaining < 8 {
            self.discard_field(name, current_bytes);
            return;
        }
        let value = limit_field(value, 1024.min(remaining));
        // Wide integers retain all sixteen bytes rather than narrowing to fit.
        if value_bytes(&value) > remaining {
            self.discard_field(name, current_bytes);
            return;
        }
        self.bytes = current_bytes + name.len() + value_bytes(&value);
        self.store_value(name, value);
    }

    /// Returns retained bytes after replacement and remaining payload bytes.
    fn remaining_budget(&self, name: &str) -> (usize, usize) {
        // Replaced fields release both their name and their former value payload.
        let previous = self
            .fields
            .get(name)
            .map_or(0, |value| name.len() + value_bytes(value));
        let current_bytes = self.bytes.saturating_sub(previous);
        let remaining = self.maximum.saturating_sub(current_bytes + name.len());
        (current_bytes, remaining)
    }

    /// Event messages stay separate; span fields retain their lexical map position.
    fn store_value(&mut self, name: &str, value: FieldValue) {
        // Message interpretation applies only to event scope, preserving existing numeric behavior.
        if name == "message" && matches!(self.scope, VisitorScope::Event) {
            self.message = match value {
                FieldValue::String(text) | FieldValue::Diagnostic(text) => text,
                FieldValue::Redacted => "<redacted>".into(),
                FieldValue::Boolean(_)
                | FieldValue::Signed(_)
                | FieldValue::Signed128(_)
                | FieldValue::Unsigned(_)
                | FieldValue::Unsigned128(_)
                | FieldValue::Float(_) => String::new(),
            };
        } else {
            self.fields.insert(bounded(name, 128), value);
        }
    }
    /// Drops an unrepresentable update instead of preserving an obsolete prior
    /// span value.
    fn discard_field(&mut self, name: &str, current_bytes: usize) {
        self.fields.remove(name);
        self.bytes = current_bytes;
    }
}
impl tracing::field::Visit for RecordVisitor<'_> {
    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.insert(field, FieldValue::Boolean(value));
    }
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.insert(field, FieldValue::Signed(value));
    }
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.insert(field, FieldValue::Unsigned(value));
    }
    fn record_i128(&mut self, field: &tracing::field::Field, value: i128) {
        self.insert(field, FieldValue::Signed128(value));
    }
    fn record_u128(&mut self, field: &tracing::field::Field, value: u128) {
        self.insert(field, FieldValue::Unsigned128(value));
    }
    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.insert(field, FieldValue::Float(value));
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.insert(field, FieldValue::String(bounded(value, 1024)));
    }
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        use std::fmt::Write as _;
        // Sensitive debug callbacks never run before redaction.
        let field_name = field.name();
        let is_redacted = self.redacted.iter().any(|name| name == field_name);
        if is_redacted {
            self.insert(field, FieldValue::Redacted);
            return;
        }
        let mut writer = BoundedWriter(String::new());
        // A formatting failure preserves the accepted prefix, including budget exhaustion.
        writer.write_fmt(format_args!("{value:?}")).unwrap_or(());
        // Retain the bounded owned prefix after the debug callback returns.
        self.insert(field, FieldValue::Diagnostic(writer.0));
    }
}
/// Truncates only owned textual values; scalar widths remain unchanged.
fn limit_field(value: FieldValue, maximum: usize) -> FieldValue {
    match value {
        FieldValue::String(text) => FieldValue::String(truncate_field(text, maximum)),
        FieldValue::Diagnostic(text) => FieldValue::Diagnostic(truncate_field(text, maximum)),
        value @ (FieldValue::Boolean(_)
        | FieldValue::Signed(_)
        | FieldValue::Signed128(_)
        | FieldValue::Unsigned(_)
        | FieldValue::Unsigned128(_)
        | FieldValue::Float(_)
        | FieldValue::Redacted) => value,
    }
}

/// Bounds diagnostic text to one field's 1024-byte retained payload budget.
struct BoundedWriter(String);
impl std::fmt::Write for BoundedWriter {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        // Remaining capacity measures bytes, including all earlier debug fragments.
        let remaining = 1024usize.saturating_sub(self.0.len());
        self.0.push_str(utf8_prefix(text, remaining));
        // Stop formatting once a fragment exceeds the retained byte budget.
        if text.len() > remaining {
            Err(std::fmt::Error)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_writer_bounds_utf8_and_refuses_the_rejected_suffix() {
        use std::fmt::Write as _;
        let mut writer = BoundedWriter(String::new());
        assert!(writer.write_str(&"x".repeat(1023)).is_ok());
        assert!(writer.write_str("😀suffix").is_err());
        assert_eq!(writer.0.len(), 1023);
        assert_eq!(bounded("😀suffix", 3), "");
    }

    #[test]
    fn contended_capture_finishes_before_the_app_releases_the_queue_guard() {
        use tracing_subscriber::prelude::*;
        let (layer, capture) = capture_layer(CaptureConfig::default()).unwrap();
        let dispatch = tracing::Dispatch::new(tracing_subscriber::registry().with(layer));
        let guard = capture.queue.pending.lock().unwrap();
        let (done, finished) = std::sync::mpsc::channel();
        let writer = std::thread::spawn(move || {
            tracing::dispatcher::with_default(&dispatch, || tracing::info!("contended"));
            done.send(()).unwrap();
        });
        assert!(
            finished
                .recv_timeout(std::time::Duration::from_secs(2))
                .is_ok()
        );
        assert_eq!(capture.dropped(), 1);
        drop(guard);
        writer.join().unwrap();
    }
}
