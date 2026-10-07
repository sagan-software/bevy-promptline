//! Output scrolling uses Bevy geometry and shaped line ranges, never
//! character-width estimates.
use super::{ConsoleRole, ConsoleTheme};
use crate::output::{Output, OutputView};
use bevy::input::mouse::MouseScrollUnit;
use bevy::prelude::*;
use bevy::text::{ComputedTextBlock, EditableText};
use std::{num::NonZeroU64, ops::Range};

/// Source ranges correspond to the last text assembled before UI layout.
#[derive(Debug, Default, Component)]
pub(super) struct OutputContent {
    /// Retained arrival identities map shaped source bytes back to stable
    /// entries.
    entries: Vec<EntryRange>,
    /// Review retains its source position rather than an independently guessed
    /// row number.
    anchor: Option<ReadingAnchor>,
    /// Replacement editors borrow this reusable source when updating their owned buffer.
    selectable_source: String,
    /// Owned span entities are reused across transcript updates.
    sections: Vec<Entity>,
}

/// A retained output section inherits font metrics from the output text root.
#[derive(Component)]
struct OutputSection;
/// A retained entry's exact UTF-8 range in the assembled transcript.
#[derive(Debug)]
struct EntryRange {
    /// Positive identity assigned by the bounded output buffer.
    sequence: NonZeroU64,
    /// Whole-entry boundaries derive from appending its unchanged UTF-8 text.
    range: Range<usize>,
}
/// Shaped source position remains stable when earlier entries are evicted.
#[derive(Debug)]
struct ReadingAnchor {
    /// Arrival identity of the entry containing the first visible shaped line.
    sequence: NonZeroU64,
    /// Relative UTF-8 byte position derived from the shaper's validated line
    /// range.
    byte: usize,
    /// Position within the shaped line, in logical viewport pixels.
    within_line: ScrollPosition,
}
/// Visible action resumes following without adding another public console role.
#[derive(Component)]
struct LatestControl;
/// Label receives the current unread entry count and theme.
#[derive(Component)]
struct LatestLabel;

/// Installs capture before transcript assembly, then positioning after actual
/// text layout.
pub(super) fn install(app: &mut App) {
    app.add_systems(
        Update,
        (capture_anchor, assemble)
            .chain()
            .after(crate::sets::PromptlineSystems::Input),
    )
    .add_systems(
        PostUpdate,
        (position, controls)
            .chain()
            .after(bevy::ui::UiSystems::PostLayout),
    );
    app.add_systems(
        PostUpdate,
        section_colors.before(bevy::ui::UiSystems::Content),
    );
}

