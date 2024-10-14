use std::sync::Arc;

use crate::hardware::{Word, MEM_SIZE};
use crate::vm::{self, Register};
use eframe::egui;
use eframe::epaint::mutex::Mutex;
use egui_extras::{Size, StripBuilder};

use super::common_state::{Breakpoint, BreakpointAction, SharedState, UIStyle};
use super::shared_ui::{draw_screen, EmulatorWidgets, Screen};
use super::vm_state::VMState;
use super::Action;

pub fn draw_vm(
    state: &VMState,
    ctx: &egui::Context,
    action: &mut Option<Action>,
    shared_state: &SharedState,
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
                .size(Size::exact(110.0))
                .size(Size::exact(512.0))
        }
        builder.horizontal(|mut strip| {
            if thin_layout {
                strip.empty();
            } else {
                strip.cell(|ui| {
                    let mut selected_file = state.selected_file.clone();
                    ui.vm_grid(
                        &state.vm.program,
                        &state.vm.run_state,
                        &mut selected_file,
                        shared_state.scroll_once,
                    );
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
                                None,
                                shared_state.scroll_once,
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
                                None,
                                shared_state.scroll_once,
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
                                None,
                                shared_state.scroll_once,
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
                                None,
                                shared_state.scroll_once,
                            );
                        });

                    egui::TopBottomPanel::bottom("temp panel")
                        .min_height(0.0)
                        .default_height(height)
                        .resizable(true)
                        .show_inside(ui, |ui| {
                            ui.ram_grid(
                                "Temp",
                                &state.vm.run_state.ram,
                                &(5..=12),
                                UIStyle::VM,
                                None,
                                shared_state.scroll_once,
                            );
                        });

                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        let that_address = &state.vm.run_state.ram[Register::THAT];
                        ui.ram_grid(
                            "That",
                            &state.vm.run_state.ram,
                            &(*that_address..=*that_address + 128),
                            UIStyle::VM,
                            None,
                            shared_state.scroll_once,
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
                                            Some(state.vm.run_state.ram[Register::SP]),
                                            shared_state.scroll_once,
                                        );
                                    });
                                    strip.cell(|ui| {
                                        ui.ram_grid(
                                            "RAM",
                                            &state.vm.run_state.ram,
                                            &(0..=((MEM_SIZE - 1) as Word)),
                                            UIStyle::VM,
                                            None,
                                            shared_state.scroll_once,
                                        );
                                    });
                                });
                        });
                    });
            });
        });
    });

    let mut breakpoints_open = shared_state.breakpoints_open;

    egui::Window::new("Breakpoints")
        .open(&mut breakpoints_open)
        .resizable(true)
        .default_width(1000.0)
        .show(ctx, |ui| {
            let breakpoints = state.vm.get_breakpoints();
            ui.horizontal(|ui| {
                let mut new_selected_breakpoint = state.selected_breakpoint;
                let selected_text = state.selected_breakpoint.variable_name();
                egui::ComboBox::from_id_source("Variable")
                    .selected_text(selected_text)
                    .width(50.0)
                    .show_ui(ui, |ui| {
                        for breakpoint_type in [
                            vm::Breakpoint::SP(0),
                            vm::Breakpoint::CurrentFunction("".to_owned()),
                            vm::Breakpoint::Line {
                                file_name: "".to_owned(),
                                line_number: 0,
                            },
                            vm::Breakpoint::RAM {
                                address: 0,
                                value: 0,
                            },
                            vm::Breakpoint::LCL(0),
                            vm::Breakpoint::Local {
                                offset: 0,
                                value: 0,
                            },
                            vm::Breakpoint::ARG(0),
                            vm::Breakpoint::Argument {
                                offset: 0,
                                value: 0,
                            },
                            vm::Breakpoint::This(0),
                            vm::Breakpoint::ThisPointer {
                                offset: 0,
                                value: 0,
                            },
                            vm::Breakpoint::That(0),
                            vm::Breakpoint::ThatPointer {
                                offset: 0,
                                value: 0,
                            },
                            vm::Breakpoint::Temp {
                                offset: 0,
                                value: 0,
                            },
                        ] {
                            ui.selectable_value(
                                &mut new_selected_breakpoint,
                                breakpoint_type,
                                breakpoint_type.variable_name(),
                            );
                        }
                    });

                if let Some(address) = state.selected_breakpoint.address() {
                    ui.label("[");
                    let mut new_address_text = address.to_string();
                    ui.add(
                        egui::TextEdit::singleline(&mut new_address_text).desired_width(50.0),
                    );
                    if let Ok(new_address) = new_address_text.parse::<Word>() {
                        new_selected_breakpoint.change_address(new_address);
                    }
                    ui.label("]");
                }

                ui.label("=");

                let mut new_value_text = state.selected_breakpoint.value().to_string();
                ui.add(egui::TextEdit::singleline(&mut new_value_text).desired_width(50.0));
                if let Ok(new_value) = new_value_text.parse::<Word>() {
                    if new_value != state.selected_breakpoint.value() {
                        new_selected_breakpoint.change_value(new_value);
                    }
                }

                if new_selected_breakpoint != state.selected_breakpoint {
                    *action = Some(Action::Breakpoint(BreakpointAction::BreakpointChanged(Breakpoint::VM(new_selected_breakpoint)
                    )));
                }

                if ui.button("Add").clicked() {
                    *action = Some(Action::Breakpoint(BreakpointAction::AddClicked));
                }
            });
            ui.label("Breakpoints:");
            let header_height = ui.text_style_height(&egui::TextStyle::Body);
            let row_height = ui.text_style_height(&egui::TextStyle::Monospace)
                + 2.0 * ui.spacing().button_padding.x;
            TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(100.0))
                .column(Column::exact(100.0))
                .column(Column::exact(70.0))
                .header(header_height, |mut header| {
                    header.col(|ui| {
                        ui.label("Variable");
                    });
                    header.col(|ui| {
                        ui.label("Value");
                    });
                    header.col(|_| {});
                })
                .body(|body| {
                    body.rows(row_height, usize::max(breakpoints.len(), 10), |mut row| {
                        let row_index = row.index();
                        let breakpoint = breakpoints.get(row_index);
                        row.col(|ui| {
                            ui.monospace(
                                breakpoint
                                    .map(|b| b.var.to_string())
                                    .unwrap_or("".to_string()),
                            );
                        });
                        row.col(|ui| {
                            ui.monospace(
                                breakpoint
                                    .map(|b| b.value.to_string())
                                    .unwrap_or("".to_string()),
                            );
                        });
                        row.col(|ui| {
                            if breakpoint.is_some() && ui.button("Remove").clicked() {
                                *action = Some(Action::Breakpoint(
                                    BreakpointAction::RemoveClicked(row_index),
                                ));
                            }
                        });
                    });
                });
        });

    if shared_state.breakpoints_open != breakpoints_open {
        assert!(shared_state.breakpoints_open);
        *action = Some(Action::Common(CommonAction::BreakpointsClosed));
    }
}
