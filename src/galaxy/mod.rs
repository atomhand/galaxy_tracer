use bevy::prelude::*;

mod galaxy_component_density;
mod galaxy_config;
mod spawn_stars;

pub use spawn_stars::{SpawnStarsPlugin, Star};

pub use galaxy_component_density::GalaxyComponentDensity;
pub use galaxy_config::{
    ArmConfig, ComponentConfig, ComponentType, GalaxyConfig, GalaxyConfigPlugin, GalaxyRenderConfig,
};

#[derive(Resource)]
pub struct StarCount {
    pub count: usize,
}
