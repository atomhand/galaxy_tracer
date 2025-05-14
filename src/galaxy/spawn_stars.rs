use super::StarCount;
use crate::prelude::*;
use bevy::prelude::*;
use rand::prelude::*;
use rayon::prelude::*;

pub struct SpawnStarsPlugin;

impl Plugin for SpawnStarsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(StarSpawningControl {
            generation: -1,
            stars_left_to_place: 0,
            next_star_index: 0,
        })
        .insert_resource(StarCount { count: 0 })
        .add_systems(Update, manage_star_instances);
    }
}

#[derive(Resource)]
pub struct StarSpawningControl {
    generation: i32,
    stars_left_to_place: i32,
    next_star_index: u32,
}

#[derive(Component)]
pub struct Star(pub u32);

/// Spawns or despawns star instances
/// Spawns in fairly small batches to avoid stutter when galaxy config changes
/// - Might be a flag active during game loading that causes the spawn to run to finish
fn manage_star_instances(
    mut commands: Commands,
    mut star_count: ResMut<StarCount>,
    galaxy_config: Res<GalaxyConfig>,
    existing_star_query: Query<Entity, With<Star>>,
    mut star_instancing: ResMut<StarSpawningControl>,
) {
    const BATCH_SIZE: i32 = 4096;

    if star_instancing.generation != galaxy_config.generation {
        // cleanup existing stars
        for entity in &existing_star_query {
            commands.entity(entity).despawn();
        }
        // update params
        star_instancing.generation = galaxy_config.generation;
        star_count.count = (galaxy_config.stars_per_arm * galaxy_config.n_arms) as usize;
        star_instancing.stars_left_to_place = star_count.count as i32;
        star_instancing.next_star_index = 0;
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
            commands.spawn((
                Transform::from_translation(pos),
                Star(star_instancing.next_star_index),
            ));
            star_instancing.next_star_index += 1;
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
