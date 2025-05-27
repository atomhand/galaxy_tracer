use crate::prelude::*;
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use rayon::prelude::*;

pub struct PsfTexturePlugin;

#[derive(Resource, Default, Clone, ExtractResource)]
pub struct PsfTexture {
    pub tex: Option<Handle<Image>>,
    dimension: u32,
}

impl Plugin for PsfTexturePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<PsfTexture>::default())
            .insert_resource(PsfTexture::default())
            .add_systems(Update, update_texture);
    }
}

use bevy::render::{
    render_asset::RenderAssetUsages,
    render_resource::{Extent3d, TextureDimension, TextureFormat},
};

fn draw_star(p: Vec2) -> f32 {
    let intensity = 1.0;
    let c = 1.0;

    let d: f32 = p.length();
    let spectrum = intensity * c;

    let mut col = spectrum / (d * d * d);

    const ARMS_SCALE: f32 = 1.0 / 1.4;

    let d = (p * vec2(50.0, 0.5)).length() * ARMS_SCALE;
    col += spectrum / (d * d * d);
    let d = (p * vec2(0.5, 50.0)).length() * ARMS_SCALE;
    col += spectrum / (d * d * d);

    col.clamp(0.0, 1.0)
}

pub fn get_texture(render_settings: &GalaxyRenderConfig) -> Image {
    let dimension = render_settings.texture_dimension.next_power_of_two();
    let mut colour_data = vec![0.0; (dimension * dimension) as usize];
    colour_data.par_iter_mut().enumerate().for_each(|(i, col)| {
        let x = i % dimension as usize;
        let y = i / dimension as usize;

        let p = Vec2::new(x as f32 / dimension as f32, y as f32 / dimension as f32)
            - Vec2::new(0.5, 0.5);

        *col = draw_star(p * 8.0);
    });

    let num_mips = dimension.ilog2();

    let mut mip_chain = vec![colour_data];
    for mip in 1..num_mips {
        let in_dimension = dimension >> (mip - 1);
        let out_dimension = dimension >> mip;

        let in_image = &mip_chain[(mip - 1) as usize];
        let mut result = vec![0.0; (out_dimension * out_dimension) as usize];

        for x in 0..out_dimension {
            for y in 0..out_dimension {
                let x0 = x * 2;
                let x1 = x * 2 + 1;
                let y0 = y * 2;
                let y1 = y * 2 + 1;

                result[(x + y * out_dimension) as usize] = (in_image
                    [(x0 + y0 * in_dimension) as usize]
                    + in_image[(x1 + y0 * in_dimension) as usize]
                    + in_image[(x0 + y1 * in_dimension) as usize]
                    + in_image[(x1 + y1 * in_dimension) as usize])
                    / 4.0;
            }
        }

        mip_chain.push(result);
    }

    let texture_data = mip_chain
        .concat()
        .iter()
        .flat_map(|colour| {
            /*[
                (colour.x as f16).to_le_bytes(),
                (colour.y as f16).to_le_bytes(),
                (colour.z as f16).to_le_bytes(),
                (colour.w as f16).to_le_bytes(),
            ].concat()*/
            colour.to_le_bytes()
        })
        .collect();

    let mut image = Image::new_uninit(
        Extent3d {
            width: dimension,
            height: dimension,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::R32Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = num_mips;
    image.data = Some(texture_data);
    image
}

fn update_texture(
    mut images: ResMut<Assets<Image>>,
    render_settings: Res<GalaxyRenderConfig>,
    mut tex_holder: ResMut<PsfTexture>,
) {
    if tex_holder.tex.is_none()
        || tex_holder.dimension != render_settings.texture_dimension.next_power_of_two()
    {
        let handle = images.add(get_texture(&render_settings));
        tex_holder.tex = Some(handle);
        tex_holder.dimension = render_settings.texture_dimension.next_power_of_two();
    }
}
