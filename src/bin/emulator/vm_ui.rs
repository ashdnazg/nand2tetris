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
                            strip.cell(|ui| {
                                let mut selected_file = state.selected_file.clone();
                                ui.vm_grid(
                                    &state.vm.program,
                                    &state.vm.run_state,
                                    &mut selected_file,
                                );
                                if selected_file != state.selected_file {
                                    *action = Some(Action::VMFileSelected(selected_file));
                                }
                            });
                            strip.strip(|builder| {
                                builder.sizes(Size::relative(1.0 / 6.0), 6).vertical(
                                    |mut strip| {
                                        let function_name = &state
                                            .vm
                                            .run_state
                                            .call_stack
                                            .last()
                                            .unwrap()
                                            .function_name;
                                        let function_metadata = &state.vm.program.files
                                            [&state.vm.run_state.current_file_name]
                                            .function_metadata[function_name];
                                        strip.cell(|ui| {
                                            let static_segment =
                                                &state.vm.program.file_name_to_static_segment
                                                    [&state.vm.run_state.current_file_name];
                                            ui.ram_grid(
                                                "Static",
                                                &state.vm.run_state.ram,
                                                static_segment,
                                                UIStyle::VM,
                                            );
                                        });
                                        strip.cell(|ui| {
                                            let local_address =
                                                &state.vm.run_state.ram[Register::LCL];
                                            ui.ram_grid(
                                                "Local",
                                                &state.vm.run_state.ram,
                                                &(*local_address
                                                    ..=*local_address
                                                        + function_metadata.local_var_count
                                                        - 1),
                                                UIStyle::VM,
                                            );
                                        });
                                        strip.cell(|ui| {
                                            let argument_address =
                                                &state.vm.run_state.ram[Register::ARG];
                                            ui.ram_grid(
                                                "Argument",
                                                &state.vm.run_state.ram,
                                                &(*argument_address
                                                    ..=*argument_address
                                                        + function_metadata.argument_count
                                                        - 1),
                                                UIStyle::VM,
                                            );
                                        });
                                        strip.cell(|ui| {
                                            let this_address =
                                                &state.vm.run_state.ram[Register::THIS];
                                            ui.ram_grid(
                                                "This",
                                                &state.vm.run_state.ram,
                                                &(*this_address..=*this_address + 4),
                                                UIStyle::VM,
                                            );
                                        });
                                        strip.cell(|ui| {
                                            let this_address =
                                                &state.vm.run_state.ram[Register::THAT];
                                            ui.ram_grid(
                                                "That",
                                                &state.vm.run_state.ram,
                                                &(*this_address..=*this_address + 4),
                                                UIStyle::VM,
                                            );
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid(
                                                "Temp",
                                                &state.vm.run_state.ram,
                                                &(5..=12),
                                                UIStyle::VM,
                                            );
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
                                                &(256..=1024),
                                                UIStyle::VM,
                                            );
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid(
                                                "RAM",
                                                &state.vm.run_state.ram,
                                                &(0..=i16::MAX),
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
