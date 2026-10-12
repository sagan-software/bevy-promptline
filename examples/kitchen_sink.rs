//! Independent advanced integration combines optional adapters and host-owned presentation controls.
use bevy::log::{BoxedLayer, LogPlugin};
use bevy::prelude::*;
use bevy::ui_widgets::ScrollArea;
use bevy_promptline::clap::ClapAppExt;
use bevy_promptline::input::{
    CommandBuilder, ConsoleRole, ConsoleTheme, ParseDecision, PromptlineSystems, PromptlineUiPlugin,
};
use bevy_promptline::logging::{
    CaptureConfig, CompactFormatter, DisplayFilter, LogCapture, LogPresentation, LogStore,
    PromptlineLogPlugin, ReadableFormatter, capture_layer,
};
use bevy_promptline::{CommandOutcome, CommandRegistry, Output, PromptlineAppExt};
use clap::{Parser, Subcommand, ValueEnum};
#[path = "support/pending.rs"]
mod pending;
#[path = "support/playground.rs"]
mod playground;

/// Host presentation mode changes root layout without changing the command engine.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Resource, ValueEnum)]
enum Layout {
    /// Drawer retains output, candidates, and contextual help.
    #[default]
    Drawer,
    /// Compact keeps one focused prompt and ghost while retaining output in the model.
    Compact,
}
/// Theme vocabulary preserves explicit dark and light modes.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Theme {
    /// Default dark console palette.
    Dark,
    /// Light palette sets every foreground role for readable contrast.
    Light,
}
/// Actor policy is independent from whether its command happens to be visible.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Resource, ValueEnum)]
enum Policy {
    /// Developer mode permits the explicitly registered mutation demonstration.
    #[default]
    Developer,
    /// Gameplay mode hides and rejects that demonstration on all submission sources.
    Gameplay,
}
/// Formatter changes reuse records instead of recapturing them.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
    /// Include elapsed time and structured field labels.
    Readable,
    /// Omit elapsed time while preserving typed field text.
    Compact,
}
/// Positive burst size bounds one diagnostic request to at most two thousand events.
#[derive(Debug, Clone, Copy)]
struct BurstCount(std::num::NonZeroU16);
impl std::str::FromStr for BurstCount {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let count = value
            .parse::<std::num::NonZeroU16>()
            .map_err(|_| "burst count must be 1..2000".to_owned())?;
        if count.get() > 2000 {
            return Err("burst count must be 1..2000".into());
        }
        Ok(Self(count))
    }
}
/// Advanced parser keeps each operation's payload in its own variant.
#[derive(Debug, Parser)]
#[command(
    name = "demo",
    version,
    about = "Change presentation, filters, policy, or emit a bounded burst"
)]
struct Demo {
    /// Exactly one advanced action runs after ordinary parsing and permission checks.
    #[command(subcommand)]
    action: Action,
}
/// Actions demonstrate customization without introducing another command engine.
#[derive(Debug, Subcommand)]
enum Action {
    /// Switch the default console color palette.
    Theme {
        #[arg(value_enum)]
        /// Selected complete console palette.
        mode: Theme,
    },
    /// Switch drawer and compact prompt layouts.
    Layout {
        #[arg(value_enum)]
        /// Selected root layout without changing retained output.
        mode: Layout,
    },
    /// Change the world-backed permission policy.
    Policy {
        #[arg(value_enum)]
        /// Selected command permission mode.
        mode: Policy,
    },
    /// Emit at most two thousand structured game events through the existing host subscriber.
    Burst {
        #[arg(default_value = "1100")]
        /// Validated positive event count capped at two thousand.
        count: BurstCount,
    },
    /// Reload only the console layer-local capture filter, or inspect its current directives.
    Filter {
        /// Optional `EnvFilter` input is validated before capture reload.
        directives: Option<String>,
    },
    /// Select retained log text without changing capture or discarding records.
    Search {
        /// Optional substring selects formatted retained log rows.
        text: Option<String>,
    },
    /// Reformat retained records without another capture.
    Format {
        #[arg(value_enum)]
        /// Selected formatter reuses retained structured records.
        mode: Format,
    },
    /// Display accepted public history entries matching an optional substring.
    History {
        /// Optional substring selects accepted public command history.
        text: Option<String>,
    },
    /// Clear direct output and retained logs without replacing the subscriber.
    Clear,
}
/// Logs panel is ordinary host UI, separate from the authoritative command editor.
#[derive(Component)]
struct LogsPanel;
/// Loss summary remains visible independently of captured row formatting.
#[derive(Component)]
struct LossSummary;

