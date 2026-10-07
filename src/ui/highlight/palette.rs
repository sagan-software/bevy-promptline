//! Input token colors remain independent of completion and submission semantics.
use bevy::prelude::*;

/// Configures token colors while the upstream editor retains text and geometry.
///
/// Absent overrides select theme-dependent defaults; changing a color never
/// changes parsing, permission checks, selection, or composition state.
#[derive(Debug, Default, Clone, Copy, Resource)]
pub struct HighlightColors {
    /// Overrides permitted command-name styling without changing registry matching.
    /// An absent value uses the theme-dependent command color; denied names
    /// continue to receive the error role and diagnostic.
    pub command: Option<Color>,
    /// Overrides metadata-derived subcommand styling independently of command names.
    /// An absent value selects the default subcommand color, and accepted
    /// spelling and completion ranges remain unchanged.
    pub subcommand: Option<Color>,
    /// Overrides option styling without running argument validators during editing.
    /// An absent value selects the default option color; runtime parsing
    /// still determines whether the submitted option is valid.
    pub option: Option<Color>,
    /// Overrides supplied-value styling independently of option and quoted-token colors.
    /// An absent value uses the ordinary input foreground, and retained
    /// value text preserves its original bytes.
    pub value: Option<Color>,
    /// Overrides complete quoted-token styling while preserving lexical quote markers.
    /// An absent value selects the default quoted color; copying and
    /// submission still use only the upstream editor's text.
    pub quoted: Option<Color>,
    /// Overrides incomplete-token styling without changing its readable editing diagnostic.
    /// An absent value selects the default warning color, and typing
    /// remains separate from command submission or execution.
    pub incomplete: Option<Color>,
    /// Overrides error-token styling independently of retained output severity colors.
    /// An absent value selects the default error color; missing and
    /// denied commands retain identical styling and diagnostic behavior.
    pub error: Option<Color>,
}

use super::super::{ConsoleTheme, SeverityColors};
use crate::{Severity, highlighting::HighlightRole};
impl HighlightColors {
    /// Resolves a token role without changing source text or editor geometry.
    pub(super) fn color(&self, role: HighlightRole, theme: &ConsoleTheme) -> Color {
        let background = theme.background.to_linear();
        // Dimensionless linear RGB luminance selects the same surface profile as output.
        let is_light = background.blue.mul_add(
            0.0722,
            background.green.mul_add(0.7152, background.red * 0.2126),
        ) > 0.5;
        let (command, subcommand, option, quoted) = if is_light {
            (
                Color::srgb(0.0, 0.35, 0.25),
                Color::srgb(0.0, 0.30, 0.50),
                Color::srgb(0.42, 0.26, 0.0),
                Color::srgb(0.38, 0.20, 0.55),
            )
        } else {
            (
                Color::srgb(0.55, 0.86, 0.75),
                Color::srgb(0.50, 0.80, 0.95),
                Color::srgb(0.95, 0.80, 0.40),
                Color::srgb(0.76, 0.64, 0.95),
            )
        };
        // Input overrides stay independent of output severity overrides.
        let severity = SeverityColors::default();
        match role {
            HighlightRole::Command => self.command.unwrap_or(command),
            HighlightRole::Subcommand => self.subcommand.unwrap_or(subcommand),
            HighlightRole::Option => self.option.unwrap_or(option),
            HighlightRole::Value => self.value.unwrap_or(theme.input),
            HighlightRole::Quoted => self.quoted.unwrap_or(quoted),
            HighlightRole::Incomplete => self
                .incomplete
                .unwrap_or_else(|| severity.color(Severity::Warn, theme)),
            HighlightRole::Error => self
                .error
                .unwrap_or_else(|| severity.color(Severity::Error, theme)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every override targets its role independently; default value color follows the theme.
    #[test]
    fn overrides_and_both_surface_defaults_resolve_all_roles() {
        let overrides = HighlightColors {
            command: Some(Color::srgb(0.1, 0.0, 0.0)),
            subcommand: Some(Color::srgb(0.2, 0.0, 0.0)),
            option: Some(Color::srgb(0.3, 0.0, 0.0)),
            value: Some(Color::srgb(0.4, 0.0, 0.0)),
            quoted: Some(Color::srgb(0.5, 0.0, 0.0)),
            incomplete: Some(Color::srgb(0.6, 0.0, 0.0)),
            error: Some(Color::srgb(0.7, 0.0, 0.0)),
        };
        for background in [Color::BLACK, Color::WHITE] {
            let theme = ConsoleTheme {
                background,
                input: Color::srgb(0.3, 0.4, 0.5),
                ..default()
            };
            let defaults = HighlightColors::default();
            for (role, red) in [
                (HighlightRole::Command, 0.1),
                (HighlightRole::Subcommand, 0.2),
                (HighlightRole::Option, 0.3),
                (HighlightRole::Value, 0.4),
                (HighlightRole::Quoted, 0.5),
                (HighlightRole::Incomplete, 0.6),
                (HighlightRole::Error, 0.7),
            ] {
                assert_eq!(overrides.color(role, &theme), Color::srgb(red, 0.0, 0.0));
                assert_ne!(defaults.color(role, &theme), overrides.color(role, &theme));
            }
            assert_eq!(defaults.color(HighlightRole::Value, &theme), theme.input);
            assert_eq!(
                defaults.color(HighlightRole::Incomplete, &theme),
                SeverityColors::default().color(Severity::Warn, &theme)
            );
            assert_eq!(
                defaults.color(HighlightRole::Error, &theme),
                SeverityColors::default().color(Severity::Error, &theme)
            );
        }
    }
}
