use super::background_camera::BackgroundImageOutput;
use bevy::{
    asset::{Handle, load_internal_asset, weak_handle},
    core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
        prepass::{PreviousViewData, PreviousViewUniformOffset, PreviousViewUniforms},
    },
    diagnostic::FrameCount,
    ecs::{
        query::QueryItem,
        system::{Commands, Query, Res, ResMut, lifetimeless::Read},
    },
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        camera::ExtractedCamera,
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_asset::RenderAssets,
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice},
        texture::{CachedTexture, GpuImage, TextureCache},
        view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    },
};

const UPSCALE_SHADER_HANDLE: Handle<Shader> = weak_handle!("a633e007-3aba-4f08-abeb-f309f98fce4b");

/// It is generally encouraged to set up post processing effects as a plugin
pub struct BackgroundUpscalePlugin;

impl Plugin for BackgroundUpscalePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            UPSCALE_SHADER_HANDLE,
            "background_upscale.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins((
            // The settings will be a component that lives in the main world but will
            // be extracted to the render world every frame.
            // This makes it possible to control the effect from the main world.
            // This plugin will take care of extracting it automatically.
            // It's important to derive [`ExtractComponent`] on [`PostProcessingSettings`]
            // for this plugin to work correctly.
            ExtractComponentPlugin::<BackgroundUpscaleSettings>::default(),
            // The settings will also be the data used in the shader.
            // This plugin will prepare the component for the GPU by creating a uniform buffer
            // and writing the data to that buffer every frame.
            UniformComponentPlugin::<BackgroundUpscaleSettings>::default(),
        ));

        // We need to get the render app from the main app
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<PreviousViewUniforms>()
            .add_systems(
                Render,
                (prepare_background_history_textures.in_set(RenderSet::PrepareResources),),
            )
            // Bevy's renderer uses a render graph which is a collection of nodes in a directed acyclic graph.
            // It currently runs on each view/camera and executes each node in the specified order.
            // It will make sure that any node that needs a dependency from another node
            // only runs when that dependency is done.
            //
            // Each node can execute arbitrary work, but it generally runs at least one render pass.
            // A node only has access to the render world, so if you need data from the main world
            // you need to extract it manually or with the plugin like above.
            // Add a [`Node`] to the [`RenderGraph`]
            // The Node needs to impl FromWorld
            //
            // The [`ViewNodeRunner`] is a special [`Node`] that will automatically run the node for each view
            // matching the [`ViewQuery`]
            .add_render_graph_node::<ViewNodeRunner<BackgroundUpscaleNode>>(
                // Specify the label of the graph, in this case we want the graph for 3d
                Core3d,
                // It also needs the label of the node
                BackgroundUpscaleLabel,
            )
            .add_render_graph_edges(
                Core3d,
                // Run just before the beginning of the main opaque pass
                (
                    Node3d::StartMainPass,
                    BackgroundUpscaleLabel,
                    Node3d::MainOpaquePass,
                    /*
                    -- Alternatively

                    Node3d::MainOpaquePass,
                    BackgroundUpscaleLabel,
                    Node3d::MainTransparentPass,
                     */
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        // We need to get the render app from the main app
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            // Initialize the pipeline
            .init_resource::<BackgroundUpscalePipeline>();
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct BackgroundUpscaleLabel;

// The post process node used for the render graph
#[derive(Default)]
struct BackgroundUpscaleNode;

// The ViewNode trait is required by the ViewNodeRunner
impl ViewNode for BackgroundUpscaleNode {
    // The node needs a query to gather data from the ECS in order to do its rendering,
    // but it's not a normal system so we need to define it manually.
    //
    // This query will only run on the view entity
    type ViewQuery = (
        Read<ViewUniformOffset>,
        Read<PreviousViewUniformOffset>,
        &'static ViewTarget,
        &'static BackgroundImageOutput,
        &'static BackgroundHistoryTextures,
        &'static BackgroundUpscaleSettings,
        // As there could be multiple post processing components sent to the GPU (one per camera),
        // we need to get the index of the one that is associated with the current view.
        &'static DynamicUniformIndex<BackgroundUpscaleSettings>,
    );

    // Runs the node logic
    // This is where you encode draw commands.
    //
    // This will run on every view on which the graph is running.
    // If you don't want your effect to run on every camera,
    // you'll need to make sure you have a marker component as part of [`ViewQuery`]
    // to identify which camera(s) should run the effect.
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            view_uniform_offset,
            previous_view_uniform_offset,
            view_target,
            background_input,
            background_history_textures,
            _background_upscale_settings, // This is just so the node doesn't run unless this component is present
            settings_index,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        // Get the pipeline resource that contains the global data we need
        // to create the render pipeline
        let background_upscale_pipeline = world.resource::<BackgroundUpscalePipeline>();

        // The pipeline cache is a cache of all previously created pipelines.
        // It is required to avoid creating a new pipeline each frame,
        // which is expensive due to shader compilation.
        let pipeline_cache = world.resource::<PipelineCache>();

        // Get the pipeline from the cache
        let Some(pipeline) =
            pipeline_cache.get_render_pipeline(background_upscale_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let gpu_image_assets = world.resource::<RenderAssets<GpuImage>>();

        let Some(input_image) = gpu_image_assets.get(&background_input.image) else {
            return Ok(());
        };

        // Get the settings uniform binding
        let settings_uniforms = world.resource::<ComponentUniforms<BackgroundUpscaleSettings>>();
        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        let view_uniforms_resource = world.resource::<ViewUniforms>();
        let prev_view_uniforms_resource = world.resource::<PreviousViewUniforms>();
        let (Some(view_uniforms), Some(prev_view_uniforms)) = (
            view_uniforms_resource.uniforms.binding(),
            prev_view_uniforms_resource.uniforms.binding(),
        ) else {
            return Ok(());
        };

        // This will start a new "post process write", obtaining two texture
        // views from the view target - a `source` and a `destination`.
        // `source` is the "current" main texture and you _must_ write into
        // `destination` because calling `post_process_write()` on the
        // [`ViewTarget`] will internally flip the [`ViewTarget`]'s main
        // texture to the `destination` texture. Failing to do so will cause
        // the current main texture information to be lost.


        
        //let post_process = view_target.post_process_write();

        // The bind_group gets created each frame.
        //
        // Normally, you would create a bind_group in the Queue set,
        // but this doesn't work with the post_process_write().
        // The reason it doesn't work is because each post_process_write will alternate the source/destination.
        // The only way to have the correct source/destination for the bind_group
        // is to make sure you get it during the node execution.
        let bind_group = render_context.render_device().create_bind_group(
            "background_upscale_bind_group",
            &background_upscale_pipeline.layout,
            // It's important for this to match the BindGroupLayout defined in the PostProcessPipeline
            &BindGroupEntries::sequential((
                // Make sure to use the source view
                //post_process.source,
                &background_history_textures.read.default_view,
                // Use the sampler created for the pipeline
                &input_image.texture_view,
                &background_upscale_pipeline.sampler,
                view_uniforms,
                prev_view_uniforms,
                // Set the settings binding
                settings_binding.clone(),
            )),
        );

        // Begin the render pass
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("background_upscale_pass"),
            color_attachments: &[
                /*Some(RenderPassColorAttachment {
                    //view: post_process.destination,
                    view: view_target.main_texture_view(),
                    resolve_target: None,
                    ops: Operations::default(),
                }),*/
                Some(view_target.get_color_attachment()),
                Some(RenderPassColorAttachment {
                    view: &background_history_textures.write.default_view,
                    resolve_target: None,
                    ops: Operations::default(),
                }),
            ],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // This is mostly just wgpu boilerplate for drawing a fullscreen triangle,
        // using the pipeline/bind_group created above
        render_pass.set_render_pipeline(pipeline);
        // By passing in the index of the post process settings on this view, we ensure
        // that in the event that multiple settings were sent to the GPU (as would be the
        // case with multiple cameras), we use the correct one.
        render_pass.set_bind_group(
            0,
            &bind_group,
            &[
                view_uniform_offset.offset,
                previous_view_uniform_offset.offset,
                settings_index.index(),
            ],
        );
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

// This contains global data used by the render pipeline. This will be created once on startup.
#[derive(Resource)]
struct BackgroundUpscalePipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for BackgroundUpscalePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // We need to define the bind group layout used for our pipeline
        let layout = render_device.create_bind_group_layout(
            "background_upscale_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                // The layout entries will only be visible in the fragment stage
                ShaderStages::FRAGMENT,
                (
                    // The screen texture
                    //texture_2d(TextureSampleType::Float { filterable: true }),
                    // history input texture
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    // background input texture
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    // Can use the same sampler for all of them (I think?)
                    sampler(SamplerBindingType::Filtering),
                    // The settings uniform that will control the effect
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<PreviousViewData>(true),
                    //
                    // The input texture
                    uniform_buffer::<BackgroundUpscaleSettings>(true),
                ),
            ),
        );

        // We can create the sampler here since it won't change at runtime and doesn't depend on the view
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("background_upscale_nearest_sampler"),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..SamplerDescriptor::default()
        });

        let pipeline_id = world
            .resource_mut::<PipelineCache>()
            // This will add the pipeline to the cache and queue its creation
            .queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("background_upscale_pipeline".into()),
                layout: vec![layout.clone()],
                // This will setup a fullscreen triangle for the vertex state
                vertex: fullscreen_shader_vertex_state(),
                fragment: Some(FragmentState {
                    shader: UPSCALE_SHADER_HANDLE,
                    shader_defs: vec![],
                    // Make sure this matches the entry point of your shader.
                    // It can be anything as long as it matches here and in the shader.
                    entry_point: "fragment".into(),
                    targets: vec![
                        Some(ColorTargetState {
                            format: TextureFormat::bevy_default(),
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        }),
                        Some(ColorTargetState {
                            format: TextureFormat::bevy_default(),
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        }),
                    ],
                }),
                // All of the following properties are not important for this effect so just use the default values.
                // This struct doesn't have the Default trait implemented because not all fields can have a default value.
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                push_constant_ranges: vec![],
                zero_initialize_workgroup_memory: false,
            });

        Self {
            layout,
            sampler,
            pipeline_id,
        }
    }
}

#[derive(Component)]
pub struct BackgroundHistoryTextures {
    write: CachedTexture,
    read: CachedTexture,
}

fn prepare_background_history_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    frame_count: Res<FrameCount>,
    views: Query<(Entity, &ExtractedCamera, &ExtractedView), With<BackgroundUpscaleSettings>>,
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

            texture_descriptor.label = Some("background_history_1_texture");
            let history_1_texture = texture_cache.get(&render_device, texture_descriptor.clone());

            texture_descriptor.label = Some("background_history_2_texture");
            let history_2_texture = texture_cache.get(&render_device, texture_descriptor);

            let textures = if frame_count.0 % 2 == 0 {
                BackgroundHistoryTextures {
                    write: history_1_texture,
                    read: history_2_texture,
                }
            } else {
                BackgroundHistoryTextures {
                    write: history_2_texture,
                    read: history_1_texture,
                }
            };

            commands.entity(entity).insert(textures);
        }
    }
}

// This is the component that will get passed to the shader
#[derive(Component, Default, Clone, ExtractComponent, ShaderType)]
pub struct BackgroundUpscaleSettings {
    pub current_pixel: f32,
    pub dimensions: Vec2,
}
