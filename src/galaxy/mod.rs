use bevy::prelude::*;

mod arm_lut_generator;
mod galaxy_config;
mod spawn_stars;

pub use spawn_stars::{SpawnStarsPlugin, Star};

pub use arm_lut_generator::ArmLutGenerator;
pub use galaxy_config::{
    ArmConfig, ComponentConfig, ComponentType, GalaxyConfig, GalaxyConfigPlugin,
};

#[derive(Resource)]
pub struct StarCount {
    pub count: usize,
}
