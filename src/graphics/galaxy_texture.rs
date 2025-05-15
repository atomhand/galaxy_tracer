use crate::prelude::*;
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use rayon::prelude::*;
pub struct GalaxyTexturePlugin;

#[derive(Resource, Default, Clone, ExtractResource)]
pub struct GalaxyTexture {
    pub tex: Option<Handle<Image>>,
    pub luts: Option<Handle<Image>>,
    dimension: u32,
    generation: i32,
}

impl Plugin for GalaxyTexturePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<GalaxyTexture>::default())
            .insert_resource(GalaxyTexture::default())
            .add_systems(Update, update_texture);
    }
}

use bevy::render::{
    render_asset::RenderAssetUsages,
    render_resource::{Extent3d, TextureDimension, TextureFormat},
};

pub fn get_lut(config: &GalaxyConfig, render_settings: &GalaxyRenderConfig) -> Image {
    let width = render_settings.texture_dimension.next_power_of_two();
    let layers = 4;

    let disk_painter = ArmLutGenerator::new(config, &config.disk_params);

    let chunk_size: usize = 4;
    let mut texture_data = vec![0u8; (width * layers) as usize * chunk_size];

    texture_data
        .par_chunks_exact_mut(chunk_size)
        .enumerate()
        .for_each(|(i, chunk)| {
            let x = i % width as usize;
            let layer = i / width as usize;

            let val = match layer {
                0 => disk_painter.get_raw_winding(x as f32 / width as f32),
                1 => 0.0,
                2 => 0.0,
                3 => 0.0,
                _ => 0.0,
            };

            let slice = [val.to_le_bytes()].concat();

            chunk.copy_from_slice(&slice);
        });

    Image::new(
        Extent3d {
            width,
            height: 1,
            depth_or_array_layers: layers,
        },
        TextureDimension::D2,
        texture_data,
        TextureFormat::R32Float,
        RenderAssetUsages::RENDER_WORLD,
    )
}

pub fn get_texture(config: &GalaxyConfig, render_settings: &GalaxyRenderConfig) -> Image {
    let dimension = render_settings.texture_dimension.next_power_of_two();

    let disk_painter = ArmLutGenerator::new(config, &config.disk_params);
    let dust_painter = ArmLutGenerator::new(config, &config.dust_params);
    let stars_painter = ArmLutGenerator::new(config, &config.stars_params);

    let mut texture_data = vec![0u8; (dimension * dimension * 8) as usize];

    texture_data
        .par_chunks_exact_mut(8)
        .enumerate()
        .for_each(|(i, chunk)| {
            let x = i % dimension as usize;
            let y = i / dimension as usize;

            let p = Vec2::new(
                x as f32 / dimension as f32 * config.radius * 2.0 - config.radius,
                y as f32 / dimension as f32 * config.radius * 2.0 - config.radius,
            ) * render_settings.padding_coeff;

            let disk = disk_painter.get_xz_intensity(p);
            let dust = dust_painter.get_xz_intensity(p);
            let stars = stars_painter.get_xz_intensity(p);
            let winding = disk_painter.pos_winding(p);

            let slice = [
                (disk as f16).to_le_bytes(),
                (dust as f16).to_le_bytes(),
                (stars as f16).to_le_bytes(),
                (winding as f16).to_le_bytes(),
            ]
            .concat();

            chunk.copy_from_slice(&slice);
        });

    Image::new(
        Extent3d {
            width: dimension,
            height: dimension,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        texture_data,
        TextureFormat::Rgba16Float,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn update_texture(
    mut images: ResMut<Assets<Image>>,
    config: Res<GalaxyConfig>,
    render_settings: Res<GalaxyRenderConfig>,
    mut tex_holder: ResMut<GalaxyTexture>,
) {
    if config.generation != tex_holder.generation
        || tex_holder.tex.is_none()
        || tex_holder.dimension != render_settings.texture_dimension.next_power_of_two()
    {
        info!("Galaxy config updated, rebaking galaxy");
        let handle = images.add(get_texture(&config, &render_settings));
        tex_holder.tex = Some(handle);
        tex_holder.dimension = render_settings.texture_dimension.next_power_of_two();

        let lut_handle = images.add(get_lut(&config, &render_settings));
        tex_holder.luts = Some(lut_handle);
        tex_holder.generation = config.generation;
    }
}
