//! Output severity colors remain independent of capture and command execution.
use super::ConsoleTheme;
use crate::Severity;
use bevy::prelude::*;

/// Configures severity colors independently of retained text and log filtering.
///
/// Absent overrides choose dark or light defaults from the console background;
/// informational output uses `ConsoleTheme::output`.
#[derive(Debug, Default, Clone, Copy, Resource)]
pub struct SeverityColors {
    /// Overrides debug output without changing its retained severity or text.
    /// An absent value uses the background-dependent default rather than hiding
    /// the entry or changing capture filters.
    pub debug: Option<Color>,
    /// Overrides warning output independently of informational output.
    /// An absent value selects a readable default for the console background;
    /// the entry remains available through the bounded output resource.
    pub warn: Option<Color>,
    /// Overrides error output without changing failure categories or command
    /// lifecycle notifications. An absent value selects the background-dependent
    /// default, and retained diagnostic text remains unchanged.
    pub error: Option<Color>,
}

impl SeverityColors {
    /// Selects presentation color without changing the retained output entry.
    pub(super) fn color(&self, severity: Severity, theme: &ConsoleTheme) -> Color {
        let background = theme.background.to_linear();
        // Linear RGB luminance distinguishes light surfaces from dark surfaces.
        let luminance = background.blue.mul_add(
            0.0722,
            background.green.mul_add(0.7152, background.red * 0.2126),
        );
        let defaults = if luminance > 0.5 {
            (
                Color::srgb(0.26, 0.34, 0.47),
                Color::srgb(0.50, 0.33, 0.0),
                Color::srgb(0.63, 0.10, 0.15),
            )
        } else {
            (
                Color::srgb(0.72, 0.77, 0.86),
                Color::srgb(1.0, 0.82, 0.40),
                Color::srgb(1.0, 0.55, 0.52),
            )
        };
        // Explicit overrides remain authoritative across theme changes.
        match severity {
            Severity::Info => theme.output,
            Severity::Trace | Severity::Debug => self.debug.unwrap_or(defaults.0),
            Severity::Warn => self.warn.unwrap_or(defaults.1),
            Severity::Error => self.error.unwrap_or(defaults.2),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both backgrounds select every default, and explicit overrides remain authoritative.
    #[test]
    fn defaults_follow_background_and_overrides_preserve_info() {
        let palette = SeverityColors::default();
        for (background, expected) in [
            (
                Color::BLACK,
                [(0.72, 0.77, 0.86), (1.0, 0.82, 0.40), (1.0, 0.55, 0.52)],
            ),
            (
                Color::WHITE,
                [(0.26, 0.34, 0.47), (0.50, 0.33, 0.0), (0.63, 0.10, 0.15)],
            ),
        ] {
            let theme = ConsoleTheme {
                background,
                ..default()
            };
            for (severity, (red, green, blue)) in [Severity::Debug, Severity::Warn, Severity::Error]
                .into_iter()
                .zip(expected)
            {
                assert_eq!(
                    palette.color(severity, &theme),
                    Color::srgb(red, green, blue)
                );
            }
            assert_eq!(
                palette.color(Severity::Trace, &theme),
                palette.color(Severity::Debug, &theme)
            );
            let overrides = SeverityColors {
                debug: Some(Color::WHITE),
                warn: Some(Color::BLACK),
                error: Some(Color::WHITE),
            };
            assert_eq!(overrides.color(Severity::Debug, &theme), Color::WHITE);
            assert_eq!(overrides.color(Severity::Warn, &theme), Color::BLACK);
            assert_eq!(overrides.color(Severity::Error, &theme), Color::WHITE);
            assert_eq!(overrides.color(Severity::Info, &theme), theme.output);
        }
    }
}
