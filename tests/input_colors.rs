//! Input colors preserve the upstream editor's authoritative state.
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy_promptline::input::{ConsoleRole, PromptlineUiPlugin, highlight::HighlightColors};

/// Palette updates must not alter the editable value or selection.
#[test]
fn input_palette_is_initialized_and_preserves_editor_state() {
    for renderer in [None, Some(SubApp::new())] {
        let mut app = App::new();
        if let Some(renderer) = renderer {
            app.insert_sub_app(RenderApp, renderer);
        }
        app.add_plugins(PromptlineUiPlugin);
        app.finish();
        app.update();
        let input = app
            .world_mut()
            .query::<(Entity, &ConsoleRole)>()
            .iter(app.world())
            .find_map(|(entity, role)| (*role == ConsoleRole::Input).then_some(entity))
            .unwrap();
        app.world_mut()
            .get_mut::<EditableText>(input)
            .unwrap()
            .editor
            .set_text("echo 日本語");
        let selection = *app
            .world()
            .get::<EditableText>(input)
            .unwrap()
            .editor
            .raw_selection();
        app.world_mut().resource_mut::<HighlightColors>().command = Some(Color::WHITE);
        app.update();
        app.update();
        let editor = app.world().get::<EditableText>(input).unwrap();
        assert_eq!(editor.value(), "echo 日本語");
        let current = editor.editor.raw_selection();
        assert_eq!(current.anchor().index(), selection.anchor().index());
        assert_eq!(current.focus().index(), selection.focus().index());
        assert_eq!(current.anchor().affinity(), selection.anchor().affinity());
        assert_eq!(current.focus().affinity(), selection.focus().affinity());
    }
}

use bevy::ecs::system::RunSystemOnce;
use bevy::math::Affine2;
use bevy::render::{Extract, MainWorld, RenderApp};
use bevy::text::{
    FontAtlasSet, FontCx, LayoutCx, ScaleCx, TextLayoutInfo, load_font_assets_into_font_collection,
};
use bevy::ui_render::{
    ExtractedGlyph, ExtractedUiItem, ExtractedUiNode, ExtractedUiNodes, RenderUiSystems,
};
use bevy_promptline::{
    CommandMetadata, CommandOutcome, ConsoleCommand, InvalidCommandName, PromptlineAppExt,
};

/// Parsing must remain absent from token painting, including the render extraction seam.
#[derive(Debug)]
struct Echo;
impl ConsoleCommand for Echo {
    fn metadata() -> Result<CommandMetadata, InvalidCommandName> {
        CommandMetadata::new("echo", "Echo text")
    }
    fn parse(_: &[String]) -> Result<Self, String> {
        panic!("painting must not parse")
    }
}

/// Bevy's editable-text systems supply real editor layout and atlas geometry without a GPU.
fn shaped(text: &str) -> (EditableText, TextLayoutInfo) {
    let mut world = World::new();
    let mut fonts = Assets::<Font>::default();
    fonts
        .insert(
            AssetId::default(),
            Font::from_bytes(bevy::text::DEFAULT_FONT_DATA.to_vec()),
        )
        .unwrap();
    world.insert_resource(fonts);
    world.init_resource::<Assets<Image>>();
    world.init_resource::<FontCx>();
    world.init_resource::<LayoutCx>();
    world.init_resource::<ScaleCx>();
    world.init_resource::<FontAtlasSet>();
    world.init_resource::<bevy::text::RemSize>();
    world.init_resource::<Time<Real>>();
    let entity = world
        .spawn((
            EditableText {
                allow_newlines: false,
                ..default()
            },
            TextFont::default(),
            TextLayout::no_wrap(),
            TextLayoutInfo::default(),
            ComputedNode {
                size: Vec2::new(640.0, 30.0),
                ..default()
            },
            ComputedUiRenderTargetInfo::default(),
        ))
        .id();
    world
        .get_mut::<EditableText>(entity)
        .unwrap()
        .editor
        .set_text(text);
    world
        .run_system_once(load_font_assets_into_font_collection)
        .unwrap();
    world
        .run_system_once(bevy::ui::widget::update_editable_text_styles)
        .unwrap();
    world
        .run_system_once(bevy::ui::widget::update_editable_text_layout)
        .unwrap();
    world
        .entity_mut(entity)
        .take::<(EditableText, TextLayoutInfo)>()
        .unwrap()
}

/// A glyph extraction fixture splits one input into multiple ordinary text nodes.
fn extract_fixture(
    inputs: Extract<'_, '_, Query<'_, '_, (Entity, &TextLayoutInfo), With<EditableText>>>,
    mut extracted: ResMut<'_, ExtractedUiNodes>,
) {
    for (entity, layout) in &inputs {
        for chunk in layout.glyphs.chunks(7) {
            let start = extracted.glyphs.len();
            extracted
                .glyphs
                .extend(chunk.iter().map(|glyph| ExtractedGlyph {
                    color: LinearRgba::BLACK,
                    translation: glyph.position,
                    rect: glyph.atlas_info.rect,
                }));
            let end = extracted.glyphs.len();
            extracted.uinodes.push(glyph_node(entity, start..end));
        }
    }
}

