use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

pub struct ConfigEguiPlugin;

impl Plugin for ConfigEguiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, configure_visuals_system)
            .add_systems(Update, ui_system);
    }
}

fn configure_visuals_system(mut contexts: EguiContexts) {
    contexts.ctx_mut().set_visuals(egui::Visuals {
        window_corner_radius: 0.0.into(),
        ..Default::default()
    });
}

use crate::galaxy_config::{ArmConfig, ComponentConfig, ComponentType, GalaxyConfigUi};

fn arm_component_ui(id: i32, arm_config: &mut ArmConfig, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new(format!("Arm config {}", id)).show(ui, |ui| {
        ui.checkbox(&mut arm_config.enabled, "Enabled");
        ui.add(egui::Slider::new(&mut arm_config.offset, 0..=360).text("Angular Offset"));
    });
    ui.separator();
}

fn component_ui(config: &mut ComponentConfig, ui: &mut egui::Ui) {
    let heading = match (config.component_type) {
        ComponentType::Disk => "Disk Config",
        ComponentType::Dust => "Dust Config",
        ComponentType::Stars => "Stars Config",
    };

    let minval = ComponentConfig::MIN;
    let maxval = ComponentConfig::MAX;

    egui::CollapsingHeader::new(heading).show(ui, |ui| {
        ui.add(
            egui::Slider::new(&mut config.strength, minval.strength..=maxval.strength)
                .text("Strength"),
        );
        ui.add(
            egui::Slider::new(&mut config.arm_width, minval.arm_width..=maxval.arm_width)
                .text("Inverse Arm Width"),
        );
        ui.add(
            egui::Slider::new(&mut config.y_offset, minval.y_offset..=maxval.y_offset)
                .text("Y Offset"),
        );
        ui.add(
            egui::Slider::new(
                &mut config.radial_start,
                minval.radial_start..=maxval.radial_start,
            )
            .text("Radial Extent"),
        );
        ui.add(
            egui::Slider::new(
                &mut config.radial_dropoff,
                minval.radial_dropoff..=maxval.radial_dropoff,
            )
            .text("Central Falloff"),
        );
        ui.add(
            egui::Slider::new(
                &mut config.delta_angle,
                minval.delta_angle..=maxval.delta_angle,
            )
            .text("Delta Angle"),
        );
        ui.add(
            egui::Slider::new(
                &mut config.winding_coefficient,
                minval.winding_coefficient..=maxval.winding_coefficient,
            )
            .text("Winding Coeff."),
        );
        // speciual case for stars, ugly hack but w/e
        if config.component_type == ComponentType::Stars {
            ui.add(egui::Slider::new(&mut config.noise_scale, 1.0..=100.0).text("Noise Scale"));
        } else {
            ui.add(
                egui::Slider::new(
                    &mut config.noise_scale,
                    minval.noise_scale..=maxval.noise_scale,
                )
                .text("Noise Scale"),
            );
        }
        ui.add(
            egui::Slider::new(
                &mut config.noise_offset,
                minval.noise_offset..=maxval.noise_offset,
            )
            .text("Noise Offset"),
        );
        ui.add(
            egui::Slider::new(
                &mut config.noise_tilt,
                minval.noise_tilt..=maxval.noise_tilt,
            )
            .text("Noise Tilt"),
        );
        ui.add(
            egui::Slider::new(
                &mut config.noise_freq,
                minval.noise_freq..=maxval.noise_freq,
            )
            .text("Noise Freq"),
        );
    });
    ui.separator();
}

fn ui_system(mut contexts: EguiContexts, mut galaxy_ui_config: ResMut<GalaxyConfigUi>) {
    let ctx = contexts.ctx_mut();

    egui::SidePanel::left("side_panel")
        .default_width(200.0)
        .show(ctx, |ui| {
            ui.heading("Configuration");

            egui::CollapsingHeader::new("Galaxy Parameters").show(ui, |ui| {
                ui.add(
                    egui::Slider::new(&mut galaxy_ui_config.radius, 100.0..=1000.0).text("Radius"),
                );
                ui.add(
                    egui::Slider::new(&mut galaxy_ui_config.texture_size, 4..=11).custom_formatter(|n, _| {
                        let n = n as u32;
                        format!("{}",2u32.pow(n))
                    })
                        .text("Texture Size"),
                );

                ui.add(
                    egui::Slider::new(&mut galaxy_ui_config.winding_b, 0.5..=3.0).text("windingB"),
                );
                ui.add(
                    egui::Slider::new(&mut galaxy_ui_config.winding_n, 0.5..=10.0).text("windingN"),
                );
            });
            egui::CollapsingHeader::new("Bulge Parameters").show(ui, |ui| {
                ui.add(
                    egui::Slider::new(&mut galaxy_ui_config.bulge_strength, 0.01..=0.1).text("Strength"),
                );

                // Mild Hack:
                // Transform the (ordinarily inverted) bulge Strength param to a range that is a bit more convenient to reason about
                //let mut str = 1.0 / galaxy_ui_config.bulge_radius;
                ui.add(
                    egui::Slider::new(&mut galaxy_ui_config.bulge_radius, 0.01..=1.0).text("Scale Factor"),
                );
                //galaxy_ui_config.bulge_radius = 1.0 / str;
            });
            ui.separator();
            for i in 0..4 {
                let arm_config = &mut galaxy_ui_config.arm_configs[i as usize];
                arm_component_ui(i, arm_config, ui);
            }

            component_ui(&mut galaxy_ui_config.disk_config, ui);
            component_ui(&mut galaxy_ui_config.dust_config, ui);
            component_ui(&mut galaxy_ui_config.star_config, ui);
        });
}
