//! Tint Bevy UI's extracted text without changing glyph, cursor, or clipping geometry.
use super::paint::GlyphColors;
use crate::ui::ConsoleRole;
use bevy::prelude::*;
use bevy::render::sync_world::{MainEntity, MainEntityHashMap};
use bevy::render::{Extract, RenderApp};
use bevy::ui_render::{ExtractedUiItem, ExtractedUiNodes, RenderUiSystems};

/// Per-input node counts recorded immediately before ordinary text extraction.
///
/// Bevy retains extracted UI nodes and rebuilds only changed entities. A rebuilt
/// entity's index map lists shadow and background nodes before ordinary glyphs,
/// so this count separates text added by the current extraction.
#[derive(Debug, Default, Resource)]
struct TextStart(
    /// Rebuilt input entities and their node counts before ordinary text extraction.
    MainEntityHashMap<usize>,
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
fn remember_start(
    extracted: Res<'_, ExtractedUiNodes>,
    mut start: ResMut<'_, TextStart>,
    inputs: Extract<'_, '_, Query<'_, '_, (Entity, &ConsoleRole), With<GlyphColors>>>,
) {
    // Counts describe only the current extraction; earlier frames' entries are stale.
    start.0.clear();
    for (entity, role) in &inputs {
        let entity = MainEntity::from(entity);
        // Retained input nodes already carry the tint applied when they were rebuilt.
        if *role != ConsoleRole::Input || !extracted.changed.contains(&entity) {
            continue;
        }
        // A newly rebuilt input may have no shadow or background nodes yet.
        let count = extracted
            .uinodes
            .get(&entity)
            .map_or(0, |(_, nodes)| nodes.len());
        start.0.insert(entity, count);
    }
}

/// Changes tint while retaining extracted glyph locations, atlas rectangles,
/// and node clips.
fn tint(
    mut extracted: ResMut<'_, ExtractedUiNodes>,
    start: Res<'_, TextStart>,
    inputs: Extract<'_, '_, Query<'_, '_, &GlyphColors>>,
) {
    for (&entity, &skip) in &start.0 {
        // A despawned input or one without a tint buffer keeps Bevy's colors.
        let Ok(colors) = inputs.get(entity.id()) else {
            continue;
        };
        // Invisible inputs produce no extracted nodes to tint.
        let Some((_, nodes)) = extracted.uinodes.get_mut(&entity) else {
            continue;
        };
        let mut tints = colors.0.iter();
        // Atlas changes split one input into several nodes without changing its glyph order.
        for node in nodes.values_mut().skip(skip) {
            // Extension decorators between glyph nodes cannot consume token tints.
            let ExtractedUiItem::Glyphs { glyphs } = &mut node.item else {
                continue;
            };
            glyphs
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
    use bevy::ecs::entity::EntityIndexMap;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::math::Affine2;
    use bevy::render::MainWorld;
    use bevy::ui_render::{ExtractedGlyph, ExtractedUiNode, NodeType};

    /// Builds one extracted glyph with a recognizable location and atlas rectangle.
    fn glyph(color: LinearRgba) -> ExtractedGlyph {
        ExtractedGlyph {
            color,
            translation: Vec2::new(2.0, 3.0),
            rect: Rect::new(0.0, 0.0, 4.0, 5.0),
        }
    }

    /// Builds one extracted node around the supplied item.
    fn node(item: ExtractedUiItem) -> ExtractedUiNode {
        ExtractedUiNode {
            z_order: 0.0,
            image: AssetId::default(),
            clip: None,
            item,
            transform: Affine2::IDENTITY,
        }
    }

    /// Borrows one extracted node's glyphs; decorator nodes have none.
    fn glyphs_at(nodes: &ExtractedUiNodes, entity: Entity, index: usize) -> &[ExtractedGlyph] {
        let (_, map) = nodes.uinodes.get(&MainEntity::from(entity)).unwrap();
        match &map.get_index(index).unwrap().1.item {
            ExtractedUiItem::Glyphs { glyphs } => glyphs,
            ExtractedUiItem::Node { .. } => &[],
        }
    }

    /// Only rebuilt input entities record their pre-text node count.
    #[test]
    fn records_counts_only_for_rebuilt_inputs() {
        let mut main = MainWorld::default();
        let rebuilt = main
            .spawn((ConsoleRole::Input, GlyphColors::default()))
            .id();
        let retained = main
            .spawn((ConsoleRole::Input, GlyphColors::default()))
            .id();
        let output = main
            .spawn((ConsoleRole::Output, GlyphColors::default()))
            .id();
        let empty = main
            .spawn((ConsoleRole::Input, GlyphColors::default()))
            .id();
        let mut world = World::new();
        let shadow = world.spawn_empty().id();
        let mut nodes = ExtractedUiNodes::default();
        let mut map = EntityIndexMap::default();
        map.insert(shadow, node(ExtractedUiItem::Glyphs { glyphs: vec![] }));
        nodes
            .uinodes
            .insert(rebuilt.into(), (Entity::PLACEHOLDER, map));
        for entity in [rebuilt, output, empty] {
            nodes.changed.insert(entity.into());
        }
        world.insert_resource(main);
        world.insert_resource(nodes);
        world.init_resource::<TextStart>();
        world
            .resource_mut::<TextStart>()
            .0
            .insert(retained.into(), 7);
        world.run_system_once(remember_start).unwrap();
        let start = &world.resource::<TextStart>().0;
        assert_eq!(start.len(), 2);
        assert_eq!(start.get(&MainEntity::from(rebuilt)), Some(&1));
        assert_eq!(start.get(&MainEntity::from(empty)), Some(&0));
    }

    /// Shadows, extension decorators, and absent tints cannot consume or change token tints.
    #[test]
    fn skips_shadows_and_decorators_before_valid_text() {
        let mut main = MainWorld::default();
        let input = main
            .spawn((
                ConsoleRole::Input,
                GlyphColors(vec![Some(LinearRgba::WHITE), None]),
            ))
            .id();
        let missing = main.spawn(ConsoleRole::Input).id();
        let absent = main
            .spawn((
                ConsoleRole::Input,
                GlyphColors(vec![Some(LinearRgba::WHITE)]),
            ))
            .id();
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
        let mut world = World::new();
        let mut map = EntityIndexMap::default();
        let red = LinearRgba::RED;
        map.insert(
            world.spawn_empty().id(),
            node(ExtractedUiItem::Glyphs {
                glyphs: vec![glyph(red)],
            }),
        );
        map.insert(world.spawn_empty().id(), node(decorator));
        map.insert(
            world.spawn_empty().id(),
            node(ExtractedUiItem::Glyphs {
                glyphs: vec![glyph(LinearRgba::BLACK), glyph(red)],
            }),
        );
        let mut nodes = ExtractedUiNodes::default();
        nodes
            .uinodes
            .insert(input.into(), (Entity::PLACEHOLDER, map));
        world.insert_resource(main);
        world.insert_resource(nodes);
        let mut start = TextStart::default();
        start.0.insert(input.into(), 1);
        start.0.insert(missing.into(), 0);
        start.0.insert(absent.into(), 0);
        world.insert_resource(start);
        world.run_system_once(tint).unwrap();
        let nodes = world.resource::<ExtractedUiNodes>();
        assert_eq!(glyphs_at(nodes, input, 0).first().unwrap().color, red);
        let (_, map) = nodes.uinodes.get(&MainEntity::from(input)).unwrap();
        assert!(
            matches!(&map.get_index(1).unwrap().1.item, ExtractedUiItem::Node { color, .. } if *color == LinearRgba::BLACK)
        );
        assert!(glyphs_at(nodes, input, 1).is_empty());
        let glyphs = glyphs_at(nodes, input, 2);
        assert_eq!(glyphs.first().unwrap().color, LinearRgba::WHITE);
        assert_eq!(glyphs.first().unwrap().translation, Vec2::new(2.0, 3.0));
        assert_eq!(glyphs.first().unwrap().rect, Rect::new(0.0, 0.0, 4.0, 5.0));
        assert_eq!(glyphs.get(1).unwrap().color, red);
    }
}
