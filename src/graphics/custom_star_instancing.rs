use super::StarInstanceMarker;
use crate::graphics::ExtinctionCache;
use crate::prelude::*;
use bevy::{
    core_pipeline::core_3d::Transparent3d,
    ecs::{
        query::QueryItem,
        system::{lifetimeless::*, SystemParamItem},
    },
    pbr::{
        MeshPipeline, MeshPipelineKey, RenderMeshInstances, SetMeshBindGroup, SetMeshViewBindGroup,
    },
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{
            allocator::MeshAllocator, MeshVertexBufferLayoutRef, RenderMesh, RenderMeshBufferInfo,
        },
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::binding_types::*,
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        storage::GpuShaderStorageBuffer,
        sync_world::MainEntity,
        view::{ExtractedView, NoFrustumCulling, RenderLayers},
        Render, RenderApp, RenderSet,
    },
};
use bytemuck::{Pod, Zeroable};

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/star_instancing2.wgsl";

pub struct StarInstancingPlugin;

impl Plugin for StarInstancingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CustomMaterialPlugin)
            .add_systems(Startup, setup)
            .add_systems(Update, manage_star_instances);
    }
}

#[derive(Component)]
struct StarInstanceHolder;

/// Spawns or despawns star instances
/// Spawns in fairly small batches to avoid stutter when galaxy config changes
/// - Might be a flag active during game loading that causes the spawn to run to finish
#[allow(clippy::too_many_arguments)]
fn manage_star_instances(
    mut commands: Commands,
    galaxy_config: Res<GalaxyConfig>,
    star_query: Query<(Entity, &Transform, &Star), Without<StarInstanceMarker>>,
    mut instance_data_query: Query<(Entity, &mut InstanceMaterialData), With<StarInstanceHolder>>,
    star_count: Res<StarCount>,
    mut extinction: ResMut<ExtinctionCache>,
    galaxy_render_settings: Res<GalaxyRenderConfig>,
) {
    extinction.required_size = star_count.count.max(1);

    let Ok((entity, mut instances)) = instance_data_query.single_mut() else {
        return;
    };

    if galaxy_render_settings.is_changed() {
        if galaxy_render_settings.draw_stars_to_background {
            commands
                .entity(entity)
                .insert(volume_upscaler::background_render_layer());
        } else {
            commands.entity(entity).insert(RenderLayers::layer(0));
        }
    }

    if !galaxy_config.star_instance_params.enabled {
        instances.0.clear();
        instances.0.push(InstanceData {
            position: Vec3::ZERO,
            index: 0.0,
            //color: star.color().extend(1.0).to_array(),
        });
        return;
    }

    if instances.0.len() != star_count.count + 1 {
        instances.0.resize(
            star_count.count + 1,
            InstanceData {
                position: Vec3::ZERO,
                index: 0.0,
                //color: [0.0; 4],
            },
        );
    }

    // add instancing components to stars that need them
    for (entity, transform, star) in star_query {
        instances.0[star.index as usize] = InstanceData {
            position: transform.translation,
            index: star.index as f32,
            //color: star.color().extend(1.0).to_array(),
        };
        commands.entity(entity).insert(StarInstanceMarker);
    }
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.spawn((
        Mesh3d(meshes.add(Rectangle::from_size(Vec2::splat(2.0)))),
        InstanceMaterialData(Vec::new()),
        // NOTE: Frustum culling is done based on the Aabb of the Mesh and the GlobalTransform.
        // As the cube is at the origin, if its Aabb moves outside the view frustum, all the
        // instanced cubes will be culled.
        // The InstanceMaterialData contains the 'GlobalTransform' information for this custom
        // instancing, and that is not taken into account with the built-in frustum culling.
        // We must disable the built-in frustum culling by adding the `NoFrustumCulling` marker
        // component to avoid incorrect culling.
        NoFrustumCulling,
        StarInstanceHolder,
    ));
}

#[derive(Component, Deref)]
struct InstanceMaterialData(Vec<InstanceData>);

impl ExtractComponent for InstanceMaterialData {
    type QueryData = (&'static InstanceMaterialData, Option<&'static RenderLayers>);
    type QueryFilter = ();
    type Out = (Self, RenderLayers);

