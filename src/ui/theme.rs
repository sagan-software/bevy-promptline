//! Presentation colors and font configuration remain separate from command
//! semantics.
use super::ConsoleRole;
use bevy::prelude::*;

/// Configures the font and semantic colors of the default replaceable console
/// roles.
/// Theme changes do not mutate input, command registration, focus ownership,
/// retained output, or capture filtering.
#[derive(Debug, Clone, Resource)]
pub struct ConsoleTheme {
    /// Font configuration is shared by input, output, help, candidates, and
    /// ghost roles.
    /// Matching input and ghost shaping keeps completion placement aligned
    /// after a theme change.
    pub font: TextFont,
    /// Root background supplies the surface against which console text must
    /// remain readable.
    /// Applications choose opacity independently of visibility and can replace
    /// the root layout through public roles.
    pub background: Color,
    /// Authoritative input foreground remains distinct from non-authoritative
    /// ghost and contextual help colors.
    /// Changing this value does not modify the editable text or submit another
    /// command.
    pub input: Color,
    /// Direct output foreground applies to the default retained output
    /// presentation role.
    /// Its visual color remains independent of the severity stored with each
    /// retained entry.
    pub output: Color,
    /// Contextual help and candidate foreground identifies metadata outside the
    /// authoritative input.
    /// Applications can replace these role entities while retaining their
    /// content and permission-aware discovery.
    pub help: Color,
    /// Ghost foreground distinguishes a non-authoritative suffix from actual
    /// editable input.
    /// It uses the shared font and shaped caret geometry rather than a
    /// character-width estimate.
    pub ghost: Color,
    /// Caret color belongs to the upstream editor's actual cursor presentation.
    /// The console does not draw a second cursor or estimate its position
    /// independently.
    pub caret: Color,
    /// Selection background belongs to the upstream editor's actual selected
    /// text range.
    /// Changing its appearance does not change selection bounds or copy ghost
    /// text into the clipboard.
    pub selection: Color,
}
impl Default for ConsoleTheme {
    fn default() -> Self {
        Self {
            font: TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            background: Color::srgba(0.025, 0.035, 0.045, 0.97),
            input: Color::srgb(0.92, 0.97, 0.97),
            output: Color::srgb(0.88, 0.93, 0.94),
            help: Color::srgb(0.76, 0.88, 0.88),
            ghost: Color::srgb(0.62, 0.8, 0.78),
            caret: Color::WHITE,
            selection: Color::srgb(0.15, 0.35, 0.45),
        }
    }
}

/// Applies theme only after changes or creation of a replacement public role.
pub(super) fn apply_theme(
    theme: Res<'_, ConsoleTheme>,
    mut roles: Query<
        '_,
        '_,
        (
            Ref<'_, ConsoleRole>,
            Option<&mut TextFont>,
            Option<&mut TextColor>,
            Option<&mut BackgroundColor>,
            Option<&mut bevy::text::TextCursorStyle>,
        ),
    >,
) {
    for (role, font, color, background, cursor) in &mut roles {
        // Unchanged roles retain their components without redundant writes.
        if !theme.is_changed() && !role.is_added() {
            continue;
        }
        apply_text_theme(&theme, *role, font, color, cursor);
        // Root opacity remains independent of editor colors and ownership.
        if *role == ConsoleRole::Root
            && let Some(mut background) = background
        {
            background.0 = theme.background;
        }
    }
}

/// Updates text and cursor styling without changing the upstream editor value.
fn apply_text_theme(
    theme: &ConsoleTheme,
    role: ConsoleRole,
    font: Option<Mut<'_, TextFont>>,
    color: Option<Mut<'_, TextColor>>,
    cursor: Option<Mut<'_, bevy::text::TextCursorStyle>>,
) {
    // Input and ghost share font identity and shaped metrics.
    if let Some(mut font) = font {
        *font = theme.font.clone();
    }
    if let Some(mut color) = color {
        color.0 = match role {
            ConsoleRole::Input => theme.input,
            ConsoleRole::Output => theme.output,
            ConsoleRole::Ghost => theme.ghost,
            ConsoleRole::Root
            | ConsoleRole::Help
            | ConsoleRole::Candidates
            | ConsoleRole::OpenControl
            | ConsoleRole::DragRegion
            | ConsoleRole::CloseControl
            | ConsoleRole::ResizeControl => theme.help,
        };
    }
    // The editor continues to own cursor geometry and selection ranges.
    if let Some(mut cursor) = cursor {
        cursor.color = theme.caret;
        cursor.selection_color = theme.selection;
    }
}
