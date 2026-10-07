//! Procedural geometry keeps screen effects on the console texture rather than the surrounding scene.
use super::settings::Settings;
use bevy::asset::RenderAssetUsages;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, TextureDimension, TextureFormat, TextureUsages,
};
use bevy::shader::ShaderRef;
use bevy_promptline::input::{ConsoleBindings, ConsoleRole, ConsoleState, ConsoleTheme};

/// Source framebuffer dimensions are fixed pixel counts and never reallocate during viewport resize.
pub(crate) const TEXTURE_SIZE: Vec2 = Vec2::new(1024.0, 640.0);
/// Physical screen width is measured in scene meters.
pub(crate) const SCREEN_WIDTH: f32 = 0.96;
/// Physical screen height is measured in scene meters.
pub(crate) const SCREEN_HEIGHT: f32 = 0.60;
/// Center of the screen plane uses scene meters with positive Z facing the viewer.
pub(crate) const SCREEN_CENTER: Vec3 = Vec3::new(0.0, 1.36, 0.27);
/// Only the glass plane receives raycast input and the custom screen material.
#[derive(Component)]
pub(crate) struct Screen;
/// Main camera is distinct from the camera that renders actual Bevy UI into the framebuffer.
#[derive(Component)]
pub(crate) struct MainCamera;
/// Prompt text stays separate from editable input while reflecting current virtual directory state.
#[derive(Component)]
pub(super) struct Prompt;
/// Camera view has two explicit states rather than unrelated zoom and editing flags.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Resource)]
pub(crate) enum View {
    /// Wider desk view keeps the procedural workstation visible.
    #[default]
    Desk,
    /// Close-up view makes real console editing readable.
    Editing,
}
/// Target handles route public console roles to the offscreen camera.
#[derive(Resource)]
pub(crate) struct ScreenTarget {
    /// Camera used by the actual Bevy UI renderer.
    pub(crate) camera: Entity,
    /// Image sampled by the screen shader and virtual pointer target.
    pub(crate) image: Handle<Image>,
}
/// Aligned material vectors serialize validated controls into WebGL2-compatible uniforms.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(crate) struct CrtMaterial {
    /// Dimensionless brightness, scanline strength, effective curvature, and effect mode.
    #[uniform(0)]
    settings: Vec4,
    /// Linear RGBA phosphor multiplier.
    #[uniform(1)]
    tint: Vec4,
    /// Actual offscreen Bevy UI framebuffer.
    #[texture(2)]
    #[sampler(3)]
    image: Handle<Image>,
}
impl Material for CrtMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/promptline_crt.wgsl".into()
    }
}
/// Creates all scene assets locally; coordinates and mesh dimensions use scene meters.
pub(crate) fn setup(
    mut commands: Commands<'_, '_>,
    mut meshes: ResMut<'_, Assets<Mesh>>,
    mut materials: ResMut<'_, Assets<StandardMaterial>>,
    mut screens: ResMut<'_, Assets<CrtMaterial>>,
    mut images: ResMut<'_, Assets<Image>>,
    settings: Res<'_, Settings>,
) {
    let mut image = Image::new_fill(
        Extent3d {
            width: 1024,
            height: 640,
            ..default()
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let image = images.add(image);
    let ui_camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: -1,
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            RenderTarget::Image(image.clone().into()),
        ))
        .id();
    commands.insert_resource(ScreenTarget {
        camera: ui_camera,
        image: image.clone(),
    });
    let screen_material = screens.add(CrtMaterial {
        settings: settings.uniform(),
        tint: settings.tint.uniform(),
        image,
    });
    commands.spawn((
        Screen,
        Mesh3d(meshes.add(Rectangle::new(SCREEN_WIDTH, SCREEN_HEIGHT))),
        MeshMaterial3d(screen_material),
        Transform::from_translation(SCREEN_CENTER),
    ));
    workstation_geometry(&mut commands, &mut meshes, &mut materials);
    // Light intensity uses lumens; range and positions use scene meters.
    commands.spawn((
        PointLight {
            intensity: 5500.0,
            range: 8.0,
            shadow_maps_enabled: true,
            color: Color::srgb(1.0, 0.76, 0.51),
            ..default()
        },
        Transform::from_xyz(-1.0, 2.7, 1.5),
    ));
    commands.spawn((
        PointLight {
            intensity: 750.0,
            range: 5.0,
            color: Color::srgb(0.4, 0.6, 0.8),
            ..default()
        },
        Transform::from_xyz(1.5, 2.0, -0.8),
    ));
    let main = commands
        .spawn((
            MainCamera,
            Camera3d::default(),
            bevy::camera::Exposure::INDOOR,
            desk_transform(),
        ))
        .id();
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(16),
                right: px(16),
                padding: px(10).all(),
                ..default()
            },
            Button,
            BackgroundColor(Color::srgb(0.08, 0.12, 0.1)),
            UiTargetCamera(main),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Edit screen"),
                TextColor(Color::WHITE),
                Pickable::IGNORE,
            ));
        })
        .observe(
            |_: On<'_, '_, Pointer<Click>>,
             mut state: ResMut<'_, ConsoleState>,
             mut view: ResMut<'_, View>| {
                state.open();
                *view = View::Editing;
            },
        );
}
/// Builds the workstation around the fixed screen plane; all dimensions and positions use meters.
fn workstation_geometry(
    commands: &mut Commands<'_, '_>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let beige = materials.add(StandardMaterial {
        base_color: Color::srgb(0.65, 0.61, 0.48),
        perceptual_roughness: 0.75,
        ..default()
    });
    let dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.10, 0.085),
        perceptual_roughness: 0.6,
        ..default()
    });
    let wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.105, 0.045),
        perceptual_roughness: 0.55,
        ..default()
    });
    // Desk, legs, monitor depth, stand, case, and keyboard all surround the physical glass plane.
    box_mesh(
        commands,
        meshes,
        wood,
        Vec3::new(2.8, 0.1, 1.3),
        Vec3::new(0.0, 0.76, 0.0),
    );
    for x in [-1.1, 1.1] {
        box_mesh(
            commands,
            meshes,
            dark.clone(),
            Vec3::new(0.08, 0.75, 0.08),
            Vec3::new(x, 0.36, 0.25),
        );
    }
    box_mesh(
        commands,
        meshes,
        beige.clone(),
        Vec3::new(1.17, 0.82, 0.40),
        Vec3::new(0.0, 1.35, 0.02),
    );
    // Thin bezel bars preserve visible depth while the screen remains a raycastable rectangle.
    for x in [-0.525, 0.525] {
        box_mesh(
            commands,
            meshes,
            dark.clone(),
            Vec3::new(0.08, 0.69, 0.05),
            Vec3::new(x, 1.36, 0.245),
        );
    }
    for y in [1.02, 1.70] {
        box_mesh(
            commands,
            meshes,
            dark.clone(),
            Vec3::new(1.1, 0.07, 0.05),
            Vec3::new(0.0, y, 0.245),
        );
    }
    box_mesh(
        commands,
        meshes,
        beige.clone(),
        Vec3::new(0.36, 0.20, 0.28),
        Vec3::new(0.0, 0.94, 0.0),
    );
    box_mesh(
        commands,
        meshes,
        beige.clone(),
        Vec3::new(0.52, 0.12, 0.40),
        Vec3::new(0.0, 0.85, 0.03),
    );
    box_mesh(
        commands,
        meshes,
        beige.clone(),
        Vec3::new(0.46, 0.76, 0.65),
        Vec3::new(0.9, 1.18, -0.02),
    );
    box_mesh(
        commands,
        meshes,
        dark.clone(),
        Vec3::new(0.31, 0.035, 0.03),
        Vec3::new(0.9, 1.37, 0.32),
    );
    box_mesh(
        commands,
        meshes,
        beige,
        Vec3::new(0.82, 0.055, 0.28),
        Vec3::new(0.0, 0.845, 0.48),
    );
    for row in 0..4u8 {
        for column in 0..12u8 {
            let position = Vec3::new(
                f32::from(column).mul_add(0.061, -0.36),
                0.881,
                f32::from(row).mul_add(0.059, 0.39),
            );
            box_mesh(
                commands,
                meshes,
                dark.clone(),
                Vec3::new(0.049, 0.017, 0.044),
                position,
            );
        }
    }
}

