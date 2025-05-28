use super::star_picking::PickableStar;
use super::GalaxyComponentDensity;
use super::StarCount;
use crate::prelude::*;
use bevy::prelude::*;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use std::ops::Range;

pub struct SpawnStarsPlugin;

impl Plugin for SpawnStarsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(StarSpawningControl {
            generation: -1,
            state: StarSpawningState::Finished,
            rng: ChaCha8Rng::seed_from_u64(0),
        })
        .insert_resource(StarCount {
            count: 0,
            major_stars_count: 0,
        })
        .add_systems(Update, manage_star_instances);
    }
}

enum StarSpawningState {
    PlacingMinorStars(usize),
    PlacingMajorStars,
    Finished,
}

#[derive(Resource)]
pub struct StarSpawningControl {
    generation: i32,
    state: StarSpawningState,
    rng: ChaCha8Rng,
}

#[derive(Component, Clone, Default)]
pub struct Star {
    pub index: u32,
    mass: f32,
    is_major: bool,
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
        let base_col = Self::simple_planck(self.temperature());

        if self.is_major {
            base_col * 50.0 + base_col.normalize() * 100.0
        } else {
            base_col * 0.25 + base_col.normalize() * 0.25
        }
    }
}

/// Spawns or despawns star instances
/// Spawns in fairly small batches to avoid stutter when galaxy config changes
/// - Might be a flag active during game loading that causes the spawn to run to finish
fn manage_star_instances(
    mut commands: Commands,
    mut star_count: ResMut<StarCount>,
    galaxy_config: Res<GalaxyConfig>,
    existing_star_query: Query<(Entity, &Star)>,
    mut star_instancing: ResMut<StarSpawningControl>,
) {
    const BATCH_SIZE: usize = 4096;

    if star_instancing.generation != galaxy_config.generation {
        // cleanup existing stars
        for (entity, _star) in &existing_star_query {
            commands.entity(entity).despawn();
        }
        // update params
        star_instancing.generation = galaxy_config.generation;
        star_count.count = (galaxy_config.stars_per_arm * galaxy_config.num_arms() as i32) as usize;
        star_count.major_stars_count = 512.min(star_count.count);
        star_instancing.state = StarSpawningState::PlacingMajorStars;

        star_instancing.rng = ChaCha8Rng::seed_from_u64(galaxy_config.seed);
    }
    if !galaxy_config.star_instance_params.enabled {
        return;
    }
    // Spawn stars for the current batch

    star_instancing.state = match star_instancing.state {
        StarSpawningState::PlacingMinorStars(current_star) => {
            if current_star < star_count.count {
                let batch_size = BATCH_SIZE.min(star_count.count - current_star);

                let star_sampler = StarSampler::new_minor(&galaxy_config);
                let stars_to_spawn = (0..batch_size)
                    .into_par_iter()
                    .map(|i| {
                        let mut rng = ChaCha8Rng::seed_from_u64(galaxy_config.seed);
                        rng.set_stream((current_star + i) as u64);

                        (
                            Transform::from_translation(star_sampler.sample_star_pos(&mut rng)),
                            Star {
                                index: (i + current_star) as u32,
                                mass: star_sampler.random_star_mass(&mut rng),
                                is_major: false,
                            },
                        )
                    })
                    .collect::<Vec<_>>();

                commands.spawn_batch(stars_to_spawn);
                StarSpawningState::PlacingMinorStars(current_star + batch_size)
            } else {
                StarSpawningState::Finished
            }
        }
        StarSpawningState::PlacingMajorStars => {
            let star_sampler = StarSampler::new_major(&galaxy_config);

            let mut confirmed = Vec::<Vec3>::new();
            let mut iterations = 0;

            while confirmed.len() < star_count.major_stars_count && iterations < 10 {
                let batch_size = star_count.major_stars_count * 10;
                // get candidate positions
                let candidates = (0..batch_size)
                    .into_par_iter()
                    .map(|i| {
                        let mut rng = ChaCha8Rng::seed_from_u64(galaxy_config.seed);
                        rng.set_stream((i + iterations * batch_size) as u64);
                        star_sampler.sample_star_pos(&mut rng)
                    })
                    .collect::<Vec<Vec3>>();
                iterations += 1;

                // confirm candidates if they are within  the spacing range
                let sqd = galaxy_config.major_stars_spacing * galaxy_config.major_stars_spacing;
                for candidate in candidates {
                    let mut clear = true;
                    for other in &confirmed {
                        if candidate.distance_squared(*other) < sqd {
                            clear = false;
                            break;
                        }
                    }
                    if clear {
                        confirmed.push(candidate);
                        if confirmed.len() == star_count.major_stars_count {
                            break;
                        }
                    }
                }
            }
            info!(
                "Spawning {} major stars (target {}) after {} iterations",
                confirmed.len(),
                star_count.major_stars_count,
                iterations
            );

            let stars_to_spawn = confirmed
                .into_par_iter()
                .enumerate()
                .map(|(i, pos)| {
                    let mut rng = ChaCha8Rng::seed_from_u64(galaxy_config.seed);
                    rng.set_stream((i) as u64);

                    (
                        Transform::from_translation(pos),
                        Star {
                            index: (i) as u32,
                            mass: star_sampler.random_star_mass(&mut rng),
                            is_major: true,
                        },
                        PickableStar {},
                    )
                })
                .collect::<Vec<_>>();

            commands.spawn_batch(stars_to_spawn);
            StarSpawningState::PlacingMinorStars(star_count.major_stars_count)
        }
        StarSpawningState::Finished => StarSpawningState::Finished,
    };
}

