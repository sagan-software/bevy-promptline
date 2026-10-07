//! Independent top drawer with reversible motion and typed scene controls.
use bevy::prelude::*;
use bevy_promptline::input::{
    ConsoleMode, ConsoleRole, ConsoleState, PromptlineSystems, PromptlineUiPlugin,
};
use bevy_promptline::{CommandMetadata, CommandOutcome, ConsoleCommand, Output, PromptlineAppExt};
#[path = "support/playground.rs"]
mod playground;

/// Drawer progress is dimensionless and remains within the closed interval zero to one.
#[derive(Debug, Default, Resource)]
struct DrawerProgress(f32);
/// Motion preference chooses animation or immediate placement without changing visibility semantics.
#[derive(Debug, Default, Clone, Copy, Resource)]
enum Motion {
    /// Animate with a fixed 240-millisecond transition while preserving reversal position.
    #[default]
    Animated,
    /// Place the drawer immediately when reduced motion is requested.
    Reduced,
}
/// Closed scene actions affect only the shared visible procedural scene.
#[derive(Debug)]
enum SceneCommand {
    /// Replace the validated angular velocity, measured in radians per second.
    Speed(playground::SpinRate),
    /// Reset primitive orientation and its default angular velocity.
    Reset,
}
impl ConsoleCommand for SceneCommand {
    fn metadata() -> Result<CommandMetadata, bevy_promptline::InvalidCommandName> {
        let mut metadata =
            CommandMetadata::new("scene", "scene speed <-10..10 rad/s> | scene reset")?;
        metadata.arguments.push(bevy_promptline::ArgumentMetadata {
            name: "action".into(),
            help: "Choose speed or reset".into(),
            values: vec!["speed".into(), "reset".into()],
        });
        Ok(metadata)
    }
    fn parse(arguments: &[String]) -> Result<Self, String> {
        match arguments {
            [action, rate] if action == "speed" => rate.parse().map(Self::Speed),
            [action] if action == "reset" => Ok(Self::Reset),
            _ => Err("scene speed <rate in rad/s> | scene reset".into()),
        }
    }
}
/// Motion command chooses an explicit presentation mode through ordinary typed dispatch.
#[derive(Debug)]
struct MotionCommand(Motion);
impl ConsoleCommand for MotionCommand {
    fn metadata() -> Result<CommandMetadata, bevy_promptline::InvalidCommandName> {
        let mut metadata = CommandMetadata::new("motion", "motion animated | motion reduced")?;
        metadata.arguments.push(bevy_promptline::ArgumentMetadata {
            name: "mode".into(),
            help: "Reduced motion places the drawer immediately".into(),
            values: vec!["animated".into(), "reduced".into()],
        });
        Ok(metadata)
    }
    fn parse(arguments: &[String]) -> Result<Self, String> {
        match arguments {
            [mode] if mode == "animated" => Ok(Self(Motion::Animated)),
            [mode] if mode == "reduced" => Ok(Self(Motion::Reduced)),
            _ => Err("motion expects animated or reduced".into()),
        }
    }
}
/// Applies scene mutation only after the shared runtime accepts typed input.
fn scene(
    In(command): In<SceneCommand>,
    mut rate: ResMut<'_, playground::SpinRate>,
    mut objects: Query<'_, '_, &mut Transform, With<playground::DemoPrimitive>>,
    mut output: ResMut<'_, Output>,
) -> CommandOutcome {
    match command {
        SceneCommand::Speed(value) => *rate = value,
        SceneCommand::Reset => {
            *rate = default();
            for mut object in &mut objects {
                object.rotation = Quat::IDENTITY;
            }
        }
    }
    output.write("Scene updated");
    CommandOutcome::Success
}
/// Moves from the current progress so a toggle midway reverses continuously.
fn animate(
    time: Res<'_, Time>,
    state: Res<'_, ConsoleState>,
    motion: Res<'_, Motion>,
    mut progress: ResMut<'_, DrawerProgress>,
    mut roots: Query<'_, '_, (&ConsoleRole, &ComputedNode, &mut Node)>,
) {
    let target = if state.mode() == ConsoleMode::Closed {
        0.0
    } else {
        1.0
    };
    progress.0 = advance(progress.0, target, time.delta(), *motion);
    for (role, computed, mut node) in &mut roots {
        if *role != ConsoleRole::Root {
            continue;
        }
        node.display = if progress.0 > 0.0 {
            Display::Flex
        } else {
            Display::None
        };
        node.height = px(340);
        node.max_height = percent(66);
        node.bottom = Val::Auto;
        let height_pixels = if computed.size.y > 0.0 {
            computed.size.y * computed.inverse_scale_factor
        } else {
            340.0
        };
        node.top = px(-height_pixels * (1.0 - progress.0));
    }
}
/// Computes bounded progress using seconds divided by the 240-millisecond transition duration.
fn advance(progress: f32, target: f32, delta: std::time::Duration, motion: Motion) -> f32 {
    let transition = std::time::Duration::from_millis(240);
    let step = delta.as_secs_f32() / transition.as_secs_f32();
    match motion {
        Motion::Reduced => target,
        Motion::Animated => progress + (target - progress).clamp(-step, step),
    }
}

/// Runs an independent app; the library never owns its window or animation.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Bevy Promptline · quake drawer".into(),
            resolution: (1100, 720).into(),
            canvas: Some("#bevy".into()),
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    }))
    .add_plugins(PromptlineUiPlugin)
    .init_resource::<DrawerProgress>()
    .init_resource::<Motion>();
    playground::install(&mut app)?;
    app.register_command::<SceneCommand, _>(scene)?;
    app.register_command::<MotionCommand, _>(
        |In(command): In<MotionCommand>, mut motion: ResMut<'_, Motion>| {
            *motion = command.0;
            CommandOutcome::Success
        },
    )?;
    app.add_systems(Update, animate.after(PromptlineSystems::Input));
    app.run();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drawer_reverses_from_its_current_position_and_reduced_motion_is_immediate() {
        let delta = std::time::Duration::from_millis(60);
        let opening = advance(0.0, 1.0, delta, Motion::Animated);
        assert!((opening - 0.25).abs() < f32::EPSILON);
        let closing = advance(
            opening,
            0.0,
            std::time::Duration::from_millis(30),
            Motion::Animated,
        );
        assert!((closing - 0.125).abs() < f32::EPSILON);
        assert!((advance(closing, 1.0, delta, Motion::Animated) - 0.375).abs() < f32::EPSILON);
        assert!((advance(closing, 1.0, delta, Motion::Reduced) - 1.0).abs() < f32::EPSILON);
        assert!(advance(0.9, 1.0, delta, Motion::Animated) <= 1.0);
        assert!(advance(0.1, 0.0, delta, Motion::Animated) >= 0.0);
    }
}
