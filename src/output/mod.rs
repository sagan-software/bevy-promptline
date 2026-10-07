//! Bounded direct output and presentation filters.
use crate::Limit;
use bevy::prelude::Resource;
use std::collections::VecDeque;

/// Retains direct game output in arrival order on the app thread. Positive
/// entry and text limits control eviction; borrowed iterators expose the
/// retained content.
#[derive(Debug, Resource)]
pub struct Output {
    /// Oldest to newest retained entries.
    entries: VecDeque<OutputEntry>,
    /// Configured retention limits.
    limits: OutputLimits,
    /// Retained text bytes.
    bytes: usize,
    /// Next sequence identity.
    sequence: u64,
    /// Evicted-entry count.
    dropped: u64,
}

mod view;
pub use self::view::OutputView;

/// Describes output importance independently of visual color. Entries retain
/// this value with their target and text, allowing presentation filters to
/// select retained output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Trace output records detailed diagnostic activity. Presentation can
    /// distinguish this severity from ordinary output while the bounded buffer
    /// retains each entry and its text.
    Trace,
    /// Debug output records information intended for troubleshooting. Its
    /// severity remains separate from its target and text, allowing
    /// presentation to filter or style it.
    Debug,
    /// Informational output is the default for direct game writes. Its severity
    /// does not indicate a failure or change the text retained in the buffer.
    #[default]
    Info,
    /// Warning output identifies a reported problem without changing command
    /// state. Applications supply the diagnostic text; presentation can
    /// distinguish this severity without relying on color.
    Warn,
    /// Error output identifies a reported failure independently of text color.
    /// The runtime uses it for rejection diagnostics, while applications can
    /// write their own error entries.
    Error,
}

/// Stores one output entry in arrival order. Its sequence, severity, target,
/// and plain text remain available until bounded retention evicts it or output
/// is cleared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputEntry {
    /// The arrival sequence assigned by this buffer. Clearing retained output
    /// preserves the counter, so later output continues after the sequence of
    /// previously retained entries.
    pub sequence: u64,
    /// The severity supplied when the entry was written. It remains available
    /// independently of target and text, so presentation does not need to infer
    /// importance.
    pub severity: Severity,
    /// The target supplied by the writer to identify output origin. Direct
    /// informational writes use the game target, while runtime rejection
    /// diagnostics use the promptline target.
    pub target: String,
    /// Plain text retained for presentation. Output is never submitted as a
    /// command, and escape characters have no terminal control or command
    /// execution semantics here.
    pub text: String,
}

/// Configures positive limits for retained entries and text bytes. An entry
/// fits both byte limits before retention, and overflow evicts the oldest
/// retained entries.
#[derive(Debug, Clone, Copy)]
pub struct OutputLimits {
    /// The positive maximum number of entries retained by this buffer.
    /// Appending beyond this count evicts the oldest entry and increments the
    /// buffer loss counter.
    pub entries: Limit,
    /// The positive maximum total UTF-8 text bytes retained across entries.
    /// Appending output evicts oldest entries until the new text fits within
    /// this byte limit.
    pub bytes: Limit,
    /// The positive maximum UTF-8 bytes retained in one entry. Longer text
    /// truncates at a character boundary with a visible marker before entering
    /// the buffer.
    pub entry_bytes: Limit,
}

impl Default for Output {
    fn default() -> Self {
        Self::new(OutputLimits {
            entries: Limit::try_from(2_000).expect("positive constant"),
            bytes: Limit::try_from(2 * 1024 * 1024).expect("positive constant"),
            entry_bytes: Limit::try_from(16 * 1024).expect("positive constant"),
        })
    }
}
impl Output {
    /// Constructs empty scrollback with positive limits supplied by the caller.
    /// Retained bytes, arrival sequence, and loss count start at zero until
    /// entries are written.
    #[must_use]
    pub const fn new(limits: OutputLimits) -> Self {
        Self {
            entries: VecDeque::new(),
            limits,
            bytes: 0,
            sequence: 0,
            dropped: 0,
        }
    }
    /// Appends plain informational text with the game target. The buffer
    /// applies its configured truncation and eviction limits before retaining
    /// the new entry in arrival order.
    ///
    /// # Panics
    ///
    /// Panics before changing retained output when the arrival counter is `u64::MAX`.
    pub fn write(&mut self, text: impl Into<String>) {
        self.push(Severity::Info, "game", text.into());
    }
    /// Borrows retained text from oldest to newest without building an
    /// intermediate collection. Values reference this buffer and remain
    /// available while the immutable borrow lasts.
    pub fn texts(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|entry| entry.text.as_str())
    }
}

