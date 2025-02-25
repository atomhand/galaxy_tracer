use super::galaxy_config::GalaxyConfig;
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<GalaxyVolumeMaterial>::default());

        app.add_systems(Startup, place_galaxy_volume);
    }
}

fn place_galaxy_volume(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut galaxy_materials: ResMut<Assets<GalaxyVolumeMaterial>>,
    galaxy_config: Res<GalaxyConfig>,
) {
    let galaxy_mesh = meshes.add(Cuboid::from_size(Vec3::splat(2.0)));
    let mat = galaxy_materials.add(GalaxyVolumeMaterial::new(galaxy_config.radius));
    commands.spawn((
        Mesh3d(galaxy_mesh),
        Transform::IDENTITY,
        Visibility::Inherited,
        MeshMaterial3d(mat),
        bevy::render::view::NoFrustumCulling,
    ));
}

// GALAXY - VOLUME MATERIAL

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GalaxyVolumeMaterial {
    #[uniform(0)]
    pub radius: f32,
    alpha_mode: AlphaMode,
}
impl GalaxyVolumeMaterial {
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            alpha_mode: AlphaMode::Add,
        }
    }
}

impl Material for GalaxyVolumeMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/shader_galaxy_volume.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "shaders/shader_galaxy_volume.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }
}