/// Records whether the log viewport follows arrivals or retains a reading position.
#[derive(Debug, Default, Clone, Copy, Component)]
enum LogScrollMode {
    /// New rows remain visible at the bottom of the viewport.
    #[default]
    Following,
    /// Manual scrolling retains the current reading position.
    Reviewing,
}
/// Constructs one capture destination before Bevy initializes its global subscriber.
fn install_capture(app: &mut App) -> Option<BoxedLayer> {
    match capture_layer(CaptureConfig::default()) {
        Ok((layer, capture)) => {
            app.insert_resource(capture);
            Some(layer)
        }
        Err(error) => {
            eprintln!("Capture setup failed: {error}");
            None
        }
    }
}
/// Applies selected demo behavior through the existing typed dispatch owner.
fn demo(
    In(command): In<Demo>,
    mut theme: ResMut<'_, ConsoleTheme>,
    mut layout: ResMut<'_, Layout>,
    mut policy: ResMut<'_, Policy>,
    capture: Option<Res<'_, LogCapture>>,
    mut display: ResMut<'_, DisplayFilter>,
    mut presentation: ResMut<'_, LogPresentation>,
    mut logs: ResMut<'_, LogStore>,
    history: Res<'_, bevy_promptline::command::CommandHistory>,
    mut output: ResMut<'_, Output>,
    mut registry: ResMut<'_, CommandRegistry>,
) -> CommandOutcome {
    match command.action {
        Action::Theme { mode } => {
            *theme = match mode {
                Theme::Dark => default(),
                Theme::Light => light_theme(),
            };
        }
        Action::Layout { mode } => *layout = mode,
        Action::Policy { mode } => {
            *policy = mode;
            registry.set_policy(actor_policy);
        }
        Action::Burst { count } => {
            for sequence in 0..count.0.get() {
                tracing::info!(target: "game", sequence, active = true, "Structured game event");
            }
        }
        Action::Filter { directives } => {
            let Some(capture) = capture else {
                output.write("Capture is unavailable; ordinary output remains active");
                return CommandOutcome::Failure("capture unavailable".into());
            };
            if let Some(directives) = directives
                && let Err(error) = capture.set_filter(&directives)
            {
                let diagnostic = error.to_string();
                output.write(&diagnostic);
                return CommandOutcome::Failure(diagnostic);
            }
            match capture.filter() { Ok(filter) => output.write(format!("Console capture: {filter}\nUpstream and compile-time filters can remove events before capture.")), Err(error) => output.write(error.to_string()) }
        }
        Action::Search { text } => display.query = text.unwrap_or_default(),
        Action::Format { mode } => {
            *presentation = match mode {
                Format::Readable => LogPresentation::new(ReadableFormatter),
                Format::Compact => LogPresentation::new(CompactFormatter),
            }
        }
        Action::History { text } => {
            let query = text.unwrap_or_default();
            for entry in history.entries().filter(|entry| entry.contains(&query)) {
                output.write(entry);
            }
        }
        Action::Clear => {
            output.clear();
            logs.clear();
        }
    }
    CommandOutcome::Success
}
/// Permission applies equally to discovery, UI submissions, and programmatic submissions.
fn actor_policy(
    world: &World,
    metadata: &bevy_promptline::CommandMetadata,
    _: bevy_promptline::command::SubmissionSource,
) -> bool {
    metadata.name.as_ref() != "danger"
        || world
            .get_resource::<Policy>()
            .is_some_and(|policy| *policy == Policy::Developer)
}
/// Light palette sets all foreground roles and editor selection, preserving readable contrast.
fn light_theme() -> ConsoleTheme {
    ConsoleTheme {
        background: Color::srgb(0.94, 0.95, 0.94),
        input: Color::BLACK,
        output: Color::srgb(0.08, 0.1, 0.1),
        help: Color::srgb(0.15, 0.2, 0.2),
        ghost: Color::srgb(0.25, 0.35, 0.3),
        caret: Color::BLACK,
        selection: Color::srgb(0.65, 0.8, 0.85),
        ..default()
    }
}
/// Creates a bounded log view with useful filter and loss state rather than a second console.
fn setup_panel(mut commands: Commands<'_, '_>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(54),
                right: px(18),
                width: percent(46),
                height: px(240),
                padding: px(12).all(),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.04, 0.04, 0.92)),
        ))
        .with_children(|parent| {
            parent.spawn((
                LossSummary,
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.85, 0.8)),
            ));
            parent
                .spawn((
                    ScrollArea,
                    LogScrollMode::default(),
                    Node {
                        flex_grow: 1.0,
                        min_height: px(0),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                ))
                .observe(mark_log_review)
                .with_children(|viewport| {
                    viewport.spawn((
                        LogsPanel,
                        Pickable::IGNORE,
                        Node {
                            width: percent(100),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.85, 0.94, 0.88)),
                    ));
                });
        });
}
/// Rebuilds retained log text only when records, display selection, or formatting change.
fn show_logs(
    logs: Res<'_, LogStore>,
    filter: Res<'_, DisplayFilter>,
    presentation: Res<'_, LogPresentation>,
    capture: Option<Res<'_, LogCapture>>,
    mut text: Query<'_, '_, &mut Text, With<LogsPanel>>,
    mut summary: Query<'_, '_, &mut Text, (With<LossSummary>, Without<LogsPanel>)>,
) {
    let is_store_changed = logs.is_changed();
    let is_filter_changed = filter.is_changed();
    let is_formatter_changed = presentation.is_changed();
    // Keep every retained matching row reachable without an intermediate collection.
    if is_store_changed || is_filter_changed || is_formatter_changed {
        for mut text in &mut text {
            text.0.clear();
            for (index, row) in logs.rows(&filter, presentation.formatter()).enumerate() {
                if index != 0 {
                    text.0.push('\n');
                }
                text.0.push_str(&row);
            }
        }
    }
    let retained = logs.records().count();
    let evicted = logs.dropped();
    let lost = capture.as_ref().map_or(0, |capture| capture.dropped());
    let label =
        format!("Captured logs: {retained} retained | {lost} capture losses | {evicted} evicted");
    for mut text in &mut summary {
        text.0.clone_from(&label);
    }
}
/// Manual wheel input begins review before Bevy applies the scroll delta.
fn mark_log_review(event: On<'_, '_, PointerScroll>, mut modes: Query<'_, '_, &mut LogScrollMode>) {
    // Horizontal movement does not change vertical reading ownership.
    if event.y != 0.0
        && let Ok(mut mode) = modes.get_mut(event.entity)
    {
        *mode = LogScrollMode::Reviewing;
    }
}