/// Adds scrolling and its visible resume action to the output viewport.
pub(super) fn decorate(mut commands: Commands<'_, '_>, viewport: Entity) {
    commands
        .entity(viewport)
        .insert((OutputView::default(), ScrollPosition::default()))
        .observe(scroll);
    let control = commands
        .spawn((
            LatestControl,
            Button,
            IgnoreScroll(BVec2::TRUE),
            Node {
                position_type: PositionType::Absolute,
                right: px(4),
                bottom: px(4),
                display: Display::None,
                padding: px(4).all(),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            GlobalZIndex(102),
        ))
        .with_children(|parent| {
            parent.spawn((
                LatestLabel,
                Text::new("Latest"),
                TextColor(Color::WHITE),
                Pickable::IGNORE,
            ));
        })
        .observe(latest)
        .id();
    commands.entity(viewport).add_child(control);
}

/// Applies the tagged Bevy scroll convention and records explicit review
/// ownership.
fn scroll(
    mut event: On<'_, '_, Pointer<Scroll>>,
    output: Res<'_, Output>,
    mut viewports: Query<'_, '_, (&ComputedNode, &mut ScrollPosition, &mut OutputView)>,
) {
    let Ok((node, mut position, mut view)) = viewports.get_mut(event.entity) else {
        return;
    };
    let delta = Vec2::new(event.x, event.y)
        * match event.unit {
            MouseScrollUnit::Line => MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR,
            MouseScrollUnit::Pixel => 1.0,
        };
    // Nonfinite external deltas cannot corrupt the retained reading position.
    if !delta.is_finite() {
        return;
    }
    // This viewport owns the wheel action after validating its delta.
    event.propagate(false);
    let maximum = scroll_range(node);
    position.0 = (position.0 - delta).clamp(Vec2::ZERO, maximum);
    // Reaching the bottom resumes following; upward review retains the arrival baseline.
    if position.y >= maximum.y {
        view.follow_latest();
    } else if view.is_following() {
        view.pause_following(&output);
    }
}

/// Computes the scroll range in logical pixels from Bevy's physical layout
/// geometry.
fn scroll_range(node: &ComputedNode) -> Vec2 {
    // Both sizes use physical pixels; inverse scale converts their difference to logical pixels.
    (node.content_size() - node.size()).max(Vec2::ZERO) * node.inverse_scale_factor
}

/// Captures the first visible shaped line before arrival can replace its source
/// ranges.
fn capture_anchor(
    viewports: Query<'_, '_, (&OutputView, &ScrollPosition, &ComputedNode)>,
    mut texts: Query<
        '_,
        '_,
        (
            &ConsoleRole,
            &ChildOf,
            &ComputedTextBlock,
            &mut OutputContent,
        ),
    >,
) {
    // Only the output role can bind this snapshot to a scroll viewport.
    for (role, parent, block, mut content) in &mut texts {
        if *role != ConsoleRole::Output {
            continue;
        }
        let Ok((view, scroll, node)) = viewports.get(parent.parent()) else {
            continue;
        };
        // Live presentation discards any anchor from an earlier review session.
        if view.is_following() {
            content.anchor = None;
            continue;
        }
        if node.is_empty() || node.inverse_scale_factor <= 0.0 {
            continue;
        }
        // Logical scrolling divided by inverse scale gives the shaper's physical pixel coordinate.
        let top = scroll.y / node.inverse_scale_factor;
        let Some(line) = block
            .buffer()
            .lines()
            .find(|line| line.metrics().block_max_coord > top)
        else {
            continue;
        };
        let byte = line.text_range().start;
        // Retained identities resolve the shaper byte into its original output entry.
        let entry = content
            .entries
            .iter()
            .rev()
            .find(|entry| entry.range.start <= byte);
        if let Some(entry) = entry {
            let relative = byte
                .saturating_sub(entry.range.start)
                .min(entry.range.len());
            let within = line
                .metrics()
                .block_min_coord
                .mul_add(-node.inverse_scale_factor, scroll.y);
            content.anchor = Some(ReadingAnchor {
                sequence: entry.sequence,
                byte: relative,
                within_line: ScrollPosition(Vec2::new(scroll.x, within.max(0.0))),
            });
        }
    }
}

/// Reuses each role's owned text buffer only when retained output or its role
/// changes.
fn assemble(
    mut commands: Commands<'_, '_>,
    output: Res<'_, Output>,
    mut spans: Query<'_, '_, &mut TextSpan, With<OutputSection>>,
    mut texts: Query<
        '_,
        '_,
        (
            Ref<'_, ConsoleRole>,
            Entity,
            Option<&mut Text>,
            Option<&mut EditableText>,
            &mut OutputContent,
        ),
    >,
) {
    // Idle frames retain source text and metadata without reassembling scrollback.
    for (role, entity, text, editor, mut content) in &mut texts {
        let is_text_added = text.as_ref().is_some_and(DetectChanges::is_added);
        let is_editor_added = editor.as_ref().is_some_and(DetectChanges::is_added);
        let is_replaced = is_text_added || is_editor_added;
        if *role != ConsoleRole::Output
            || (!output.is_changed() && !role.is_added() && !is_replaced)
        {
            continue;
        }
        // Plain text writes directly into its existing buffer without a second transcript copy.
        if let Some(mut text) = text {
            let OutputContent {
                entries,
                selectable_source,
                ..
            } = &mut *content;
            assemble_source(&output, selectable_source, entries);
            first_section(&output, &mut text);
            rebuild_sections(
                commands.reborrow(),
                entity,
                &output,
                &mut content.sections,
                spans.reborrow(),
            );
        } else if let Some(mut editor) = editor {
            // Selectable editors own a single buffer and cannot retain text-span children.
            clear_sections(commands.reborrow(), &mut content.sections);
            // The upstream editor owns its buffer; retain capacity in its source adapter too.
            let OutputContent {
                entries,
                selectable_source,
                ..
            } = &mut *content;
            assemble_source(&output, selectable_source, entries);
            editor.editor.set_text(selectable_source);
        }
    }
}

/// Removes span children when a selectable editor replaces plain output text.
fn clear_sections(mut commands: Commands<'_, '_>, sections: &mut Vec<Entity>) {
    for entity in sections.drain(..) {
        commands.entity(entity).despawn();
    }
}

/// The root text owns the first entry; an empty transcript clears its existing buffer.
fn first_section(output: &Output, text: &mut Text) {
    if let Some(entry) = output.entries().next() {
        text.0.clone_from(&entry.text);
    } else {
        text.0.clear();
    }
}

/// Reuses section entities while preserving entry order and original UTF-8 text.
fn rebuild_sections(
    mut commands: Commands<'_, '_>,
    root: Entity,
    output: &Output,
    sections: &mut Vec<Entity>,
    mut spans: Query<'_, '_, &mut TextSpan, With<OutputSection>>,
) {
    let mut used = 0;
    // The root renders the first entry; following sections include their separator.
    for entry in output.entries().skip(1) {
        // Reuse the existing child at this display position before creating another entity.
        let entity = sections.get(used).copied().unwrap_or_else(|| {
            let entity = commands
                .spawn((OutputSection, TextSpan::default(), ChildOf(root)))
                .id();
            sections.push(entity);
            entity
        });
        // Existing buffers retain capacity; new entities receive their initial text once.
        if let Ok(mut span) = spans.get_mut(entity) {
            span.0.clear();
            span.0.push('\n');
            span.0.push_str(&entry.text);
        } else {
            let text = &entry.text;
            commands
                .entity(entity)
                .insert(TextSpan::new(format!("\n{text}")));
        }
        used += 1;
    }
    // Eviction and clearing remove obsolete display sections without changing retained output.
    for entity in sections.drain(used..) {
        commands.entity(entity).despawn();
    }
}

/// Applies severity colors after ordinary theme updates, without changing source text.
fn section_colors(
    output: Res<'_, Output>,
    palette: Res<'_, super::SeverityColors>,
    theme: Res<'_, ConsoleTheme>,
    roots: Query<
        '_,
        '_,
        (
            Entity,
            &ConsoleRole,
            &OutputContent,
            Ref<'_, TextFont>,
            Ref<'_, Text>,
        ),
        (With<Text>, Without<OutputSection>),
    >,
    mut colors: Query<'_, '_, &mut TextColor>,
    mut fonts: Query<'_, '_, (Ref<'_, OutputSection>, &mut TextFont), With<OutputSection>>,
) {
    // Each root and its retained section list follow the same output iterator order.
    for (root, role, content, font, text) in &roots {
        if *role != ConsoleRole::Output {
            continue;
        }
        // Idle frames do not traverse every retained section or rewrite its styling.
        let is_palette_changed = output.is_changed() || palette.is_changed() || theme.is_changed();
        let is_font_changed = font.is_changed() || text.is_added();
        if !is_palette_changed && !is_font_changed {
            continue;
        }
        for (entity, entry) in std::iter::once(root)
            .chain(content.sections.iter().copied())
            .zip(output.entries())
        {
            // Color and font updates are independent; both preserve source text and ownership.
            apply_section_color(
                entity,
                palette.color(entry.severity, &theme),
                colors.reborrow(),
            );
            apply_section_font(entity, &font, font.is_changed(), fonts.reborrow());
        }
    }
}

/// Changes only a section whose resolved severity color differs from its current color.
fn apply_section_color(entity: Entity, next: Color, mut colors: Query<'_, '_, &mut TextColor>) {
    if let Ok(mut color) = colors.get_mut(entity)
        && color.0 != next
    {
        color.0 = next;
    }
}

/// Shares the root's font metrics after creation or a font change.
fn apply_section_font(
    entity: Entity,
    font: &TextFont,
    is_changed: bool,
    mut fonts: Query<'_, '_, (Ref<'_, OutputSection>, &mut TextFont), With<OutputSection>>,
) {
    if let Ok((marker, mut section_font)) = fonts.get_mut(entity)
        && (marker.is_added() || is_changed)
    {
        section_font.clone_from(font);
    }
}

/// Appends exact UTF-8 entries and records their ranges for either presentation.
fn assemble_source(output: &Output, source: &mut String, entries: &mut Vec<EntryRange>) {
    // Both buffers retain capacity between arrivals.
    source.clear();
    entries.clear();
    for entry in output.entries() {
        // Output reserves positive identities before retaining entries in every build profile.
        let sequence = NonZeroU64::new(entry.sequence).expect("output assigns positive identities");
        if !entries.is_empty() {
            source.push('\n');
        }
        // Whole strings preserve every indexed UTF-8 boundary.
        let start = source.len();
        source.push_str(&entry.text);
        entries.push(EntryRange {
            sequence,
            range: start..source.len(),
        });
    }
}

/// Follows latest output or resolves the captured source anchor in the new
/// shaped layout.
fn position(
    output: Res<'_, Output>,
    texts: Query<'_, '_, (&ConsoleRole, &ChildOf, &ComputedTextBlock, &OutputContent)>,
    mut viewports: Query<'_, '_, (&ComputedNode, &mut ScrollPosition, &mut OutputView)>,
) {
    // Resolve public role hierarchy before accessing the viewport state.
    for (role, parent, block, content) in &texts {
        if *role != ConsoleRole::Output {
            continue;
        }
        let Ok((node, mut scroll, mut view)) = viewports.get_mut(parent.parent()) else {
            continue;
        };
        // Clearing or replacing output invalidates the prior review baseline.
        view.synchronize(&output);
        if node.is_empty() {
            continue;
        }
        let maximum = scroll_range(node);
        if view.is_following() {
            scroll.y = maximum.y;
        } else if let Some(anchor) = &content.anchor {
            restore_anchor(&mut scroll, node, block, content, anchor);
        }
        // Resizing can shorten either axis; keep both coordinates within actual geometry.
        scroll.0 = scroll.0.clamp(Vec2::ZERO, maximum);
    }
}

/// Eviction clamps a missing anchor to the first retained entry instead of
/// jumping to latest.
fn restore_anchor(
    scroll: &mut ScrollPosition,
    node: &ComputedNode,
    block: &ComputedTextBlock,
    content: &OutputContent,
    anchor: &ReadingAnchor,
) {
    // A missing retained identity means eviction and clamps review to history start.
    let entry = content
        .entries
        .iter()
        .find(|entry| entry.sequence == anchor.sequence);
    let Some(entry) = entry else {
        scroll.y = 0.0;
        return;
    };
    let byte = entry.range.start + anchor.byte.min(entry.range.len());
    // Shaped byte ranges and metrics remain authoritative after wrapping or font changes.
    let line = block.buffer().lines().find(|line| {
        let range = line.text_range();
        range.start <= byte && (byte < range.end || range.start == byte)
    });
    if let Some(line) = line {
        let height = line.metrics().line_height * node.inverse_scale_factor;
        scroll.y = line
            .metrics()
            .block_min_coord
            .mul_add(node.inverse_scale_factor, anchor.within_line.y.min(height));
        scroll.x = anchor.within_line.x;
    }
}

/// The visible resume control discards review without changing input focus.
fn latest(
    event: On<'_, '_, Pointer<Click>>,
    parents: Query<'_, '_, &ChildOf>,
    mut views: Query<'_, '_, &mut OutputView>,
) {
    if let Ok(parent) = parents.get(event.entity)
        && let Ok(mut view) = views.get_mut(parent.parent())
    {
        view.follow_latest();
    }
}

/// Shows arrival state only while reviewing and applies the host's current
/// theme.
fn controls(
    output: Res<'_, Output>,
    theme: Res<'_, ConsoleTheme>,
    views: Query<'_, '_, &OutputView>,
    mut buttons: Query<
        '_,
        '_,
        (Entity, &ChildOf, &mut Node, &mut BackgroundColor),
        With<LatestControl>,
    >,
    mut labels: Query<
        '_,
        '_,
        (
            Ref<'_, LatestLabel>,
            &ChildOf,
            &mut Text,
            &mut TextFont,
            &mut TextColor,
        ),
        With<LatestLabel>,
    >,
) {
    // The resume action belongs to its viewport, independently of editor focus.
    for (entity, parent, mut node, mut background) in &mut buttons {
        let Ok(view) = views.get(parent.parent()) else {
            continue;
        };
        node.display = if view.is_following() {
            Display::None
        } else {
            Display::Flex
        };
        // Theme colors apply to both the action surface and its arrival label.
        background.0 = theme.background;
        update_label(
            entity,
            view,
            &output,
            &theme,
            theme.is_changed(),
            labels.reborrow(),
        );
    }
}

/// Updates only the resume label associated with this viewport control.
fn update_label(
    entity: Entity,
    view: &OutputView,
    output: &Output,
    theme: &ConsoleTheme,
    is_theme_changed: bool,
    mut labels: Query<
        '_,
        '_,
        (
            Ref<'_, LatestLabel>,
            &ChildOf,
            &mut Text,
            &mut TextFont,
            &mut TextColor,
        ),
        With<LatestLabel>,
    >,
) {
    // Each label derives its unread count from the current arrival sequence.
    for (marker, parent, mut text, mut font, mut color) in &mut labels {
        if parent.parent() != entity {
            continue;
        }
        // Only the label in this control receives its derived arrival count.
        let unread = view.unread_entries(output);
        let label = if unread == 0 {
            "Latest".into()
        } else {
            format!("{unread} new | Latest")
        };
        // Unchanged labels preserve text change ticks and avoid layout work.
        if text.0 != label {
            text.0 = label;
        }
        // Matching font and colors preserve the host's current palette.
        if is_theme_changed || marker.is_added() {
            *font = theme.font.clone();
            color.0 = theme.help;
        }
    }
}

/// Geometry tests use Bevy's actual bundled font and Parley shaping without a GPU.
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::text::{FontCx, LayoutCx, LetterSpacing, LineHeight, TextBounds, TextPipeline};

    /// Selectable source ranges retain whole UTF-8 entries and discard cleared content.
    #[test]
    fn assembled_ranges_preserve_unicode_and_clear_with_output() {
        let mut output = Output::default();
        output.write("日本語");
        output.write("line\nrésumé");
        let mut source = String::new();
        let mut entries = Vec::new();
        assemble_source(&output, &mut source, &mut entries);
        assert_eq!(source, "日本語\nline\nrésumé");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].range, 0..9);
        assert_eq!(entries[1].range, 10..23);
        assert_eq!(&source[entries[1].range.clone()], "line\nrésumé");
        let capacity = source.capacity();
        output.clear();
        assemble_source(&output, &mut source, &mut entries);
        assert!(source.is_empty());
        assert!(entries.is_empty());
        assert_eq!(source.capacity(), capacity);
    }

    /// Capture binds actual shaped bytes to entry identity in logical viewport pixels.
    #[test]
    fn capture_records_unicode_entry_and_discards_anchor_when_following() {
        let mut output = Output::default();
        output.write("日本語");
        output.write("second\nthird");
        let mut source = String::new();
        let mut entries = Vec::new();
        assemble_source(&output, &mut source, &mut entries);
        let block = shape(&source, 400.0);
        let second_top = block
            .buffer()
            .lines()
            .nth(1)
            .unwrap()
            .metrics()
            .block_min_coord;
        let mut view = OutputView::default();
        view.pause_following(&output);

        // Half-scale geometry places the viewport three logical pixels inside entry two.
        let mut app = App::new();
        app.insert_resource(output)
            .add_systems(Update, (capture_anchor, position).chain());
        let viewport = app
            .world_mut()
            .spawn((
                view,
                ScrollPosition(Vec2::new(4.0, second_top * 0.5 + 3.0)),
                ComputedNode {
                    size: Vec2::new(400.0, 40.0),
                    content_size: Vec2::new(400.0, 200.0),
                    inverse_scale_factor: 0.5,
                    ..default()
                },
            ))
            .id();
        let text = app
            .world_mut()
            .spawn((
                ConsoleRole::Output,
                ChildOf(viewport),
                block,
                OutputContent {
                    entries,
                    ..default()
                },
            ))
            .id();
        app.update();
        let anchor = app
            .world()
            .get::<OutputContent>(text)
            .unwrap()
            .anchor
            .as_ref()
            .unwrap();
        assert_eq!(anchor.sequence.get(), 2);
        assert_eq!(anchor.byte, 0);
        assert_eq!(anchor.within_line.x.to_bits(), 4.0_f32.to_bits());
        assert!((anchor.within_line.y - 3.0).abs() < 0.01);
        assert!(
            (app.world().get::<ScrollPosition>(viewport).unwrap().y - (second_top * 0.5 + 3.0))
                .abs()
                < 0.01
        );

        // Resuming follow clears the source snapshot rather than retaining stale review state.
        app.world_mut()
            .get_mut::<OutputView>(viewport)
            .unwrap()
            .follow_latest();
        app.update();
        assert!(
            app.world()
                .get::<OutputContent>(text)
                .unwrap()
                .anchor
                .is_none()
        );
        // Non-output roles and missing viewport parents cannot reposition the real viewport.
        *app.world_mut().get_mut::<ConsoleRole>(text).unwrap() = ConsoleRole::Help;
        app.update();
        let unrelated = app.world_mut().spawn_empty().id();
        *app.world_mut().get_mut::<ConsoleRole>(text).unwrap() = ConsoleRole::Output;
        app.world_mut().entity_mut(text).insert(ChildOf(unrelated));
        app.update();
        assert!((app.world().get::<ScrollPosition>(viewport).unwrap().y - 80.0).abs() < 0.01);
    }

    /// Unavailable layout and hierarchy never fabricate a source position.
    #[test]
    fn capture_ignores_unavailable_geometry_and_unmatched_roles() {
        let mut output = Output::default();
        output.write("entry");
        let mut view = OutputView::default();
        view.pause_following(&output);
        let mut app = App::new();
        app.add_systems(Update, capture_anchor);
        let viewport = app
            .world_mut()
            .spawn((view, ScrollPosition::default(), ComputedNode::default()))
            .id();
        let text = app
            .world_mut()
            .spawn((
                ConsoleRole::Output,
                ChildOf(viewport),
                shape("entry", 400.0),
                OutputContent::default(),
            ))
            .id();

        // Empty geometry cannot establish an anchor even with a shaped line.
        app.update();
        assert!(
            app.world()
                .get::<OutputContent>(text)
                .unwrap()
                .anchor
                .is_none()
        );
        *app.world_mut().get_mut::<ComputedNode>(viewport).unwrap() = ComputedNode {
            size: Vec2::new(400.0, 40.0),
            inverse_scale_factor: 0.0,
            ..default()
        };
        app.update();
        assert!(
            app.world()
                .get::<OutputContent>(text)
                .unwrap()
                .anchor
                .is_none()
        );

        // Missing source ranges leave review unanchored instead of inventing an identity.
        app.world_mut()
            .get_mut::<ComputedNode>(viewport)
            .unwrap()
            .inverse_scale_factor = 1.0;
        app.update();
        assert!(
            app.world()
                .get::<OutputContent>(text)
                .unwrap()
                .anchor
                .is_none()
        );
        app.world_mut()
            .get_mut::<ScrollPosition>(viewport)
            .unwrap()
            .y = 100_000.0;
        app.update();
        assert!(
            app.world()
                .get::<OutputContent>(text)
                .unwrap()
                .anchor
                .is_none()
        );

        // Other roles and a parent without viewport state do not acquire review ownership.
        *app.world_mut().get_mut::<ConsoleRole>(text).unwrap() = ConsoleRole::Help;
        app.update();
        assert!(
            app.world()
                .get::<OutputContent>(text)
                .unwrap()
                .anchor
                .is_none()
        );
        *app.world_mut().get_mut::<ConsoleRole>(text).unwrap() = ConsoleRole::Output;
        app.world_mut().entity_mut(viewport).remove::<OutputView>();
        app.update();
        assert!(
            app.world()
                .get::<OutputContent>(text)
                .unwrap()
                .anchor
                .is_none()
        );
    }

    /// Hidden labels use the host theme without changing text while idle.
    #[test]
    fn latest_label_tracks_theme_and_retains_idle_text_ticks() {
        let mut app = App::new();
        app.add_plugins(super::super::PromptlineUiPlugin);
        app.world_mut().resource_mut::<Output>().write("old");
        app.update();
        let label = app
            .world_mut()
            .query_filtered::<Entity, With<LatestLabel>>()
            .iter(app.world())
            .next()
            .unwrap();
        assert_eq!(
            app.world().get::<TextColor>(label).unwrap().0,
            app.world().resource::<ConsoleTheme>().help
        );
        assert_eq!(
            app.world().get::<TextFont>(label).unwrap().font_size,
            app.world().resource::<ConsoleTheme>().font.font_size
        );

        // Hidden controls receive later theme changes before review begins.
        {
            let mut theme = app.world_mut().resource_mut::<ConsoleTheme>();
            theme.font.font_size = FontSize::Px(27.0);
            theme.help = Color::srgb(0.2, 0.4, 0.6);
        }
        app.update();
        assert_eq!(
            app.world().get::<TextColor>(label).unwrap().0,
            Color::srgb(0.2, 0.4, 0.6)
        );
        assert_eq!(
            app.world().get::<TextFont>(label).unwrap().font_size,
            FontSize::Px(27.0)
        );
        let changed = app
            .world_mut()
            .query::<Ref<'_, Text>>()
            .get(app.world(), label)
            .unwrap()
            .last_changed();
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<Ref<'_, Text>>()
                .get(app.world(), label)
                .unwrap()
                .last_changed(),
            changed
        );

        // A label attached to a different parent is outside this control's theme ownership.
        let unrelated = app.world_mut().spawn_empty().id();
        let other_label = app
            .world_mut()
            .spawn((
                LatestLabel,
                ChildOf(unrelated),
                Text::new("host"),
                TextFont::default(),
                TextColor(Color::WHITE),
            ))
            .id();
        app.update();
        assert_eq!(app.world().get::<Text>(other_label).unwrap().0, "host");
        assert_eq!(
            app.world().get::<TextColor>(other_label).unwrap().0,
            Color::WHITE
        );
    }

    /// Malformed control hierarchy cannot mutate another viewport's review state.
    #[test]
    fn unattached_pointer_targets_leave_review_unchanged() {
        let mut app = App::new();
        app.add_plugins(super::super::PromptlineUiPlugin);
        app.world_mut().resource_mut::<Output>().write("old");
        app.update();
        let viewport = app
            .world_mut()
            .query_filtered::<Entity, With<OutputView>>()
            .iter(app.world())
            .next()
            .unwrap();
        app.world_mut()
            .resource_scope(|world, output: Mut<'_, Output>| {
                world
                    .get_mut::<OutputView>(viewport)
                    .unwrap()
                    .pause_following(&output);
            });

        // No viewport components means this scroll target owns no reading state.
        let target = app
            .world_mut()
            .spawn_empty()
            .observe(scroll)
            .observe(latest)
            .id();
        app.world_mut()
            .trigger(wheel(target, MouseScrollUnit::Pixel, 100.0));
        let click = |entity| {
            Pointer::new(
                bevy::picking::pointer::PointerId::Mouse,
                wheel(entity, MouseScrollUnit::Pixel, 0.0).pointer_location,
                Click {
                    button: PointerButton::Primary,
                    count: 1,
                    hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                    duration: std::time::Duration::ZERO,
                },
                entity,
            )
        };
        app.world_mut().trigger(click(target));
        assert!(
            !app.world()
                .get::<OutputView>(viewport)
                .unwrap()
                .is_following()
        );

        // A parent lacking OutputView leaves both clicks and control updates inert.
        let parent = app.world_mut().spawn_empty().id();
        app.world_mut().entity_mut(target).insert((
            ChildOf(parent),
            LatestControl,
            Node {
                display: Display::Flex,
                ..default()
            },
            BackgroundColor(Color::WHITE),
        ));
        app.world_mut().trigger(click(target));
        app.update();
        assert!(
            !app.world()
                .get::<OutputView>(viewport)
                .unwrap()
                .is_following()
        );
        assert_eq!(
            app.world().get::<Node>(target).unwrap().display,
            Display::Flex
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(target).unwrap().0,
            Color::WHITE
        );
    }

    /// Pending font layout preserves review coordinates until a shaped line is available.
    #[test]
    fn unavailable_shaped_line_preserves_review_position() {
        let sequence = NonZeroU64::new(1).unwrap();
        let content = OutputContent {
            entries: vec![EntryRange {
                sequence,
                range: 0..5,
            }],
            ..default()
        };
        let anchor = ReadingAnchor {
            sequence,
            byte: 0,
            within_line: ScrollPosition(Vec2::new(4.0, 12.0)),
        };
        let node = ComputedNode {
            size: Vec2::new(400.0, 40.0),
            content_size: Vec2::new(400.0, 200.0),
            ..default()
        };
        let mut position = ScrollPosition(Vec2::new(2.0, 7.0));
        restore_anchor(
            &mut position,
            &node,
            &ComputedTextBlock::default(),
            &content,
            &anchor,
        );
        assert_eq!(
            position.0.to_array().map(f32::to_bits),
            [2.0_f32.to_bits(), 7.0_f32.to_bits()]
        );
    }

    /// Shapes exact UTF-8 text through the same tagged Bevy pipeline as UI text.
    fn shape(text: &str, width: f32) -> ComputedTextBlock {
        let mut app = App::new();
        let mut fonts = Assets::<Font>::default();
        fonts
            .insert(
                AssetId::default(),
                Font::from_bytes(bevy::text::DEFAULT_FONT_DATA.to_vec()),
            )
            .unwrap();
        app.insert_resource(fonts)
            .init_resource::<FontCx>()
            .init_resource::<LayoutCx>()
            .init_resource::<TextPipeline>()
            .add_systems(Update, bevy::text::load_font_assets_into_font_collection);
        app.update();
        let mut block = ComputedTextBlock::default();
        app.world_mut()
            .resource_scope(|world, mut pipeline: Mut<'_, TextPipeline>| {
                world.resource_scope(|world, mut font_cx: Mut<'_, FontCx>| {
                    world.resource_scope(|world, mut layout_cx: Mut<'_, LayoutCx>| {
                        let font = TextFont::default();
                        pipeline
                            .update_buffer(
                                world.resource::<Assets<Font>>(),
                                std::iter::once((
                                    Entity::PLACEHOLDER,
                                    0,
                                    text,
                                    &font,
                                    Color::WHITE,
                                    LineHeight::default(),
                                    LetterSpacing::default(),
                                )),
                                LineBreak::WordOrCharacter,
                                Justify::Left,
                                TextBounds {
                                    width: Some(width),
                                    height: None,
                                },
                                1.0,
                                &mut block,
                                &mut font_cx,
                                &mut layout_cx,
                                Vec2::new(width, 600.0),
                                16.0,
                            )
                            .unwrap();
                    });
                });
            });
        block
    }

    #[test]
    fn shaped_anchor_survives_arrival_and_clamps_after_eviction() {
        let before = shape("first\nsecond", 400.0);
        let second = before
            .buffer()
            .lines()
            .nth(1)
            .unwrap()
            .metrics()
            .block_min_coord;
        let block = shape("first\nsecond\nthird", 400.0);
        let node = ComputedNode {
            size: Vec2::new(400.0, 20.0),
            content_size: Vec2::new(400.0, 200.0),
            ..default()
        };
        let content = OutputContent {
            entries: vec![
                EntryRange {
                    sequence: NonZeroU64::new(1).unwrap(),
                    range: 0..5,
                },
                EntryRange {
                    sequence: NonZeroU64::new(2).unwrap(),
                    range: 6..12,
                },
                EntryRange {
                    sequence: NonZeroU64::new(3).unwrap(),
                    range: 13..18,
                },
            ],
            anchor: None,
            ..default()
        };
        let anchor = ReadingAnchor {
            sequence: NonZeroU64::new(2).unwrap(),
            byte: 0,
            within_line: default(),
        };
        let mut scroll = ScrollPosition::default();
        restore_anchor(&mut scroll, &node, &block, &content, &anchor);
        assert!((scroll.y - second).abs() < 0.01);
        let evicted = OutputContent {
            entries: vec![EntryRange {
                sequence: NonZeroU64::new(3).unwrap(),
                range: 0..5,
            }],
            anchor: None,
            ..default()
        };
        restore_anchor(
            &mut scroll,
            &node,
            &shape("third", 400.0),
            &evicted,
            &anchor,
        );
        assert!(scroll.y.abs() < f32::EPSILON);
    }

    #[test]
    fn scroll_range_converts_physical_geometry_to_logical_pixels() {
        let node = ComputedNode {
            size: Vec2::new(200.0, 100.0),
            content_size: Vec2::new(300.0, 400.0),
            inverse_scale_factor: 0.5,
            ..default()
        };
        assert_eq!(scroll_range(&node), Vec2::new(50.0, 150.0));
        assert_eq!(scroll_range(&ComputedNode::default()), Vec2::ZERO);
    }
    /// Builds a physical-pixel scroll event targeting the actual default viewport.
    fn wheel(entity: Entity, unit: MouseScrollUnit, y: f32) -> Pointer<Scroll> {
        Pointer::new(
            bevy::picking::pointer::PointerId::Mouse,
            bevy::picking::pointer::Location {
                target: bevy::camera::RenderTarget::None {
                    size: UVec2::new(200, 100),
                }
                .normalize(None)
                .unwrap(),
                position: Vec2::ZERO,
            },
            Scroll {
                unit,
                x: 0.0,
                y,
                hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                phase: bevy::input::touch::TouchPhase::Moved,
            },
            entity,
        )
    }

    #[test]
    fn wheel_review_preserves_position_counts_arrivals_and_latest_resumes() {
        let mut app = App::new();
        app.add_plugins(super::super::PromptlineUiPlugin);
        app.world_mut().resource_mut::<Output>().write("old");
        app.update();
        let viewport = app
            .world_mut()
            .query_filtered::<Entity, With<OutputView>>()
            .iter(app.world())
            .next()
            .unwrap();
        *app.world_mut().get_mut::<ComputedNode>(viewport).unwrap() = ComputedNode {
            size: Vec2::new(200.0, 100.0),
            content_size: Vec2::new(200.0, 500.0),
            ..default()
        };
        app.update();
        assert!((app.world().get::<ScrollPosition>(viewport).unwrap().y - 400.0).abs() < 0.01);
        app.world_mut()
            .trigger(wheel(viewport, MouseScrollUnit::Line, 1.0));
        assert!(
            !app.world()
                .get::<OutputView>(viewport)
                .unwrap()
                .is_following()
        );
        assert!((app.world().get::<ScrollPosition>(viewport).unwrap().y - 300.0).abs() < 0.01);
        app.world_mut()
            .resource_mut::<Output>()
            .write("arrival\nwrap");
        app.update();
        assert!((app.world().get::<ScrollPosition>(viewport).unwrap().y - 300.0).abs() < 0.01);
        assert_eq!(
            app.world()
                .get::<OutputView>(viewport)
                .unwrap()
                .unread_entries(app.world().resource::<Output>()),
            1
        );
        let button = app
            .world_mut()
            .query_filtered::<Entity, With<LatestControl>>()
            .iter(app.world())
            .next()
            .unwrap();
        assert_eq!(
            app.world().get::<Node>(button).unwrap().display,
            Display::Flex
        );
        let location = wheel(viewport, MouseScrollUnit::Pixel, 0.0).pointer_location;
        app.world_mut().trigger(Pointer::new(
            bevy::picking::pointer::PointerId::Mouse,
            location,
            Click {
                button: PointerButton::Primary,
                count: 1,
                hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                duration: std::time::Duration::ZERO,
            },
            button,
        ));
        app.update();
        assert!(
            app.world()
                .get::<OutputView>(viewport)
                .unwrap()
                .is_following()
        );
        assert_eq!(
            app.world().get::<Node>(button).unwrap().display,
            Display::None
        );
    }

    #[test]
    fn pixel_scroll_can_resume_at_bottom_and_nonfinite_deltas_have_no_effect() {
        let mut app = App::new();
        app.add_plugins(super::super::PromptlineUiPlugin);
        app.world_mut().resource_mut::<Output>().write("old");
        app.update();
        let viewport = app
            .world_mut()
            .query_filtered::<Entity, With<OutputView>>()
            .iter(app.world())
            .next()
            .unwrap();
        *app.world_mut().get_mut::<ComputedNode>(viewport).unwrap() = ComputedNode {
            size: Vec2::new(200.0, 100.0),
            content_size: Vec2::new(200.0, 500.0),
            ..default()
        };
        app.update();
        app.world_mut()
            .trigger(wheel(viewport, MouseScrollUnit::Pixel, 100.0));
        assert!(
            !app.world()
                .get::<OutputView>(viewport)
                .unwrap()
                .is_following()
        );
        // Further review scrolling cannot reset the unread baseline after an arrival.
        app.world_mut()
            .resource_mut::<Output>()
            .write("during review");
        app.world_mut()
            .trigger(wheel(viewport, MouseScrollUnit::Pixel, 10.0));
        assert_eq!(
            app.world()
                .get::<OutputView>(viewport)
                .unwrap()
                .unread_entries(app.world().resource::<Output>()),
            1
        );
        app.world_mut()
            .trigger(wheel(viewport, MouseScrollUnit::Pixel, f32::NAN));
        assert!((app.world().get::<ScrollPosition>(viewport).unwrap().y - 290.0).abs() < 0.01);
        app.world_mut()
            .trigger(wheel(viewport, MouseScrollUnit::Pixel, -110.0));
        assert!(
            app.world()
                .get::<OutputView>(viewport)
                .unwrap()
                .is_following()
        );
    }
}
