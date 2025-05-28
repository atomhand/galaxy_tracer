use super::StarCount;
use crate::prelude::*;
use bevy::prelude::*;
use rand::prelude::*;
use rayon::prelude::*;
use rand_chacha::ChaCha8Rng;

pub struct SpawnStarsPlugin;

impl Plugin for SpawnStarsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(StarSpawningControl {
            generation: -1,
            stars_placed: 0,
            rng : ChaCha8Rng::seed_from_u64(0),
        })
        .insert_resource(StarCount { count: 0 })
        .add_systems(Update, manage_star_instances);
    }
}

#[derive(Resource)]
pub struct StarSpawningControl {
    generation: i32,
    stars_placed: usize,
    rng : ChaCha8Rng,
}

#[derive(Component)]
pub struct Star {
    pub index: u32,
    mass: f32,
}

impl Star {
    fn temperature(&self) -> f32 {
        self.mass.powf(0.625) * 5772.0
    }
    fn simple_planck(temperature: f32) -> Vec3 {
        let mut res: Vec3 = Vec3::ZERO;
        let m = 1.0;
        for i in 0..3 {
            // +=.1 if you want to better sample the spectrum.
            let f = 1. + 0.5 * i as f32;
            res[i as usize] += 10.0 / m * (f * f * f) / (f32::exp(19.0e3 * f / temperature) - 1.);
            // Planck law
        }

        //res = res / res.max_element();
        res
    }

    pub fn color(&self) -> Vec3 {
        Self::simple_planck(self.temperature())
    }
}

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
    const BATCH_SIZE: usize = 4096;

    if star_instancing.generation != galaxy_config.generation {
        // cleanup existing stars
        for entity in &existing_star_query {
            commands.entity(entity).despawn();
        }
        // update params
        star_instancing.generation = galaxy_config.generation;
        star_count.count = (galaxy_config.stars_per_arm * galaxy_config.num_arms() as i32) as usize;
        star_instancing.stars_placed = 0;
        
        star_instancing.rng = ChaCha8Rng::seed_from_u64(galaxy_config.seed);
    }
    if !galaxy_config.star_instance_params.enabled {
        return;
    }
    // Spawn stars for the current batch
    if star_instancing.stars_placed < star_count.count {
        let batch_size = BATCH_SIZE.min(star_count.count - star_instancing.stars_placed);

        let stars_to_spawn = (0..batch_size).into_par_iter().map(|i| {
            let mut rng = ChaCha8Rng::seed_from_u64(galaxy_config.seed);
            rng.set_stream((star_instancing.stars_placed + i) as u64);

            (
                Transform::from_translation(sample_star_pos(&galaxy_config, &mut rng)),
                Star {
                    index : (i+star_instancing.stars_placed) as u32,
                mass : random_star_mass(&mut rng),
                }
            )
        }).collect::<Vec<(Transform,Star)>>();

        commands.spawn_batch(stars_to_spawn);
        star_instancing.stars_placed += batch_size;
    }
}

fn random_star_mass(rng: &mut ChaCha8Rng) -> f32 {
    let in_ranges = [
        (0.08..0.45, 0.25),  // M (Red Dwarf)
        (0.45..0.8, 0.5),    // K
        (0.8..1.04, 1.),     // G (Sol range)
        (1.04..1.4, 1.),     // F
        (1.4..2.1, 1.),      // A
        (2.1..16., 0.1),     // B
        (16. ..152., 0.001), // O
    ];
    let range = in_ranges
        .choose_weighted(rng, |item| item.1)
        .unwrap()
        .0
        .clone();
    rng.random_range(range)
}

fn sample_unit_circle(rng: &mut ChaCha8Rng) -> Vec2 {
    let length = rng.random::<f32>().sqrt();
    let angle = std::f32::consts::PI * rng.random_range(0.0..2.0);

    vec2(angle.cos(), angle.sin()) * length
}

fn sample_pos(rng: &mut ChaCha8Rng, radius: f32) -> Vec3 {
    let circle_sample = sample_unit_circle(rng) * radius;
    let height_sample: f32 = rng.random_range(-2.0..2.0);

    //height_sample /= height_sample.abs().sqrt();

    vec3(circle_sample.x, height_sample, circle_sample.y) * 2.0
}

fn sample_star_pos(galaxy_config: &GalaxyConfig, rng: &mut ChaCha8Rng) -> Vec3 {
    let arm_painter =
        super::GalaxyComponentDensity::new(galaxy_config, &galaxy_config.star_instance_params);

    let current_pos = sample_pos(rng, galaxy_config.radius);
    let mut best = current_pos;
    let weight = arm_painter.xyz_density(current_pos);
    let mut weight_sum = weight;

    for _ in 0..256 {
        let current_pos = sample_pos(rng, galaxy_config.radius);
        let weight = arm_painter.xyz_density(current_pos) + 0.0001;
        weight_sum += weight;

        if rng.random::<f32>() < weight / weight_sum {
            best = current_pos;
        }
    }

    best
}
