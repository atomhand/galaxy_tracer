use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;

mod camera;
mod config_egui;
mod fps_widget;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            fps_widget::FpsWidgetPlugin,
            config_egui::ConfigEguiPlugin,
            camera::CameraPlugin,
        ))
        // Egui mouse input culling (see below)
        .add_systems(
            PreUpdate, 
            absorb_egui_inputs
                .after(bevy_egui::input::write_egui_input_system)
                .before(bevy_egui::begin_pass_system),
        );
    }
}

// Cull mouse inputs pre-frame if Egui has captured the pointer

// Source and useful discussion here https://github.com/vladbat00/bevy_egui/issues/47
// Not doing anything about keyboard input for now
//   - Ideally an active textbox should capture the input
//   - I think that's probably a little more complicated to implement

fn absorb_egui_inputs(
    mut contexts: bevy_egui::EguiContexts,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut mouse_wheel: ResMut<Events<MouseWheel>>,
) {
    let ctx = contexts.ctx_mut();
    if !(ctx.wants_pointer_input() || ctx.is_pointer_over_area()) {
        return;
    }

    mouse.reset_all();
    mouse_wheel.clear();
}