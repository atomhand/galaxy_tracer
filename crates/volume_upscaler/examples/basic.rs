use bevy::{prelude::*, render::view::RenderLayers};
use volume_upscaler::{BackgroundCamera,BackgroundRenderingPlugin,background_render_layer,BACKGROUND_RENDER_LAYER};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BackgroundRenderingPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, rotator_system)
        .run();
}

// Marks the first pass cube (rendered to a texture.)
#[derive(Component)]
struct FirstPassCube;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Cuboid::new(4.0, 4.0, 4.0));
    let cube_material_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.7, 0.6),
        reflectance: 0.02,
        unlit: false,
        ..default()
    });

    // The cube that will be rendered to the texture.
    commands.spawn((
        Mesh3d(cube_handle.clone()),
        MeshMaterial3d(cube_material_handle.clone()),
        Transform::from_translation(Vec3::new(3.0, 0.0, 4.0)).with_scale(Vec3::splat(1.)),
        FirstPassCube,
        background_render_layer(),
    ));
    // comparison cube
    commands.spawn((
        Mesh3d(cube_handle.clone()),
        MeshMaterial3d(cube_material_handle.clone()),
        Transform::from_translation(Vec3::new(-3.0, 0.0, 4.0)).with_scale(Vec3::splat(1.)),
        FirstPassCube,
    ));
    // Light
    // NOTE: we add the light to both layers so it affects both the rendered-to-texture cube, and the cube on which we display the texture
    // Setting the layer to RenderLayers::layer(0) would cause the main view to be lit, but the rendered-to-texture cube to be unlit.
    // Setting the layer to RenderLayers::layer(1) would cause the rendered-to-texture cube to be lit, but the main view to be unlit.
    commands.spawn((
        PointLight::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        RenderLayers::layer(0).with(BACKGROUND_RENDER_LAYER),
    ));

    // The main pass camera.
    commands.spawn((
        Msaa::Off,
        Camera3d::default(),
        BackgroundCamera,
        Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Rotates the inner cube (first pass)
fn rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<FirstPassCube>>) {
    for mut transform in &mut query {
        if time.elapsed_secs() % 2. < 0.5 {
            transform.rotate_x(1.5 * time.delta_secs());
            transform.rotate_z(1.3 * time.delta_secs());
        }
    }
}
