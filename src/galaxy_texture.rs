use super::galaxy_xz_painter::GalaxyPainter;
use bevy::prelude::*;
pub struct GalaxyTexturePlugin;

#[derive(Resource, Default)]
pub struct GalaxyTexture {
    pub tex: Option<Handle<Image>>,
}

impl Plugin for GalaxyTexturePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GalaxyTexture::default())
            .add_systems(Update, update_texture);
    }
}

use crate::GalaxyConfig;

use bevy::render::{
    render_asset::RenderAssetUsages,
    render_resource::{Extent3d, TextureDimension, TextureFormat},
};

fn get_texture(config: &GalaxyConfig) -> Image {
    const DIMENSIONS: u32 = 1024; //((config.radius * 1.2).ceil() as u32).next_power_of_two();

    let painter = GalaxyPainter::from(config);

    let mut texture_data = Vec::<u8>::with_capacity((DIMENSIONS * DIMENSIONS * 4) as usize);
    for y in 0..DIMENSIONS {
        for x in 0..DIMENSIONS {
            let p = Vec2::new(
                x as f32 / DIMENSIONS as f32 * config.radius * 2.0 - config.radius,
                y as f32 / DIMENSIONS as f32 * config.radius * 2.0 - config.radius,
            ) * config.padding_coeff;

            let val = painter.get_xz_intensity(p, 0.0);

            texture_data.extend_from_slice(&val.intensity.to_le_bytes());
            //texture_data.extend_from_slice(&val.winding.to_le_bytes());
        }
    }

    Image::new(
        Extent3d {
            width: DIMENSIONS,
            height: DIMENSIONS,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        texture_data,
        TextureFormat::R32Float,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn update_texture(
    mut images: ResMut<Assets<Image>>,
    mut config: ResMut<GalaxyConfig>,
    mut tex_holder: ResMut<GalaxyTexture>,
) {
    if config.should_rebake || tex_holder.tex.is_none() {
        info!("Galaxy config updated, rebaking galaxy");
        let handle = images.add(get_texture(&config));
        tex_holder.tex = Some(handle);
        config.should_rebake = false;
    }
}
