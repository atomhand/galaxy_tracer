use bevy::{
    image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::{RenderAssetUsages, RenderAssets},
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{binding_types::texture_3d, binding_types::texture_storage_2d, *},
        renderer::{RenderContext, RenderDevice},
        texture::GpuImage,
        Render, RenderApp, RenderSet,
    },
};
use std::borrow::Cow;

const SHADER_ASSET_PATH: &str = "shaders/noise_compute.wgsl";

const SIZE: (u32, u32, u32) = (256, 8, 256);
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
        render_app.add_systems(
            Render,
            prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
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

#[derive(Resource)]
struct NoiseTextureImageBindGroups([BindGroup; 1]);

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<NoiseTexturePipeline>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    noise_texture_images: Res<NoiseTextureImages>,
    render_device: Res<RenderDevice>,
) {
    let view_a = gpu_images
        .get(&noise_texture_images.disk_component)
        .unwrap();
    let view_b = gpu_images
        .get(&noise_texture_images.dust_component)
        .unwrap();

    let bind_group_0 = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::sequential((&view_a.texture_view, &view_b.texture_view)),
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
    Init,
    Update(usize),
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
                        self.state = NoiseUpdateState::Init;
                    }
                    CachedPipelineState::Err(err) => {
                        panic!("Intializing assets/{SHADER_ASSET_PATH}:\n{err}")
                    }
                    _ => {}
                }
            }
            NoiseUpdateState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.octave_noise_pipeline)
                {
                    //self.state = NoiseUpdateState::Update(1);
                    self.state = NoiseUpdateState::Init;
                }
            }
            //NoiseUpdateState::Update(0) => self.state = NoiseUpdateState::Update(1),
            //NoiseUpdateState::Update(1) => self.state = NoiseUpdateState::Update(0),
            NoiseUpdateState::Update(_) => unreachable!(),
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
            NoiseUpdateState::Init => {
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
            }
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
            }
        }

        Ok(())
    }
}
