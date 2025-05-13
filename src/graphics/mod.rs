use bevy::prelude::*;

mod galaxy_texture;
mod noise_texture;
mod galaxy_volume_render;

mod shader_types;

use galaxy_texture::GalaxyTexture;

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            galaxy_volume_render::GalaxyVolumePlugin,
            galaxy_texture::GalaxyTexturePlugin,
            noise_texture::NoiseTexturePlugin,
            volume_upscaler::BackgroundRenderingPlugin
        ));
    }
}
