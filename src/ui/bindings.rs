//! Physical console keys remain independent of presentation and registered
//! command parsing.
use bevy::prelude::{KeyCode, Resource};

/// Configures physical keys for console visibility, submission, completion, and
/// history.
/// Visibility keys take priority over editing actions; Shift reverses the
/// configured completion key's direction.
#[derive(Debug, Clone, Copy, Resource)]
pub struct ConsoleBindings {
    /// Optional physical key toggles visibility while the visible open control
    /// remains available.
    /// Absence disables keyboard toggling without removing pointer access or
    /// changing actor policy.
    pub toggle: Option<KeyCode>,
    /// Physical key closes a focused console before any editing action can be
    /// submitted.
    /// Applications can replace Escape while retaining the same focus
    /// restoration and frame ownership behavior.
    pub close: KeyCode,
    /// Physical key submits the actual applied draft after upstream editor work
    /// has completed.
    /// It never submits a ghost preview and defers while composition or an
    /// earlier paste is pending.
    pub submit: KeyCode,
    /// Physical key accepts a completion step without invoking a command parser
    /// or handler.
    /// Holding either Shift key reverses cycling through the same frozen
    /// candidate set.
    pub complete: KeyCode,
    /// Physical key recalls an older accepted command while preserving the
    /// unsent draft.
    /// At the oldest retained entry, repeated actions keep that entry selected
    /// without deleting history.
    pub previous: KeyCode,
    /// Physical key recalls a newer accepted command and then restores the
    /// original unsent draft.
    /// Outside history browsing, this action leaves the actual editable input
    /// unchanged.
    pub next: KeyCode,
}
impl Default for ConsoleBindings {
    fn default() -> Self {
        Self {
            toggle: Some(KeyCode::Backquote),
            close: KeyCode::Escape,
            submit: KeyCode::Enter,
            complete: KeyCode::Tab,
            previous: KeyCode::ArrowUp,
            next: KeyCode::ArrowDown,
        }
    }
}
