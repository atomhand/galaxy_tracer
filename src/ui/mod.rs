use bevy::prelude::*;

mod config_egui;
mod fps_widget;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((fps_widget::FpsWidgetPlugin, config_egui::ConfigEguiPlugin));
    }
}