    fn extract_component(
        (data, render_layers): QueryItem<'_, Self::QueryData>,
    ) -> Option<Self::Out> {
        let render_layers = render_layers.unwrap_or_default();
        Some((InstanceMaterialData(data.0.clone()), render_layers.clone()))
    }
}

struct CustomMaterialPlugin;

impl Plugin for CustomMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<InstanceMaterialData>::default());
        app.sub_app_mut(RenderApp)
            .init_resource::<StarInstancingUniforms>()
            .add_render_command::<Transparent3d, DrawCustom>()
            .init_resource::<SpecializedMeshPipelines<CustomPipeline>>()
            .add_systems(
                Render,
                (
                    queue_custom.in_set(RenderSet::QueueMeshes),
                    (prepare_instance_buffers, prepare_uniforms)
                        .in_set(RenderSet::PrepareResources),
                    prepare_star_instancing_bind_group.in_set(RenderSet::PrepareBindGroups),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp).init_resource::<CustomPipeline>();
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct InstanceData {
    position: Vec3,
    index: f32,
    //color: [f32; 4],
}

#[allow(clippy::too_many_arguments)]
fn queue_custom(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    custom_pipeline: Res<CustomPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<CustomPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    material_meshes: Query<(Entity, &MainEntity, &RenderLayers), With<InstanceMaterialData>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<(&ExtractedView, &Msaa, Option<&RenderLayers>)>,
) {
    let draw_custom = transparent_3d_draw_functions.read().id::<DrawCustom>();

    for (view, msaa, view_layers) in &views {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples());
        let view_layers = view_layers.unwrap_or_default();

        let view_key = msaa_key | MeshPipelineKey::from_hdr(view.hdr);
        let rangefinder = view.rangefinder3d();
        for (entity, main_entity, entity_layers) in &material_meshes {
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*main_entity)
            else {
                continue;
            };
            let Some(mesh) = meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };
            // skip if the view/entity render layers don't intersect
            if !view_layers.intersects(entity_layers) {
                continue;
            }

            let key = view_key
                | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology())
                | bevy::pbr::alpha_mode_pipeline_key(AlphaMode::Add, msaa);
            let pipeline = pipelines
                .specialize(&pipeline_cache, &custom_pipeline, key, &mesh.layout)
                .unwrap();
            transparent_phase.add(Transparent3d {
                entity: (entity, *main_entity),
                pipeline,
                draw_function: draw_custom,
                distance: rangefinder.distance_translation(&mesh_instance.translation),
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::None,
                indexed: true,
            });
        }
    }
}

#[derive(Component)]
struct InstanceBuffer {
    buffer: Buffer,
    length: usize,
}

fn prepare_instance_buffers(
    mut commands: Commands,
    query: Query<(Entity, &InstanceMaterialData)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, instance_data) in &query {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("instance data buffer"),
            contents: bytemuck::cast_slice(instance_data.as_slice()),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        commands.entity(entity).insert(InstanceBuffer {
            buffer,
            length: instance_data.len(),
        });
    }
}

#[derive(Resource)]
struct CustomPipeline {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
    bind_group_layout: BindGroupLayout,
}

impl FromWorld for CustomPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let mesh_pipeline = world.resource::<MeshPipeline>();
        let bind_group_layout = render_device.create_bind_group_layout(
            "star_instancing_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (
                    storage_buffer_read_only::<Vec4>(false),
                    uniform_buffer::<StarInstancingSetting>(false),
                ),
            ),
        );
        CustomPipeline {
            shader: world.load_asset(SHADER_ASSET_PATH),
            mesh_pipeline: mesh_pipeline.clone(),
            bind_group_layout,
        }
    }
}

impl SpecializedMeshPipeline for CustomPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;
        descriptor.label = Some("star_instancing_pipeline".into());
        descriptor.layout.push(self.bind_group_layout.clone());

        descriptor.vertex.shader = self.shader.clone();
        descriptor.vertex.buffers.push(VertexBufferLayout {
            array_stride: size_of::<InstanceData>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 3, // shader locations 0-2 are taken up by Position, Normal and UV attributes
                },
                /*
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size(),
                    shader_location: 4,
                },
                */
            ],
        });
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
        Ok(descriptor)
    }
}

