use bevy::prelude::*;
mod background_camera;
mod background_upscale;
pub mod prelude;

pub use background_camera::{BackgroundCamera,BACKGROUND_RENDER_LAYER,background_render_layer};

pub struct BackgroundRenderingPlugin;

impl Plugin for BackgroundRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            background_upscale::BackgroundUpscalePlugin,
            background_camera::BackgroundCameraPlugin,
        ));
    }
}
