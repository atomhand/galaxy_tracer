use bevy::prelude::*;

mod galaxy_texture;
mod galaxy_volume_render;

mod extinction_cache;
mod shader_types;

mod star_instancing;
pub use star_instancing::{StarInstanceMarker, StarInstancingPlugin};

pub use extinction_cache::ExtinctionCache;
use galaxy_texture::GalaxyTexture;

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            galaxy_volume_render::GalaxyVolumePlugin,
            galaxy_texture::GalaxyTexturePlugin,
            extinction_cache::ExtinctionCachePlugin,
            volume_upscaler::BackgroundRenderingPlugin,
        ));
    }
}
