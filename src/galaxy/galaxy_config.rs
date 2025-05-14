use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};

#[derive(Resource, Clone, PartialEq, ExtractResource)]
pub struct GalaxyConfig {
    pub generation: i32,
    pub raymarch_steps: u32,

    pub draw_volume_to_background: bool,

    pub texture_root: u32,
    pub texture_dimension: u32,
    pub radius: f32,
    pub n_arms: i32,
    pub arm_offsets: [f32; 4],

    pub winding_b: f32,
    pub winding_n: f32,
    pub exposure: f32,

    pub spacing: f32,
    pub padding_coeff: f32,

    pub arm_configs: [ArmConfig; 4],

    pub bulge_strength: f32,
    pub bulge_radius: f32,
    pub bulge_intensity: f32,

    pub diagnostic_mode: bool,

    pub stars_per_arm: i32,
    pub draw_stars_to_background: bool,

    pub disk_params: ComponentConfig,
    pub dust_params: ComponentConfig,
    pub stars_params: ComponentConfig,
}

#[derive(Resource)]
struct GalaxyConfigOld(GalaxyConfig);

impl Default for GalaxyConfigOld {
    fn default() -> Self {
        Self(GalaxyConfig {
            generation: -1,
            ..default()
        })
    }
}

#[derive(Clone, PartialEq)]
pub enum ComponentType {
    Disk,
    Dust,
    Stars,
}

#[derive(Clone, PartialEq)]
pub struct ComponentConfig {
    pub component_type: ComponentType,
    pub enabled: bool,
    pub strength: f32,
    pub arm_width: f32,
    pub y_thickness: f32,
    pub radial_extent: f32,
    pub radial_dropoff: f32,
    pub angular_offset: f32,
    pub noise_winding_factor: f32,
    pub noise_scale: f32,
    pub noise_offset: f32,
    pub noise_tilt: f32,
    pub noise_persistence: f32,
    pub noise_octaves: u32,
    pub noise_enabled: bool,
}

impl Default for ComponentConfig {
    fn default() -> Self {
        Self {
            component_type: ComponentType::Disk,
            enabled: true,
            strength: 1.0,
            arm_width: 0.5,
            y_thickness: 0.001,
            radial_extent: 0.5,
            radial_dropoff: 0.1,
            angular_offset: 0.0,
            noise_winding_factor: 0.5,
            noise_scale: 1.0,
            noise_offset: 0.0,
            noise_tilt: 1.0,
            noise_persistence: 1.0,
            noise_octaves: 5,
            noise_enabled: true,
        }
    }
}

impl ComponentConfig {
    pub const MIN: Self = Self {
        component_type: ComponentType::Disk,
        enabled: false,
        strength: 0.0,
        arm_width: 0.001,
        y_thickness: 0.001,
        radial_extent: 0.0,
        radial_dropoff: 0.05,
        angular_offset: -180.0,
        noise_winding_factor: 0.0,
        noise_scale: 0.5,
        noise_offset: -1.0,
        noise_tilt: -1.0,
        noise_persistence: 0.1,
        noise_octaves: 0,
        noise_enabled: false,
    };
    pub const MAX: Self = Self {
        component_type: ComponentType::Disk,
        enabled: true,
        strength: 2000.0,
        arm_width: 1.0,
        y_thickness: 0.05,
        radial_extent: 1.0,
        radial_dropoff: 0.6,
        angular_offset: 180.0,
        noise_winding_factor: 0.5,
        noise_scale: 10.0,
        noise_offset: 1.0,
        noise_tilt: 1.0,
        noise_persistence: 2.0,
        noise_octaves: 10,
        noise_enabled: true,
    };
}
pub struct GalaxyConfigPlugin;

impl Plugin for GalaxyConfigPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GalaxyConfig::default())
            .insert_resource(GalaxyConfigOld::default())
            .add_systems(Update, apply_ui_updates)
            .add_plugins(ExtractResourcePlugin::<GalaxyConfig>::default());
    }
}

fn apply_ui_updates(
    mut galaxy_config_old: ResMut<GalaxyConfigOld>,
    mut galaxy_config: ResMut<GalaxyConfig>,
) {
    if galaxy_config.is_changed() && *galaxy_config != galaxy_config_old.0 {
        galaxy_config.generation += 1;

        galaxy_config.texture_dimension = 2u32.pow(galaxy_config.texture_root);

        let mut arms = 0;
        for i in 0..4 {
            let ui = galaxy_config.arm_configs[i];

            if ui.enabled {
                galaxy_config.arm_offsets[arms] = (ui.offset as f32).to_radians();
                arms += 1;
            }
        }
        galaxy_config.n_arms = arms as i32;

        galaxy_config_old.0 = galaxy_config.clone();
    }
}

#[derive(Default, Clone, Copy, PartialEq)]
pub struct ArmConfig {
    pub enabled: bool,
    pub offset: i32, // in degrees
}

impl Default for GalaxyConfig {
    fn default() -> Self {
        Self {
            diagnostic_mode: false,
            draw_volume_to_background: true,
            raymarch_steps: 128,
            generation: 1,
            texture_root: 9,
            texture_dimension: 512,
            bulge_strength: 100.0,
            bulge_radius: 9.0,
            bulge_intensity: 1.0,
            exposure: 0.01,
            radius: 500.0, // in parsecs
            stars_per_arm: 10000,
            // Tends to look extremely bad in motion
            draw_stars_to_background: false,
            spacing: 40.0,
            n_arms: 3,
            arm_configs: [
                ArmConfig {
                    enabled: true,
                    offset: 0,
                },
                ArmConfig {
                    enabled: false,
                    offset: 90,
                },
                ArmConfig {
                    enabled: true,
                    offset: 180,
                },
                ArmConfig {
                    enabled: false,
                    offset: 270,
                },
            ],
            arm_offsets: [
                0.0,
                90f32.to_radians(),
                180f32.to_radians(),
                270f32.to_radians(),
            ],
            winding_b: 0.5,
            winding_n: 4.0,
            padding_coeff: 1.5,
            disk_params: ComponentConfig {
                component_type: ComponentType::Disk,
                strength: 900.0,
                arm_width: 0.3,
                y_thickness: 0.02,
                radial_dropoff: 0.05,
                radial_extent: 0.4,
                noise_octaves: 10,
                noise_tilt: 0.3,
                noise_winding_factor: 0.1,
                ..default()
            },
            dust_params: ComponentConfig {
                component_type: ComponentType::Dust,
                strength: 900.0,
                arm_width: 0.25,
                y_thickness: 0.02,
                radial_extent: 0.45,
                radial_dropoff: 0.05,
                noise_scale: 6.0,
                angular_offset: -45.,
                noise_offset: 1.0,
                noise_octaves: 5,
                noise_winding_factor: 0.25,
                ..default()
            },
            stars_params: ComponentConfig {
                component_type: ComponentType::Stars, // Match disk
                strength: 900.0,
                arm_width: 0.3,
                y_thickness: 0.02,
                radial_dropoff: 0.05,
                radial_extent: 0.4,
                noise_octaves: 10,
                noise_tilt: 0.3,
                noise_winding_factor: 0.1,
                ..default()
            },
        }
    }
}
