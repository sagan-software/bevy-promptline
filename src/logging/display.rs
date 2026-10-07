//! Display selection inspects retained records independently of subscriber
//! capture.
use bevy::prelude::Resource;

/// Selects retained log rows without discarding records or changing upstream
/// capture. Target selection uses exact case-sensitive matching; text queries
/// inspect the current formatter's plain output.
#[derive(Debug, Clone, Resource)]
pub struct DisplayFilter {
    /// Lowest displayed severity keeps level selection independent of theme
    /// colors. Records below this threshold remain retained and can appear
    /// after a later display change.
    pub minimum: crate::Severity,
    /// An optional exact target restricts displayed records without modifying
    /// capture directives. Absence permits every target; an empty target
    /// matches only an actual empty target.
    pub target: Option<String>,
    /// Case-sensitive text query applies to the currently formatted retained
    /// row. An empty query permits every otherwise selected record and does not
    /// require substring matching.
    pub query: String,
}
impl Default for DisplayFilter {
    fn default() -> Self {
        Self {
            minimum: crate::Severity::Trace,
            target: None,
            query: String::new(),
        }
    }
}
