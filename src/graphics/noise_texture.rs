use crate::prelude::*;
use bevy::{
    image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::{RenderAssetUsages, RenderAssets},
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{
            binding_types::texture_3d, binding_types::texture_storage_2d,
            binding_types::uniform_buffer, *,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::GpuImage,
        Render, RenderApp, RenderSet,
    },
};
use bytemuck::{Pod, Zeroable};
use std::borrow::Cow;

const SHADER_ASSET_PATH: &str = "shaders/noise_compute.wgsl";

const SIZE: (u32, u32, u32) = (64, 16, 64);
const WORKGROUP_SIZE: u32 = 8;

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut image = Image::new_fill(
        Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: SIZE.2,
        },
        TextureDimension::D3,
        &[0, 0, 0, 255],
        TextureFormat::R32Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    // wrapping sampling
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    });
    image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let image0 = images.add(image.clone());
    let image1 = images.add(image);
    commands.insert_resource(NoiseTextureImages {
        disk_component: image0,
        dust_component: image1,
    });
}

pub struct NoiseTexturePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct NoiseTextureLabel;

impl Plugin for NoiseTexturePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<NoiseTextureImages>::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<NoiseSettingsBuffers>()
            .add_systems(
                Render,
                prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
            )
            .add_systems(
                Render,
                prepare_noise_settings_buffers.in_set(RenderSet::PrepareResources),
            );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(NoiseTextureLabel, NoiseUpdateNode::default());
        render_graph.add_node_edge(NoiseTextureLabel, bevy::render::graph::CameraDriverLabel);

        app.add_systems(Startup, setup);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<NoiseTexturePipeline>();
    }
}

#[derive(Resource, Default, Clone, ExtractResource)]
pub struct NoiseTextureImages {
    pub disk_component: Handle<Image>,
    pub dust_component: Handle<Image>,
}

#[derive(Resource, Default)]
struct NoiseSettingsBuffers {
    disk_settings: UniformBuffer<NoiseSettingsUniform>,
    dust_settings: UniformBuffer<NoiseSettingsUniform>,
}

fn prepare_noise_settings_buffers(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    galaxy_config: Res<GalaxyConfig>,
    mut noise_settings_buffer: ResMut<NoiseSettingsBuffers>,
) {
    let disk_settings = noise_settings_buffer.disk_settings.get_mut();
    disk_settings.persistence = galaxy_config.disk_params.noise_persistence;
    disk_settings.scale = galaxy_config.disk_params.noise_scale;
    disk_settings.octaves = galaxy_config.disk_params.noise_octaves as f32;
    disk_settings.tilt = galaxy_config.disk_params.noise_tilt;
    disk_settings.offset = galaxy_config.disk_params.noise_offset;

    let dust_settings = noise_settings_buffer.dust_settings.get_mut();
    dust_settings.persistence = galaxy_config.dust_params.noise_persistence;
    dust_settings.scale = galaxy_config.dust_params.noise_scale;
    dust_settings.octaves = galaxy_config.dust_params.noise_octaves as f32;
    dust_settings.tilt = galaxy_config.dust_params.noise_tilt;
    dust_settings.offset = galaxy_config.dust_params.noise_offset;

    noise_settings_buffer
        .disk_settings
        .write_buffer(&render_device, &render_queue);
    noise_settings_buffer
        .dust_settings
        .write_buffer(&render_device, &render_queue);
}

#[derive(Resource)]
struct NoiseTextureImageBindGroups([BindGroup; 1]);

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<NoiseTexturePipeline>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    noise_texture_images: Res<NoiseTextureImages>,
    noise_settings_buffers: Res<NoiseSettingsBuffers>,
    render_device: Res<RenderDevice>,
) {
    let view_a = gpu_images
        .get(&noise_texture_images.disk_component)
        .unwrap();
    let view_b = gpu_images
        .get(&noise_texture_images.dust_component)
        .unwrap();

    let uniform_a = noise_settings_buffers.disk_settings.binding().unwrap();
    let uniform_b = noise_settings_buffers.dust_settings.binding().unwrap();

    let bind_group_0 = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::sequential((
            &view_a.texture_view,
            &view_b.texture_view,
            uniform_a.clone(),
            uniform_b.clone(),
        )),
    );
    /*
    let bind_group_1 = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::sequential((&view_b.texture_view, &view_a.texture_view)),
    );
    */
    commands.insert_resource(NoiseTextureImageBindGroups([bind_group_0]));
}

#[derive(ShaderType, Pod, Zeroable, Clone, Copy, Debug, Default)]
#[repr(C)]
struct NoiseSettingsUniform {
    persistence: f32,
    scale: f32,
    offset: f32,
    tilt: f32,
    octaves: f32,
}

