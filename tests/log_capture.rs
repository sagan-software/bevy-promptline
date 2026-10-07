//! Layer-local capture and retained structured records through a caller-owned subscriber.
#![cfg(feature = "log")]
use bevy_promptline::logging::{
    CaptureConfig, CompactFormatter, FieldValue, LogFormatter, LogStore, capture_layer,
};
use tracing_subscriber::prelude::*;

fn emit_reload_case() {
    tracing::info!(target: "game", count = 7u64, enabled = true, password = "hidden", message = "ready");
}

#[test]
fn reload_enables_a_previously_disabled_callsite_and_preserves_typed_redacted_fields() {
    let (layer, capture) = capture_layer(CaptureConfig {
        directives: "warn".into(),
        ..Default::default()
    })
    .unwrap();
    let subscriber = tracing_subscriber::registry().with(layer);
    let mut store = LogStore::default();
    tracing::subscriber::with_default(subscriber, || {
        emit_reload_case();
        capture.drain_into(&mut store);
        assert_eq!(store.records().count(), 0);
        capture.set_filter("info,other=error").unwrap();
        emit_reload_case();
        assert!(capture.set_filter("game=impossible").is_err());
        assert!(capture.filter().unwrap().contains("other=error"));
        emit_reload_case();
    });
    capture.drain_into(&mut store);
    let records: Vec<_> = store.records().collect();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].message, "ready");
    assert_eq!(records[0].fields["count"], FieldValue::Unsigned(7));
    assert_eq!(records[0].fields["enabled"], FieldValue::Boolean(true));
    assert_eq!(records[0].fields["password"], FieldValue::Redacted);
    assert_eq!(
        CompactFormatter.format(records[0]),
        "INFO game: ready count=7 enabled=true password=<redacted>"
    );
    assert!(records[1].sequence > records[0].sequence);
}

#[test]
fn display_and_format_changes_reuse_records_without_recapture() {
    use bevy_promptline::Severity;
    use bevy_promptline::logging::{DisplayFilter, ReadableFormatter};
    let (layer, capture) = capture_layer(CaptureConfig::default()).unwrap();
    let mut store = LogStore::default();
    tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), || {
        tracing::info!(target: "game", "visible");
        tracing::warn!(target: "network", "retry");
    });
    capture.drain_into(&mut store);
    let filter = DisplayFilter {
        minimum: Severity::Warn,
        ..Default::default()
    };
    assert_eq!(
        store.rows(&filter, &CompactFormatter).collect::<Vec<_>>(),
        ["WARN network: retry"]
    );
    let readable = ReadableFormatter;
    let rows = store
        .rows(&DisplayFilter::default(), &readable)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 2);
    assert!(rows[0].starts_with('['));
    assert!(rows[0].ends_with("INFO game: visible"));
    assert_eq!(store.records().count(), 2);
}

/// Another attached sink counts events independently of the console layer-local filter.
#[derive(Clone)]
struct CountSink(std::sync::Arc<std::sync::atomic::AtomicUsize>);
impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for CountSink {
    fn on_event(&self, _: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

#[test]
fn capture_reload_never_changes_another_sink_and_upstream_rejection_remains_final() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let count = Arc::new(AtomicUsize::new(0));
    let (layer, capture) = capture_layer(CaptureConfig {
        directives: "error".into(),
        ..Default::default()
    })
    .unwrap();
    let subscriber = tracing_subscriber::registry()
        .with(layer)
        .with(CountSink(Arc::clone(&count)));
    let mut store = LogStore::default();
    tracing::subscriber::with_default(subscriber, || {
        emit_reload_case();
        capture.set_filter("info").unwrap();
        emit_reload_case();
        capture.set_filter("error").unwrap();
        emit_reload_case();
    });
    capture.drain_into(&mut store);
    assert_eq!(count.load(Ordering::Relaxed), 3);
    assert_eq!(store.records().count(), 1);
    let (layer, capture) = capture_layer(CaptureConfig::default()).unwrap();
    let subscriber = tracing_subscriber::registry()
        .with(layer)
        .with(tracing_subscriber::filter::LevelFilter::WARN);
    tracing::subscriber::with_default(subscriber, || {
        capture.set_filter("trace").unwrap();
        emit_reload_case();
    });
    let mut store = LogStore::default();
    capture.drain_into(&mut store);
    assert_eq!(store.records().count(), 0);
}

#[test]
fn stalled_drain_keeps_enqueue_bounded_and_limits_per_frame_work() {
    let (layer, capture) = capture_layer(CaptureConfig::default()).unwrap();
    tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), || {
        for index in 0..1100 {
            tracing::info!(index, "burst");
        }
    });
    assert_eq!(capture.dropped(), 76);
    let mut store = LogStore::default();
    capture.drain_into(&mut store);
    assert_eq!(store.records().count(), 64);
    for _ in 0..15 {
        capture.drain_into(&mut store);
    }
    assert_eq!(store.records().count(), 1024);
    let sequences: Vec<_> = store
        .records()
        .map(|record| record.sequence.get())
        .collect();
    assert_eq!(sequences, (1..=1024).collect::<Vec<_>>());
}

