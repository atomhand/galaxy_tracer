use bevy::prelude::*;

#[derive(Resource)]
pub struct GalaxyConfig {
    pub radius: f32,
    pub n_arms: i32,
    pub arm_offsets: [f32; 4],
    pub winding_b: f32,
    pub winding_n: f32,
    pub max_stars: i32,
    pub spacing: f32,
    pub padding_coeff: f32,
}

#[derive(Resource)]
pub struct GalaxyConfigUi {
    pub arm_configs: [ArmConfig; 4],
}

impl Default for GalaxyConfigUi {
    fn default() -> Self {
        Self {
            arm_configs: [
                ArmConfig {
                    enabled: true,
                    offset: 0,
                },
                ArmConfig {
                    enabled: true,
                    offset: 90,
                },
                ArmConfig {
                    enabled: true,
                    offset: 180,
                },
                ArmConfig {
                    enabled: true,
                    offset: 270,
                },
            ],
        }
    }
}

pub struct GalaxyConfigPlugin;

impl Plugin for GalaxyConfigPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GalaxyConfig::default())
            .insert_resource(GalaxyConfigUi::default())
            .add_systems(Update, apply_ui_updates);
    }
}

fn apply_ui_updates(
    galaxy_config_ui: Res<GalaxyConfigUi>,
    mut galaxy_config: ResMut<GalaxyConfig>,
) {
    if galaxy_config_ui.is_changed() {
        let mut arms = 0;

        for i in 0..4 {
            let ui = galaxy_config_ui.arm_configs[i];

            if ui.enabled {
                galaxy_config.arm_offsets[arms] = (ui.offset as f32).to_radians();
                arms += 1;
            }
        }
        galaxy_config.n_arms = arms as i32;
    }
}

// NOTE
// The world space coordinate system is scaled to parsecs, for now
// In the future it would probably be wise to use separate coordinate systems for the galaxy and systems
//  --- The transitions will get a bit fiddly tho which is why I have not bothered for now
//  --- I'm assuming precision errors will eventually become a problem and force my hand

#[derive(Default, Clone, Copy)]
pub struct ArmConfig {
    pub enabled: bool,
    pub offset: i32, // in degrees
}

impl GalaxyConfig {
    pub const AU_SCALE: f32 = 0.1; // scale of 1 AU to a Parsec
    pub const CELESTIAL_BODIES_SCALE: f32 = 20.0; // boost to the radius of celestial bodies relative to distance
    pub const PLANETS_SCALE: f32 = 1.0; // radius of a jupiter-sized planet relative to a sun-size star
    pub const SOLAR_RADIUS: f32 = 0.00465 * Self::AU_SCALE * Self::CELESTIAL_BODIES_SCALE; // Radius of Sol in Au
    pub const MAX_SYSTEM_BODIES: usize = 12; // Used for UI slots and stuff

    pub const HYPERLANE_VISUAL_STAR_CLEARANCE: f32 = 10.0;

    pub const GALACTIC_INTEGER_SCALE: i32 = 10000;
}
impl Default for GalaxyConfig {
    fn default() -> Self {
        Self {
            radius: 500.0, // in parsecs
            max_stars: 1000,
            spacing: 40.0,
            n_arms: 3,
            arm_offsets: [
                0.0,
                90f32.to_radians(),
                180f32.to_radians(),
                270f32.to_radians(),
            ],
            winding_b: 1.0,
            winding_n: 6.0,
            padding_coeff: 1.5,
        }
    }
}
