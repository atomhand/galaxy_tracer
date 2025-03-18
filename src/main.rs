#![feature(f16)]
#![feature(test)]
use bevy::prelude::*;
use bevy_egui::EguiPlugin;

use bevy::window::{PresentMode, WindowTheme};

mod benchmarks;
mod camera;
mod galaxy_config;
mod galaxy_xz_painter;
mod render;
mod ui;
pub use galaxy_config::GalaxyConfig;
mod galaxy_texture;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Galaxy Tracer".into(),
                name: Some("bevy.app".into()),
                //resolution: (1920.,1080.).into(),
                present_mode: PresentMode::AutoNoVsync,
                fit_canvas_to_parent: true,
                prevent_default_event_handling: false,
                window_theme: Some(WindowTheme::Dark),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin)
        .add_plugins((
            galaxy_config::GalaxyConfigPlugin,
            camera::CameraPlugin,
            render::RenderPlugin,
            ui::UiPlugin,
            galaxy_texture::GalaxyTexturePlugin,
        ))
        .run();
}