#[test]
fn invalid_initial_directives_fail_and_disconnected_reload_reports_unavailability() {
    assert!(
        capture_layer(CaptureConfig {
            directives: "game=impossible".into(),
            ..Default::default()
        })
        .is_err()
    );
    let (layer, capture) = capture_layer(CaptureConfig::default()).unwrap();
    drop(layer);
    assert!(capture.set_filter("debug").is_err());
}

#[test]
fn capture_retains_bounded_span_context_and_available_thread_metadata() {
    let (layer, capture) = capture_layer(CaptureConfig::default()).unwrap();
    let mut store = LogStore::default();
    tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), || {
        let span = tracing::info_span!(
            "request",
            request_id = 17u64,
            token = "sensitive",
            message = "scope"
        );
        let _entered = span.enter();
        span.record("request_id", 18u64);
        tracing::info!(diagnostic = ?vec![1, 2], "inside");
    });
    capture.drain_into(&mut store);
    let record = store.records().next().unwrap();
    assert_eq!(record.spans.len(), 1);
    assert_eq!(record.spans[0].name, "request");
    assert_eq!(
        record.spans[0].fields["message"],
        FieldValue::String("scope".into())
    );
    assert_eq!(
        record.spans[0].fields["request_id"],
        FieldValue::Unsigned(18)
    );
    assert_eq!(record.spans[0].fields["token"], FieldValue::Redacted);
    assert_eq!(
        record.fields["diagnostic"],
        FieldValue::Diagnostic("[1, 2]".into())
    );
    assert!(record.thread.is_some());
    let mut record = record.clone();
    record.elapsed = std::time::Duration::from_millis(1234);
    assert_eq!(
        bevy_promptline::logging::ReadableFormatter.format(&record),
        "[1.234] INFO log_capture: inside diagnostic=[1, 2]"
    );
}

#[test]
fn formatter_options_control_elapsed_precision_labels_fields_and_source() {
    use bevy_promptline::logging::{ConfiguredFormatter, ElapsedPrecision, FormatOptions};
    let (layer, capture) = capture_layer(CaptureConfig::default()).unwrap();
    let mut store = LogStore::default();
    tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), emit_reload_case);
    capture.drain_into(&mut store);
    let mut record = store.records().next().unwrap().clone();
    record.elapsed = std::time::Duration::from_micros(1_234_567);
    record.file = Some("game.rs".into());
    record.line = Some(42);
    let options = FormatOptions {
        elapsed: Some(ElapsedPrecision::Microseconds),
        level: false,
        target: false,
        fields: false,
        source: true,
    };
    assert_eq!(
        ConfiguredFormatter::new(options).format(&record),
        "[1.234567] ready (game.rs:42)"
    );
    let options = FormatOptions {
        elapsed: Some(ElapsedPrecision::Seconds),
        source: false,
        ..options
    };
    assert_eq!(
        ConfiguredFormatter::new(options).format(&record),
        "[1] ready"
    );
    let options = FormatOptions {
        elapsed: None,
        ..options
    };
    assert_eq!(ConfiguredFormatter::new(options).format(&record), "ready");
}

#[test]
fn wide_integer_fields_preserve_values_beyond_the_64_bit_range() {
    let (layer, capture) = capture_layer(CaptureConfig::default()).unwrap();
    let mut store = LogStore::default();
    tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), || {
        tracing::info!(signed = i128::MIN, unsigned = u128::MAX, "wide");
    });
    capture.drain_into(&mut store);
    let record = store.records().next().unwrap();
    assert_eq!(record.fields["signed"], FieldValue::Signed128(i128::MIN));
    assert_eq!(
        record.fields["unsigned"],
        FieldValue::Unsigned128(u128::MAX)
    );
}

#[test]
fn wide_fields_cannot_exceed_the_event_payload_budget() {
    let (layer, capture) = capture_layer(CaptureConfig::default()).unwrap();
    let mut store = LogStore::default();
    let block = "x".repeat(1024);
    let tail = "x".repeat(994);
    tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), || {
        tracing::event!(
            tracing::Level::INFO,
            a0 = block.as_str(),
            a1 = block.as_str(),
            a2 = block.as_str(),
            a3 = block.as_str(),
            a4 = block.as_str(),
            a5 = block.as_str(),
            a6 = block.as_str(),
            tail = tail.as_str(),
            wide = i128::MAX
        );
    });
    capture.drain_into(&mut store);
    let record = store.records().next().unwrap();
    let bytes: usize = record
        .fields
        .iter()
        .map(|(name, value)| {
            name.len()
                + match value {
                    FieldValue::String(text) | FieldValue::Diagnostic(text) => text.len(),
                    FieldValue::Signed128(_) | FieldValue::Unsigned128(_) => 16,
                    _ => 8,
                }
        })
        .sum();
    assert!(bytes <= 8192, "retained {bytes} field bytes");
    assert!(!record.fields.contains_key("wide"));
}

