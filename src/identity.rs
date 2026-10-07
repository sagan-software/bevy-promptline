//! Validated command identities and positive limits.
use std::{num::NonZeroUsize, str::FromStr};
use thiserror::Error;

/// A case-sensitive identity retaining the spelling validated at registration.
///
/// Ordinary construction uses the ASCII identifier profile. The optional Clap
/// adapter also accepts nonempty UTF-8 labels encodable by the single-line
/// command profile.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandName(String);

/// An identity failed the command name grammar. Registration returns this error
/// before adding metadata or creating a system that could execute input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("command names must start with an ASCII letter and contain only letters, digits, _ or -")]
pub struct InvalidCommandName;

impl FromStr for CommandName {
    type Err = InvalidCommandName;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut characters = value.chars();
        // Validate the first character independently from the remaining name
        // grammar.
        let has_initial_letter = characters
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic());
        let has_valid_remainder = characters
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'));
        if !has_initial_letter || !has_valid_remainder {
            return Err(InvalidCommandName);
        }
        Ok(Self(value.to_owned()))
    }
}

impl CommandName {
    /// Borrows the spelling accepted during validation. Case and character
    /// order remain unchanged, so callers can use the same spelling in command
    /// metadata.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A validated positive limit for a configured capacity. The containing
/// configuration field defines whether the count measures entries or UTF-8 text
/// bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limit(NonZeroUsize);

/// A requested limit was zero. Construct limits through `TryFrom<usize>` so
/// output retention and command acceptance always have a positive configured
/// capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("limit must be positive")]
pub struct ZeroLimit;

impl TryFrom<usize> for Limit {
    type Error = ZeroLimit;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        NonZeroUsize::new(value).map(Self).ok_or(ZeroLimit)
    }
}
impl Limit {
    /// Returns the configured count without allocating or changing it. The
    /// containing configuration field supplies its unit, and construction
    /// guarantees a nonzero value.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0.get()
    }
}

impl AsRef<str> for CommandName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for CommandName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[cfg(feature = "clap")]
impl CommandName {
    /// Validates an existing Clap label without narrowing its Unicode or
    /// whitespace spelling.
    pub(crate) fn from_clap(value: &str) -> Result<Self, InvalidCommandName> {
        if value.is_empty() || value.contains(['\0', '\n', '\r']) {
            return Err(InvalidCommandName);
        }
        Ok(Self(value.to_owned()))
    }
}

/// Describes a registration failure before adding a handler. Invalid identities
/// and name collisions remain distinct, so applications can explain why a
/// command could not register.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RegistrationError {
    /// Command metadata failed primary identity validation. Registration
    /// returns the original identity error before checking collisions or
    /// creating a system that could execute command input.
    #[error(transparent)]
    InvalidIdentity(#[from] InvalidCommandName),
    /// A primary name or alias conflicts with a registered identity or repeats
    /// within the new metadata. Registration rejects the whole command before
    /// adding its handler.
    #[error("command identity already registered: {0}")]
    Collision(String),
}
