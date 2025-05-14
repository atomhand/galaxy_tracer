use crate::graphics::ExtinctionCache;
use crate::prelude::*;
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::{
        mesh::MeshTag,
        render_resource::{AsBindGroup, ShaderRef},
        storage::ShaderStorageBuffer,
    },
};

const SHADER_ASSET_PATH: &str = "shaders/star_instancing.wgsl";

pub struct StarInstancingPlugin;

impl Plugin for StarInstancingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<StarInstanceMaterial>::default())
            .add_systems(Startup, init_resource)
            .add_systems(PostUpdate, (manage_star_instances, update_material));
    }
}

#[derive(Resource)]
struct StarInstancingControl {
    mesh_handle: Handle<Mesh>,
    material_handle: Handle<StarInstanceMaterial>,
}

/// Sets up the star instancing resource with the shared material and mesh
fn init_resource(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StarInstanceMaterial>>,
) {
    let mesh_handle = meshes.add(Rectangle::from_size(Vec2::splat(2.0)));

    let material_handle = materials.add(StarInstanceMaterial {
        alpha_mode: AlphaMode::Add,
        colors: Handle::default(),
        supersampling_offset_scale: 1.0,
    });

    commands.insert_resource(StarInstancingControl {
        mesh_handle,
        material_handle,
    });
}

fn update_material(
    star_instancing: Res<StarInstancingControl>,
    extinction: Res<crate::graphics::ExtinctionCache>,
    mut materials: ResMut<Assets<StarInstanceMaterial>>,
) {
    if extinction.is_changed() {
        if let Some(mat) = materials.get_mut(&star_instancing.material_handle) {
            mat.colors = extinction.output_buffer.clone();
        };
    }
}

#[derive(Component)]
pub struct StarInstanceMarker;

/// Spawns or despawns star instances
/// Spawns in fairly small batches to avoid stutter when galaxy config changes
/// - Might be a flag active during game loading that causes the spawn to run to finish
fn manage_star_instances(
    mut commands: Commands,
    galaxy_config: Res<GalaxyConfig>,
    star_query: Query<(Entity, &Star), Without<StarInstanceMarker>>,
    star_count: Res<StarCount>,
    star_instancing: Res<StarInstancingControl>,
    mut materials: ResMut<Assets<StarInstanceMaterial>>,
    mut extinction: ResMut<ExtinctionCache>,
) {
    extinction.required_size = star_count.count;

    if !galaxy_config.stars_params.enabled {
        return;
    }

    // update supersampling offset
    if let Some(mat) = materials.get_mut(&star_instancing.material_handle) {
        mat.supersampling_offset_scale = if galaxy_config.draw_stars_to_background {
            0.25
        } else {
            1.0
        };
    };

    // add instancing components to stars that need them
    for (entity, star) in star_query {
        commands
            .entity(entity)
            .insert((
                // For automatic instancing to take effect you need to
                // use the same mesh handle and material handle for each instance
                Mesh3d(star_instancing.mesh_handle.clone()),
                MeshMaterial3d(star_instancing.material_handle.clone()),
                // This is an optional component that can be used to help tie external data to a mesh instance
                MeshTag(star.index),
                StarInstanceMarker,
            ))
            .insert_if(volume_upscaler::background_render_layer(), || {
                galaxy_config.draw_stars_to_background
            });
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct StarInstanceMaterial {
    #[storage(0, read_only)]
    colors: Handle<ShaderStorageBuffer>,
    #[uniform(1)]
    supersampling_offset_scale: f32,
    alpha_mode: AlphaMode,
}

impl Material for StarInstanceMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }
}