/// The extraction seam must preserve these renderer-owned transforms and clips.
fn glyph_node(entity: Entity, range: std::ops::Range<usize>) -> ExtractedUiNode {
    ExtractedUiNode {
        z_order: 0.0,
        image: AssetId::default(),
        clip: Some(Rect::new(0.0, 0.0, 40.0, 40.0)),
        extracted_camera_entity: Entity::PLACEHOLDER,
        item: ExtractedUiItem::Glyphs { range },
        main_entity: entity.into(),
        render_entity: Entity::PLACEHOLDER,
        transform: Affine2::IDENTITY,
    }
}

/// Builds a shaped input with a CPU extraction seam and three explicit token colors.
fn paint_fixture() -> (App, Entity, Vec<(Vec2, Rect)>) {
    let mut app = App::new();
    let mut render = SubApp::new();
    render
        .init_resource::<ExtractedUiNodes>()
        .configure_sets(
            ExtractSchedule,
            (
                RenderUiSystems::ExtractTextShadows,
                RenderUiSystems::ExtractText,
                RenderUiSystems::ExtractCursor,
            )
                .chain(),
        )
        .add_systems(
            ExtractSchedule,
            extract_fixture.in_set(RenderUiSystems::ExtractText),
        );
    app.insert_sub_app(RenderApp, render);
    app.add_plugins(PromptlineUiPlugin);
    app.register_command::<Echo, _>(|_: In<Echo>| CommandOutcome::Success)
        .unwrap();
    app.finish();
    app.update();
    let input = app
        .world_mut()
        .query::<(Entity, &ConsoleRole)>()
        .iter(app.world())
        .find_map(|(entity, role)| (*role == ConsoleRole::Input).then_some(entity))
        .unwrap();
    let text = "echo --size '日本語'";
    app.world_mut()
        .get_mut::<EditableText>(input)
        .unwrap()
        .editor
        .set_text(text);
    let (editor, layout) = shaped(text);
    let geometry = layout
        .glyphs
        .iter()
        .map(|glyph| (glyph.position, glyph.atlas_info.rect))
        .collect::<Vec<_>>();
    assert!(geometry.len() > 12);
    app.world_mut().entity_mut(input).insert((editor, layout));
    app.insert_resource(HighlightColors {
        command: Some(Color::srgb(1.0, 0.0, 0.0)),
        option: Some(Color::srgb(1.0, 1.0, 0.0)),
        quoted: Some(Color::srgb(0.0, 1.0, 0.0)),
        ..default()
    });
    app.update();
    (app, input, geometry)
}

/// Token tints preserve actual shaping and leave shadow glyphs untouched across node splits.
#[test]
fn extracted_input_colors_preserve_geometry_and_shadow_colors() {
    let (mut app, input, geometry) = paint_fixture();
    {
        let mut extracted = app
            .sub_app_mut(RenderApp)
            .world_mut()
            .resource_mut::<ExtractedUiNodes>();
        extracted.glyphs.push(ExtractedGlyph {
            color: LinearRgba::BLACK,
            translation: Vec2::ZERO,
            rect: Rect::default(),
        });
        extracted.uinodes.push(glyph_node(input, 0..1));
    }
    let mut main = MainWorld::default();
    std::mem::swap(&mut *main, app.world_mut());
    let render = app.sub_app_mut(RenderApp);
    render.world_mut().insert_resource(main);
    render.world_mut().run_schedule(ExtractSchedule);
    let extracted = render.world().resource::<ExtractedUiNodes>();
    assert_eq!(extracted.glyphs.first().unwrap().color, LinearRgba::BLACK);
    let glyphs = extracted.glyphs.get(1..).unwrap();
    assert_eq!(
        glyphs
            .iter()
            .map(|glyph| (glyph.translation, glyph.rect))
            .collect::<Vec<_>>(),
        geometry
    );
    let colors = glyphs.iter().map(|glyph| glyph.color).collect::<Vec<_>>();
    assert!(
        colors
            .get(..4)
            .unwrap()
            .iter()
            .all(|color| *color == Color::srgb(1.0, 0.0, 0.0).to_linear())
    );
    assert_eq!(*colors.get(4).unwrap(), LinearRgba::BLACK);
    assert!(
        colors
            .get(5..11)
            .unwrap()
            .iter()
            .all(|color| *color == Color::srgb(1.0, 1.0, 0.0).to_linear())
    );
    assert_eq!(*colors.get(11).unwrap(), LinearRgba::BLACK);
    assert!(
        colors
            .get(12..)
            .unwrap()
            .iter()
            .all(|color| *color == Color::srgb(0.0, 1.0, 0.0).to_linear())
    );
    assert!(
        extracted
            .uinodes
            .iter()
            .all(|node| node.clip == Some(Rect::new(0.0, 0.0, 40.0, 40.0))
                && node.transform == Affine2::IDENTITY)
    );
}