/// Follows new rows while manual review retains its logical-pixel position.
fn follow_logs(mut views: Query<'_, '_, (&ComputedNode, &mut ScrollPosition, &mut LogScrollMode)>) {
    for (node, mut position, mut mode) in &mut views {
        // Layout sizes are physical pixels; inverse scale converts to logical scroll units.
        let maximum = ((node.content_size() - node.size()) * node.inverse_scale_factor)
            .max(Vec2::ZERO)
            .y;
        // Reaching the bottom resumes following without changing an earlier reading position.
        match *mode {
            LogScrollMode::Following => {
                if position.y.to_bits() != maximum.to_bits() {
                    position.y = maximum;
                }
            }
            LogScrollMode::Reviewing if position.y >= maximum => *mode = LogScrollMode::Following,
            LogScrollMode::Reviewing => {}
        }
    }
}

/// Compact mode hides presentation roles while preserving their retained runtime data.
fn apply_layout(
    layout: Res<'_, Layout>,
    mut nodes: Query<'_, '_, (&ConsoleRole, &mut Node, Option<&ChildOf>)>,
    mut viewports: Query<'_, '_, &mut Node, Without<ConsoleRole>>,
) {
    for (role, mut node, parent) in &mut nodes {
        if *role == ConsoleRole::Root {
            node.height = match *layout {
                Layout::Drawer => px(360),
                Layout::Compact => px(68),
            };
            node.padding = match *layout {
                Layout::Drawer => px(16).all(),
                Layout::Compact => px(8).all(),
            };
        }
        if matches!(
            role,
            ConsoleRole::Output | ConsoleRole::Help | ConsoleRole::Candidates
        ) {
            node.display = match *layout {
                Layout::Drawer => Display::Flex,
                Layout::Compact => Display::None,
            };
            // Hide the output viewport along with its replaceable text role.
            if *role == ConsoleRole::Output
                && let Some(parent) = parent
                && let Ok(mut viewport) = viewports.get_mut(parent.parent())
            {
                viewport.display = node.display;
            }
        }
    }
}
/// Reports lifecycle identities without echoing raw submitted command arguments.
fn lifecycle(
    mut messages: MessageReader<'_, '_, bevy_promptline::CommandNotification>,
    mut output: ResMut<'_, Output>,
) {
    for message in messages.read() {
        let invocation = message.invocation;
        let status = &message.status;
        output.write(format!("{invocation:?}: {status:?}"));
    }
}
/// Runs its own native window or Wasm canvas with caller-installed logging integration.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(LogPlugin {
                custom_layer: install_capture,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy Promptline · kitchen sink".into(),
                    resolution: (1100, 720).into(),
                    canvas: Some("#bevy".into()),
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }),
    )
    .add_plugins((PromptlineUiPlugin, PromptlineLogPlugin))
    .init_resource::<Layout>()
    .init_resource::<Policy>();
    playground::install(&mut app)?;
    pending::install(&mut app)?;
    app.register_clap::<Demo, _>(demo)?;
    app.register_builder(
        CommandBuilder::new(
            bevy_promptline::CommandMetadata::new("danger", "Developer-only demonstration")?,
            |_| Ok(ParseDecision::Run(())),
        ),
        |_: In<()>, mut output: ResMut<'_, Output>| {
            output.write("Developer command accepted");
            CommandOutcome::Success
        },
    )?;
    app.world_mut()
        .resource_mut::<CommandRegistry>()
        .set_policy(actor_policy);
    app.add_systems(Startup, setup_panel);
    app.add_systems(PostUpdate, follow_logs.after(bevy::ui::UiSystems::Layout));
    app.add_systems(Update, apply_layout.after(PromptlineSystems::Input));
    app.add_systems(
        PostUpdate,
        (show_logs, lifecycle).after(PromptlineSystems::Output),
    );
    app.run();
    Ok(())
}

