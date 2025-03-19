use bevy::prelude::*;

#[derive(Resource, Clone, PartialEq)]
pub struct GalaxyConfig {
    pub generation : i32,


    pub texture_dimension: u32,
    pub radius: f32,
    pub n_arms: i32,
    pub arm_offsets: [f32; 4],

    pub winding_b: f32,
    pub winding_n: f32,
    pub exposure : f32,

    pub max_stars: i32,
    pub spacing: f32,
    pub padding_coeff: f32,

    pub arm_configs : [ArmConfig; 4],

    pub bulge_strength : f32,
    pub bulge_radius : f32,
    pub bulge_intensity : f32,

    pub disk_params: ComponentConfig,
    pub dust_params: ComponentConfig,
    pub stars_params: ComponentConfig,
}

#[derive(Resource)]
struct GalaxyConfigOld(GalaxyConfig);

impl Default for GalaxyConfigOld {
    fn default() -> Self {
        Self(GalaxyConfig {
            generation : -1,
            .. default()
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
    pub strength: f32,
    pub arm_width: f32,
    pub y_offset: f32,
    pub radial_start: f32,
    pub radial_dropoff: f32,
    pub delta_angle: f32,
    pub winding_coefficient: f32,
    pub noise_scale: f32,
    pub noise_offset: f32,
    pub noise_tilt: f32,
    pub noise_freq: f32,
}

impl Default for ComponentConfig {
    fn default() -> Self {
        Self {
            component_type: ComponentType::Disk,
            strength: 1.0,
            arm_width: 0.5,
            y_offset: 0.001,
            radial_start: 0.5,
            radial_dropoff: 0.1,
            delta_angle: 0.0,
            winding_coefficient: 0.5,
            noise_scale: 1.0,
            noise_offset: 0.0,
            noise_tilt: 1.0,
            noise_freq: 1.0,
        }
    }
}

impl ComponentConfig {
    pub const MIN: Self = Self {
        component_type: ComponentType::Disk,
        strength: 1.0,
        arm_width: 0.001,
        y_offset: 0.001,
        radial_start: 0.0,
        radial_dropoff: 0.1,
        delta_angle: -180.0,
        winding_coefficient: 0.0,
        noise_scale: 0.01,
        noise_offset: -1.0,
        noise_tilt: -1.0,
        noise_freq: 0.1,
    };
    pub const MAX: Self = Self {
        component_type: ComponentType::Disk,
        strength: 1000.0,
        arm_width: 1.0,
        y_offset: 0.05,
        radial_start: 1.0,
        radial_dropoff: 0.6,
        delta_angle: 180.0,
        winding_coefficient: 0.5,
        noise_scale: 5.0,
        noise_offset: 1.0,
        noise_tilt: 1.0,
        noise_freq: 2.0,
    };
}
pub struct GalaxyConfigPlugin;

impl Plugin for GalaxyConfigPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GalaxyConfig::default())
            .insert_resource(GalaxyConfigOld::default())
            .add_systems(Update, apply_ui_updates);
    }
}

fn apply_ui_updates(
    mut galaxy_config_old: ResMut<GalaxyConfigOld>,
    mut galaxy_config: ResMut<GalaxyConfig>,
) {
    if galaxy_config.is_changed() {
        if *galaxy_config != galaxy_config_old.0 {
            galaxy_config.generation += 1;

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
}

#[derive(Default, Clone, Copy, PartialEq)]
pub struct ArmConfig {
    pub enabled: bool,
    pub offset: i32, // in degrees
}

impl Default for GalaxyConfig {
    fn default() -> Self {
        Self {
            generation : 1,
            texture_dimension: 512,
            bulge_strength : 30.0,
            bulge_radius : 5.0,
            bulge_intensity : 1.0,
            exposure : 0.01,
            radius: 500.0, // in parsecs
            max_stars: 1000,
            spacing: 40.0,
            n_arms: 3,
            arm_configs: [
                ArmConfig{
                    enabled : true,
                    offset : 0
                },
                ArmConfig{
                    enabled : false,
                    offset : 90
                },
                ArmConfig{
                    enabled : true,
                    offset : 180
                },
                ArmConfig{
                    enabled : false,
                    offset : 270
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
            disk_params: ComponentConfig{
                component_type : ComponentType::Disk,
                strength : 900.0,
                arm_width : 0.3,
                y_offset : 0.02,
                radial_dropoff : 0.,
                radial_start : 0.4,
                //noise_tilt : 0.3,
                //wirl : 0.1,
                .. default()
            },
            dust_params: ComponentConfig{
                component_type : ComponentType::Dust,
                strength : 250.0,
                arm_width : 0.25,
                y_offset : 0.02,
                radial_start : 0.45,
                noise_scale : 3.0,
                .. default()
            },
            stars_params: ComponentConfig{
                component_type : ComponentType::Stars,
                .. default()
            },
        }
    }
}
