use std::sync::Arc;

use eframe::{
    egui,
    epaint::{mutex::Mutex, Vec2},
};
use egui_extras::{Size, StripBuilder, TableBuilder};
use nand2tetris::hardware::BreakpointVar;

use crate::common_state::{Action, CommonAction, UIStyle};
use crate::hardware_state::{BreakpointAction, HardwareState};
use crate::shared_ui::*;

impl HardwareState {
    pub fn draw(
        &self,
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
                            .size(Size::initial(140.0).at_least(140.0))
                            .size(Size::initial(110.0).at_least(110.0))
                            .horizontal(|mut strip| {
                                strip.cell(|ui| {
                                    ui.rom_grid(
                                        "ROM",
                                        &self.hardware.rom,
                                        0..=i16::MAX,
                                        self.hardware.pc,
                                    );
                                });
                                strip.cell(|ui| {
                                    ui.ram_grid(
                                        "RAM",
                                        &self.hardware.ram,
                                        0..=i16::MAX,
                                        UIStyle::Hardware,
                                    );
                                });
                            });
                    });
                    strip.cell(|ui| {
                        ui.allocate_ui(Vec2::new(512.0, 256.0), |ui| {
                            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                                draw_screen(ui, screen, &self.hardware.ram, frame);
                            });
                        });
                    });
                });
        });

        let mut breakpoints_open = self.shared_state.breakpoints_open;

        egui::Window::new("Breakpoints")
            .open(&mut breakpoints_open)
            .resizable(true)
            .default_width(1000.0)
            .show(ctx, |ui| {
                let breakpoints = self.hardware.get_breakpoints();
                ui.horizontal(|ui| {
                    let breakpoint_address =
                        if let BreakpointVar::Mem(address) = self.selected_breakpoint_var {
                            address
                        } else {
                            0
                        };

                    let mut new_selected_breakpoint_var = self.selected_breakpoint_var;
                    let selected_text = match self.selected_breakpoint_var {
                        BreakpointVar::Mem(_) => "Mem".to_string(),
                        _ => self.selected_breakpoint_var.to_string(),
                    };
                    egui::ComboBox::from_id_source("Variable")
                        .selected_text(selected_text)
                        .width(50.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut new_selected_breakpoint_var,
                                BreakpointVar::A,
                                "A",
                            );
                            ui.selectable_value(
                                &mut new_selected_breakpoint_var,
                                BreakpointVar::D,
                                "D",
                            );
                            ui.selectable_value(
                                &mut new_selected_breakpoint_var,
                                BreakpointVar::M,
                                "M",
                            );
                            ui.selectable_value(
                                &mut new_selected_breakpoint_var,
                                BreakpointVar::PC,
                                "PC",
                            );
                            ui.selectable_value(
                                &mut new_selected_breakpoint_var,
                                BreakpointVar::Mem(breakpoint_address),
                                "Mem",
                            );
                        });

                    if let BreakpointVar::Mem(address) = self.selected_breakpoint_var {
                        ui.label("[");
                        let mut new_address_text = address.to_string();
                        ui.add(
                            egui::TextEdit::singleline(&mut new_address_text).desired_width(50.0),
                        );
                        if let Ok(new_address) = new_address_text.parse::<i16>() {
                            if new_address != address {
                                new_selected_breakpoint_var = BreakpointVar::Mem(new_address);
                            }
                        }
                        ui.label("]");
                    }

                    if new_selected_breakpoint_var != self.selected_breakpoint_var {
                        *action = Some(Action::Breakpoint(BreakpointAction::VariableChanged(
                            new_selected_breakpoint_var,
                        )));
                    }
                    ui.label("=");

                    let mut new_value_text = self.breakpoint_value.to_string();
                    ui.add(egui::TextEdit::singleline(&mut new_value_text).desired_width(50.0));
                    if let Ok(new_value) = new_value_text.parse::<i16>() {
                        if new_value != self.breakpoint_value {
                            *action = Some(Action::Breakpoint(BreakpointAction::ValueChanged(
                                new_value,
                            )));
                        }
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
                    .column(Size::exact(100.0))
                    .column(Size::exact(100.0))
                    .column(Size::exact(70.0))
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
                        body.rows(
                            row_height,
                            usize::max(breakpoints.len(), 10),
                            |row_index, mut row| {
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
                                    if breakpoint.is_some() {
                                        if ui.button("Remove").clicked() {
                                            *action = Some(Action::Breakpoint(
                                                BreakpointAction::RemoveClicked(row_index),
                                            ));
                                        }
                                    }
                                });
                            },
                        );
                    });
            });

        if self.shared_state.breakpoints_open != breakpoints_open {
            assert!(self.shared_state.breakpoints_open);
            *action = Some(Action::Common(CommonAction::BreakpointsClosed));
        }
    }
}