/// Editor and shaping exceptions preserve Bevy's original glyph colors.
#[derive(Clone, Copy)]
enum PreserveCase {
    /// Color glyphs retain their texture's own color.
    Emoji,
    /// Explicit selected-text color remains authoritative.
    Selection,
    /// An incomplete shaping correspondence cannot tint unrelated glyphs.
    Mismatch,
    /// Invalid analysis clears stale token styling.
    Invalid,
    /// A reassigned role cannot retain input-only styling.
    Role,
    /// Analysis from before a later edit cannot style that edit.
    Stale,
    /// Active composition retains its editor-owned preedit styling.
    Composition,
    /// Pending editor layout cannot supply current glyph correspondence.
    PendingLayout,
}

/// Each exceptional boundary keeps its original renderer treatment and geometry.
#[test]
fn editor_exceptions_preserve_existing_glyph_colors() {
    for case in [
        PreserveCase::Emoji,
        PreserveCase::Selection,
        PreserveCase::Mismatch,
        PreserveCase::Invalid,
        PreserveCase::Role,
        PreserveCase::Stale,
        PreserveCase::Composition,
        PreserveCase::PendingLayout,
    ] {
        let (mut app, input, _) = paint_fixture();
        match case {
            PreserveCase::Emoji => {
                app.world_mut()
                    .get_mut::<TextLayoutInfo>(input)
                    .unwrap()
                    .glyphs
                    .first_mut()
                    .unwrap()
                    .atlas_info
                    .is_alpha_mask = false;
            }
            PreserveCase::Selection => {
                let rect = {
                    let layout = app.world().get::<TextLayoutInfo>(input).unwrap();
                    let glyph = layout.glyphs.first().unwrap();
                    Rect::from_center_size(glyph.position, glyph.atlas_info.rect.size())
                };
                app.world_mut()
                    .get_mut::<TextLayoutInfo>(input)
                    .unwrap()
                    .selection_rects
                    .push(rect);
                app.world_mut()
                    .get_mut::<bevy::text::TextCursorStyle>(input)
                    .unwrap()
                    .selected_text_color = Some(Color::WHITE);
            }
            PreserveCase::Mismatch => {
                app.world_mut()
                    .get_mut::<TextLayoutInfo>(input)
                    .unwrap()
                    .glyphs
                    .pop();
            }
            PreserveCase::Invalid => app
                .world_mut()
                .get_mut::<EditableText>(input)
                .unwrap()
                .editor
                .set_text("bad\0draft"),
            PreserveCase::Role => {
                app.world_mut()
                    .entity_mut(input)
                    .insert(ConsoleRole::DragRegion);
            }
            PreserveCase::Stale => {
                app.add_systems(
                    PostUpdate,
                    (move |mut editors: Query<'_, '_, &mut EditableText>| {
                        editors
                            .get_mut(input)
                            .unwrap()
                            .editor
                            .set_text("changed after analysis");
                    })
                    .after(bevy_promptline::input::PromptlineSystems::Presentation)
                    .before(bevy::ui::UiSystems::PostLayout),
                );
            }
            PreserveCase::PendingLayout => app
                .world_mut()
                .get_mut::<EditableText>(input)
                .unwrap()
                .editor
                .set_text("echo --size '日本語'"),
            PreserveCase::Composition => {
                let mut font_cx = FontCx::default();
                let mut layout_cx = LayoutCx::default();
                app.world_mut()
                    .get_mut::<EditableText>(input)
                    .unwrap()
                    .editor
                    .driver(&mut font_cx, &mut layout_cx)
                    .set_compose("compose", None);
            }
        }
        app.update();
        let geometry = app
            .world()
            .get::<TextLayoutInfo>(input)
            .unwrap()
            .glyphs
            .iter()
            .map(|glyph| (glyph.position, glyph.atlas_info.rect))
            .collect::<Vec<_>>();
        let mut main = MainWorld::default();
        std::mem::swap(&mut *main, app.world_mut());
        let render = app.sub_app_mut(RenderApp);
        render.world_mut().insert_resource(main);
        render.world_mut().run_schedule(ExtractSchedule);
        let extracted = render.world().resource::<ExtractedUiNodes>();
        assert_eq!(
            extracted
                .glyphs
                .iter()
                .map(|glyph| (glyph.translation, glyph.rect))
                .collect::<Vec<_>>(),
            geometry
        );
        match case {
            PreserveCase::Emoji | PreserveCase::Selection => {
                assert_eq!(extracted.glyphs.first().unwrap().color, LinearRgba::BLACK);
            }
            PreserveCase::Mismatch
            | PreserveCase::Invalid
            | PreserveCase::Role
            | PreserveCase::Stale
            | PreserveCase::Composition
            | PreserveCase::PendingLayout => assert!(
                extracted
                    .glyphs
                    .iter()
                    .all(|glyph| glyph.color == LinearRgba::BLACK)
            ),
        }
    }
}
