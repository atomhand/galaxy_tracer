use crate::prelude::*;
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

fn arm_component_ui(id: i32, arm_config: &mut ArmConfig, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new(format!("Arm config {}", id))
        .default_open(true)
        .show(ui, |ui| {
            ui.checkbox(&mut arm_config.enabled, "Enabled");
            ui.add(egui::Slider::new(&mut arm_config.offset, 0..=360).text("Angular Offset"));
        });
}

fn component_ui(config: &mut ComponentConfig, ui: &mut egui::Ui) {
    let heading = match config.component_type {
        ComponentType::Disk => "Disk Config",
        ComponentType::Dust => "Dust Config",
        ComponentType::Stars => "Stars Config",
    };

    let minval = ComponentConfig::MIN;
    let maxval = ComponentConfig::MAX;

    egui::CollapsingHeader::new(heading).show(ui, |ui| {
        ui.checkbox(&mut config.enabled, "Component Enabled");
        ui.add(
            egui::Slider::new(&mut config.strength, minval.strength..=maxval.strength)
                .text("Strength"),
        );

        ui.label("Shape");
        ui.group(|ui| {
            ui.add(
                egui::Slider::new(&mut config.arm_width, minval.arm_width..=maxval.arm_width)
                    .text("Arm Width (Inverse)"),
            );
            ui.add(
                egui::Slider::new(
                    &mut config.y_thickness,
                    minval.y_thickness..=maxval.y_thickness,
                )
                .text("Thickness (Y)"),
            );
            ui.add(
                egui::Slider::new(
                    &mut config.radial_extent,
                    minval.radial_extent..=maxval.radial_extent,
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
                    &mut config.angular_offset,
                    minval.angular_offset..=maxval.angular_offset,
                )
                .text("Angular Offset"),
            );
        });
        ui.label("Noise");

        ui.group(|ui| {
            ui.checkbox(&mut config.noise_enabled, "Enabled");
            // speciual case for stars, ugly hack but w/e
            if config.component_type == ComponentType::Stars {
                ui.add(egui::Slider::new(&mut config.noise_scale, 1.0..=100.0).text("Frequency"));
            } else {
                ui.add(
                    egui::Slider::new(
                        &mut config.noise_scale,
                        minval.noise_scale..=maxval.noise_scale,
                    )
                    .text("Frequency"),
                );
            }
            ui.add(
                egui::Slider::new(
                    &mut config.noise_texture_frequency,
                    minval.noise_texture_frequency..=maxval.noise_texture_frequency,
                )
                .text("Frequency (Texture)"),
            );
            ui.add(
                egui::Slider::new(
                    &mut config.noise_offset,
                    minval.noise_offset..=maxval.noise_offset,
                )
                .text("Offset"),
            );
            ui.add(
                egui::Slider::new(
                    &mut config.noise_winding_factor,
                    minval.noise_winding_factor..=maxval.noise_winding_factor,
                )
                .text("Winding Factor"),
            );
            ui.add(
                egui::Slider::new(
                    &mut config.noise_tilt,
                    minval.noise_tilt..=maxval.noise_tilt,
                )
                .text("Tilt"),
            );
            ui.add(
                egui::Slider::new(
                    &mut config.noise_persistence,
                    minval.noise_persistence..=maxval.noise_persistence,
                )
                .text("Persistence"),
            );

            ui.add(
                egui::Slider::new(
                    &mut config.noise_octaves,
                    minval.noise_octaves..=maxval.noise_octaves,
                )
                .text("Octaves"),
            );
        });
    });
    ui.separator();
}

fn ui_system(mut contexts: EguiContexts, mut galaxy_config: ResMut<GalaxyConfig>) {
    let ctx = contexts.ctx_mut();

    egui::SidePanel::left("side_panel")
        .default_width(200.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Configuration");

                egui::CollapsingHeader::new("Galaxy Parameters").show(ui, |ui| {
                    ui.add(
                        egui::Slider::new(&mut galaxy_config.radius, 100.0..=1000.0).text("Radius"),
                    );
                    ui.add(
                        egui::Slider::new(&mut galaxy_config.texture_root, 4..=11)
                            .custom_formatter(|n, _| {
                                let n = n as u32;
                                format!("{}", 2u32.pow(n))
                            })
                            .text("Texture Size"),
                    );
                    ui.add(
                        egui::Slider::new(&mut galaxy_config.padding_coeff, 1.0..=2.0)
                            .text("Padding Coefficient"),
                    );
                    ui.add(
                        egui::Slider::new(&mut galaxy_config.raymarch_steps, 1..=256)
                            .text("Raymarch Steps"),
                    );
                    ui.checkbox(
                        &mut galaxy_config.runtime_noise,
                        "Procedural Noise Evaluation",
                    );
                    let mut inv_exposure = 1.0 / galaxy_config.exposure;
                    ui.add(
                        egui::Slider::new(&mut inv_exposure, 1.0..=1000.0)
                            .text("Exposure (Inverse)"),
                    );
                    galaxy_config.exposure = 1.0 / inv_exposure;

                    ui.add(
                        egui::Slider::new(&mut galaxy_config.winding_b, 0.5..=3.0).text("windingB"),
                    );
                    ui.add(
                        egui::Slider::new(&mut galaxy_config.winding_n, 0.5..=10.0)
                            .text("windingN"),
                    );

                    ui.checkbox(&mut galaxy_config.diagnostic_mode, "Performance Diagnostic");
                    ui.checkbox(&mut galaxy_config.flat_mode, "Flat Render");
                });

                ui.separator();
                egui::CollapsingHeader::new("Arms").show(ui, |ui| {
                    for i in 0..4 {
                        let arm_config = &mut galaxy_config.arm_configs[i as usize];
                        arm_component_ui(i, arm_config, ui);
                    }
                });
                ui.separator();

                egui::CollapsingHeader::new("Bulge Parameters").show(ui, |ui| {
                    ui.add(
                        egui::Slider::new(&mut galaxy_config.bulge_strength, 1.0..=50.0)
                            .text("Strength"),
                    );
                    ui.add(
                        egui::Slider::new(&mut galaxy_config.bulge_radius, 1.0..=20.0)
                            .text("Scale Factor"),
                    );
                });
                ui.separator();

                component_ui(&mut galaxy_config.disk_params, ui);
                component_ui(&mut galaxy_config.dust_params, ui);
                component_ui(&mut galaxy_config.stars_params, ui);
            });
        });
}
