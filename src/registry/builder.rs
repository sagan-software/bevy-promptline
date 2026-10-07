//! Plain command definitions connect author-supplied parsing to typed Bevy
//! systems.
use super::ParseDecision;

/// Defines command metadata and typed parsing without implementing a metadata
/// trait.
/// Construction remains inert; validation and authorization precede parser
/// execution through the shared runtime.
pub struct CommandBuilder<C> {
    /// Metadata retained until the whole identity set can be checked
    /// atomically.
    pub(super) metadata: crate::CommandMetadata,
    /// Author-supplied parser invoked only after runtime validation and
    /// authorization.
    pub(super) parse: Box<Parser<C>>,
}

/// A reusable parser produces either typed handler input or explicit
/// information.
type Parser<C> = dyn Fn(&[String]) -> Result<ParseDecision<C>, String> + Send + Sync;

impl<C> CommandBuilder<C> {
    /// Defines an inert command whose parser creates typed input or
    /// information.
    /// Registration shares the normal collision checks, actor policy, dispatch
    /// order, and lifecycle notifications.
    #[must_use]
    pub fn new(
        metadata: crate::CommandMetadata,
        parse: impl Fn(&[String]) -> Result<ParseDecision<C>, String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            metadata,
            parse: Box::new(parse),
        }
    }
}
impl<C> std::fmt::Debug for CommandBuilder<C> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CommandBuilder")
            .field("metadata", &self.metadata)
            .field("parse", &"<redacted callback>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Probe counts when the builder releases its privately owned callback
    /// state.
    struct DropProbe(std::sync::Arc<std::sync::atomic::AtomicUsize>);
    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
    #[test]
    fn discarded_builder_releases_the_owned_parser_capture_once() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        let drops = Arc::new(AtomicUsize::new(0));
        let probe = DropProbe(Arc::clone(&drops));
        let builder = CommandBuilder::new(
            crate::CommandMetadata::new("probe", "Ownership fixture").unwrap(),
            move |_| {
                let _captured = &probe;
                Ok(ParseDecision::Run(()))
            },
        );
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        drop(builder);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
    }
}
