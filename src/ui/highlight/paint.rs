//! Glyph colors follow the editor's shaped clusters and original source ranges.
use super::{HighlightAnalysis, HighlightColors};
use crate::ui::{ConsoleRole, ConsoleTheme};
use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle, TextLayoutInfo};

/// One optional tint per extracted glyph; absent tints preserve Bevy's color.
#[derive(Debug, Default, Component)]
pub(super) struct GlyphColors(
    /// Tints in the editor's existing visual glyph order.
    pub(super) Vec<Option<LinearRgba>>,
);

/// Projects source roles onto the existing editor glyph order after layout.
pub(super) fn prepare(
    mut commands: Commands<'_, '_>,
    analysis: Res<'_, HighlightAnalysis>,
    palette: Res<'_, HighlightColors>,
    theme: Res<'_, ConsoleTheme>,
    mut editors: Query<
        '_,
        '_,
        (
            Entity,
            Ref<'_, ConsoleRole>,
            Ref<'_, EditableText>,
            Ref<'_, TextLayoutInfo>,
            Ref<'_, TextCursorStyle>,
            Option<&mut GlyphColors>,
        ),
    >,
) {
    for (entity, role, editor, layout, cursor, existing) in &mut editors {
        if *role != ConsoleRole::Input {
            continue;
        }
        // Idle input retains its tint buffer; caret, selection, shaping, and palette changes refresh it.
        let is_style_changed = analysis.is_changed() || palette.is_changed() || theme.is_changed();
        let is_editor_changed = is_layout_changed(&role, &editor, &layout, &cursor);
        if existing.is_some() && !is_style_changed && !is_editor_changed {
            continue;
        }
        // Projection reuses the existing buffer or initializes one for a replacement input.
        let is_new = existing.is_none();
        let mut fresh = GlyphColors::default();
        let target = existing.map_or(&mut fresh.0, |existing| &mut existing.into_inner().0);
        project(
            target,
            analysis.0.as_ref(),
            &editor,
            &layout,
            &cursor,
            &palette,
            &theme,
        );
        // Existing components already own their reusable buffer; new input owns the fresh one.
        if is_new {
            commands.entity(entity).insert(fresh);
        }
    }
}

/// Separates layout and editing changes from palette and analysis changes.
fn is_layout_changed(
    role: &Ref<'_, ConsoleRole>,
    editor: &Ref<'_, EditableText>,
    layout: &Ref<'_, TextLayoutInfo>,
    cursor: &Ref<'_, TextCursorStyle>,
) -> bool {
    role.is_changed() || editor.is_changed() || layout.is_changed() || cursor.is_changed()
}

/// Matches visual clusters to source roles without reshaping text.
fn project(
    colors: &mut Vec<Option<LinearRgba>>,
    analysis: Option<&crate::completion::Analysis>,
    editor: &EditableText,
    layout: &TextLayoutInfo,
    cursor: &TextCursorStyle,
    palette: &HighlightColors,
    theme: &ConsoleTheme,
) {
    colors.clear();
    // Composition and stale analysis retain the editor's rendering for the whole draft.
    let Some(analysis) = analysis
        .filter(|snapshot| !editor.is_composing() && editor.value() == snapshot.source_text())
    else {
        return;
    };
    project_roles(colors, analysis, editor, palette, theme);
    // A changed shaping correspondence must never color the wrong glyph.
    if colors.len() != layout.glyphs.len() {
        colors.clear();
        return;
    }
    preserve_editor_colors(colors, layout, cursor);
}

/// Follows the editor's visual glyph order after layout becomes available.
fn project_roles(
    colors: &mut Vec<Option<LinearRgba>>,
    analysis: &crate::completion::Analysis,
    editor: &EditableText,
    palette: &HighlightColors,
    theme: &ConsoleTheme,
) {
    // Pending editor layout cannot safely supply cluster-to-source correspondence.
    let Some(layout) = editor.editor.try_layout() else {
        return;
    };
    // Parley and Bevy traverse visual clusters identically, including bidi and font fallback.
    for line in layout.lines() {
        for run in line.runs() {
            for cluster in run.visual_clusters() {
                let tint = source_tint(analysis.highlights(), cluster.text_range(), palette, theme);
                // Editable text's rasterizer processes every shaped glyph in this order.
                colors.extend(cluster.glyphs().map(|_| tint));
            }
        }
    }
}

/// A cluster spanning multiple roles retains the original foreground color.
fn source_tint(
    spans: &[crate::highlighting::HighlightSpan],
    range: std::ops::Range<usize>,
    palette: &HighlightColors,
    theme: &ConsoleTheme,
) -> Option<LinearRgba> {
    // Ordered spans permit a lookup without scanning every token.
    let index = spans.partition_point(|span| span.source_range().end <= range.start);
    spans
        .get(index)
        .filter(|span| {
            let source = span.source_range();
            source.start <= range.start && range.end <= source.end
        })
        .map(|span| palette.color(span.role(), theme).to_linear())
}

/// Preserves colored emoji and explicit selected-text colors after role projection.
fn preserve_editor_colors(
    colors: &mut [Option<LinearRgba>],
    layout: &TextLayoutInfo,
    cursor: &TextCursorStyle,
) {
    // Selection uses Bevy's exact atlas-glyph rectangle containment rule.
    for (tint, glyph) in colors.iter_mut().zip(&layout.glyphs) {
        // True-color glyph textures retain their own foreground.
        if !glyph.atlas_info.is_alpha_mask {
            *tint = None;
            continue;
        }
        let rect = Rect::from_center_size(glyph.position, glyph.atlas_info.rect.size());
        let is_selected = cursor.selected_text_color.is_some()
            && is_fully_selected(&layout.selection_rects, rect);
        if is_selected {
            *tint = None;
        }
    }
}

/// Uses the renderer's inclusive glyph containment test for selection overrides.
fn is_fully_selected(selections: &[Rect], rect: Rect) -> bool {
    selections
        .iter()
        .any(|selection| selection.contains(rect.min) && selection.contains(rect.max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlighting::{HighlightRole, HighlightSpan};

    /// Whitespace, absent spans, and a cluster crossing roles retain the original foreground.
    #[test]
    fn only_wholly_contained_clusters_receive_token_colors() {
        let spans = [
            HighlightSpan::new(0..4, HighlightRole::Command),
            HighlightSpan::new(5..9, HighlightRole::Value),
        ];
        let theme = ConsoleTheme::default();
        let palette = HighlightColors::default();
        assert_eq!(
            source_tint(&spans, 0..4, &palette, &theme),
            Some(palette.color(HighlightRole::Command, &theme).to_linear())
        );
        assert_eq!(
            source_tint(&spans, 5..9, &palette, &theme),
            Some(theme.input.to_linear())
        );
        for range in [4..5, 4..6, 0..6, 9..10] {
            assert_eq!(source_tint(&spans, range, &palette, &theme), None);
        }
        assert_eq!(source_tint(&[], 0..1, &palette, &theme), None);
    }
}
