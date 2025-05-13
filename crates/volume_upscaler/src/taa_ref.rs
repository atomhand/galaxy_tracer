use bevy::app::{App, Plugin};
use bevy::asset::{load_internal_asset, weak_handle, Handle};
use bevy::core_pipeline::{
    core_3d::graph::{Core3d, Node3d},
    fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::Camera3d,
    prepass::{DepthPrepass, MotionVectorPrepass, ViewPrepassTextures},
};
use bevy::diagnostic::FrameCount;
use bevy::ecs::{
    prelude::{Component, Entity, ReflectComponent},
    query::{QueryItem, With},
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy::image::BevyDefault as _;
use bevy::math::vec2;
use bevy::reflect::{std_traits::ReflectDefault, Reflect};
use bevy::render::{
    camera::{ExtractedCamera, MipBias, TemporalJitter},
    prelude::{Camera, Projection},
    render_graph::{NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner},
    render_resource::{
        binding_types::{sampler, texture_2d, texture_depth_2d},
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId,
        ColorTargetState, ColorWrites, Extent3d, FilterMode, FragmentState, MultisampleState,
        Operations, PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, Shader,
        ShaderStages, SpecializedRenderPipeline, SpecializedRenderPipelines, TextureDescriptor,
        TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    },
    renderer::{RenderContext, RenderDevice},
    sync_component::SyncComponentPlugin,
    sync_world::RenderEntity,
    texture::{CachedTexture, TextureCache},
    view::{ExtractedView, Msaa, ViewTarget},
    ExtractSchedule, MainWorld, Render, RenderApp, RenderSet,
};
use bevy::prelude::*;

const TAA_SHADER_HANDLE: Handle<Shader> = weak_handle!("096a793e-eff3-449e-813d-25da83e98fe6");

/// Plugin for temporal anti-aliasing.
///
/// See [`TemporalAntiAliasing`] for more details.
pub struct VolumeUpscalingPlugin;

impl Plugin for VolumeUpscalingPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, TAA_SHADER_HANDLE, "upscale.wgsl", Shader::from_wgsl);

        app.register_type::<VolumetricUpscaling>();

        app.add_plugins(SyncComponentPlugin::<VolumetricUpscaling>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<SpecializedRenderPipelines<TaaPipeline>>()
            .add_systems(ExtractSchedule, extract_taa_settings)
            .add_systems(
                Render,
                (
                    prepare_taa_jitter_and_mip_bias.in_set(RenderSet::ManageViews),
                    prepare_taa_pipelines.in_set(RenderSet::Prepare),
                    prepare_taa_history_textures.in_set(RenderSet::PrepareResources),
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<TemporalAntiAliasNode>>(Core3d, Node3d::Taa)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndMainPass,
                    Node3d::MotionBlur, // Running before TAA reduces edge artifacts and noise
                    Node3d::Taa,
                    Node3d::Bloom,
                    Node3d::Tonemapping,
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<TaaPipeline>();
    }
}

#[derive(Component, Reflect, Clone)]
#[reflect(Component, Clone)]
#[require( DepthPrepass, MotionVectorPrepass)]
pub struct VolumetricUpscaling {
    /// Set to true to delete the saved temporal history (past frames).
    ///
    /// Useful for preventing ghosting when the history is no longer
    /// representative of the current frame, such as in sudden camera cuts.
    ///
    /// After setting this to true, it will automatically be toggled
    /// back to false at the end of the frame.
    pub reset: bool,
    pub input : Handle<Image>
}

/// Render [`bevy_render::render_graph::Node`] used by temporal anti-aliasing.
#[derive(Default)]
pub struct TemporalAntiAliasNode;

impl ViewNode for TemporalAntiAliasNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static TemporalAntiAliasHistoryTextures,
        &'static ViewPrepassTextures,
        &'static TemporalAntiAliasPipelineId,
        &'static Msaa,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (camera, view_target, taa_history_textures, prepass_textures, taa_pipeline_id, msaa): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if *msaa != Msaa::Off {
            warn!("Temporal anti-aliasing requires MSAA to be disabled");
            return Ok(());
        }

        let (Some(pipelines), Some(pipeline_cache)) = (
            world.get_resource::<TaaPipeline>(),
            world.get_resource::<PipelineCache>(),
        ) else {
            return Ok(());
        };
        let (Some(taa_pipeline), Some(prepass_motion_vectors_texture), Some(prepass_depth_texture)) = (
            pipeline_cache.get_render_pipeline(taa_pipeline_id.0),
            &prepass_textures.motion_vectors,
            &prepass_textures.depth,
        ) else {
            return Ok(());
        };
        let view_target = view_target.post_process_write();

        let taa_bind_group = render_context.render_device().create_bind_group(
            "taa_bind_group",
            &pipelines.taa_bind_group_layout,
            &BindGroupEntries::sequential((
                view_target.source,
                &taa_history_textures.read.default_view,
                &prepass_motion_vectors_texture.texture.default_view,
                &prepass_depth_texture.texture.default_view,
                &pipelines.nearest_sampler,
                &pipelines.linear_sampler,
            )),
        );

        {
            let mut taa_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("taa_pass"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: view_target.destination,
                        resolve_target: None,
                        ops: Operations::default(),
                    }),
                    Some(RenderPassColorAttachment {
                        view: &taa_history_textures.write.default_view,
                        resolve_target: None,
                        ops: Operations::default(),
                    }),
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            taa_pass.set_render_pipeline(taa_pipeline);
            taa_pass.set_bind_group(0, &taa_bind_group, &[]);
            if let Some(viewport) = camera.viewport.as_ref() {
                taa_pass.set_camera_viewport(viewport);
            }
            taa_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
}

