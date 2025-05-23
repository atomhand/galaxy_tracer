use super::{galaxy_texture::GalaxyTexture, shader_types::*, StarInstanceMarker};
use crate::prelude::*;
use crate::ui::CameraMain;
use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{binding_types::*, *},
        renderer::{RenderContext, RenderDevice, RenderQueue},
        storage::{GpuShaderStorageBuffer, ShaderStorageBuffer},
        texture::GpuImage,
        Render, RenderApp, RenderSet,
    },
};
use std::borrow::Cow;

const SHADER_ASSET_PATH: &str = "shaders/extinction_cache_compute.wgsl";
const WORKGROUP_SIZE: u32 = 64;
///
/// This is a strategy to complement point-rendering/PSF rendering (ie. stars)
///
/// Basically, the idea is that, given a list of world positions, we evaluate the extinction along the
/// ray from each world position to the camera and cache the results to a texture.
/// This can then be evaluated in any shader that needs it.
/// -- Consuming shader doesn't need to know anything about the galaxy volume or extinction algorithm
/// -- Extinction update can be staggered
/// ---- Either update subset of stars per frame
/// ---- Or MC accumulation on all stars at once (probably better)
/// ---- > I think this one is particularly neat because even a noticeably slow response will look more like a stylisation decision than a real visual flaw
///
/// API
/// -> ExtinctionCache resource holds the output texture (I guess 2 textures that alternate by frame)
/// -> VolumeStar or VolumeStarList are marker components
/// --> Providing the texture lookup offsets:
/// --- Could either be done by adding a new component, or using an Option/marker value in the source component
pub struct ExtinctionCachePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct ExtinctionCacheLabel;

impl Plugin for ExtinctionCachePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<ExtinctionCache>::default())
            .add_systems(Startup, init_cache_resource)
            .add_systems(Update, update_positions);

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<ExtinctionCacheGalaxyUniforms>()
            .add_systems(
                Render,
                prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
            )
            .add_systems(Render, prepare_uniforms.in_set(RenderSet::PrepareResources));

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(ExtinctionCacheLabel, ExtinctionCacheNode::default());
        render_graph.add_node_edge(ExtinctionCacheLabel, bevy::render::graph::CameraDriverLabel);
    }
    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<ExtinctionCachePipeline>();
    }
}

#[derive(Resource, Default, Clone, ExtractResource)]
pub struct ExtinctionCache {
    pub output_buffer: Handle<ShaderStorageBuffer>,
    pub required_size: usize,
    positions: Vec<Vec4>,
    colours: Vec<Vec4>,
    positions_buffer: Handle<ShaderStorageBuffer>,
    colours_buffer: Handle<ShaderStorageBuffer>,
    size: usize,
}

fn update_positions(
    mut extinction_cache: ResMut<ExtinctionCache>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    query: Query<(&Transform, &Star), Added<StarInstanceMarker>>,
) {
    if extinction_cache.size != extinction_cache.required_size {
        let size = extinction_cache.required_size;
        extinction_cache.size = size;
        extinction_cache.positions.resize(size, Vec4::ZERO);
        extinction_cache.colours.resize(size, Vec4::ZERO);

        if let Some(buffer) = buffers.get_mut(&extinction_cache.output_buffer) {
            buffer.set_data(vec![Vec4::ZERO; size]);
        }
    }

    if query.is_empty() {
        return;
    }

    for (transform, star) in &query {
        extinction_cache.positions[star.index as usize] = transform.translation.extend(1.0);
        extinction_cache.colours[star.index as usize] = star.color().extend(1.0);
    }

    if let Some(buffer) = buffers.get_mut(&extinction_cache.positions_buffer) {
        buffer.set_data(extinction_cache.positions.as_slice());
    }
    if let Some(buffer) = buffers.get_mut(&extinction_cache.colours_buffer) {
        buffer.set_data(extinction_cache.colours.as_slice());
    }
}

fn init_cache_resource(mut commands: Commands, mut buffers: ResMut<Assets<ShaderStorageBuffer>>) {
    let size = 0;
    commands.insert_resource(ExtinctionCache {
        output_buffer: buffers.add(ShaderStorageBuffer::from(vec![Vec4::ZERO; size])),
        positions: vec![Vec4::ZERO; size],
        colours: vec![Vec4::ZERO; size],
        positions_buffer: buffers.add(ShaderStorageBuffer::from(vec![Vec4::ZERO; size])),
        colours_buffer: buffers.add(ShaderStorageBuffer::from(vec![Vec4::ZERO; size])),
        required_size: size,
        size,
    });
}

#[derive(Resource, Default)]
struct ExtinctionCacheGalaxyUniforms {
    galaxy_params: UniformBuffer<GalaxyParams>,
    bulge_params: UniformBuffer<BulgeParams>,
    disk_params: UniformBuffer<ComponentParams>,
    dust_params: UniformBuffer<ComponentParams>,
    camera_uniform: UniformBuffer<Vec4>,
}

fn prepare_uniforms(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    galaxy_config: Res<GalaxyConfig>,
    galaxy_render_settings: Res<GalaxyRenderConfig>,
    mut uniforms: ResMut<ExtinctionCacheGalaxyUniforms>,
    camera: Query<&CameraMain>,
) {
    uniforms
        .galaxy_params
        .set(GalaxyParams::read(&galaxy_config, &galaxy_render_settings));
    uniforms.bulge_params.set(BulgeParams::read(&galaxy_config));
    uniforms
        .disk_params
        .set(ComponentParams::read(&galaxy_config.disk_params));
    uniforms
        .dust_params
        .set(ComponentParams::read(&galaxy_config.dust_params));

    if let Ok(camera) = camera.single() {
        uniforms.camera_uniform.set(camera.translation.extend(1.0));
    }

    uniforms
        .galaxy_params
        .write_buffer(&render_device, &render_queue);
    uniforms
        .bulge_params
        .write_buffer(&render_device, &render_queue);
    uniforms
        .disk_params
        .write_buffer(&render_device, &render_queue);
    uniforms
        .dust_params
        .write_buffer(&render_device, &render_queue);
    uniforms
        .camera_uniform
        .write_buffer(&render_device, &render_queue);
}

