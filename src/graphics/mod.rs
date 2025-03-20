use bevy::prelude::*;

mod galaxy_texture;
mod noise_texture;
mod render;

mod benchmarks;

use galaxy_texture::GalaxyTexture;

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            render::RenderPlugin,
            galaxy_texture::GalaxyTexturePlugin,
            noise_texture::NoiseTexturePlugin,
        ));
    }
}
