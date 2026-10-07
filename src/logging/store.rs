//! App-thread retained records and display selection independent of capture.
use super::record::LogRecord;
use bevy::prelude::Resource;
use std::collections::VecDeque;

/// Retains bounded structured records independently of formatting and display
/// selection. Changing presentation can reuse these records without generating
/// another tracing event or duplicate capture.
#[derive(Debug, Default, Resource)]
pub struct LogStore {
    /// Successfully drained records in capture order.
    records: VecDeque<LogRecord>,
    /// Total retained approximate UTF-8 payload size.
    bytes: usize,
    /// Records evicted to maintain retention limits.
    dropped: u64,
}
impl LogStore {
    /// Borrows captured records from oldest to newest without copying their
    /// fields. Display selection and formatting can inspect these retained
    /// values independently of the capture filter.
    #[must_use]
    pub fn records(&self) -> impl DoubleEndedIterator<Item = &LogRecord> {
        self.records.iter()
    }

    /// Lazily formats selected retained records without copying the underlying
    /// collection. Changing display selection or formatting does not discard
    /// records or produce duplicate capture events.
    pub fn rows<'a>(
        &'a self,
        filter: &'a super::display::DisplayFilter,
        formatter: &'a dyn super::formatter::LogFormatter,
    ) -> impl DoubleEndedIterator<Item = String> + 'a {
        self.records
            .iter()
            .filter(move |record| {
                record.severity >= filter.minimum
                    && filter
                        .target
                        .as_ref()
                        .is_none_or(|target| *target == record.target)
            })
            .map(move |record| formatter.format(record))
            .filter(move |text| filter.query.is_empty() || text.contains(&filter.query))
    }

    /// Reports retention eviction separately from capture queue overflow or
    /// writer contention. This counter increases when old records leave storage
    /// to maintain the configured fixed budgets.
    #[must_use]
    pub const fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Removes retained records without changing the capture filter or
    /// subscriber. Future events can still arrive, and clearing storage does
    /// not reset its eviction counter.
    pub fn clear(&mut self) {
        self.records.clear();
        self.bytes = 0;
    }

    /// Retains one already bounded record while enforcing entry and byte
    /// budgets.
    pub(crate) fn push(&mut self, record: LogRecord) {
        // Measure the incoming owned payload before deciding which older records must leave.
        let bytes = record_bytes(&record);
        while self.records.len() >= 2000 || self.bytes + bytes > 2 * 1024 * 1024 {
            if let Some(old) = self.records.pop_front() {
                self.bytes -= record_bytes(&old);
                self.dropped = self.dropped.saturating_add(1);
            }
        }
        // Accounting and insertion follow eviction, preserving retained arrival order.
        self.bytes += bytes;
        self.records.push_back(record);
    }
}

/// Measures retained text payload; numeric fields contribute fixed value
/// storage.
fn record_bytes(record: &LogRecord) -> usize {
    record
        .spans
        .iter()
        .map(|span| {
            span.name.len()
                + span.target.len()
                + span
                    .fields
                    .iter()
                    .map(|(name, value)| {
                        name.len()
                            + match value {
                                super::record::FieldValue::String(text)
                                | super::record::FieldValue::Diagnostic(text) => text.len(),
                                super::record::FieldValue::Signed128(_)
                                | super::record::FieldValue::Unsigned128(_) => 16,
                                super::record::FieldValue::Boolean(_)
                                | super::record::FieldValue::Signed(_)
                                | super::record::FieldValue::Unsigned(_)
                                | super::record::FieldValue::Float(_)
                                | super::record::FieldValue::Redacted => 8,
                            }
                    })
                    .sum::<usize>()
        })
        .sum::<usize>()
        + record.thread.as_ref().map_or(0, |thread| {
            thread.id.len() + thread.name.as_ref().map_or(0, String::len)
        })
        + record.message.len()
        + record.target.len()
        + record.file.as_ref().map_or(0, String::len)
        + record
            .fields
            .iter()
            .map(|(name, value)| {
                name.len()
                    + match value {
                        super::record::FieldValue::String(text)
                        | super::record::FieldValue::Diagnostic(text) => text.len(),
                        super::record::FieldValue::Signed128(_)
                        | super::record::FieldValue::Unsigned128(_) => 16,
                        super::record::FieldValue::Boolean(_)
                        | super::record::FieldValue::Signed(_)
                        | super::record::FieldValue::Unsigned(_)
                        | super::record::FieldValue::Float(_)
                        | super::record::FieldValue::Redacted => 8,
                    }
            })
            .sum::<usize>()
}
