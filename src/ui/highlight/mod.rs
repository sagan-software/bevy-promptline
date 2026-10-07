//! Token colors adapt Bevy's extracted glyphs without changing editor layout.
mod paint;
mod palette;
mod render;
pub use self::palette::HighlightColors;
pub(super) use self::render::install as install_render;
use crate::completion::Analysis;
use bevy::prelude::*;

/// Retains an existing analysis snapshot only while its source and roles match.
#[derive(Debug, Default, Resource)]
struct HighlightAnalysis(
    /// Last valid snapshot whose roles and source drive glyph tints.
    Option<Analysis>,
);

/// Keeps styling change detection separate from candidates and changing help text.
pub(super) fn retain_analysis(world: &mut World, next: Analysis) {
    // Missing styling state cannot create an analysis cache.
    let Some(mut current) = world.get_resource_mut::<HighlightAnalysis>() else {
        return;
    };
    // Role and source equality preserve change detection for idle input.
    let is_changed = current
        .0
        .as_ref()
        .is_none_or(|previous| is_styling_changed(previous, &next));
    // Move the valid snapshot into the cache without copying its source or spans.
    if is_changed {
        current.0 = Some(next);
    }
}

/// Compares only the source and roles used by glyph painting.
fn is_styling_changed(previous: &Analysis, next: &Analysis) -> bool {
    previous.source_text() != next.source_text() || previous.highlights() != next.highlights()
}

/// Invalid or absent input cannot retain glyph colors from an older snapshot.
pub(super) fn clear_analysis(world: &mut World) {
    if let Some(mut current) = world.get_resource_mut::<HighlightAnalysis>()
        && current.0.is_some()
    {
        current.0 = None;
    }
}

/// Initializes only styling data; rendering remains owned by the host app.
pub(super) fn install(app: &mut App) {
    app.init_resource::<HighlightAnalysis>().add_systems(
        PostUpdate,
        paint::prepare
            .after(bevy::ui::UiSystems::PostLayout)
            .after(crate::sets::PromptlineSystems::Presentation),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A private cache cannot appear before installation; equal roles do not trigger refresh.
    #[test]
    fn cache_handles_absence_and_preserves_change_detection() {
        let mut app = App::new();
        app.add_plugins(crate::PromptlineRuntimePlugin);
        let next = crate::completion::analyze(app.world_mut(), "", 0).unwrap();
        let mut world = World::new();
        retain_analysis(&mut world, next);
        clear_analysis(&mut world);
        assert!(world.get_resource::<HighlightAnalysis>().is_none());
        world.init_resource::<HighlightAnalysis>();
        let next = crate::completion::analyze(app.world_mut(), "", 0).unwrap();
        retain_analysis(&mut world, next);
        let changed = world
            .get_resource_ref::<HighlightAnalysis>()
            .unwrap()
            .last_changed();
        world.increment_change_tick();
        let next = crate::completion::analyze(app.world_mut(), "", 0).unwrap();
        retain_analysis(&mut world, next);
        assert_eq!(
            world
                .get_resource_ref::<HighlightAnalysis>()
                .unwrap()
                .last_changed(),
            changed
        );
        let next = crate::completion::analyze(app.world_mut(), "other", 5).unwrap();
        retain_analysis(&mut world, next);
        assert_ne!(
            world
                .get_resource_ref::<HighlightAnalysis>()
                .unwrap()
                .last_changed(),
            changed
        );
        clear_analysis(&mut world);
        let cleared = world
            .get_resource_ref::<HighlightAnalysis>()
            .unwrap()
            .last_changed();
        world.increment_change_tick();
        clear_analysis(&mut world);
        assert_eq!(
            world
                .get_resource_ref::<HighlightAnalysis>()
                .unwrap()
                .last_changed(),
            cleared
        );
    }
}