/// Adds one procedural box without downloading or copying another game's assets.
fn box_mesh(
    commands: &mut Commands<'_, '_>,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    size: Vec3,
    position: Vec3,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(size))),
        MeshMaterial3d(material),
        Transform::from_translation(position),
    ));
}
/// Targets only newly created console roles, including the access control outside its root.
pub(crate) fn attach_console(
    mut commands: Commands<'_, '_>,
    target: Option<Res<'_, ScreenTarget>>,
    roles: Query<'_, '_, (Entity, &ConsoleRole), Added<ConsoleRole>>,
    mut nodes: Query<'_, '_, &mut Node>,
    mut state: ResMut<'_, ConsoleState>,
) {
    let Some(target) = target else {
        return;
    };
    let input = roles
        .iter()
        .find_map(|(entity, role)| (*role == ConsoleRole::Input).then_some(entity));
    for (entity, role) in &roles {
        if matches!(role, ConsoleRole::Root | ConsoleRole::OpenControl) {
            commands
                .entity(entity)
                .insert(UiTargetCamera(target.camera));
        }
        if *role == ConsoleRole::Root {
            if let Ok(mut node) = nodes.get_mut(entity) {
                node.top = px(0);
                node.bottom = Val::Auto;
                node.height = percent(100);
                node.max_height = percent(100);
            }
            if let Some(input) = input {
                let row = commands
                    .spawn(Node {
                        width: percent(100),
                        height: px(32),
                        min_height: px(32),
                        align_items: AlignItems::Center,
                        column_gap: px(8),
                        ..default()
                    })
                    .id();
                let prompt = commands
                    .spawn((
                        Prompt,
                        Text::new("C:\\>"),
                        TextColor(Color::WHITE),
                        TextFont {
                            font_size: FontSize::Px(22.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(row).add_children(&[prompt, input]);
                commands.entity(entity).insert_children(1, &[row]);
                if let Ok(mut node) = nodes.get_mut(input) {
                    node.width = Val::Auto;
                    node.flex_grow = 1.0;
                }
            }
            state.show();
        }
    }
}
/// Escaping releases focus before the wider camera pose is applied.
pub(crate) fn release(keys: Res<'_, ButtonInput<KeyCode>>, mut state: ResMut<'_, ConsoleState>) {
    if keys.just_pressed(KeyCode::Escape) && state.is_editing() {
        state.release_focus();
    }
}
/// Camera pose derives from editing ownership after console requests have applied.
pub(crate) fn camera(
    state: Res<'_, ConsoleState>,
    mut view: ResMut<'_, View>,
    mut cameras: Query<'_, '_, &mut Transform, With<MainCamera>>,
) {
    *view = if state.is_editing() {
        View::Editing
    } else {
        View::Desk
    };
    for mut transform in &mut cameras {
        *transform = match *view {
            View::Desk => desk_transform(),
            View::Editing => {
                Transform::from_xyz(0.0, 1.36, 1.55).looking_at(SCREEN_CENTER, Vec3::Y)
            }
        };
    }
}
/// Material uniforms copy from the same validated settings authority used by pointer mapping.
pub(crate) fn update_material(
    settings: Res<'_, Settings>,
    mut materials: ResMut<'_, Assets<CrtMaterial>>,
) {
    if !settings.is_changed() {
        return;
    }
    for (_, material) in materials.iter_mut() {
        material.settings = settings.uniform();
        material.tint = settings.tint.uniform();
    }
}
/// Current directory changes only this display label, leaving the actual editor untouched.
pub(crate) fn update_prompt(
    fs: Res<'_, super::dos::VirtualFs>,
    mut prompts: Query<'_, '_, &mut Text, With<Prompt>>,
) {
    if !fs.is_changed() {
        return;
    }
    for mut prompt in &mut prompts {
        prompt.0 = fs.prompt().into();
    }
}
/// Wider view includes the desk, case, monitor depth, and keyboard.
fn desk_transform() -> Transform {
    Transform::from_xyz(1.75, 1.72, 2.85).looking_at(Vec3::new(0.25, 1.14, 0.0), Vec3::Y)
}
/// Applies a restrained source texture palette; the shader owns phosphor color.
pub(crate) fn theme() -> ConsoleTheme {
    ConsoleTheme {
        font: TextFont {
            font_size: FontSize::Px(22.0),
            ..default()
        },
        background: Color::srgb(0.005, 0.008, 0.005),
        input: Color::WHITE,
        output: Color::WHITE,
        help: Color::srgb(0.8, 0.8, 0.8),
        ghost: Color::srgb(0.55, 0.55, 0.55),
        ..default()
    }
}
/// CRT owns Escape release, while the generic close action remains explicitly rebound.
pub(crate) fn bindings() -> ConsoleBindings {
    ConsoleBindings {
        toggle: None,
        close: KeyCode::F12,
        ..default()
    }
}
