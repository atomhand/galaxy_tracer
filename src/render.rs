use super::galaxy_config::GalaxyConfig;
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef, ShaderType},
};
use bytemuck::{Pod, Zeroable};

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
    let mat = galaxy_materials.add(GalaxyVolumeMaterial::new(&galaxy_config));
    commands.spawn((
        Mesh3d(galaxy_mesh),
        Transform::IDENTITY,
        Visibility::Inherited,
        MeshMaterial3d(mat),
        bevy::render::view::NoFrustumCulling,
    ));
}

// GALAXY - VOLUME MATERIAL

// These structs are duplicated in intensity_shared.wgsl, so make sure to update both
#[derive(ShaderType, Pod, Zeroable, Clone, Copy, Debug)]
#[repr(C)]
struct GalaxyParams {
    arm_offsets : Vec4,
    radius : f32,
    num_arms : i32,
    winding_b : f32,
    winding_n : f32
}
#[derive(ShaderType, Pod, Zeroable, Clone, Copy, Debug)]
#[repr(C)]
struct BulgeParams {
    strength : f32,
    r0 : f32, // (inverse) width
}
#[derive(ShaderType, Pod, Zeroable, Clone, Copy, Debug)]
#[repr(C)]
struct ComponentParams {
    strength : f32,
    arm_width : f32, // inverse
    y0 : f32,
    r0 : f32, // radial intensity start
    r1 : f32, // radial falloff start
    angular_offset : f32,
    winding : f32,
    noise_scale : f32,
    noise_offset : f32,
    tilt : f32,
    ks : f32
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GalaxyVolumeMaterial {
    #[uniform(0)]
    galaxy_params: GalaxyParams,
    #[uniform(1)]
    bulge_params : BulgeParams,
    alpha_mode: AlphaMode,
}
impl GalaxyVolumeMaterial {
    pub fn new(galaxy_config : &GalaxyConfig) -> Self {
        Self {
            galaxy_params : GalaxyParams{
                radius : galaxy_config.radius,
                num_arms : galaxy_config.n_arms,
                arm_offsets : Vec4::from_array(galaxy_config.arm_offsets),
                winding_b : galaxy_config.winding_b,
                winding_n : galaxy_config.winding_n
            },
            bulge_params : BulgeParams {
                strength : 1.0,
                r0 : 0.2,
            },
            alpha_mode : AlphaMode::Add
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
