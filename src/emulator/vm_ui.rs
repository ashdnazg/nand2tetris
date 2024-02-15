use std::sync::Arc;

use crate::vm::Register;
use eframe::egui;
use eframe::epaint::mutex::Mutex;
use egui_extras::{Size, StripBuilder};

use super::common_state::UIStyle;
use super::shared_ui::{draw_screen, EmulatorWidgets, Screen};
use super::vm_state::VMState;
use super::Action;

pub fn draw_vm(
    state: &VMState,
    ctx: &egui::Context,
    action: &mut Option<Action>,
    screen: &Arc<Mutex<Screen>>,
    frame: &eframe::Frame,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let available_width = ui.available_width();
        let thin_layout = available_width < 1024.0;
        let mut builder =
            StripBuilder::new(ui).cell_layout(egui::Layout::right_to_left(egui::Align::Center));
        if thin_layout {
            builder = builder
                .size(Size::remainder())
                .size(Size::exact(available_width.min(512.0)))
        } else {
            builder = builder
                .size(Size::remainder())
                .size(Size::remainder())
                .size(Size::exact(512.0))
        }
        builder.horizontal(|mut strip| {
            if thin_layout {
                strip.empty()
            } else {
                strip.cell(|ui| {
                    let mut selected_file = state.selected_file.clone();
                    ui.vm_grid(&state.vm.program, &state.vm.run_state, &mut selected_file);
                    if selected_file != state.selected_file {
                        *action = Some(Action::VMFileSelected(selected_file));
                    }
                });
                strip.cell(|ui| {
                    let function_index =
                        &state.vm.run_state.call_stack.last().unwrap().function_index;
                    let function_metadata = &state.vm.program.function_metadata[*function_index];
                    let height = ui.available_height() / 6.0;
                    egui::TopBottomPanel::top("static panel")
                        .min_height(0.0)
                        .default_height(height)
                        .resizable(true)
                        .show_inside(ui, |ui| {
                            let static_segment = &state.vm.program.files
                                [state.vm.run_state.current_file_index]
                                .static_segment;
                            ui.ram_grid(
                                "Static",
                                &state.vm.run_state.ram,
                                static_segment,
                                UIStyle::VM,
                            );
                        });

                    egui::TopBottomPanel::top("local panel")
                        .default_height(height)
                        .resizable(true)
                        .show_inside(ui, |ui| {
                            let local_address = &state.vm.run_state.ram[Register::LCL];
                            ui.ram_grid(
                                "Local",
                                &state.vm.run_state.ram,
                                &(*local_address
                                    ..=*local_address + function_metadata.local_var_count - 1),
                                UIStyle::VM,
                            );
                        });

                    egui::TopBottomPanel::top("argument panel")
                        .default_height(height)
                        .resizable(true)
                        .show_inside(ui, |ui| {
                            let argument_address = &state.vm.run_state.ram[Register::ARG];
                            ui.ram_grid(
                                "Argument",
                                &state.vm.run_state.ram,
                                &(*argument_address
                                    ..=*argument_address + function_metadata.argument_count - 1),
                                UIStyle::VM,
                            );
                        });

                    egui::TopBottomPanel::top("this panel")
                        .default_height(height)
                        .resizable(true)
                        .show_inside(ui, |ui| {
                            let this_address = &state.vm.run_state.ram[Register::THIS];
                            ui.ram_grid(
                                "This",
                                &state.vm.run_state.ram,
                                &(*this_address..=*this_address + 128),
                                UIStyle::VM,
                            );
                        });

                    egui::TopBottomPanel::bottom("temp panel")
                        .min_height(0.0)
                        .default_height(height)
                        .resizable(true)
                        .show_inside(ui, |ui| {
                            ui.ram_grid("Temp", &state.vm.run_state.ram, &(5..=12), UIStyle::VM);
                        });

                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        let that_address = &state.vm.run_state.ram[Register::THAT];
                        ui.ram_grid(
                            "That",
                            &state.vm.run_state.ram,
                            &(*that_address..=*that_address + 128),
                            UIStyle::VM,
                        );
                    });
                });
            }

            strip.strip(|builder| {
                builder
                    .cell_layout(egui::Layout::top_down(egui::Align::Center))
                    .size(Size::exact((available_width / 2.0).min(256.0)))
                    .size(Size::remainder())
                    .vertical(|mut strip| {
                        strip.cell(|ui| {
                            draw_screen(ui, screen, &state.vm.run_state.ram, frame);
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
