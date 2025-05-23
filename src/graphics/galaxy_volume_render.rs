use crate::prelude::*;
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::{
        render_resource::{AsBindGroup, Face, ShaderRef},
        view::RenderLayers,
    },
};

use super::shader_types::*;

pub struct GalaxyVolumePlugin;

impl Plugin for GalaxyVolumePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<GalaxyVolumeMaterial>::default());

        app.add_systems(Startup, setup_galaxy_volume)
            .add_systems(Update, (update_volume_material, update_galaxy_volume));
    }
}

#[derive(Component)]
struct GalaxyVolume;

fn setup_galaxy_volume(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut galaxy_materials: ResMut<Assets<GalaxyVolumeMaterial>>,
    galaxy_config: Res<GalaxyConfig>,
    galaxy_render_settings: Res<GalaxyRenderConfig>,
) {
    let galaxy_mesh = meshes.add(Sphere::new(1.0));
    let mat = galaxy_materials.add(GalaxyVolumeMaterial::new(
        &galaxy_config,
        &galaxy_render_settings,
    ));
    commands.spawn((
        Mesh3d(galaxy_mesh),
        Transform::IDENTITY,
        Visibility::Inherited,
        MeshMaterial3d(mat),
        GalaxyVolume,
        volume_upscaler::background_render_layer(),
        bevy::render::view::NoFrustumCulling,
    ));
}

fn update_galaxy_volume(
    mut commands: Commands,
    query: Query<Entity, With<GalaxyVolume>>,
    galaxy_render_settings: Res<GalaxyRenderConfig>,
) {
    if galaxy_render_settings.is_changed() {
    if let Ok(entity) = query.single() {
        commands
            .entity(entity)
            .insert(if galaxy_render_settings.draw_volume_to_background {
                volume_upscaler::background_render_layer()
            } else {
                RenderLayers::layer(0)
            });
    }
}

    }

fn update_volume_material(
    galaxy_mat: Query<&MeshMaterial3d<GalaxyVolumeMaterial>, With<GalaxyVolume>>,
    galaxy_texture: Res<super::GalaxyTexture>,
    galaxy_config: Res<GalaxyConfig>,
    galaxy_render_settings: Res<GalaxyRenderConfig>,
    mut galaxy_materials: ResMut<Assets<GalaxyVolumeMaterial>>,
) {
    if galaxy_texture.is_changed() || galaxy_render_settings.is_changed() {
        let Ok(galaxy) = galaxy_mat.single() else {
            return;
        };
        let Some(mat) = galaxy_materials.get_mut(&galaxy.0) else {
            return;
        };

        mat.update(&galaxy_config, &galaxy_render_settings);

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
    #[texture(4)]
    #[sampler(5)]
    xz_texture: Option<Handle<Image>>,
    #[texture(6, dimension = "2d_array")]
    #[sampler(7)]
    lut: Option<Handle<Image>>,
    //alpha_mode: AlphaMode,
    diagnostic_mode: bool,
}
impl GalaxyVolumeMaterial {
    pub fn update(
        &mut self,
        galaxy_config: &GalaxyConfig,
        galaxy_render_settings: &GalaxyRenderConfig,
    ) {
        self.galaxy_params = GalaxyParams::read(galaxy_config, galaxy_render_settings);
        self.bulge_params = BulgeParams::read(galaxy_config);
        self.disk_params = ComponentParams::read(&galaxy_config.disk_params);
        self.dust_params = ComponentParams::read(&galaxy_config.dust_params);
        self.diagnostic_mode = galaxy_render_settings.diagnostic_mode;
    }
    pub fn new(galaxy_config: &GalaxyConfig, galaxy_render_settings: &GalaxyRenderConfig) -> Self {
        let mut ret = Self::default();
        ret.update(galaxy_config, galaxy_render_settings);
        ret
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
        AlphaMode::Opaque
    }

    // prevents issues when rendering stars and galaxy volume together in the background camera
    // TODO: would be better to just disable depth write completely
    fn depth_bias(&self) -> f32 {
        -1000.0
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = Some(Face::Front);
        if key.bind_group_data.diagnostic_mode {
            let fragment = descriptor.fragment.as_mut().unwrap();
            fragment.shader_defs.push("DIAGNOSTIC".into());
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
}

impl From<&GalaxyVolumeMaterial> for GalaxyMaterialKey {
    fn from(material: &GalaxyVolumeMaterial) -> Self {
        Self {
            diagnostic_mode: material.diagnostic_mode,
        }
    }
}
