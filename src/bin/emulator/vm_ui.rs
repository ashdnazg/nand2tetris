use eframe::egui;
use egui_extras::{StripBuilder, Size};

use crate::shared_ui::EmulatorWidgets;
use crate::Action;
use crate::vm_state::VMState;

pub fn draw_vm(state: &VMState, ctx: &egui::Context, action: &mut Option<Action>) {
    egui::CentralPanel::default().show(ctx, |ui| {
        StripBuilder::new(ui)
            .size(Size::relative(0.5))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.strip(|builder| {
                    builder
                        .size(Size::relative(0.5))
                        .size(Size::remainder())
                        .horizontal(|mut strip| {
                            strip.cell(|_| {});
                            strip.strip(|builder| {
                                builder.sizes(Size::relative(1.0 / 6.0), 6).vertical(
                                    |mut strip| {
                                        strip.cell(|ui| {
                                            ui.ram_grid("Static", &state.vm.ram, 0..=5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("Local", &state.vm.ram, 0..=5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("Argument", &state.vm.ram, 0..=5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("This", &state.vm.ram, 0..=5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("That", &state.vm.ram, 0..=5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("Temp", &state.vm.ram, 0..=5);
                                        });
                                    },
                                );
                            });
                        });
                });

                strip.strip(|builder| {
                    builder
                        .size(Size::relative(0.5))
                        .size(Size::remainder())
                        .vertical(|mut strip| {
                            strip.cell(|_| {});
                            strip.strip(|builder| {
                                builder
                                    .size(Size::relative(0.5))
                                    .size(Size::remainder())
                                    .horizontal(|mut strip| {
                                        strip.cell(|ui| {
                                            ui.ram_grid("Global Stack", &state.vm.ram, 256..=1024);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("RAM", &state.vm.ram, 0..=i16::MAX);
                                        });
                                    });
                            });
                        });
                });
            });
    });
}