#[derive(ShaderType, Pod, Zeroable, Clone, Copy, Debug, Default)]
#[repr(C)]
struct StarInstancingSetting {
    supersampling_offset: f32,
    padding: Vec3,
}

#[derive(Resource, Default)]
struct StarInstancingUniforms {
    supersampling_offset: UniformBuffer<StarInstancingSetting>,
}

fn prepare_uniforms(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    galaxy_render_settings: Res<GalaxyRenderConfig>,
    mut uniforms: ResMut<StarInstancingUniforms>,
) {
    uniforms.supersampling_offset.set(StarInstancingSetting {
        supersampling_offset: if galaxy_render_settings.draw_stars_to_background {
            0.25
        } else {
            1.0
        },
        padding: Vec3::ZERO,
    });
    uniforms
        .supersampling_offset
        .write_buffer(&render_device, &render_queue);
}

type DrawCustom = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetStarUniformsBindGroup<2>,
    DrawMeshInstanced,
);

#[derive(Resource)]
struct StarInstancingBindgroup(BindGroup);

fn prepare_star_instancing_bind_group(
    mut commands: Commands,
    star_instancing_pipeline: Res<CustomPipeline>,
    uniforms: Res<StarInstancingUniforms>,
    render_device: Res<RenderDevice>,
    extinction_cache: Res<ExtinctionCache>,
    ssbos: Res<RenderAssets<GpuShaderStorageBuffer>>,
) {
    let output_buffer = ssbos.get(&extinction_cache.output_buffer).unwrap();

    let uniform = uniforms.supersampling_offset.binding().unwrap();

    commands.insert_resource(StarInstancingBindgroup(render_device.create_bind_group(
        "Star_instancing_bind_group",
        &star_instancing_pipeline.bind_group_layout,
        &BindGroupEntries::sequential((output_buffer.buffer.as_entire_buffer_binding(), uniform)),
    )));
}

struct SetStarUniformsBindGroup<const I: usize>;
impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetStarUniformsBindGroup<I> {
    type Param = SRes<StarInstancingBindgroup>;
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        _: Option<()>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let bind_group = bind_group.into_inner();
        pass.set_bind_group(I, &bind_group.0, &[]);
        RenderCommandResult::Success
    }
}

struct DrawMeshInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawMeshInstanced {
    type Param = (
        SRes<RenderAssets<RenderMesh>>,
        SRes<RenderMeshInstances>,
        SRes<MeshAllocator>,
    );
    type ViewQuery = ();
    type ItemQuery = Read<InstanceBuffer>;

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        instance_buffer: Option<&'w InstanceBuffer>,
        (meshes, render_mesh_instances, mesh_allocator): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        // A borrow check workaround.
        let mesh_allocator = mesh_allocator.into_inner();

        let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(item.main_entity())
        else {
            return RenderCommandResult::Skip;
        };
        let Some(gpu_mesh) = meshes.into_inner().get(mesh_instance.mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };
        let Some(instance_buffer) = instance_buffer else {
            return RenderCommandResult::Skip;
        };
        let Some(vertex_buffer_slice) =
            mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id)
        else {
            return RenderCommandResult::Skip;
        };

        pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

        match &gpu_mesh.buffer_info {
            RenderMeshBufferInfo::Indexed {
                index_format,
                count,
            } => {
                let Some(index_buffer_slice) =
                    mesh_allocator.mesh_index_slice(&mesh_instance.mesh_asset_id)
                else {
                    return RenderCommandResult::Skip;
                };

                pass.set_index_buffer(index_buffer_slice.buffer.slice(..), 0, *index_format);
                pass.draw_indexed(
                    index_buffer_slice.range.start..(index_buffer_slice.range.start + count),
                    vertex_buffer_slice.range.start as i32,
                    0..instance_buffer.length as u32,
                );
            }
            RenderMeshBufferInfo::NonIndexed => {
                pass.draw(vertex_buffer_slice.range, 0..instance_buffer.length as u32);
            }
        }
        RenderCommandResult::Success
    }
}
