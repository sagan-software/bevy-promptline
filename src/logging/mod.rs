//! Optional structured tracing capture and retained presentation APIs.
//!
//! Construct the capture layer before Bevy's `LogPlugin` initializes its
//! subscriber. The layer-local `EnvFilter` narrows events allowed by the host.
//! Retained records support independent formatting and display selection
//! without recapture. Queue writers never wait for app-thread drain and report
//! contention or overflow loss.
mod capture;
mod display;
mod format_options;
mod formatter;
mod plugin;
mod record;
mod store;
pub use self::capture::{CaptureConfig, CaptureError, LogCapture, capture_layer};
pub use self::display::DisplayFilter;
pub use self::format_options::{ConfiguredFormatter, ElapsedPrecision, FormatOptions};
pub use self::formatter::{CompactFormatter, LogFormatter, LogPresentation, ReadableFormatter};
pub use self::plugin::PromptlineLogPlugin;
pub use self::record::{CaptureSequence, FieldValue, LogRecord, SpanRecord, ThreadRecord};
pub use self::store::LogStore;
