//! Tint Bevy UI's extracted text without changing glyph, cursor, or clipping geometry.
use super::paint::GlyphColors;
use crate::ui::ConsoleRole;
use bevy::prelude::*;
use bevy::render::{Extract, RenderApp};
use bevy::ui_render::{ExtractedUiItem, ExtractedUiNodes, RenderUiSystems};

/// Beginning of ordinary text extraction, after shadow glyphs have been extracted.
#[derive(Debug, Default, Resource)]
struct TextStart(
    /// Node count immediately before ordinary text extraction.
    usize,
);

/// Adds the tint pass only when the host installed Bevy's UI renderer.
pub(in crate::ui) fn install(app: &mut App) {
    // Test renderer availability after the host finishes constructing its plugins.
    let Some(render) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    if !render.world().contains_resource::<ExtractedUiNodes>() {
        return;
    }
    render.init_resource::<TextStart>().add_systems(
        ExtractSchedule,
        (
            remember_start
                .after(RenderUiSystems::ExtractTextShadows)
                .before(RenderUiSystems::ExtractText),
            tint.after(RenderUiSystems::ExtractText)
                .before(RenderUiSystems::ExtractCursor),
        ),
    );
}

/// Excludes shadow glyphs while retaining the ordinary renderer's extraction order.
fn remember_start(extracted: Res<'_, ExtractedUiNodes>, mut start: ResMut<'_, TextStart>) {
    start.0 = extracted.uinodes.len();
}

/// Changes tint while retaining extracted glyph locations, atlas rectangles,
/// and node clips.
fn tint(
    mut extracted: ResMut<'_, ExtractedUiNodes>,
    start: Res<'_, TextStart>,
    inputs: Extract<'_, '_, Query<'_, '_, (Entity, &ConsoleRole, &GlyphColors)>>,
) {
    // Borrow existing renderer buffers without creating another glyph collection.
    let ExtractedUiNodes { uinodes, glyphs } = &mut *extracted;
    for (entity, role, colors) in &inputs {
        // Only authoritative input roles opt into this pass.
        if *role != ConsoleRole::Input {
            continue;
        }
        let mut tints = colors.0.iter();
        // Atlas changes split one input into several nodes without changing its glyph order.
        for node in uinodes
            .iter()
            .skip(start.0)
            .filter(|node| node.main_entity.id() == entity)
        {
            let ExtractedUiItem::Glyphs { range } = &node.item else {
                continue;
            };
            // A malformed host-provided range must not index another node's glyphs.
            let Some(section) = glyphs.get_mut(range.clone()) else {
                continue;
            };
            section
                .iter_mut()
                .zip(tints.by_ref())
                .filter_map(|(glyph, color)| color.map(|color| (glyph, color)))
                .for_each(|(glyph, color)| glyph.color = color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::math::Affine2;
    use bevy::render::MainWorld;
    use bevy::ui_render::{ExtractedGlyph, ExtractedUiNode, NodeType};

    /// Extension decorators and invalid ranges cannot consume another node's token tint.
    #[test]
    fn ignores_decorators_and_invalid_ranges_before_valid_text() {
        let mut main = MainWorld::default();
        let input = main
            .spawn((
                ConsoleRole::Input,
                GlyphColors(vec![Some(LinearRgba::WHITE)]),
            ))
            .id();
        let make_node = |item| ExtractedUiNode {
            z_order: 0.0,
            image: AssetId::default(),
            clip: None,
            extracted_camera_entity: Entity::PLACEHOLDER,
            item,
            main_entity: input.into(),
            render_entity: Entity::PLACEHOLDER,
            transform: Affine2::IDENTITY,
        };
        let decorator = ExtractedUiItem::Node {
            color: LinearRgba::BLACK,
            rect: Rect::default(),
            atlas_scaling: None,
            flip_x: false,
            flip_y: false,
            border_radius: default(),
            border: default(),
            node_type: NodeType::Rect,
        };
        let nodes = ExtractedUiNodes {
            uinodes: vec![
                make_node(decorator),
                make_node(ExtractedUiItem::Glyphs { range: 10..11 }),
                make_node(ExtractedUiItem::Glyphs { range: 0..1 }),
            ],
            glyphs: vec![ExtractedGlyph {
                color: LinearRgba::BLACK,
                translation: Vec2::new(2.0, 3.0),
                rect: Rect::new(0.0, 0.0, 4.0, 5.0),
            }],
        };
        let mut world = World::new();
        world.insert_resource(main);
        world.insert_resource(nodes);
        world.init_resource::<TextStart>();
        world.run_system_once(tint).unwrap();
        let nodes = world.resource::<ExtractedUiNodes>();
        let glyph = nodes.glyphs.first().unwrap();
        assert_eq!(glyph.color, LinearRgba::WHITE);
        assert_eq!(glyph.translation, Vec2::new(2.0, 3.0));
        assert_eq!(glyph.rect, Rect::new(0.0, 0.0, 4.0, 5.0));
        assert!(
            matches!(&nodes.uinodes.first().unwrap().item, ExtractedUiItem::Node { color, .. } if *color == LinearRgba::BLACK)
        );
    }
}