#[derive(Resource)]
struct TaaPipeline {
    taa_bind_group_layout: BindGroupLayout,
    nearest_sampler: Sampler,
    linear_sampler: Sampler,
}

impl FromWorld for TaaPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let nearest_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("taa_nearest_sampler"),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..SamplerDescriptor::default()
        });
        let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("taa_linear_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..SamplerDescriptor::default()
        });

        let taa_bind_group_layout = render_device.create_bind_group_layout(
            "taa_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    // View target (read)
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    // TAA History (read)
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    // Motion Vectors
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    // Depth
                    texture_depth_2d(),
                    // Nearest sampler
                    sampler(SamplerBindingType::NonFiltering),
                    // Linear sampler
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        TaaPipeline {
            taa_bind_group_layout,
            nearest_sampler,
            linear_sampler,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct TaaPipelineKey {
    hdr: bool,
    reset: bool,
}

impl SpecializedRenderPipeline for TaaPipeline {
    type Key = TaaPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = vec![];

        let format = if key.hdr {
            shader_defs.push("TONEMAP".into());
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        if key.reset {
            shader_defs.push("RESET".into());
        }

        RenderPipelineDescriptor {
            label: Some("taa_pipeline".into()),
            layout: vec![self.taa_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: TAA_SHADER_HANDLE,
                shader_defs,
                entry_point: "taa".into(),
                targets: vec![
                    Some(ColorTargetState {
                        format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                ],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: Vec::new(),
            zero_initialize_workgroup_memory: false,
        }
    }
}

fn extract_taa_settings(mut commands: Commands, mut main_world: ResMut<MainWorld>) {
    let mut cameras_3d = main_world.query_filtered::<(
        RenderEntity,
        &Camera,
        &Projection,
        &mut VolumetricUpscaling,
    ), (
        With<Camera3d>,
        With<TemporalJitter>,
        With<DepthPrepass>,
        With<MotionVectorPrepass>,
    )>();

    for (entity, camera, camera_projection, mut taa_settings) in
        cameras_3d.iter_mut(&mut main_world)
    {
        let has_perspective_projection = matches!(camera_projection, Projection::Perspective(_));
        let mut entity_commands = commands
            .get_entity(entity)
            .expect("Camera entity wasn't synced.");
        if camera.is_active && has_perspective_projection {
            entity_commands.insert(taa_settings.clone());
            taa_settings.reset = false;
        } else {
            // TODO: needs better strategy for cleaning up
            entity_commands.remove::<(
                VolumetricUpscaling,
                // components added in prepare systems (because `TemporalAntiAliasNode` does not query extracted components)
                TemporalAntiAliasHistoryTextures,
                TemporalAntiAliasPipelineId,
            )>();
        }
    }
}

fn prepare_taa_jitter_and_mip_bias(
    frame_count: Res<FrameCount>,
    mut query: Query<(Entity, &mut TemporalJitter, Option<&MipBias>), With<VolumetricUpscaling>>,
    mut commands: Commands,
) {
    // Halton sequence (2, 3) - 0.5, skipping i = 0
    let halton_sequence = [
        vec2(0.0, -0.16666666),
        vec2(-0.25, 0.16666669),
        vec2(0.25, -0.3888889),
        vec2(-0.375, -0.055555552),
        vec2(0.125, 0.2777778),
        vec2(-0.125, -0.2777778),
        vec2(0.375, 0.055555582),
        vec2(-0.4375, 0.3888889),
    ];

    let offset = halton_sequence[frame_count.0 as usize % halton_sequence.len()];

    for (entity, mut jitter, mip_bias) in &mut query {
        jitter.offset = offset;

        if mip_bias.is_none() {
            commands.entity(entity).insert(MipBias(-1.0));
        }
    }
}

#[derive(Component)]
pub struct TemporalAntiAliasHistoryTextures {
    write: CachedTexture,
    read: CachedTexture,
}

fn prepare_taa_history_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    frame_count: Res<FrameCount>,
    views: Query<(Entity, &ExtractedCamera, &ExtractedView), With<VolumetricUpscaling>>,
) {
    for (entity, camera, view) in &views {
        if let Some(physical_target_size) = camera.physical_target_size {
            let mut texture_descriptor = TextureDescriptor {
                label: None,
                size: Extent3d {
                    depth_or_array_layers: 1,
                    width: physical_target_size.x,
                    height: physical_target_size.y,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: if view.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                },
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            };

            texture_descriptor.label = Some("taa_history_1_texture");
            let history_1_texture = texture_cache.get(&render_device, texture_descriptor.clone());

            texture_descriptor.label = Some("taa_history_2_texture");
            let history_2_texture = texture_cache.get(&render_device, texture_descriptor);

            let textures = if frame_count.0 % 2 == 0 {
                TemporalAntiAliasHistoryTextures {
                    write: history_1_texture,
                    read: history_2_texture,
                }
            } else {
                TemporalAntiAliasHistoryTextures {
                    write: history_2_texture,
                    read: history_1_texture,
                }
            };

            commands.entity(entity).insert(textures);
        }
    }
}

#[derive(Component)]
pub struct TemporalAntiAliasPipelineId(CachedRenderPipelineId);

fn prepare_taa_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TaaPipeline>>,
    pipeline: Res<TaaPipeline>,
    views: Query<(Entity, &ExtractedView, &VolumetricUpscaling)>,
) {
    for (entity, view, taa_settings) in &views {
        let mut pipeline_key = TaaPipelineKey {
            hdr: view.hdr,
            reset: taa_settings.reset,
        };
        let pipeline_id = pipelines.specialize(&pipeline_cache, &pipeline, pipeline_key.clone());

        // Prepare non-reset pipeline anyways - it will be necessary next frame
        if pipeline_key.reset {
            pipeline_key.reset = false;
            pipelines.specialize(&pipeline_cache, &pipeline, pipeline_key);
        }

        commands
            .entity(entity)
            .insert(TemporalAntiAliasPipelineId(pipeline_id));
    }
}