struct StarSampler<'a> {
    star_types: Vec<(Range<f32>, f32)>, // mass range, weight
    base_position_weight: f32,
    galaxy_config: &'a GalaxyConfig,
    arm_painter: GalaxyComponentDensity<'a>,
}

impl<'a> StarSampler<'a> {
    fn new_minor(galaxy_config: &'a GalaxyConfig) -> Self {
        Self {
            star_types: vec![
                (0.08..0.45, 0.25), // M (Red Dwarf)
                (0.45..0.8, 0.5),   // K
                (0.8..1.04, 1.),    // G (Sol range)
                (1.04..1.4, 1.),    // F
                (1.4..2.1, 1.),     // A
                                    //(2.1..16., 0.1),     // B
                                    //(16. ..152., 0.001), // O
            ],
            base_position_weight: 0.0001,
            galaxy_config,
            arm_painter: GalaxyComponentDensity::new(
                galaxy_config,
                &galaxy_config.star_instance_params,
            ),
        }
    }
    fn new_major(galaxy_config: &'a GalaxyConfig) -> Self {
        Self {
            star_types: vec![
                (0.08..0.45, 0.5), // M (Red Dwarf)
                (0.45..0.8, 1.),   // K
                (0.8..1.04, 1.),   // G (Sol range)
                (1.04..1.4, 1.),   // F
                (1.4..2.1, 1.),    // A
                (2.1..16., 0.2),   // B
                (16. ..152., 0.1), // O
            ],
            base_position_weight: 0.,
            galaxy_config,
            arm_painter: GalaxyComponentDensity::new(
                galaxy_config,
                &galaxy_config.star_instance_params,
            ),
        }
    }

    fn random_star_mass(&self, rng: &mut ChaCha8Rng) -> f32 {
        let range = self
            .star_types
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

    fn sample_pos(&self, rng: &mut ChaCha8Rng) -> Vec3 {
        let circle_sample = Self::sample_unit_circle(rng) * self.galaxy_config.radius;
        let height_sample: f32 = rng.random_range(-2.0..2.0);

        //height_sample /= height_sample.abs().sqrt();

        vec3(circle_sample.x, height_sample, circle_sample.y) * 2.0
    }

    fn sample_star_pos(&self, rng: &mut ChaCha8Rng) -> Vec3 {
        let current_pos = self.sample_pos(rng);
        let mut best = current_pos;
        let weight = self.arm_painter.xyz_density(current_pos);
        let mut weight_sum = weight;

        for _ in 0..256 {
            let current_pos = self.sample_pos(rng);
            let weight = self.arm_painter.xyz_density(current_pos) + self.base_position_weight;
            weight_sum += weight;

            if rng.random::<f32>() < weight / weight_sum {
                best = current_pos;
            }
        }

        best
    }
}
