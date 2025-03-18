use bevy::prelude::*;

use bevy_simple_text_input::TextInputPlugin;

mod config_ui;
mod fps_widget;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            fps_widget::FpsWidgetPlugin,
            config_ui::ConfigPlugin,
            TextInputPlugin,
        ));
    }
}
