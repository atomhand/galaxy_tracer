use bevy::{
    diagnostic::FrameCount,
    prelude::*,
    render::{
        MainWorld, Render, RenderApp, RenderSet,
        camera::{MipBias, TemporalJitter},
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        sync_component::SyncComponentPlugin,
        sync_world::RenderEntity,
        view::RenderLayers,
    },
};

use super::background_upscale::BackgroundUpscaleSettings;

const UPSCALE_FACTOR: i32 = 4;
pub const BACKGROUND_RENDER_LAYER: usize = 999;

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
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct BackgroundImageOutput {
    pub image: Handle<Image>,
    pub reset: bool,
}

/// shorthand
pub fn background_render_layer() -> RenderLayers {
    RenderLayers::none().with(BACKGROUND_RENDER_LAYER)
}

pub struct BackgroundCameraPlugin;

impl Plugin for BackgroundCameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<BackgroundImageOutput>();
        app.add_plugins(SyncComponentPlugin::<BackgroundImageOutput>::default());
        app.add_plugins((ExtractComponentPlugin::<BackgroundChildCamera>::default(),));
        app.add_systems(
            Update,
            (
                setup_new_camera,
                update_uniform,
                update_target_size,
                cleanup,
            ),
        );

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_systems(ExtractSchedule, extract_background_output)
            .add_systems(
                Render,
                prepare_background_jitter_and_mip_bias.in_set(RenderSet::ManageViews),
            );
    }
}

/// This could be implemented more robustly
fn cleanup(
    mut commands: Commands,
    mut q_child: Query<(Entity, &mut Camera, &ChildOf), With<BackgroundChildCamera>>,
    q_parent: Query<&Camera, (With<BackgroundCamera>, Without<BackgroundChildCamera>)>,
) {
    // if parent is alive, copy their HDR settings
    // otherwise clean up the stray child
    for (entity, mut camera, child_of) in q_child.iter_mut() {
        if let Ok(parent_camera) = q_parent.get(child_of.parent()) {
            camera.hdr = parent_camera.hdr;
        } else {
            commands.entity(entity).despawn();
        }
    }
}

fn update_target_size(
    mut query: Query<(&Camera, &mut BackgroundImageOutput)>,
    mut images: ResMut<Assets<Image>>,
) {
    for (camera, background_image_output) in query.iter_mut() {
        let Some(viewport) = camera.physical_viewport_rect() else {
            continue;
        };

        let w = (viewport.max.x - viewport.min.x) / UPSCALE_FACTOR as u32;
        let h = (viewport.max.y - viewport.min.y) / UPSCALE_FACTOR as u32;

        let Some(image) = images.get(&background_image_output.image) else {
            continue;
        };

        if image.size() != uvec2(w, h) {
            let size = Extent3d {
                width: w,
                height: h,
                ..default()
            };
            if let Some(image) = images.get_mut(&background_image_output.image) {
                image.resize(size);
            }
        }
    }
}

fn setup_new_camera(
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
            &[0; 8],
            TextureFormat::Rgba16Float,
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
                    bevy::core_pipeline::tonemapping::Tonemapping::None,
                    Camera {
                        hdr: camera.hdr,
                        target: image_handle.clone().into(),
                        order: -1, // background camera needs to render before main pass
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
                .insert((
                    Msaa::Off,
                    BackgroundImageOutput {
                        image: image_handle.clone(),
                        reset: true,
                    },
                ))
                .add_child(cam);
        }
    }
}

fn update_uniform(frame_count: Res<FrameCount>, mut query: Query<&mut BackgroundUpscaleSettings>) {
    for mut pass_settings in query.iter_mut() {
        pass_settings.current_pixel =
            (frame_count.0 as i32 % (UPSCALE_FACTOR * UPSCALE_FACTOR)) as f32;
    }
}

/// custom extract schedule for BackgroundImageOutput, mostly just so we can toggle Reset after extracting
fn extract_background_output(mut commands: Commands, mut main_world: ResMut<MainWorld>) {
    let mut cameras_3d = main_world
        .query_filtered::<(RenderEntity, &Camera, &mut BackgroundImageOutput), With<Camera3d>>();

    for (entity, camera, mut background_output) in cameras_3d.iter_mut(&mut main_world) {
        let mut entity_commands = commands
            .get_entity(entity)
            .expect("Camera entity wasn't synced.");
        if camera.is_active {
            entity_commands.insert(background_output.clone());
            background_output.reset = false;
        } else {
            // TODO: needs better strategy for cleaning up
            entity_commands.remove::<(BackgroundImageOutput,)>();
        }
    }
}

fn prepare_background_jitter_and_mip_bias(
    frame_count: Res<FrameCount>,
    mut query: Query<(Entity, &mut TemporalJitter, Option<&MipBias>), With<BackgroundChildCamera>>,
    mut commands: Commands,
) {
    // The -1 frame offset is a clunky hack for a sytem ordering change that is caused by giving the camera an order of -1
    // Honestly I haven't worked out WHAT systems are being executed out of order (intuitively I didn't think it should be affected)
    // But it dsynced the jitter offsets used in the camera and material respectively I added this offset
    // There may be a more elegant way to do this
    let p = (frame_count.0 as i32 - 1) % (UPSCALE_FACTOR * UPSCALE_FACTOR);
    let mapping = [4, 6, 12, 15, 0, 3, 14, 10, 2, 7, 9, 1, 11, 5, 8, 13];
    // inverse mapping is array(4, 11, 8, 5, 0, 13, 1, 9, 14, 10, 7, 12, 2, 15, 6, 3);

    let p = mapping[p as usize];
    let sub_coord = ivec2(p % UPSCALE_FACTOR, p / UPSCALE_FACTOR);

    let offset = vec2(-0.5, -0.5) + (sub_coord.as_vec2() + 0.5) / UPSCALE_FACTOR as f32;

    for (entity, mut jitter, mip_bias) in &mut query {
        jitter.offset = offset;

        if mip_bias.is_none() {
            commands.entity(entity).insert(MipBias(-1.0));
        }
    }
}
