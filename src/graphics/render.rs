use crate::prelude::*;
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef, ShaderType},
};
use bytemuck::{Pod, Zeroable};

use super::shader_types::*;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<GalaxyVolumeMaterial>::default());

        app.add_systems(Startup, setup_galaxy_volume)
            .add_systems(Update, update_volume_material);
    }
}

#[derive(Component)]
struct GalaxyRenderer;

fn setup_galaxy_volume(
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

fn update_volume_material(
    galaxy_mat: Query<&MeshMaterial3d<GalaxyVolumeMaterial>, With<GalaxyRenderer>>,
    galaxy_texture: Res<super::GalaxyTexture>,
    galaxy_config: Res<GalaxyConfig>,
    mut galaxy_materials: ResMut<Assets<GalaxyVolumeMaterial>>,
    noise_images: Res<super::noise_texture::NoiseTextureImages>,
) {
    // it would be good to divorce parameter updates from texture updates I guess
    // the texture update is quite cheap so it's not huge deal though
    if galaxy_texture.is_changed() {
        let Ok(galaxy) = galaxy_mat.get_single() else {
            return;
        };
        let Some(mat) = galaxy_materials.get_mut(&galaxy.0) else {
            return;
        };

        mat.update(&galaxy_config);

        mat.disk_noise_texture = Some(noise_images.disk_component.clone());
        mat.dust_noise_texture = Some(noise_images.dust_component.clone());
        mat.dust_detail_texture = Some(noise_images.dust_detail.clone());

        mat.xz_texture = galaxy_texture.tex.clone();
        mat.lut = galaxy_texture.luts.clone();
    }
}

// GALAXY - VOLUME MATERIAL


#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
#[bind_group_data(GalaxyMaterialKey)]
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
    #[texture(7, dimension = "2d_array")]
    #[sampler(8)]
    lut: Option<Handle<Image>>,
    #[texture(9, dimension = "3d")]
    #[sampler(10)]
    disk_noise_texture: Option<Handle<Image>>,
    #[texture(11, dimension = "3d")]
    #[sampler(12)]
    dust_noise_texture: Option<Handle<Image>>,
    #[texture(13, dimension = "3d")]
    #[sampler(14)]
    dust_detail_texture: Option<Handle<Image>>,
    alpha_mode: AlphaMode,
    diagnostic_mode: bool,
    flat_mode: bool,
    runtime_noise: bool,
}
impl GalaxyVolumeMaterial {
    pub fn update(&mut self, galaxy_config: &GalaxyConfig) {
        self.galaxy_params = GalaxyParams::read(galaxy_config);
        self.bulge_params = BulgeParams::read(galaxy_config);
        self.disk_params = ComponentParams::read(&galaxy_config.disk_params);
        self.dust_params = ComponentParams::read(&galaxy_config.dust_params);
        self.stars_params = ComponentParams::read(&galaxy_config.stars_params);
        self.diagnostic_mode = galaxy_config.diagnostic_mode;
        self.flat_mode = galaxy_config.flat_mode;
        self.runtime_noise = galaxy_config.runtime_noise;
    }
    pub fn new(galaxy_config: &GalaxyConfig) -> Self {
        let mut ret = Self {
            alpha_mode: AlphaMode::Add,
            ..default()
        };
        ret.update(galaxy_config);
        return ret;
    }
}

use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::render::{
    mesh::MeshVertexBufferLayoutRef,
    render_resource::{RenderPipelineDescriptor, SpecializedMeshPipelineError},
};

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

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if key.bind_group_data.diagnostic_mode {
            let fragment = descriptor.fragment.as_mut().unwrap();
            fragment.shader_defs.push("DIAGNOSTIC".into());
        }
        if key.bind_group_data.runtime_noise {
            let fragment = descriptor.fragment.as_mut().unwrap();
            fragment.shader_defs.push("RUNTIME_NOISE".into());
        }
        if key.bind_group_data.flat_mode {
            let fragment: &mut bevy::render::render_resource::FragmentState =
                descriptor.fragment.as_mut().unwrap();
            fragment.shader_defs.push("FLAT_DIAGNOSTIC".into());
        }
        Ok(())
    }
}
// This key is used to identify a specific permutation of this material pipeline.
// In this case, we specialize on whether or not to configure the "IS_RED" shader def.
// Specialization keys should be kept as small / cheap to hash as possible,
// as they will be used to look up the pipeline for each drawn entity with this material type.
#[derive(Eq, PartialEq, Hash, Clone)]
pub struct GalaxyMaterialKey {
    diagnostic_mode: bool,
    runtime_noise: bool,
    flat_mode: bool,
}

impl From<&GalaxyVolumeMaterial> for GalaxyMaterialKey {
    fn from(material: &GalaxyVolumeMaterial) -> Self {
        Self {
            diagnostic_mode: material.diagnostic_mode,
            runtime_noise: material.runtime_noise,
            flat_mode: material.flat_mode,
        }
    }
}