#[derive(Resource)]
struct NoiseTexturePipeline {
    texture_bind_group_layout: BindGroupLayout,
    octave_noise_pipeline: CachedComputePipelineId,
    ridge_noise_pipeline: CachedComputePipelineId,
}

impl FromWorld for NoiseTexturePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let texture_bind_group_layout = render_device.create_bind_group_layout(
            "NoiseTextureImages",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    BindingType::StorageTexture {
                        access: StorageTextureAccess::WriteOnly,
                        format: TextureFormat::R32Float,
                        view_dimension: TextureViewDimension::D3,
                    },
                    BindingType::StorageTexture {
                        access: StorageTextureAccess::WriteOnly,
                        format: TextureFormat::R32Float,
                        view_dimension: TextureViewDimension::D3,
                    },
                    uniform_buffer::<NoiseSettingsUniform>(false),
                    uniform_buffer::<NoiseSettingsUniform>(false),
                ),
            ),
        );
        let shader = world.load_asset(SHADER_ASSET_PATH);
        let pipeline_cache = world.resource::<PipelineCache>();
        let octave_noise_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: None,
                layout: vec![texture_bind_group_layout.clone()],
                push_constant_ranges: Vec::new(),
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: Cow::from("cache_octave_noise"),
                zero_initialize_workgroup_memory: false,
            });

        let ridge_noise_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: None,
                layout: vec![texture_bind_group_layout.clone()],
                push_constant_ranges: Vec::new(),
                shader,
                shader_defs: vec![],
                entry_point: Cow::from("cache_ridge_noise"),
                zero_initialize_workgroup_memory: false,
            });

        NoiseTexturePipeline {
            texture_bind_group_layout,
            octave_noise_pipeline,
            ridge_noise_pipeline,
        }
    }
}

enum NoiseUpdateState {
    Loading,
    Ready,
}

struct NoiseUpdateNode {
    state: NoiseUpdateState,
}

impl Default for NoiseUpdateNode {
    fn default() -> Self {
        Self {
            state: NoiseUpdateState::Loading,
        }
    }
}

impl render_graph::Node for NoiseUpdateNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<NoiseTexturePipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        match self.state {
            NoiseUpdateState::Loading => {
                match pipeline_cache.get_compute_pipeline_state(pipeline.octave_noise_pipeline) {
                    CachedPipelineState::Ok(_) => {
                        match pipeline_cache
                            .get_compute_pipeline_state(pipeline.ridge_noise_pipeline)
                        {
                            CachedPipelineState::Ok(_) => {
                                self.state = NoiseUpdateState::Ready;
                            }
                            CachedPipelineState::Err(err) => {
                                panic!("Intializing assets/{SHADER_ASSET_PATH}:\n{err}")
                            }
                            _ => {}
                        }
                    }
                    CachedPipelineState::Err(err) => {
                        panic!("Intializing assets/{SHADER_ASSET_PATH}:\n{err}")
                    }
                    _ => {}
                }
            }
            NoiseUpdateState::Ready => {
                // Don't think we need to do anything here
            }
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let bind_groups = &world.resource::<NoiseTextureImageBindGroups>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<NoiseTexturePipeline>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        // select the pipeline based on the current state
        match self.state {
            NoiseUpdateState::Loading => {}
            NoiseUpdateState::Ready => {
                let octave_noise_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.octave_noise_pipeline)
                    .unwrap();
                pass.set_bind_group(0, &bind_groups[0], &[]);
                pass.set_pipeline(octave_noise_pipeline);
                pass.dispatch_workgroups(
                    SIZE.0 / WORKGROUP_SIZE,
                    SIZE.1 / WORKGROUP_SIZE,
                    SIZE.2 / WORKGROUP_SIZE,
                );

                let ridge_noise_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.ridge_noise_pipeline)
                    .unwrap();
                pass.set_bind_group(0, &bind_groups[0], &[]);
                pass.set_pipeline(ridge_noise_pipeline);
                pass.dispatch_workgroups(
                    SIZE.0 / WORKGROUP_SIZE,
                    SIZE.1 / WORKGROUP_SIZE,
                    SIZE.2 / WORKGROUP_SIZE,
                );
            } /*
              NoiseUpdateState::Update(index) => {
                  let ridge_noise_pipeline = pipeline_cache
                      .get_compute_pipeline(pipeline.ridge_noise_pipeline)
                      .unwrap();
                  pass.set_bind_group(0, &bind_groups[index], &[]);
                  pass.set_pipeline(ridge_noise_pipeline);
                  pass.dispatch_workgroups(
                      SIZE.0 / WORKGROUP_SIZE,
                      SIZE.1 / WORKGROUP_SIZE,
                      SIZE.2 / WORKGROUP_SIZE,
                  );
              } */
        }

        Ok(())
    }
}
