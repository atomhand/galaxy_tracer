#![feature(f16)]
#![feature(test)]
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowTheme};
use bevy_egui::EguiPlugin;

mod galaxy;
mod graphics;
mod ui;

mod prelude;

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
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: false,
        })
        .add_plugins((
            galaxy::GalaxyConfigPlugin,
            ui::UiPlugin,
            graphics::GraphicsPlugin,
        ))
        .run();
}
