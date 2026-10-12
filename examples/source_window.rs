//! Independent movable and resizable window with actual Bevy editor output selection.
use bevy::prelude::*;
use bevy::text::{EditableText, EditableTextSystems, TextCursorStyle, TextEdit};
use bevy_promptline::input::{ConsoleRole, ConsoleState, PromptlineSystems, PromptlineUiPlugin};
#[path = "support/playground.rs"]
mod playground;

/// Logical-pixel window bounds are independent from console focus and command state.
#[derive(Debug, Resource)]
struct Bounds {
    /// Top-left position measured in logical viewport pixels.
    position: Vec2,
    /// Window dimensions measured in logical viewport pixels.
    size: Vec2,
}
impl Default for Bounds {
    fn default() -> Self {
        Self {
            position: Vec2::new(120.0, 100.0),
            size: Vec2::new(760.0, 440.0),
        }
    }
}
impl Bounds {
    /// Clamps both dimensions and position, including viewports smaller than the preferred minimum.
    fn constrain(&mut self, viewport: Vec2) {
        let viewport = viewport.max(Vec2::ONE);
        let minimum = Vec2::new(280.0, 240.0).min(viewport);
        self.size = self.size.max(minimum).min(viewport);
        self.position = self.position.max(Vec2::ZERO).min(viewport - self.size);
    }
}
/// Decorates only a newly created public root; its input and runtime remain unchanged.
fn decorate(
    mut commands: Commands<'_, '_>,
    roots: Query<'_, '_, (Entity, &ConsoleRole), Added<ConsoleRole>>,
) {
    for (entity, role) in &roots {
        if *role != ConsoleRole::Root {
            continue;
        }
        let title = commands
            .spawn((
                ConsoleRole::DragRegion,
                Node {
                    height: px(28),
                    min_height: px(28),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.16, 0.2, 0.23)),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new("Console"),
                    TextFont {
                        font_size: FontSize::Px(18.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Pickable::IGNORE,
                ));
                parent
                    .spawn((
                        ConsoleRole::CloseControl,
                        bevy::ui_widgets::Button,
                        Node {
                            padding: px(6).all(),
                            ..default()
                        },
                    ))
                    .with_children(|button| {
                        button.spawn((Text::new("Close"), TextColor(Color::WHITE)));
                    })
                    .observe(
                        |_: On<'_, '_, PointerClick>, mut state: ResMut<'_, ConsoleState>| {
                            state.close();
                        },
                    );
            })
            .observe(drag_window)
            .id();
        let grip = commands
            .spawn((
                ConsoleRole::ResizeControl,
                bevy::ui_widgets::Button,
                Node {
                    position_type: PositionType::Absolute,
                    right: px(2),
                    bottom: px(2),
                    width: px(24),
                    height: px(24),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.28, 0.36, 0.4)),
            ))
            .with_children(|parent| {
                parent.spawn((Text::new("↘"), TextColor(Color::WHITE), Pickable::IGNORE));
            })
            .observe(resize_window)
            .id();
        commands
            .entity(entity)
            .insert_children(0, &[title])
            .add_child(grip);
    }
}
/// Picking retains the initial drag target after the pointer leaves the title region.
fn drag_window(event: On<'_, '_, PointerDrag>, mut bounds: ResMut<'_, Bounds>) {
    bounds.position += event.delta;
}
/// Resize deltas use the same logical-pixel coordinate system as the window bounds.
fn resize_window(event: On<'_, '_, PointerDrag>, mut bounds: ResMut<'_, Bounds>) {
    bounds.size += event.delta;
}
/// Viewport changes clamp saved bounds before applying them to the public root.
fn place(
    windows: Query<'_, '_, &Window>,
    mut bounds: ResMut<'_, Bounds>,
    mut roots: Query<'_, '_, (&ConsoleRole, &mut Node)>,
) {
    let Some(window) = windows.iter().next() else {
        return;
    };
    bounds.constrain(Vec2::new(window.width(), window.height()));
    for (role, mut node) in &mut roots {
        if *role != ConsoleRole::Root {
            continue;
        }
        node.top = px(bounds.position.y);
        node.left = px(bounds.position.x);
        node.bottom = Val::Auto;
        node.width = px(bounds.size.x);
        node.height = px(bounds.size.y);
        node.max_height = Val::Auto;
    }
}
/// Replaces the output role with a selectable upstream editor that never submits commands.
fn selectable_output(
    mut commands: Commands<'_, '_>,
    roles: Query<'_, '_, (Entity, &ConsoleRole), Added<ConsoleRole>>,
) {
    for (entity, role) in &roles {
        if *role == ConsoleRole::Output {
            commands.entity(entity).remove::<Text>().insert((
                EditableText {
                    allow_newlines: true,
                    ..default()
                },
                TextCursorStyle {
                    color: Color::srgba(0.0, 0.0, 0.0, 0.0),
                    selection_color: Color::srgb(0.18, 0.35, 0.45),
                    ..default()
                },
            ));
        }
    }
}
/// Read-only output permits navigation and copying while excluding mutation operations.
fn protect_output(mut editors: Query<'_, '_, (&ConsoleRole, &mut EditableText)>) {
    for (role, mut editor) in &mut editors {
        if *role != ConsoleRole::Output {
            continue;
        }
        editor.pending_edits.retain(|edit| {
            matches!(
                edit,
                TextEdit::Copy
                    | TextEdit::Left(_)
                    | TextEdit::Right(_)
                    | TextEdit::WordLeft(_)
                    | TextEdit::WordRight(_)
                    | TextEdit::Up(_)
                    | TextEdit::Down(_)
                    | TextEdit::TextStart(_)
                    | TextEdit::TextEnd(_)
                    | TextEdit::HardLineStart(_)
                    | TextEdit::HardLineEnd(_)
                    | TextEdit::LineStart(_)
                    | TextEdit::LineEnd(_)
                    | TextEdit::SelectAll
                    | TextEdit::SelectAllIfCollapsed
                    | TextEdit::CollapseSelection
                    | TextEdit::MoveToPoint(_)
                    | TextEdit::SelectWordAtPoint(_)
                    | TextEdit::SelectLineAtPoint(_)
                    | TextEdit::SelectedHardLineAtPoint(_)
                    | TextEdit::ExtendSelectionToPoint(_)
                    | TextEdit::ShiftClickExtension(_)
            )
        });
    }
}
/// Runs a distinct app and Wasm artifact with host-owned window movement and sizing.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Bevy Promptline · source window".into(),
            resolution: (1100, 720).into(),
            canvas: Some("#bevy".into()),
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    }))
    .add_plugins(PromptlineUiPlugin)
    .init_resource::<Bounds>();
    playground::install(&mut app)?;
    app.add_systems(
        Update,
        (
            decorate,
            selectable_output,
            place.after(PromptlineSystems::Input),
        ),
    );
    app.add_systems(PostUpdate, protect_output.before(EditableTextSystems));
    app.run();
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_remain_inside_resized_and_narrow_viewports() {
        let mut bounds = Bounds {
            position: Vec2::new(-30.0, 900.0),
            size: Vec2::new(2000.0, 10.0),
        };
        bounds.constrain(Vec2::new(640.0, 480.0));
        assert_eq!(bounds.position, Vec2::new(0.0, 240.0));
        assert_eq!(bounds.size, Vec2::new(640.0, 240.0));
        bounds.constrain(Vec2::new(200.0, 180.0));
        assert_eq!(bounds.position, Vec2::ZERO);
        assert_eq!(bounds.size, Vec2::new(200.0, 180.0));
    }
}