#[derive(Resource)]
struct ExtinctionCacheBindGroups([BindGroup; 1]);

#[allow(clippy::too_many_arguments)]
fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<ExtinctionCachePipeline>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    galaxy_texture: Res<GalaxyTexture>,
    cache_image: Res<ExtinctionCache>,
    uniforms_buffer: Res<ExtinctionCacheGalaxyUniforms>,
    render_device: Res<RenderDevice>,
    ssbos: Res<RenderAssets<GpuShaderStorageBuffer>>,
) {
    let input_positions = ssbos.get(&cache_image.positions_buffer).unwrap();
    let input_colours = ssbos.get(&cache_image.colours_buffer).unwrap();
    let output_buffer = ssbos.get(&cache_image.output_buffer).unwrap();

    let galaxy_uniform = uniforms_buffer.galaxy_params.binding().unwrap();
    let bulge_params = uniforms_buffer.bulge_params.binding().unwrap();
    let disk_params = uniforms_buffer.disk_params.binding().unwrap();
    let dust_params = uniforms_buffer.dust_params.binding().unwrap();
    let camera_uniform = uniforms_buffer.camera_uniform.binding().unwrap();

    let galaxy_view = galaxy_texture
        .tex
        .as_ref()
        .and_then(|x| gpu_images.get(x))
        .unwrap();
    let lut_view = galaxy_texture
        .luts
        .as_ref()
        .and_then(|x| gpu_images.get(x))
        .unwrap();

    let bind_group = render_device.create_bind_group(
        "extinction_cache_bind_group",
        &pipeline.bind_group_layout,
        &BindGroupEntries::sequential((
            camera_uniform,
            output_buffer.buffer.as_entire_buffer_binding(),
            input_positions.buffer.as_entire_buffer_binding(),
            input_colours.buffer.as_entire_buffer_binding(),
            galaxy_uniform,
            bulge_params,
            disk_params,
            dust_params,
            &galaxy_view.texture_view,
            &galaxy_view.sampler,
            &lut_view.texture_view,
            &lut_view.sampler,
        )),
    );
    commands.insert_resource(ExtinctionCacheBindGroups([bind_group]));
}

#[derive(Resource)]
struct ExtinctionCachePipeline {
    bind_group_layout: BindGroupLayout,
    pipeline: CachedComputePipelineId,
}

impl FromWorld for ExtinctionCachePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let bind_group_layout = render_device.create_bind_group_layout(
            "extinction_cache_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    // camera pos
                    uniform_buffer::<Vec4>(false),
                    // Extinction output
                    storage_buffer::<Vec4>(false),
                    // positions input buffer
                    storage_buffer_read_only::<Vec4>(false),
                    // colours input buffer
                    storage_buffer_read_only::<Vec4>(false),
                    uniform_buffer::<GalaxyParams>(false),
                    uniform_buffer::<BulgeParams>(false),
                    uniform_buffer::<ComponentParams>(false),
                    uniform_buffer::<ComponentParams>(false),
                    texture_2d(TextureSampleType::Float { filterable: true }), // Galaxy texture
                    sampler(SamplerBindingType::Filtering),                    // sampler
                    texture_2d_array(TextureSampleType::Float { filterable: true }), // LUT
                    sampler(SamplerBindingType::Filtering),                    // LUT sampler
                ),
            ),
        );
        let shader = world.load_asset(SHADER_ASSET_PATH);
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![
                "COMPUTE_BINDINGS".into(),
                "RUNTIME_NOISE".into(),
                "EXTINCTION_ONLY".into(),
            ],
            entry_point: Cow::from("cache_extinction"),
            zero_initialize_workgroup_memory: false,
        });

        ExtinctionCachePipeline {
            bind_group_layout,
            pipeline,
        }
    }
}

enum NoiseUpdateState {
    Loading,
    Run,
}

struct ExtinctionCacheNode {
    state: NoiseUpdateState,
}

impl Default for ExtinctionCacheNode {
    fn default() -> Self {
        Self {
            state: NoiseUpdateState::Loading,
        }
    }
}

impl render_graph::Node for ExtinctionCacheNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<ExtinctionCachePipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        match self.state {
            NoiseUpdateState::Loading => {
                match pipeline_cache.get_compute_pipeline_state(pipeline.pipeline) {
                    CachedPipelineState::Ok(_) => {
                        self.state = NoiseUpdateState::Run;
                    }
                    CachedPipelineState::Err(err) => {
                        panic!("Intializing assets/{SHADER_ASSET_PATH}:\n{err}");
                    }
                    _ => {}
                }
            }
            NoiseUpdateState::Run => {}
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let bind_groups = &world.resource::<ExtinctionCacheBindGroups>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline: &ExtinctionCachePipeline = world.resource::<ExtinctionCachePipeline>();
        let size = world.resource::<ExtinctionCache>().size;

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        // select the pipeline based on the current state
        match self.state {
            NoiseUpdateState::Loading => {}
            NoiseUpdateState::Run => {
                let pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.pipeline)
                    .unwrap();
                pass.set_bind_group(0, &bind_groups[0], &[]);
                pass.set_pipeline(pipeline);
                pass.dispatch_workgroups(size as u32 / WORKGROUP_SIZE, 1, 1);
            }
        }

        Ok(())
    }
}
