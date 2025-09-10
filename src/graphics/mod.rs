use bevy::prelude::*;

mod galaxy_texture;
mod galaxy_volume_render;

mod extinction_cache;
mod shader_types;

mod star_instancing;
use star_instancing::StarInstanceMarker;

use extinction_cache::ExtinctionCache;
use galaxy_texture::GalaxyTexture;

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            //classic_star_instancing::StarInstancingPlugin,
            galaxy_volume_render::GalaxyVolumePlugin,
            galaxy_texture::GalaxyTexturePlugin,
            extinction_cache::ExtinctionCachePlugin,
            volume_upscaler::BackgroundRenderingPlugin,
            star_instancing::StarInstancingPlugin,
        ));
    }
}
