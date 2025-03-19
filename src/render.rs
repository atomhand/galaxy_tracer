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

        app.add_systems(Startup, place_galaxy_volume)
            .add_systems(Update, update_volume_mat);
    }
}

#[derive(Component)]
struct GalaxyRenderer;

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
        GalaxyRenderer,
        bevy::render::view::NoFrustumCulling,
    ));
}

fn update_volume_mat(
    galaxy_mat: Query<&MeshMaterial3d<GalaxyVolumeMaterial>, With<GalaxyRenderer>>,
    galaxy_texture: Res<crate::galaxy_texture::GalaxyTexture>,
    galaxy_config: Res<GalaxyConfig>,
    mut galaxy_materials: ResMut<Assets<GalaxyVolumeMaterial>>,
) {
    if galaxy_texture.is_changed() {
        let Ok(galaxy) = galaxy_mat.get_single() else {
            return;
        };
        let Some(mat) = galaxy_materials.get_mut(&galaxy.0) else {
            return;
        };

        mat.galaxy_params = GalaxyParams {
            padding_coefficient: galaxy_config.padding_coeff,
            radius: galaxy_config.radius,
            num_arms: galaxy_config.n_arms,
            arm_offsets: Vec4::from_array(galaxy_config.arm_offsets),
            winding_b: galaxy_config.winding_b,
            winding_n: galaxy_config.winding_n,
            pad: Vec3::ZERO,
        };
        mat.bulge_params.r0 = galaxy_config.bulge_radius;
        mat.bulge_params.strength =galaxy_config.bulge_strength;
        mat.disk_params = ComponentParams::from(galaxy_config.disk_params.clone());
        mat.dust_params = ComponentParams::from(galaxy_config.dust_params.clone());
        mat.stars_params = ComponentParams::from(galaxy_config.stars_params.clone());

        mat.xz_texture = galaxy_texture.tex.clone();
    }
}

// GALAXY - VOLUME MATERIAL

// These structs are duplicated in intensity_shared.wgsl, so make sure to update both
#[derive(ShaderType, Pod, Zeroable, Clone, Copy, Debug)]
#[repr(C)]
struct GalaxyParams {
    arm_offsets: Vec4,
    radius: f32,
    num_arms: i32,
    winding_b: f32,
    winding_n: f32,
    padding_coefficient: f32,
    pad: Vec3,
}

#[derive(ShaderType, Pod, Zeroable, Clone, Copy, Debug)]
#[repr(C)]
struct BulgeParams {
    strength: f32,
    r0: f32, // (inverse) width
}
#[derive(ShaderType, Pod, Zeroable, Clone, Copy, Debug)]
#[repr(C)]
struct ComponentParams {
    strength: f32,
    arm_width: f32, // inverse
    y0: f32,
    r0: f32, // radial intensity start
    r1: f32, // radial falloff start
    angular_offset: f32,
    winding: f32,
    noise_scale: f32,
    noise_offset: f32,
    tilt: f32,
    ks: f32,
}
use crate::galaxy_config::ComponentConfig;
impl From<ComponentConfig> for ComponentParams {
    fn from(other: ComponentConfig) -> Self {
        Self {
            strength: other.strength,
            arm_width: other.arm_width,
            y0: other.y_offset,
            r0: other.radial_start,
            r1: other.radial_dropoff,
            angular_offset: other.delta_angle,
            winding: other.winding_coefficient,
            noise_scale: other.noise_scale,
            noise_offset: other.noise_offset,
            tilt: other.noise_tilt,
            ks: other.noise_freq,
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GalaxyVolumeMaterial {
    #[uniform(0)]
    galaxy_params: GalaxyParams,
    #[uniform(1)]
    bulge_params: BulgeParams,
    #[uniform(2)]
    disk_params: ComponentParams,
    #[uniform(3)]
    dust_params: ComponentParams,
    #[uniform(4)]
    stars_params: ComponentParams,
    #[texture(5)]
    #[sampler(6)]
    xz_texture: Option<Handle<Image>>,
    alpha_mode: AlphaMode,
}
impl GalaxyVolumeMaterial {
    pub fn new(galaxy_config: &GalaxyConfig) -> Self {
        Self {
            galaxy_params: GalaxyParams {
                padding_coefficient: galaxy_config.padding_coeff,
                radius: galaxy_config.radius,
                num_arms: galaxy_config.n_arms,
                arm_offsets: Vec4::from_array(galaxy_config.arm_offsets),
                winding_b: galaxy_config.winding_b,
                winding_n: galaxy_config.winding_n,
                pad: Vec3::ZERO,
            },
            bulge_params: BulgeParams {
                strength: galaxy_config.bulge_strength,
                r0: galaxy_config.bulge_radius,
            },
            disk_params: ComponentParams::from(galaxy_config.disk_params.clone()),
            dust_params: ComponentParams::from(galaxy_config.dust_params.clone()),
            stars_params: ComponentParams::from(galaxy_config.stars_params.clone()),
            alpha_mode: AlphaMode::Add,
            xz_texture: None,
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
