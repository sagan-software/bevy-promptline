//! Independent minimal integration without Clap or log capture.
use bevy::prelude::*;
use bevy_promptline::input::PromptlineUiPlugin;
#[path = "support/playground.rs"]
mod playground;

/// Owns the example's platform plugins, rendering, camera, and procedural scene.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Bevy Promptline · minimal".into(),
            resolution: (1100, 720).into(),
            canvas: Some("#bevy".into()),
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    }))
    .add_plugins(PromptlineUiPlugin);
    playground::install(&mut app)?;
    app.run();
    Ok(())
}
