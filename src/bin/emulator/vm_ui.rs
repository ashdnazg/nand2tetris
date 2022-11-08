use std::sync::Arc;

use eframe::egui;
use eframe::epaint::mutex::Mutex;
use eframe::epaint::Vec2;
use egui_extras::{Size, StripBuilder};
use nand2tetris::vm::Register;

use crate::common_state::UIStyle;
use crate::shared_ui::{draw_screen, EmulatorWidgets, Screen};
use crate::vm_state::VMState;
use crate::Action;

pub fn draw_vm(
    state: &VMState,
    ctx: &egui::Context,
    action: &mut Option<Action>,
    screen: &Arc<Mutex<Screen>>,
    frame: &eframe::Frame,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        StripBuilder::new(ui)
            .size(Size::remainder())
            .size(Size::exact(512.0))
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
                                            ui.ram_grid(
                                                "Static",
                                                &state.vm.run_state.ram,
                                                0..=5,
                                                UIStyle::VM,
                                            );
                                        });
                                        strip.cell(|ui| {
                                            let this_address = &state.vm.run_state.ram[Register::LCL];
                                            ui.ram_grid(
                                                "Local",
                                                &state.vm.run_state.ram,
                                                *this_address..=*this_address + 4,
                                                UIStyle::VM,
                                            );
                                        });
                                        strip.cell(|ui| {
                                            let this_address = &state.vm.run_state.ram[Register::ARG];
                                            ui.ram_grid(
                                                "Argument",
                                                &state.vm.run_state.ram,
                                                *this_address..=*this_address + 4,
                                                UIStyle::VM,
                                            );
                                        });
                                        strip.cell(|ui| {
                                            let this_address = &state.vm.run_state.ram[Register::THIS];
                                            ui.ram_grid(
                                                "This",
                                                &state.vm.run_state.ram,
                                                *this_address..=*this_address + 4,
                                                UIStyle::VM,
                                            );
                                        });
                                        strip.cell(|ui| {
                                            let this_address = &state.vm.run_state.ram[Register::THAT];
                                            ui.ram_grid(
                                                "That",
                                                &state.vm.run_state.ram,
                                                *this_address..=*this_address + 4,
                                                UIStyle::VM,
                                            );
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("Temp", &state.vm.run_state.ram, 5..=10, UIStyle::VM);
                                        });
                                    },
                                );
                            });
                        });
                });

                strip.strip(|builder| {
                    builder
                        .size(Size::exact(256.0))
                        .size(Size::remainder())
                        .vertical(|mut strip| {
                            strip.cell(|ui| {
                                ui.allocate_ui(Vec2::new(512.0, 256.0), |ui| {
                                    egui::Frame::canvas(ui.style()).show(ui, |ui| {
                                        draw_screen(ui, screen, &state.vm.run_state.ram, frame);
                                    });
                                });
                            });
                            strip.strip(|builder| {
                                builder
                                    .size(Size::relative(0.5))
                                    .size(Size::remainder())
                                    .horizontal(|mut strip| {
                                        strip.cell(|ui| {
                                            ui.ram_grid(
                                                "Global Stack",
                                                &state.vm.run_state.ram,
                                                256..=1024,
                                                UIStyle::VM,
                                            );
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid(
                                                "RAM",
                                                &state.vm.run_state.ram,
                                                0..=i16::MAX,
                                                UIStyle::VM,
                                            );
                                        });
                                    });
                            });
                        });
                });
            });
    });
}
