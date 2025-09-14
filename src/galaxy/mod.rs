use bevy::prelude::*;

mod galaxy_component_density;
mod galaxy_config;
pub mod galaxy_generator;
mod spawn_stars;
mod star_picking;
pub use star_picking::{PickableStar, StarPickingPlugin};

pub use spawn_stars::{SpawnStarsPlugin, Star};

pub use galaxy_component_density::GalaxyComponentDensity;
pub use galaxy_config::{
    ArmConfig, ComponentConfig, ComponentType, GalaxyConfig, GalaxyConfigPlugin, GalaxyRenderConfig,
};

#[derive(Resource)]
pub struct StarCount {
    pub count: usize,
    pub major_stars_count: usize,
}
