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
    egui::CollapsingHeader::new(format!("Arm config {id}"))
        .default_open(true)
        .show(ui, |ui| {
            ui.checkbox(&mut arm_config.enabled, "Enabled");
            ui.add(egui::Slider::new(&mut arm_config.offset, 0..=360).text("Angular Offset"));
        });
}

fn component_ui(config: &mut ComponentConfig, has_noise: bool, ui: &mut egui::Ui) {
    let heading = match config.component_type {
        ComponentType::Disk => "Disk Config",
        ComponentType::Dust => "Dust Config",
        ComponentType::Stars => "Stars Config",
    };

    let minval = ComponentConfig::MIN;
    let maxval = ComponentConfig::MAX;

    let mut section = |ui: &mut egui::Ui| {
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
        if has_noise {
            ui.label("Noise");

            ui.group(|ui| {
                ui.checkbox(&mut config.noise_enabled, "Enabled");
                // speciual case for stars, ugly hack but w/e
                if config.component_type == ComponentType::Stars {
                    ui.add(
                        egui::Slider::new(&mut config.noise_scale, 1.0..=100.0).text("Frequency"),
                    );
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
        };
    };
    if has_noise {
        egui::CollapsingHeader::new(heading).show(ui, section);
    } else {
        (section(ui));
    }
    ui.separator();
}

fn ui_system(
    mut contexts: EguiContexts,
    mut galaxy_config: ResMut<GalaxyConfig>,
    mut rendering_config: ResMut<GalaxyRenderConfig>,
) {
    let ctx = contexts.ctx_mut();

    let mut new_galaxy_config = galaxy_config.clone();
    let mut new_rendering_config = rendering_config.clone();

    egui::SidePanel::left("side_panel")
        .default_width(200.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Configuration");

                egui::CollapsingHeader::new("Galaxy Parameters").show(ui, |ui| {
                    ui.add(
                        egui::Slider::new(&mut new_galaxy_config.radius, 100.0..=1000.0).text("Radius"),
                    );

                    let mut texture_root = new_rendering_config.texture_dimension.checked_ilog2().unwrap_or(1);
                    ui.add(
                        egui::Slider::new(&mut texture_root, 4..=11)
                            .custom_formatter(|n, _| {
                                let n = n as u32;
                                format!("{}", 2u32.pow(n))
                            })
                            .text("Texture Size"),
                    );
                    new_rendering_config.texture_dimension = 2u32.pow(texture_root);

                    ui.add(
                        egui::Slider::new(&mut new_rendering_config.padding_coeff, 1.0..=2.0)
                            .text("Padding Coefficient"),
                    );
                    ui.add(
                        egui::Slider::new(&mut new_rendering_config.raymarch_steps, 1..=256)
                            .text("Raymarch Steps"),
                    );
                    ui.checkbox(
                        &mut new_rendering_config.draw_volume_to_background,
                        "Draw volume to background layer",
                    );
                    let mut inv_exposure = 1.0 / new_rendering_config.exposure;
                    ui.add(
                        egui::Slider::new(&mut inv_exposure, 1.0..=1000.0)
                            .text("Exposure (Inverse)"),
                    );
                    new_rendering_config.exposure = 1.0 / inv_exposure;

                    ui.add(
                        egui::Slider::new(&mut new_galaxy_config.winding_b, 0.05..=1.0)
                            .text("windingB"),
                    );
                    ui.add(
                        egui::Slider::new(&mut new_galaxy_config.winding_n, 1.0..=10.0)
                            .text("windingN"),
                    );

                    ui.checkbox(
                        &mut new_rendering_config.diagnostic_mode,
                        "Performance Diagnostic",
                    );
                });

                ui.separator();
                egui::CollapsingHeader::new("Arms").show(ui, |ui| {
                    for i in 0..4 {
                        let arm_config = &mut new_galaxy_config.arm_configs[i as usize];
                        arm_component_ui(i, arm_config, ui);
                    }
                });
                ui.separator();

                egui::CollapsingHeader::new("Bulge Parameters").show(ui, |ui| {
                    ui.add(
                        egui::Slider::new(&mut new_galaxy_config.bulge_strength, 1.0..=200.0)
                            .text("Strength"),
                    );
                    ui.add(
                        egui::Slider::new(&mut new_galaxy_config.bulge_radius, 1.0..=20.0)
                            .text("Scale Factor"),
                    );
                });
                ui.separator();

                component_ui(&mut new_galaxy_config.disk_params, true, ui);
                component_ui(&mut new_galaxy_config.dust_params, true, ui);

                egui::CollapsingHeader::new("Stars Parameters").show(ui, |ui| {
                    ui.add(
                        egui::Slider::new(&mut new_galaxy_config.stars_per_arm, 4096..=65536)
                            .text("Stars per arm"),
                    );
                    ui.checkbox(
                        &mut new_rendering_config.draw_stars_to_background,
                        "Draw stars to background",
                    );
                    component_ui(&mut new_galaxy_config.stars_params, false, ui);
                });
            });
        });

    if new_galaxy_config != *galaxy_config {
        new_galaxy_config.update_arms();
        *galaxy_config = new_galaxy_config;
    }
    if new_rendering_config != *rendering_config {
        *rendering_config = new_rendering_config;
    }
}
