//! Independent workstation owns its cameras, physical scene, material, and virtual filesystem.
mod dos;
mod mapping;
mod pointer;
mod scene;
mod settings;
use bevy::picking::PickingSystems;
use bevy::prelude::*;
use bevy_promptline::input::{PromptlineSystems, PromptlineUiPlugin};

/// Runs the independent native or WebGL2 app without adding 3D dependencies to the default library.
pub(crate) fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.005, 0.006, 0.008)))
        .insert_resource(scene::theme())
        .insert_resource(scene::bindings())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Promptline · CRT workstation".into(),
                resolution: (1200, 800).into(),
                canvas: Some("#bevy".into()),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            PromptlineUiPlugin,
            MaterialPlugin::<scene::CrtMaterial>::default(),
        ))
        .init_resource::<scene::View>();
    dos::install(&mut app)?;
    settings::install(&mut app)?;
    app.add_systems(Startup, (scene::setup, spawn_pointer));
    app.add_systems(First, pointer::drive.in_set(PickingSystems::Input));
    app.add_systems(
        Update,
        (
            scene::attach_console,
            scene::release.before(PromptlineSystems::Input),
            scene::camera.after(PromptlineSystems::Input),
        ),
    );
    app.add_systems(
        PostUpdate,
        (scene::update_material, scene::update_prompt).after(PromptlineSystems::Dispatch),
    );
    app.run();
    Ok(())
}
/// Creates the virtual pointer before picking begins, with its required location components.
fn spawn_pointer(mut commands: Commands<'_, '_>) {
    commands.spawn(pointer::VIRTUAL_POINTER);
}
