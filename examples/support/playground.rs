//! Native and Wasm console integration with visible color and rotation changes.
use bevy::prelude::*;
use bevy_promptline::input::{ConsoleBindings, PromptlineSystems};
use bevy_promptline::{CommandMetadata, CommandOutcome, ConsoleCommand, Output, PromptlineAppExt};
use std::str::FromStr;

/// Shortcut hint follows configured bindings independently of command input.
#[derive(Component)]
struct KeyboardHint;

/// Primitive changed by typed console commands.
#[derive(Component)]
pub(crate) struct DemoPrimitive;

/// Rotation rate, in radians per second, constrained to a finite interval.
#[derive(Debug, Clone, Copy, Resource)]
pub(crate) struct SpinRate(pub(crate) f32);
impl Default for SpinRate {
    fn default() -> Self {
        Self(0.6)
    }
}
impl FromStr for SpinRate {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value: f32 = value
            .parse()
            .map_err(|_| "spin needs a number in radians per second".to_owned())?;
        if !value.is_finite() || !(-10.0..=10.0).contains(&value) {
            return Err("spin must be finite and between -10 and 10 rad/s".to_owned());
        }
        Ok(Self(value))
    }
}

/// Supported scene colors, independent of their typed wire spelling.
#[derive(Debug, Clone, Copy)]
enum SceneColor {
    /// Default blue scene color.
    Blue,
    /// Green scene color selected by the color command.
    Green,
    /// Amber scene color selected by the color command.
    Amber,
    /// Red scene color selected by the color command.
    Red,
}
impl FromStr for SceneColor {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "blue" => Ok(Self::Blue),
            "green" => Ok(Self::Green),
            "amber" => Ok(Self::Amber),
            "red" => Ok(Self::Red),
            _ => Err("color expects blue, green, amber, or red".to_owned()),
        }
    }
}
impl SceneColor {
    /// Projects a closed scene color into Bevy's rendering color type.
    const fn color(self) -> Color {
        match self {
            Self::Blue => Color::srgb(0.22, 0.62, 0.92),
            Self::Green => Color::srgb(0.24, 0.82, 0.5),
            Self::Amber => Color::srgb(0.96, 0.65, 0.22),
            Self::Red => Color::srgb(0.94, 0.32, 0.3),
        }
    }
}

/// Echo text parsed without Clap.
#[derive(Debug)]
struct Echo(String);
impl ConsoleCommand for Echo {
    fn metadata() -> Result<CommandMetadata, bevy_promptline::InvalidCommandName> {
        CommandMetadata::new("echo", "Print text. Usage: echo <text>")
    }
    fn parse(arguments: &[String]) -> Result<Self, String> {
        Ok(Self(arguments.join(" ")))
    }
}

/// A validated scene color delivered to an ordinary game system.
#[derive(Debug)]
struct ColorCommand(SceneColor);
impl ConsoleCommand for ColorCommand {
    fn metadata() -> Result<CommandMetadata, bevy_promptline::InvalidCommandName> {
        let mut metadata = CommandMetadata::new("color", "Change the primitive color")?;
        metadata.arguments.push(bevy_promptline::ArgumentMetadata {
            name: "color".into(),
            help: "Choose blue, green, amber, or red".into(),
            values: ["blue", "green", "amber", "red"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        });
        Ok(metadata)
    }
    fn parse(arguments: &[String]) -> Result<Self, String> {
        match arguments {
            [value] => value.parse().map(Self),
            _ => Err("color expects one color name".to_owned()),
        }
    }
}

/// A validated rotation rate delivered to an ordinary game system.
#[derive(Debug)]
struct SpinCommand(SpinRate);
impl ConsoleCommand for SpinCommand {
    fn metadata() -> Result<CommandMetadata, bevy_promptline::InvalidCommandName> {
        CommandMetadata::new("spin", "Set rotation speed in rad/s. Usage: spin <-10..10>")
    }
    fn parse(arguments: &[String]) -> Result<Self, String> {
        match arguments {
            [value] => value.parse().map(Self),
            _ => Err("spin expects one rate in radians per second".to_owned()),
        }
    }
}

/// Writes direct output through the shared runtime.
fn echo(In(command): In<Echo>, mut output: ResMut<'_, Output>) -> CommandOutcome {
    output.write(command.0);
    CommandOutcome::Success
}

/// Applies the typed color only after parsing and authorization succeed.
fn color(
    In(command): In<ColorCommand>,
    mut materials: ResMut<'_, Assets<ColorMaterial>>,
    objects: Query<'_, '_, &MeshMaterial2d<ColorMaterial>, With<DemoPrimitive>>,
    mut output: ResMut<'_, Output>,
) -> CommandOutcome {
    for material in &objects {
        if let Some(mut material) = materials.get_mut(&material.0) {
            material.color = command.0.color();
        }
    }
    output.write("Primitive color changed");
    CommandOutcome::Success
}

/// Stores the validated rate without retaining its raw command spelling.
fn spin(
    In(command): In<SpinCommand>,
    mut rate: ResMut<'_, SpinRate>,
    mut output: ResMut<'_, Output>,
) -> CommandOutcome {
    *rate = command.0;
    output.write("Rotation speed changed");
    CommandOutcome::Success
}

/// Creates procedural scene content without external model or font downloads.
pub(crate) fn setup(
    mut commands: Commands<'_, '_>,
    mut meshes: ResMut<'_, Assets<Mesh>>,
    mut materials: ResMut<'_, Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);
    commands.spawn((
        Mesh2d(meshes.add(RegularPolygon::new(75.0, 3))),
        MeshMaterial2d(materials.add(SceneColor::Blue.color())),
        Transform::from_xyz(0.0, 70.0, 0.0),
        DemoPrimitive,
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb(0.16, 0.23, 0.27), Vec2::new(600.0, 8.0)),
        Transform::from_xyz(0.0, -60.0, 0.0),
    ));
    commands.spawn((
        KeyboardHint,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.9, 0.92)),
        Node {
            position_type: PositionType::Absolute,
            top: px(16),
            left: px(16),
            right: px(16),
            ..default()
        },
    ));
}

/// Converts rate times elapsed seconds into a rotation angle in radians.
pub(crate) fn rotate(
    time: Res<'_, Time>,
    rate: Res<'_, SpinRate>,
    mut objects: Query<'_, '_, &mut Transform, With<DemoPrimitive>>,
) {
    let angle_radians = rate.0 * time.delta_secs();
    for mut transform in &mut objects {
        transform.rotate_z(angle_radians);
    }
}

/// Updates the visible shortcut hint when the host changes its console bindings.
fn update_hint(
    bindings: Res<'_, ConsoleBindings>,
    mut hints: Query<'_, '_, &mut Text, With<KeyboardHint>>,
) {
    if !bindings.is_changed() {
        return;
    }
    let label = bindings.toggle.map_or_else(String::new, |key| {
        if key == KeyCode::Backquote {
            "Press ~ to open the console".into()
        } else {
            format!("Press {key:?} to open the console")
        }
    });
    for mut hint in &mut hints {
        hint.0.clone_from(&label);
    }
}

/// Installs the shared procedural scene and typed commands in a host-owned app.
pub(crate) fn install(app: &mut App) -> Result<(), bevy_promptline::RegistrationError> {
    app.init_resource::<SpinRate>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (rotate.after(PromptlineSystems::Input), update_hint),
        );
    app.register_command::<Echo, _>(echo)?;
    app.register_command::<ColorCommand, _>(color)?;
    app.register_command::<SpinCommand, _>(spin)?;
    Ok(())
}
