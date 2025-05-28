use crate::prelude::*;
use bevy::prelude::*;
use rand::{distr::Distribution, prelude::*};
use rand_chacha::ChaCha8Rng;
use rand_distr::Normal;
use std::ops::Range;

pub struct GalaxyGenerationSettings {
    pub seed : Option<u64>,
    pub arms_range: Range<u32>,
    pub winding_b_range: Range<f32>,
    pub winding_n_range: Range<f32>,
    pub dust_distribution: Normal<f32>,
}

impl GalaxyGenerationSettings {
    pub fn new(seed : Option<u64>) -> Self {
        Self {
            seed,
            arms_range: 2..6,
            winding_b_range: 0.2..1.0,
            winding_n_range: 1.0..6.0,
            dust_distribution: Normal::new(900.0, 500.0).unwrap(),
        }
    }
}

pub fn generate_galaxy(settings: GalaxyGenerationSettings) -> GalaxyConfig {
    let seed = settings.seed.unwrap_or(rand::rng().random());

    let radius = 500.0f32;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let num_arms = rng.random_range(settings.arms_range);
    let base_width = (num_arms as f32 * 0.25).min(2.0);
    let width_distribution = Normal::new(base_width, base_width / 4.0).unwrap();

    let base_angle_offset = 360 / num_arms as i32;

    let mut current_angle = 0;
    let mut upper_angle = 0;

    let mut arms = Vec::<ArmConfig>::new();
    for _i in 0..num_arms {
        arms.push(ArmConfig {
            offset: current_angle,
        });
        upper_angle += base_angle_offset;
        current_angle = if rng.random_bool(0.75) {
            current_angle + base_angle_offset
        } else {
            rng.random_range((current_angle + base_angle_offset / 2)..upper_angle)
        }
    }

    let winding_b = rng.random_range(settings.winding_b_range);
    let winding_n = rng.random_range(settings.winding_n_range);

    let bulge_strength = rng.random_range(100.0..200.0);
    let bulge_radius = 10.0;

    let stars_per_arm = (radius * radius) as i32 / 50;

    let disk_params = ComponentConfig {
        strength: 900.,
        component_type: ComponentType::Disk,
        arm_width: width_distribution.sample(&mut rng),
        y_thickness: 0.02,
        radial_extent: 0.4,
        radial_dropoff: 0.05,
        noise_octaves: 4,
        noise_tilt: 0.3,
        noise_winding_factor: 0.5,
        noise_scale: 5.0,
        ..default()
    };

    let dust_params = ComponentConfig {
        component_type: ComponentType::Dust,
        strength: settings.dust_distribution.sample(&mut rng),
        arm_width: width_distribution.sample(&mut rng),
        y_thickness: rng.random_range(0.02..0.05),
        radial_extent: 0.45,
        radial_dropoff: 0.05,
        angular_offset: rng.random_range(-50f32..-15.),

        noise_scale: 6.0,
        noise_offset: 1.0,
        noise_octaves: 5,
        noise_winding_factor: rng.random_range(0.25..0.5),
        ..default()
    };
    let star_volume_params = ComponentConfig {
        component_type: ComponentType::StarVolume,
        strength: 150.0,
        arm_width: width_distribution.sample(&mut rng) + 0.4,
        y_thickness: 0.01,
        angular_offset: -20.,

        radial_dropoff: 0.05,
        radial_extent: 0.45,

        noise_tilt: -1.0,
        noise_winding_factor: rng.random_range(0.75..1.0),
        noise_scale: 9.0,
        noise_persistence: 2.0,
        noise_offset: 10.0,
        noise_octaves: 3,
        ..default()
    };
    let star_instance_params = ComponentConfig {
        component_type: ComponentType::StarInstances, // Match disk
        ..disk_params
    };
    let h2_params = ComponentConfig {
        enabled : false,
        component_type: ComponentType::H2,
        strength: 250.0,
        arm_width: width_distribution.sample(&mut rng),
        y_thickness: 0.005,
        angular_offset: -10.,

        radial_dropoff: 0.1,
        radial_extent: 0.45,

        noise_tilt: 0.0,
        noise_winding_factor: rng.random_range(0.2..0.5),
        noise_scale: 6.0,
        noise_persistence: 1.0,
        noise_offset: 0.0,
        noise_octaves: 6,
        ..default()
    };

    GalaxyConfig {
        seed,
        generation: 1,
        radius,
        winding_b,
        winding_n,
        arms,
        bulge_strength,
        bulge_radius,
        bulge_intensity: 1.0,
        stars_per_arm,
        disk_params,
        dust_params,
        star_volume_params,
        star_instance_params,
        h2_params,
    }
}