impl Output {
    /// Consumes output text for retention with its severity and target.
    /// Overlong text truncates at a UTF-8 boundary, and oldest entries are
    /// evicted when limits require it.
    ///
    /// # Panics
    ///
    /// Panics before changing retained output when the arrival counter is `u64::MAX`.
    pub fn push(&mut self, severity: Severity, target: impl Into<String>, text: String) {
        // Reserve a positive identity before truncation or eviction can change the buffer.
        let next_sequence = self
            .sequence
            .checked_add(1)
            .expect("output arrival sequence exhausted");
        let maximum = self.limits.entry_bytes.get().min(self.limits.bytes.get());
        // Truncate before eviction so the new entry can fit an otherwise empty
        // buffer.
        let text = truncate(text, maximum);
        self.evict(text.len());
        self.bytes += text.len();
        self.sequence = next_sequence;
        // Move the owned text into retained storage instead of cloning it.
        self.entries.push_back(OutputEntry {
            sequence: self.sequence,
            severity,
            target: target.into(),
            text,
        });
    }
    /// Evicts oldest entries until both retention bounds permit the incoming
    /// text.
    fn evict(&mut self, additional: usize) {
        while self.is_full(additional) {
            // Each eviction preserves byte accounting and increments the
            // visible loss count.
            if let Some(entry) = self.entries.pop_front() {
                self.bytes -= entry.text.len();
                self.dropped += 1;
            }
        }
    }
    /// Rechecks capacity for each eviction rather than caching a changing
    /// predicate.
    fn is_full(&self, additional: usize) -> bool {
        // Compare entry counts independently from the retained-text byte budget.
        let retained_entries = self.entries.len();
        let maximum_entries = self.limits.entries.get();
        // Saturation keeps overflow from weakening the byte limit.
        let incoming_bytes = self.bytes.saturating_add(additional);
        let maximum_bytes = self.limits.bytes.get();
        let is_entry_limit = retained_entries >= maximum_entries;
        let is_byte_limit = incoming_bytes > maximum_bytes;
        is_entry_limit || is_byte_limit
    }
    /// Borrows retained entries in arrival order with their sequence, severity,
    /// target, and text. The iterator supports reverse traversal without
    /// constructing a separate owned collection.
    #[must_use]
    pub fn entries(&self) -> impl DoubleEndedIterator<Item = &OutputEntry> {
        self.entries.iter()
    }
    /// Returns the number of entries evicted by retention limits since
    /// construction. Clearing output does not reset this counter or count
    /// cleared entries as overflow loss.
    #[must_use]
    pub const fn dropped(&self) -> u64 {
        self.dropped
    }
    /// Returns the UTF-8 bytes currently retained in entry text. This count
    /// changes with writes, truncation, eviction, and clearing; it excludes
    /// metadata and allocation overhead.
    #[must_use]
    pub const fn retained_bytes(&self) -> usize {
        self.bytes
    }
    /// Removes retained entries and resets retained text bytes to zero. Arrival
    /// sequence and overflow loss counters remain unchanged, allowing later
    /// output to continue their history.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }
}

/// Truncates owned text while keeping a visible marker even at a one-byte
/// limit.
fn truncate(mut text: String, maximum: usize) -> String {
    if text.len() <= maximum {
        return text;
    }
    // The ASCII marker fits tiny limits; the ellipsis fits ordinary UTF-8
    // limits.
    let marker = if maximum >= '…'.len_utf8() {
        '…'
    } else {
        '~'
    };
    let mut end = maximum - marker.len_utf8();
    // Character boundaries prevent truncation from splitting a UTF-8 encoded
    // scalar.
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text.push(marker);
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exhaustion cannot evict retained output or create a zero arrival identity.
    #[test]
    fn exhausted_sequence_panics_before_changing_retained_output() {
        let mut output = Output::new(OutputLimits {
            entries: Limit::try_from(1).unwrap(),
            bytes: Limit::try_from(8).unwrap(),
            entry_bytes: Limit::try_from(8).unwrap(),
        });
        // Seed the last representable arrival instead of executing 2^64 writes.
        output.sequence = u64::MAX - 1;
        output.write("retained");
        assert_eq!(output.entries.back().unwrap().sequence, u64::MAX);
        let previous = output.entries.clone();
        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            output.write("incoming");
        }));
        assert!(failed.is_err());
        assert_eq!(output.entries, previous);
        assert_eq!(output.bytes, 8);
        assert_eq!(output.dropped, 0);
        assert_eq!(output.sequence, u64::MAX);
    }
}