/// Host layout checks use public roles and preserve the library's clipping tree.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_hides_output_viewport_and_drawer_restores_it() {
        let mut app = App::new();
        app.add_plugins(PromptlineUiPlugin)
            .init_resource::<Layout>();
        app.add_systems(Update, apply_layout.after(PromptlineSystems::Input));
        app.update();
        let output = app
            .world_mut()
            .query::<(Entity, &ConsoleRole)>()
            .iter(app.world())
            .find_map(|(entity, role)| (*role == ConsoleRole::Output).then_some(entity))
            .unwrap();
        let parent = app.world().get::<ChildOf>(output).unwrap().parent();
        for (layout, display) in [
            (Layout::Compact, Display::None),
            (Layout::Drawer, Display::Flex),
        ] {
            *app.world_mut().resource_mut::<Layout>() = layout;
            app.update();
            assert_eq!(app.world().get::<Node>(output).unwrap().display, display);
            assert_eq!(app.world().get::<Node>(parent).unwrap().display, display);
        }
    }

    /// Wrapped retained rows must remain reachable through the actual log viewport.
    #[test]
    fn wrapped_log_rows_scroll_without_moving_the_loss_summary() {
        use bevy::input::{mouse::MouseScrollUnit, touch::TouchPhase};
        use bevy::picking::{
            backend::HitData,
            events::{Pointer, Scroll},
            pointer::{Location, PointerId},
        };
        use bevy::ui_widgets::ScrollAreaPlugin;
        let mut app = App::new();
        app.add_plugins(ScrollAreaPlugin)
            .add_systems(Startup, setup_panel);
        app.update();
        let rows = app
            .world_mut()
            .query_filtered::<Entity, With<LogsPanel>>()
            .single(app.world())
            .unwrap();
        let viewport = app.world().get::<ChildOf>(rows).unwrap().parent();
        let summary = app
            .world_mut()
            .query_filtered::<Entity, With<LossSummary>>()
            .single(app.world())
            .unwrap();
        let pickable = app.world().get::<Pickable>(rows).unwrap();
        assert!(!pickable.is_hoverable);
        assert!(!pickable.should_block_lower);
        app.world_mut().entity_mut(viewport).insert(ComputedNode {
            size: Vec2::new(200.0, 100.0),
            content_size: Vec2::new(200.0, 400.0),
            inverse_scale_factor: 1.0,
            ..default()
        });
        app.world_mut().trigger(Pointer::new(
            PointerId::Mouse,
            Location {
                target: bevy::camera::RenderTarget::None {
                    size: UVec2::new(200, 100),
                }
                .normalize(None)
                .unwrap(),
                position: Vec2::ZERO,
            },
            Scroll {
                unit: MouseScrollUnit::Pixel,
                x: 0.0,
                y: -50.0,
                hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                phase: TouchPhase::Moved,
            },
            viewport,
        ));
        let position = app
            .world()
            .get::<ScrollPosition>(viewport)
            .map_or(0.0, |position| position.y);
        assert_eq!(position, 50.0, "wrapped log rows must scroll");
        assert_ne!(
            app.world().get::<ChildOf>(summary).unwrap().parent(),
            viewport
        );
    }

    /// Every retained matching row stays reachable, and idle frames do not call the formatter.
    #[test]
    fn full_log_history_reformats_only_after_records_or_selection_change() {
        use bevy_promptline::logging::{LogFormatter, LogRecord};
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        use tracing_subscriber::prelude::*;
        /// Measures formatter calls through the public customization trait.
        struct CountingFormatter {
            /// Number of retained rows formatted by presentation.
            calls: Arc<AtomicUsize>,
        }
        impl LogFormatter for CountingFormatter {
            fn format(&self, record: &LogRecord) -> String {
                self.calls.fetch_add(1, Ordering::Relaxed);
                CompactFormatter.format(record)
            }
        }
        let (layer, capture) = capture_layer(CaptureConfig::default()).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut app = App::new();
        app.add_plugins(PromptlineLogPlugin)
            .insert_resource(capture)
            .insert_resource(LogPresentation::new(CountingFormatter {
                calls: Arc::clone(&calls),
            }))
            .add_systems(Startup, setup_panel)
            .add_systems(PostUpdate, show_logs.after(PromptlineSystems::Output));
        tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), || {
            for index in 0..12 {
                tracing::info!(index, "retained row");
            }
        });
        app.update();
        let rows = app
            .world_mut()
            .query_filtered::<Entity, With<LogsPanel>>()
            .single(app.world())
            .unwrap();
        assert_eq!(app.world().get::<Text>(rows).unwrap().0.lines().count(), 12);
        assert!(app.world().get::<Text>(rows).unwrap().0.contains("index=0"));
        assert_eq!(calls.load(Ordering::Relaxed), 12);
        app.update();
        assert_eq!(calls.load(Ordering::Relaxed), 12);
        app.world_mut().resource_mut::<DisplayFilter>().query = "index=3".into();
        app.update();
        assert_eq!(calls.load(Ordering::Relaxed), 24);
        assert_eq!(app.world().get::<Text>(rows).unwrap().0.lines().count(), 1);
        assert!(app.world().get::<Text>(rows).unwrap().0.contains("index=3"));
        assert_eq!(app.world().resource::<LogStore>().records().count(), 12);
    }

    /// Logical scrolling follows arrivals, preserves review, and resumes at the bottom.
    #[test]
    fn log_following_uses_scaled_geometry_and_preserves_idle_positions() {
        let mut app = App::new();
        app.add_systems(PostUpdate, follow_logs);
        let viewport = app
            .world_mut()
            .spawn((
                ComputedNode {
                    size: Vec2::new(200.0, 200.0),
                    content_size: Vec2::new(200.0, 600.0),
                    inverse_scale_factor: 0.5,
                    ..default()
                },
                ScrollPosition::default(),
                LogScrollMode::Following,
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<ScrollPosition>(viewport).unwrap().y,
            200.0
        );
        let changed = app
            .world()
            .entity(viewport)
            .get_ref::<ScrollPosition>()
            .unwrap()
            .last_changed();
        app.update();
        assert_eq!(
            app.world()
                .entity(viewport)
                .get_ref::<ScrollPosition>()
                .unwrap()
                .last_changed(),
            changed
        );
        app.world_mut().entity_mut(viewport).insert((
            LogScrollMode::Reviewing,
            ScrollPosition(Vec2::new(0.0, 50.0)),
        ));
        app.world_mut()
            .get_mut::<ComputedNode>(viewport)
            .unwrap()
            .content_size
            .y = 900.0;
        app.update();
        assert_eq!(app.world().get::<ScrollPosition>(viewport).unwrap().y, 50.0);
        assert!(matches!(
            app.world().get::<LogScrollMode>(viewport),
            Some(LogScrollMode::Reviewing)
        ));
        app.world_mut()
            .get_mut::<ScrollPosition>(viewport)
            .unwrap()
            .y = 350.0;
        app.update();
        assert!(matches!(
            app.world().get::<LogScrollMode>(viewport),
            Some(LogScrollMode::Following)
        ));
        app.world_mut()
            .get_mut::<ComputedNode>(viewport)
            .unwrap()
            .content_size
            .y = 1100.0;
        app.update();
        assert_eq!(
            app.world().get::<ScrollPosition>(viewport).unwrap().y,
            450.0
        );
        app.world_mut()
            .get_mut::<ComputedNode>(viewport)
            .unwrap()
            .content_size
            .y = 200.0;
        app.update();
        assert_eq!(app.world().get::<ScrollPosition>(viewport).unwrap().y, 0.0);
    }
}
