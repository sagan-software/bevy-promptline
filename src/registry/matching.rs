//! Name matching is an explicit local profile; ordinary console identities
//! remain case-sensitive.

/// Selects exact lookup or an explicitly requested ASCII-insensitive
/// command-name profile.
/// Argument spelling, submitted input, history, and Unicode characters remain
/// unchanged under either profile.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NameMatching {
    /// Exact spelling is the default and preserves ordinary case-sensitive
    /// command registration.
    /// Distinct uppercase and lowercase identities can belong to different
    /// handlers in this profile.
    #[default]
    Exact,
    /// ASCII letters compare without case while all non-ASCII bytes retain
    /// exact spelling.
    /// Enabling this profile rejects folded identity collisions before changing
    /// registry lookup behavior.
    AsciiInsensitive,
}
impl NameMatching {
    /// Compares validated identities under the selected lookup profile.
    pub(super) fn is_identity_match(
        self,
        name: &crate::CommandName,
        candidate: &crate::CommandName,
    ) -> bool {
        let name = name.as_ref();
        let candidate = candidate.as_ref();
        self.is_match(name, candidate)
    }
    /// Compares one complete spelling without normalizing either stored or
    /// submitted text.
    pub(super) fn is_match(self, name: &str, candidate: &str) -> bool {
        match self {
            Self::Exact => name == candidate,
            Self::AsciiInsensitive => name.eq_ignore_ascii_case(candidate),
        }
    }
    /// Borrows a prefix so case-insensitive discovery needs no temporary
    /// lowercase strings.
    pub(super) fn is_prefix(self, name: &str, prefix: &str) -> bool {
        name.get(..prefix.len())
            .is_some_and(|head| self.is_match(head, prefix))
    }
}
