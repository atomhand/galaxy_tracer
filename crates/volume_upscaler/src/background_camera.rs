use bevy::{
    diagnostic::FrameCount,
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        camera::{MipBias, TemporalJitter},
        extract_component::{ExtractComponent, ExtractComponentPlugin,
        },
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        view::RenderLayers,
    },
};

use super::background_upscale::BackgroundUpscaleSettings;

const UPSCALE_FACTOR: i32 = 4;
pub const BACKGROUND_RENDER_LAYER : usize = 999;

/// The target camera
#[derive(Component)]
#[require(BackgroundUpscaleSettings)]
pub struct BackgroundCamera;

/// The child camera that renders the lowres background image
#[derive(Component, Default, Clone, ExtractComponent)]
#[require(TemporalJitter)]
struct BackgroundChildCamera;

/// This is the lowres background texture
/// Component on the parent camera, render target of the child camera
#[derive(Component, Default, Clone, Reflect, ExtractComponent)]
#[reflect(Component, Default, Clone)]
pub struct BackgroundImageOutput {
    pub image: Handle<Image>,
}

/// shorthand
pub fn background_render_layer() -> RenderLayers {
    RenderLayers::none().with(BACKGROUND_RENDER_LAYER)
}

pub struct BackgroundCameraPlugin;

impl Plugin for BackgroundCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<BackgroundImageOutput>::default(),
            ExtractComponentPlugin::<BackgroundChildCamera>::default(),
        ));
        app.add_systems(Update, (setup, update));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(
            Render,
            prepare_background_jitter_and_mip_bias.in_set(RenderSet::ManageViews),
        );
    }
}

fn setup(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &Camera,
            Option<&mut BackgroundImageOutput>,
            &mut BackgroundUpscaleSettings,
        ),
        Added<BackgroundCamera>,
    >,
    mut images: ResMut<Assets<Image>>,
) {
    for (entity, camera, background_camera, mut pass_settings) in query.iter_mut() {
        let Some(viewport) = camera.physical_viewport_rect() else {
            continue;
        };

        let w = (viewport.max.x - viewport.min.x) / UPSCALE_FACTOR as u32;
        let h = (viewport.max.y - viewport.min.y) / UPSCALE_FACTOR as u32;

        pass_settings.dimensions = Vec2::new(w as f32, h as f32);

        let size = Extent3d {
            width: w,
            height: h,
            ..default()
        };

        let mut image = Image::new_fill(
            size,
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Bgra8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_DST
            | TextureUsages::RENDER_ATTACHMENT;

        let image_handle = images.add(image);

        if let Some(mut background_camera) = background_camera {
            // modify existing images
            background_camera.image = image_handle;
        } else {
            // spawn new camera and background pass images
            let cam = commands
                .spawn((
                    Msaa::Off,
                    Camera3d::default(),
                    Camera {
                        target: image_handle.clone().into(),
                        clear_color: Color::BLACK.into(),
                        ..default()
                    },
                    Transform::from_translation(Vec3::ZERO).looking_at(Vec3::ZERO, Vec3::Y),
                    BackgroundChildCamera,
                    RenderLayers::none().with(BACKGROUND_RENDER_LAYER),
                ))
                .id();

            commands
                .entity(entity)
                .insert(
                    (Msaa::Off,BackgroundImageOutput {
                    image: image_handle.clone(),
                }))
                .add_child(cam);
        }
    }
}

fn update(frame_count: Res<FrameCount>, mut query: Query<&mut BackgroundUpscaleSettings>) {
    for mut pass_settings in query.iter_mut() {
        pass_settings.current_pixel =
            (frame_count.0 as i32 % (UPSCALE_FACTOR * UPSCALE_FACTOR)) as f32;
    }
}

fn prepare_background_jitter_and_mip_bias(
    frame_count: Res<FrameCount>,
    mut query: Query<(Entity, &mut TemporalJitter, Option<&MipBias>), With<BackgroundChildCamera>>,
    mut commands: Commands,
) {
    let p = frame_count.0 as i32 % (UPSCALE_FACTOR * UPSCALE_FACTOR);
    let mapping = [4,6,12,15,0,3,14,10,2,7,9,1,11,5,8,13];
    // inverse mapping is array(4, 11, 8, 5, 0, 13, 1, 9, 14, 10, 7, 12, 2, 15, 6, 3);

    let p =mapping[p as usize];
    let sub_coord = ivec2(p % UPSCALE_FACTOR, p / UPSCALE_FACTOR);

    let offset = vec2(-0.5, -0.5) + (sub_coord.as_vec2() + 0.5) / UPSCALE_FACTOR as f32;

    for (entity, mut jitter, mip_bias) in &mut query {
        jitter.offset = offset;

        if mip_bias.is_none() {
            commands.entity(entity).insert(MipBias(-1.0));
        }
    }
}
