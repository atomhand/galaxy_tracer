use super::shader_types::{BulgeParams, ComponentParams, GalaxyParams};
use crate::prelude::*;
use bevy::asset::{weak_handle, Handle};
use bevy::core_pipeline::{core_3d::CORE_3D_DEPTH_FORMAT, prepass::PreviousViewUniforms};
use bevy::ecs::{
    prelude::{Component, Entity},
    query::{QueryItem, With},
    reflect::ReflectComponent,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
};
use bevy::prelude::*;
use bevy::reflect::{std_traits::ReflectDefault, Reflect};
use bevy::render::{
    extract_component::{
        ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
        UniformComponentPlugin,
    },
    render_asset::RenderAssets,
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
        CachedRenderPipelineId, CompareFunction, DepthStencilState, FragmentState,
        MultisampleState, PipelineCache, RenderPipelineDescriptor, Shader, ShaderStages,
        SpecializedRenderPipeline, SpecializedRenderPipelines, *,
    },
    renderer::RenderDevice,
    texture::GpuImage,
    view::{ExtractedView, Msaa, ViewTarget, ViewUniform, ViewUniforms},
    Render, RenderApp, RenderSet,
};
use bevy::utils::prelude::default;

const GALAXY_SHADER_HANDLE: Handle<Shader> = weak_handle!("9262acb0-5254-4f2d-9ddc-c70010442e57");

pub struct GalaxyRenderPlugin;

impl Plugin for GalaxyRenderPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedRenderPipelines<GalaxyPipeline>>()
            .add_systems(
                Render,
                (
                    prepare_galaxy_pipelines.in_set(RenderSet::Prepare),
                    prepare_galaxy_bind_groups.in_set(RenderSet::PrepareBindGroups),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        let render_device = render_app.world().resource::<RenderDevice>().clone();
        render_app.insert_resource(GalaxyPipeline::new(&render_device));
    }
}

#[derive(Component, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct GalaxyRender {
    pub image: Handle<Image>,
}

impl Default for GalaxyRender {
    fn default() -> Self {
        GalaxyRender { image : Handle::default() }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct GalaxyPipelineKey {
    hdr: bool,
    samples: u32,
    depth_format: TextureFormat,
}

#[derive(Component, ShaderType, Clone)]
struct GalaxyUniforms {
    galaxy_params: GalaxyParams,
    dust_params: ComponentParams,
    disk_params: ComponentParams,
    bulge_params: BulgeParams,
}

#[derive(Resource)]
struct GalaxyPipeline {
    volume_bind_group_layout: BindGroupLayout,
    projection_bind_group_layout: BindGroupLayout,
}

impl GalaxyPipeline {
    fn new(render_device: &RenderDevice) -> Self {
        Self {
            volume_bind_group_layout: render_device.create_bind_group_layout(
                "galaxy_volume_bind_group_layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        uniform_buffer::<ViewUniform>(true)
                            .visibility(ShaderStages::VERTEX_FRAGMENT),
                        uniform_buffer::<GalaxyUniforms>(true),
                    ),
                ),
            ),
            projection_bind_group_layout: render_device.create_bind_group_layout(
                "galaxy_projection_bind_group_layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        uniform_buffer::<ViewUniform>(true)
                            .visibility(ShaderStages::VERTEX_FRAGMENT),
                        uniform_buffer::<GalaxyUniforms>(true),
                    ),
                ),
            ),
        }
    }
}

impl SpecializedRenderPipeline for GalaxyPipeline {
    type Key = GalaxyPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("galaxy_pipeline".into()),
            layout: vec![
                self.volume_bind_group_layout.clone(),
                self.projection_bind_group_layout.clone(),
            ],
            push_constant_ranges: Vec::new(),
            vertex: VertexState {
                shader: GALAXY_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "galaxy_vertex".into(),
                buffers: Vec::new(),
            },
            primitive: PrimitiveState::default(),
            depth_stencil: Some(DepthStencilState {
                format: key.depth_format,
                depth_write_enabled: false,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                shader: GALAXY_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "galaxy_fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            zero_initialize_workgroup_memory: false,
        }
    }
}

#[derive(Component)]
struct GalaxyBindGroup {
    volume_bind_group: BindGroup,
    projection_bind_group: BindGroup,
}

#[derive(Component)]
pub struct GalaxyPipelineId(pub CachedRenderPipelineId);

fn prepare_galaxy_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<GalaxyPipeline>>,
    pipeline: Res<GalaxyPipeline>,
    views: Query<(Entity, &ExtractedView, &Msaa), With<GalaxyRender>>,
) {
    for (entity, view, msaa) in &views {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            GalaxyPipelineKey {
                hdr: view.hdr,
                samples: msaa.samples(),
                depth_format: CORE_3D_DEPTH_FORMAT,
            },
        );

        commands
            .entity(entity)
            .insert(GalaxyPipelineId(pipeline_id));
    }
}

fn prepare_galaxy_bind_groups(
    mut commands: Commands,
    pipeline: Res<GalaxyPipeline>,
    view_uniforms: Res<ViewUniforms>,
    galaxy_uniforms: Res<ComponentUniforms<GalaxyUniforms>>,
    images: Res<RenderAssets<GpuImage>>,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &GalaxyRender, &DynamicUniformIndex<GalaxyUniforms>)>,
) {
    for (entity, galaxyrender, galaxy_uniform_index) in &views {
        if let (Some(tex), Some(view_uniforms), Some(galaxy_uniforms)) = (
            images.get(&galaxyrender.image),
            view_uniforms.uniforms.binding(),
            galaxy_uniforms.binding(),
        ) {
            let volume_bind_group = render_device.create_bind_group(
                "galaxy_volume_bind_group",
                &pipeline.volume_bind_group_layout,
                &BindGroupEntries::sequential((
                    &tex.texture_view,
                    &tex.sampler,
                    view_uniforms.clone(),
                    galaxy_uniforms.clone(),
                )),
            );
            let projection_bind_group = render_device.create_bind_group(
                "galaxy_volume_bind_group",
                &pipeline.projection_bind_group_layout,
                &BindGroupEntries::sequential((
                    &tex.texture_view,
                    &tex.sampler,
                    view_uniforms,
                    galaxy_uniforms,
                )),
            );

            commands.entity(entity).insert(GalaxyBindGroup {
                volume_bind_group,
                projection_bind_group,
            });
        }
    }
}
