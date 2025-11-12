use std::sync::Arc;

use crate::hardware::{self, BreakpointVar, Word, MEM_SIZE};
use eframe::{
    egui,
    epaint::{mutex::Mutex, Vec2},
};
use egui_extras::{Column, Size, StripBuilder, TableBuilder};

use super::common_state::{
    Action, Breakpoint, BreakpointAction, CommonAction, SharedState, UIStyle,
};
use super::hardware_state::HardwareState;
use super::shared_ui::*;

impl HardwareState {
    pub fn draw(
        &self,
        ctx: &egui::Context,
        action: &mut Option<Action>,
        shared_state: &SharedState,
        screen: &Arc<Screen>,
        frame: &eframe::Frame,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let available_width = ui.available_width();
            let thin_layout = available_width < 768.0;
            StripBuilder::new(ui)
                .size(Size::remainder())
                .size(Size::exact(available_width.min(512.0)))
                .horizontal(|mut strip| {
                    if thin_layout {
                        strip.empty();
                    } else {
                        strip.strip(|builder| {
                            builder
                                .size(Size::initial(140.0).at_least(140.0))
                                .size(Size::exact(110.0))
                                .horizontal(|mut strip| {
                                    strip.strip(|builder| {
                                        builder
                                            .size(Size::remainder())
                                            .size(Size::exact(10.0))
                                            .size(Size::exact(20.0))
                                            .vertical(|mut strip| {
                                                strip.cell(|ui| {
                                                    ui.rom_grid(
                                                        "ROM",
                                                        &self.hardware.rom,
                                                        &(0..=((MEM_SIZE - 1) as Word)),
                                                        self.hardware.pc,
                                                        shared_state.scroll_once,
                                                    );
                                                });

                                                strip.empty();
                                                strip.cell(|ui| {
                                                    ui.horizontal(|ui| {
                                                        egui::Frame::none()
                                                            .stroke(egui::Stroke::new(
                                                                1.0,
                                                                ui.style().visuals.text_color(),
                                                            ))
                                                            .inner_margin(2.0)
                                                            .show(ui, |ui| {
                                                                ui.label("PC");
                                                                ui.allocate_ui_with_layout(
                                                                    ui.available_size(),
                                                                    egui::Layout::right_to_left(
                                                                        egui::Align::Min,
                                                                    ),
                                                                    |ui| {
                                                                        ui.label(
                                                                            self.hardware
                                                                                .pc
                                                                                .to_string(),
                                                                        );
                                                                    },
                                                                );
                                                            });
                                                    });
                                                });
                                            });
                                    });
                                    strip.strip(|builder| {
                                        builder
                                            .size(Size::remainder())
                                            .size(Size::exact(10.0))
                                            .size(Size::exact(20.0))
                                            .vertical(|mut strip| {
                                                strip.cell(|ui| {
                                                    ui.ram_grid(
                                                        "RAM",
                                                        &self.hardware.ram,
                                                        &(0..=((MEM_SIZE - 1) as Word)),
                                                        UIStyle::Hardware,
                                                        Some(self.hardware.a),
                                                        shared_state.scroll_once,
                                                    );
                                                });

                                                strip.empty();
                                                strip.cell(|ui| {
                                                    ui.horizontal(|ui| {
                                                        egui::Frame::none()
                                                            .stroke(egui::Stroke::new(
                                                                1.0,
                                                                ui.style().visuals.text_color(),
                                                            ))
                                                            .inner_margin(2.0)
                                                            .show(ui, |ui| {
                                                                ui.label("A");
                                                                ui.allocate_ui_with_layout(
                                                                    ui.available_size(),
                                                                    egui::Layout::right_to_left(
                                                                        egui::Align::Min,
                                                                    ),
                                                                    |ui| {
                                                                        ui.label(
                                                                            self.hardware
                                                                                .a
                                                                                .to_string(),
                                                                        );
                                                                    },
                                                                );
                                                            });
                                                    });
                                                });
                                            });
                                    });
                                });
                        });
                    }
                    strip.cell(|ui| {
                        ui.vertical(|ui| {
                            let screen_height = (available_width / 2.0).min(256.0);
                            let screen_width = available_width.min(512.0);
                            ui.allocate_ui(Vec2::new(screen_width, screen_height), |ui| {
                                draw_screen(ui, screen, &self.hardware.ram, frame);
                            });
                            ui.add_space(screen_height + 20.0);
                            ui.horizontal(|ui| {
                                ui.add_space(screen_width / 2.0 - 70.0);
                                egui::Frame::none()
                                    .stroke(egui::Stroke::new(1.0, ui.style().visuals.text_color()))
                                    .inner_margin(2.0)
                                    .show(ui, |ui| {
                                        ui.label("D");
                                        ui.allocate_ui_with_layout(
                                            [110.0, ui.available_height()].into(),
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(self.hardware.d.to_string());
                                            },
                                        );
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
                let breakpoints = self.hardware.get_breakpoints();
                ui.horizontal(|ui| {
                    let breakpoint_address =
                        if let BreakpointVar::RAM(address) = self.selected_breakpoint.var {
                            address
                        } else {
                            0
                        };

                    let mut new_selected_breakpoint_var = self.selected_breakpoint.var;
                    let selected_text = match self.selected_breakpoint.var {
                        BreakpointVar::RAM(_) => "Mem".to_string(),
                        _ => self.selected_breakpoint.var.to_string(),
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
                                BreakpointVar::RAM(breakpoint_address),
                                "Mem",
                            );
                        });

                    if let BreakpointVar::RAM(address) = self.selected_breakpoint.var {
                        ui.label("[");
                        let mut new_address_text = address.to_string();
                        ui.add(
                            egui::TextEdit::singleline(&mut new_address_text).desired_width(50.0),
                        );
                        if let Ok(new_address) = new_address_text.parse::<Word>() {
                            if new_address != address {
                                new_selected_breakpoint_var = BreakpointVar::RAM(new_address);
                            }
                        }
                        ui.label("]");
                    }

                    if new_selected_breakpoint_var != self.selected_breakpoint.var {
                        *action = Some(Action::Breakpoint(BreakpointAction::BreakpointChanged(
                            Breakpoint::Hardware(hardware::Breakpoint {
                                var: new_selected_breakpoint_var,
                                value: self.selected_breakpoint.value,
                            }),
                        )));
                    }
                    ui.label("=");

                    let mut new_value_text = self.selected_breakpoint.value.to_string();
                    ui.add(egui::TextEdit::singleline(&mut new_value_text).desired_width(50.0));
                    if let Ok(new_value) = new_value_text.parse::<Word>() {
                        if new_value != self.selected_breakpoint.value {
                            *action =
                                Some(Action::Breakpoint(BreakpointAction::BreakpointChanged(
                                    Breakpoint::Hardware(hardware::Breakpoint {
                                        var: self.selected_breakpoint.var,
                                        value: new_value,
                                    }),
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
}
