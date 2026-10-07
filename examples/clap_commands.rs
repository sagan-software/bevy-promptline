//! Independent derive and builder integration with live game-model completion.
use bevy::prelude::*;
use bevy_promptline::clap::ClapAppExt;
use bevy_promptline::input::{Candidate, CompletionRequest, PromptlineUiPlugin};
use bevy_promptline::{CommandOutcome, Output, PromptlineAppExt};
use clap::{Parser, Subcommand};
use std::str::FromStr;
#[path = "support/playground.rs"]
mod playground;

/// Validated model identity keeps model lookup distinct from command registration names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ModelName(bevy_promptline::CommandName);
impl FromStr for ModelName {
    type Err = bevy_promptline::InvalidCommandName;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}
impl AsRef<str> for ModelName {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
/// Scene model handles are game data, not static parser choices.
#[derive(Debug, Default, Resource)]
struct Models(std::collections::BTreeMap<ModelName, Handle<Mesh>>);
/// Spawn count stays positive and bounds one request to twelve visible objects.
#[derive(Debug, Clone, Copy)]
struct SpawnCount(std::num::NonZeroU8);
impl FromStr for SpawnCount {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let count = value
            .parse::<std::num::NonZeroU8>()
            .map_err(|_| "count must be 1 through 12".to_owned())?;
        if count.get() > 12 {
            return Err("count must be 1 through 12".into());
        }
        Ok(Self(count))
    }
}
/// Dimensionless scene brightness remains finite and constrained to zero through two.
#[derive(Debug, Clone, Copy)]
struct Brightness(f32);
impl FromStr for Brightness {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let brightness = value
            .parse::<f32>()
            .map_err(|_| "brightness must be a number".to_owned())?;
        if !brightness.is_finite() || !(0.0..=2.0).contains(&brightness) {
            return Err("brightness must be finite and between 0 and 2".into());
        }
        Ok(Self(brightness))
    }
}
/// Existing derive parser declares nested actions while the plugin owns no process exit.
#[derive(Debug, Parser)]
#[command(
    name = "scene",
    version,
    about = "Spawn models, set brightness, or change rotation"
)]
struct Scene {
    /// Exactly one nested scene action supplies its own validated typed payload.
    #[command(subcommand)]
    action: Action,
}
/// Scene actions remain distinct types instead of combining incompatible optional payloads.
#[derive(Debug, Subcommand)]
enum Action {
    /// Spawn a currently available game model with a bounded positive count.
    #[command(visible_alias = "add")]
    Spawn {
        /// Model identity is resolved against the live game catalog after parsing.
        model: ModelName,
        /// Positive number of objects created by this one request, capped at twelve.
        #[arg(long, short, default_value = "1", help = "Number of objects, 1..12")]
        count: SpawnCount,
    },
    /// Apply a finite dimensionless brightness multiplier to the visible scene colors.
    Light {
        /// Brightness scales linear RGB values and has no physical lighting unit.
        #[arg(long, default_value = "1", help = "Dimensionless brightness, 0..2")]
        brightness: Brightness,
    },
    /// Set finite angular velocity while preserving negative-number parsing semantics.
    Rotate {
        /// Angular velocity is measured in radians per second, bounded to -10 through 10.
        #[arg(
            long,
            allow_negative_numbers = true,
            help = "Angular velocity in rad/s, -10..10"
        )]
        speed: playground::SpinRate,
    },
}
/// Allocates procedural model assets and exposes their names through game-owned data.
fn model_assets(mut models: ResMut<'_, Models>, mut meshes: ResMut<'_, Assets<Mesh>>) {
    for (name, sides) in [("triangle", 3), ("square", 4), ("hexagon", 6)] {
        models.0.insert(
            name.parse().expect("static model identity"),
            meshes.add(RegularPolygon::new(45.0, sides)),
        );
    }
}
/// Runs accepted scene effects through an ordinary Bevy system with typed input.
fn scene(
    In(scene): In<Scene>,
    mut commands: Commands<'_, '_>,
    models: Res<'_, Models>,
    mut materials: ResMut<'_, Assets<ColorMaterial>>,
    objects: Query<'_, '_, &MeshMaterial2d<ColorMaterial>, With<playground::DemoPrimitive>>,
    mut rate: ResMut<'_, playground::SpinRate>,
    mut output: ResMut<'_, Output>,
) -> CommandOutcome {
    match scene.action {
        Action::Spawn { model, count } => {
            let Some(mesh) = models.0.get(&model) else {
                output.write("That model is not currently available");
                return CommandOutcome::Failure("model unavailable".into());
            };
            if objects.iter().count() + usize::from(count.0.get()) > 64 {
                output.write("Scene object limit reached (64)");
                return CommandOutcome::Failure("scene object limit".into());
            }
            for index in 0..count.0.get() {
                // Index times 80 logical scene pixels places each requested object apart.
                let x = f32::from(index).mul_add(80.0, -300.0);
                commands.spawn((
                    Mesh2d(mesh.clone()),
                    MeshMaterial2d(materials.add(Color::srgb(0.8, 0.5, 0.25))),
                    Transform::from_xyz(x, 200.0, 0.0),
                    playground::DemoPrimitive,
                ));
            }
            output.write("Models spawned");
        }
        Action::Light { brightness } => {
            for handle in &objects {
                if let Some(mut material) = materials.get_mut(&handle.0) {
                    material.color =
                        Color::from(LinearRgba::new(0.22, 0.62, 0.92, 1.0) * brightness.0);
                }
            }
            output.write("Scene brightness changed");
        }
        Action::Rotate { speed } => {
            *rate = speed;
            output.write("Rotation changed");
        }
    }
    CommandOutcome::Success
}
/// Read-only game data supplies model candidates without invoking Clap validators.
fn complete_models(
    InRef(request): InRef<'_, CompletionRequest>,
    models: Res<'_, Models>,
) -> Vec<Candidate> {
    let is_spawn = request
        .tokens()
        .get(1)
        .is_some_and(|token| matches!(token.value.as_str(), "spawn" | "add"));
    if !is_spawn || request.position() != 2 {
        return Vec::new();
    }
    models
        .0
        .keys()
        .take(64)
        .map(|name| Candidate {
            value: name.as_ref().to_owned(),
            description: "Available procedural model".into(),
        })
        .collect()
}
/// Builder output preserves decoded values, including leading hyphens after the end-of-options marker.
fn announce(In(matches): In<clap::ArgMatches>, mut output: ResMut<'_, Output>) -> CommandOutcome {
    let mut text = String::new();
    for part in matches.get_many::<String>("text").into_iter().flatten() {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(part);
    }
    output.write(text);
    CommandOutcome::Success
}
/// Runs a separate app and artifact with no custom parser macro in the library.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Bevy Promptline · Clap commands".into(),
            resolution: (1100, 720).into(),
            canvas: Some("#bevy".into()),
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    }))
    .add_plugins(PromptlineUiPlugin)
    .init_resource::<Models>();
    playground::install(&mut app)?;
    app.add_systems(Startup, model_assets);
    app.register_clap::<Scene, _>(scene)?;
    app.register_completion_system("scene", complete_models)?;
    app.register_clap_builder(
        clap::Command::new("announce")
            .visible_alias("say")
            .version("1.0")
            .about("Print decoded text, including values after --")
            .arg(clap::Arg::new("text").num_args(1..).required(true)),
        announce,
    )?;
    app.run();
    Ok(())
}
