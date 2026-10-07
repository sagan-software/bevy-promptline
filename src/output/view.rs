//! Live and reviewing modes keep output arrival counts independent from layout.
use super::Output;
use bevy::prelude::Component;
use std::num::NonZeroU64;

/// Controls whether a scrollback viewport follows new output or preserves its
/// reading position.
///
/// Review counts newly appended entries, rather than lines,
/// because one retained entry can wrap or contain several line breaks.
#[derive(Debug, Default, Component)]
pub struct OutputView {
    /// Only review retains an arrival baseline; following cannot retain stale
    /// unread state.
    mode: ViewMode,
}
/// Closed modes prevent a live viewport from carrying a review baseline.
#[derive(Debug, Default)]
enum ViewMode {
    /// New output should move the viewport to its latest visible row.
    #[default]
    Following,
    /// New output leaves the reading position intact until explicit resumption.
    Reviewing {
        /// Last arrival observed when review began; output owns this positive
        /// identity.
        newest_seen: NonZeroU64,
    },
}
impl OutputView {
    /// Reports whether the viewport follows newly appended output. A live
    /// viewport displays no unread count; reviewing remains independent from
    /// whether the console currently owns keyboard focus.
    #[must_use]
    pub const fn is_following(&self) -> bool {
        matches!(self.mode, ViewMode::Following)
    }

    /// Starts review only when output exists, retaining the current arrival
    /// identity. Empty output stays live because there is no reading position
    /// or arrival baseline to preserve.
    pub fn pause_following(&mut self, output: &Output) {
        if output.entries.is_empty() {
            return;
        }
        if let Some(newest_seen) = NonZeroU64::new(output.sequence) {
            self.mode = ViewMode::Reviewing { newest_seen };
        }
    }
    /// Resumes following and discards the previous review baseline. The default
    /// presentation moves to the latest row after layout, while replacement
    /// presentations can implement their own scroll positioning.
    pub const fn follow_latest(&mut self) {
        self.mode = ViewMode::Following;
    }

    /// Counts entries appended since review began, including entries evicted by
    /// retention. Multiline text still counts as one entry, and following or
    /// empty output reports zero instead of an inferred line count.
    #[must_use]
    pub fn unread_entries(&self, output: &Output) -> u64 {
        if output.entries.is_empty() {
            return 0;
        }
        match self.mode {
            ViewMode::Following => 0,
            ViewMode::Reviewing { newest_seen } => {
                output.sequence.saturating_sub(newest_seen.get())
            }
        }
    }
    /// Reconciles clearing and replacement before reading-position updates.
    /// Empty output or a replacement with an earlier arrival identity resumes
    /// following instead of retaining an invalid review baseline.
    pub fn synchronize(&mut self, output: &Output) {
        let is_replaced = matches!(self.mode, ViewMode::Reviewing { newest_seen } if output.sequence < newest_seen.get());
        if output.entries.is_empty() || is_replaced {
            self.follow_latest();
        }
    }
}
