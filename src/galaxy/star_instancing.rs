use crate::prelude::*;
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::{
        mesh::MeshTag,
        render_resource::{AsBindGroup, ShaderRef}, storage::ShaderStorageBuffer,
    },
};
use rand::prelude::*;
use rayon::prelude::*;

const SHADER_ASSET_PATH: &str = "shaders/star_instancing.wgsl";

pub struct StarInstancingPlugin;

impl Plugin for StarInstancingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<StarInstanceMaterial>::default())
            .add_systems(Startup, init_resource)
            .add_systems(Update, (manage_star_instances, update_material));
    }
}

#[derive(Resource)]
struct StarInstancingControl {
    generation: i32,
    stars_left_to_place: i32,
    num_stars: i32,
    current_star_index: u32,
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
        generation: -1,
        stars_left_to_place: 0,
        current_star_index: 0,
        num_stars: 0,
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
    existing_star_query: Query<Entity, With<StarInstanceMarker>>,
    mut star_instancing: ResMut<StarInstancingControl>,
    mut materials: ResMut<Assets<StarInstanceMaterial>>,
    mut extinction: ResMut<crate::graphics::ExtinctionCache>,
) {
    const BATCH_SIZE: i32 = 4096;

    if star_instancing.generation != galaxy_config.generation {
        // cleanup existing stars
        for entity in &existing_star_query {
            commands.entity(entity).despawn();
        }
        // update params
        star_instancing.generation = galaxy_config.generation;
        star_instancing.num_stars = galaxy_config.stars_per_arm * galaxy_config.n_arms;
        star_instancing.stars_left_to_place = star_instancing.num_stars;
        star_instancing.current_star_index = 0;
        extinction.required_size = star_instancing.num_stars as usize;

        if let Some(mat) = materials.get_mut(&star_instancing.material_handle) {
            mat.supersampling_offset_scale = if galaxy_config.draw_stars_to_background {
                0.25
            } else {
                1.0
            };
        };
    }
    if !galaxy_config.stars_params.enabled {
        return;
    }
    // Spawn stars for the current batch
    if star_instancing.stars_left_to_place > 0 {
        let batch_size = star_instancing.stars_left_to_place.min(BATCH_SIZE);

        let mut star_positions = vec![Vec3::ZERO; batch_size as usize];
        star_positions.par_iter_mut().for_each(|pos| {
            *pos = sample_star_pos(&galaxy_config);
        });

        for pos in star_positions {
            commands
                .spawn((
                    // For automatic instancing to take effect you need to
                    // use the same mesh handle and material handle for each instance
                    Mesh3d(star_instancing.mesh_handle.clone()),
                    MeshMaterial3d(star_instancing.material_handle.clone()),
                    // This is an optional component that can be used to help tie external data to a mesh instance
                    MeshTag(star_instancing.current_star_index),
                    Transform::from_translation(pos),
                    StarInstanceMarker,
                ))
                .insert_if(volume_upscaler::background_render_layer(), || {
                    galaxy_config.draw_stars_to_background
                });
            star_instancing.current_star_index += 1;
        }
        star_instancing.stars_left_to_place -= batch_size;
    }
}

fn sample_unit_circle(rng: &mut ThreadRng) -> Vec2 {
    let length = rng.random::<f32>().sqrt();
    let angle = std::f32::consts::PI * rng.random_range(0.0..2.0);

    vec2(angle.cos(), angle.sin()) * length
}

fn sample_pos(rng: &mut ThreadRng, radius: f32) -> Vec3 {
    let circle_sample = sample_unit_circle(rng) * radius;
    let height_sample: f32 = rng.random_range(-2.0..2.0);

    //height_sample /= height_sample.abs().sqrt();

    vec3(circle_sample.x, height_sample, circle_sample.y) * 2.0
}

fn sample_star_pos(galaxy_config: &GalaxyConfig) -> Vec3 {
    let mut rng = rand::rng();

    let arm_painter = super::ArmLutGenerator::new(galaxy_config, &galaxy_config.stars_params);

    let current_pos = sample_pos(&mut rng, galaxy_config.radius);
    let mut best = current_pos;
    let weight = arm_painter.get_xyz_intensity(current_pos);
    let mut weight_sum = weight;

    for _ in 0..256 {
        let current_pos = sample_pos(&mut rng, galaxy_config.radius);
        let weight = arm_painter.get_xyz_intensity(current_pos) + 0.0001;
        weight_sum += weight;

        if rng.random::<f32>() < weight / weight_sum {
            best = current_pos;
        }
    }

    best
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
