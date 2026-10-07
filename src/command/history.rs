//! Bounded accepted-command history and reversible draft browsing.
use crate::command::{Sensitivity, SubmittedText};
use bevy::prelude::Resource;
use std::collections::VecDeque;

/// Retains accepted public command text while excluding commands marked
/// sensitive. Browsing saves the current draft and restores it after moving
/// beyond the newest retained entry.
#[derive(Debug, Resource)]
pub struct CommandHistory {
    /// Oldest to newest public input, with redacted diagnostic formatting.
    entries: VecDeque<SubmittedText>,
    /// Total retained UTF-8 input bytes.
    bytes: usize,
    /// Active browsing position and saved draft.
    cursor: HistoryCursor,
}

/// Closed browsing states keep the saved draft with its validated history
/// position.
#[derive(Debug)]
enum HistoryCursor {
    /// The editor has not begun browsing retained input.
    Draft,
    /// Saved draft was restored once and is retained until the next action.
    Restored(SubmittedText),
    /// The editor is recalling one entry and retains its original unsent draft.
    Recalling {
        /// Index into the retained entry list.
        index: usize,
        /// Draft saved before the first previous action.
        draft: SubmittedText,
    },
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            bytes: 0,
            cursor: HistoryCursor::Draft,
        }
    }
}

impl CommandHistory {
    /// Retains accepted public input within 200 entries and 512 KiB of UTF-8
    /// text. Sensitive input and entries exceeding the total byte budget are
    /// never stored.
    pub fn record(&mut self, input: &str, sensitivity: Sensitivity) {
        // Sensitive input never reaches retention, including an overlong entry.
        if sensitivity == Sensitivity::Redacted || input.len() > 512 * 1024 {
            return;
        }
        // Evict before insertion so both limits hold after every accepted write.
        while self.entries.len() >= 200 || self.bytes + input.len() > 512 * 1024 {
            if let Some(entry) = self.entries.pop_front() {
                self.bytes -= entry.0.len();
            }
        }
        self.bytes += input.len();
        self.entries.push_back(SubmittedText(input.to_owned()));
        self.cursor = HistoryCursor::Draft;
    }

    /// Borrows retained public input from oldest to newest without copying the
    /// collection. Sensitive submissions are absent because the retention
    /// boundary excludes them before storage.
    #[must_use]
    pub fn entries(&self) -> impl DoubleEndedIterator<Item = &str> {
        self.entries.iter().map(|entry| entry.0.as_str())
    }

    /// Recalls older input while saving the unsent draft on the first browsing
    /// action. At the oldest retained entry, further previous actions keep that
    /// same entry selected.
    pub fn previous(&mut self, draft: &str) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }
        // Compute the next legal position before transferring the saved draft.
        let index = match &self.cursor {
            HistoryCursor::Draft | HistoryCursor::Restored(_) => self.entries.len() - 1,
            HistoryCursor::Recalling { index, .. } => index.saturating_sub(1),
        };
        // Preserve the original draft across every step within this browsing session.
        let saved = match std::mem::replace(&mut self.cursor, HistoryCursor::Draft) {
            HistoryCursor::Draft | HistoryCursor::Restored(_) => SubmittedText(draft.to_owned()),
            HistoryCursor::Recalling { draft, .. } => draft,
        };
        self.cursor = HistoryCursor::Recalling {
            index,
            draft: saved,
        };
        self.entries.get(index).map(|entry| entry.0.as_str())
    }

    /// Recalls newer input and finally restores the original draft after the
    /// newest entry. Calling next outside a browsing session returns no
    /// replacement and leaves input unchanged.
    #[expect(
        clippy::should_implement_trait,
        reason = "History navigation returns a self-borrowed value and is not an iterator."
    )]
    pub fn next(&mut self) -> Option<&str> {
        // Move the saved draft through exactly one browsing transition.
        let cursor = std::mem::replace(&mut self.cursor, HistoryCursor::Draft);
        match cursor {
            HistoryCursor::Draft | HistoryCursor::Restored(_) => None,
            HistoryCursor::Recalling { index, draft } => {
                if index + 1 < self.entries.len() {
                    self.cursor = HistoryCursor::Recalling {
                        index: index + 1,
                        draft,
                    };
                    self.entries.get(index + 1).map(|entry| entry.0.as_str())
                } else {
                    // A distinct restored state returns the draft only once.
                    self.cursor = HistoryCursor::Restored(draft);
                    match &self.cursor {
                        HistoryCursor::Restored(draft) => Some(draft.0.as_str()),
                        HistoryCursor::Draft | HistoryCursor::Recalling { .. } => None,
                    }
                }
            }
        }
    }

    /// Ends browsing after an external edit without altering retained accepted
    /// commands.
    pub(crate) fn stop_browsing(&mut self) {
        self.cursor = HistoryCursor::Draft;
    }
}