/// Field display preserves typed values and literal strings at the formatting boundary.
#[test]
fn field_display_preserves_numeric_edges_and_literal_text() {
    for (value, expected) in [
        (FieldValue::Boolean(true), "true".to_owned()),
        (FieldValue::Boolean(false), "false".to_owned()),
        (FieldValue::Signed(i64::MIN), i64::MIN.to_string()),
        (FieldValue::Signed128(i128::MIN), i128::MIN.to_string()),
        (FieldValue::Unsigned(u64::MAX), u64::MAX.to_string()),
        (FieldValue::Unsigned128(u128::MAX), u128::MAX.to_string()),
        (FieldValue::Float(0.125), "0.125".to_owned()),
        (FieldValue::Float(-0.0), "-0".to_owned()),
        (FieldValue::Float(f64::NAN), "NaN".to_owned()),
        (FieldValue::Float(f64::INFINITY), "inf".to_owned()),
        (FieldValue::Float(f64::NEG_INFINITY), "-inf".to_owned()),
        (
            FieldValue::String("literal\\n".into()),
            "literal\\n".to_owned(),
        ),
        (
            FieldValue::Diagnostic("{debug}".into()),
            "{debug}".to_owned(),
        ),
        (FieldValue::Redacted, "<redacted>".to_owned()),
    ] {
        assert_eq!(value.to_string(), expected);
    }
}

/// Capture retains a failed debug formatter's prefix and skips redacted callbacks.
#[test]
fn debug_failure_retains_prefix_without_formatting_redacted_values() {
    /// Writes a diagnostic prefix before reporting a formatting failure.
    struct FailingDebug;
    impl std::fmt::Debug for FailingDebug {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("accepted prefix")?;
            Err(std::fmt::Error)
        }
    }
    /// Fails the test if capture evaluates a redacted diagnostic.
    struct RedactedDebug;
    impl std::fmt::Debug for RedactedDebug {
        fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            panic!("redacted debug callback ran");
        }
    }
    let (layer, capture) = capture_layer(CaptureConfig::default()).unwrap();
    let mut store = LogStore::default();
    tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), || {
        tracing::info!(diagnostic = ?FailingDebug, password = ?RedactedDebug, "ready");
    });
    capture.drain_into(&mut store);
    let record = store.records().next().unwrap();
    assert_eq!(record.message, "ready");
    assert_eq!(
        record.fields["diagnostic"],
        FieldValue::Diagnostic("accepted prefix".into())
    );
    assert_eq!(record.fields["password"], FieldValue::Redacted);
}

/// Idle capture drains must preserve change detection for retained-log presentation.
#[test]
fn idle_capture_does_not_mark_retained_records_changed() {
    use bevy::prelude::*;
    use bevy_promptline::input::PromptlineSystems;
    use bevy_promptline::logging::PromptlineLogPlugin;

    /// Counts frames in which retained records require presentation updates.
    #[derive(Default, Resource)]
    struct StoreChanges {
        /// Observed resource changes, including initial construction.
        observations: usize,
    }
    /// Observes change detection through the public retained-store resource.
    fn observe(store: Res<'_, LogStore>, mut changes: ResMut<'_, StoreChanges>) {
        if store.is_changed() {
            changes.observations += 1;
        }
    }
    let (layer, capture) = capture_layer(CaptureConfig::default()).unwrap();
    let subscriber = tracing_subscriber::registry().with(layer);
    let mut app = App::new();
    app.add_plugins(PromptlineLogPlugin)
        .insert_resource(capture)
        .init_resource::<StoreChanges>()
        .add_systems(PostUpdate, observe.after(PromptlineSystems::Output));
    app.update();
    assert_eq!(app.world().resource::<StoreChanges>().observations, 1);
    app.update();
    assert_eq!(app.world().resource::<StoreChanges>().observations, 1);
    tracing::subscriber::with_default(subscriber, || tracing::info!("one retained event"));
    app.update();
    assert_eq!(app.world().resource::<StoreChanges>().observations, 2);
    assert_eq!(app.world().resource::<LogStore>().records().count(), 1);
    app.update();
    assert_eq!(app.world().resource::<StoreChanges>().observations, 2);
}
