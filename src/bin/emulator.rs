#![forbid(unsafe_code)]
// #![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

use std::ops::Range;

use eframe::egui;
use eframe::emath::Rect;
use eframe::epaint::Color32;
use eframe::epaint::Vec2;

use egui_extras::{Size, StripBuilder};

use nand2tetris::hardware::*;
use nand2tetris::vm::*;

struct HardwareState {
    hardware: Hardware,
}

struct VMState {
    vm: VM,
}

enum AppState {
    Hardware(HardwareState),
    VM(VMState),
    Start,
}

impl Default for AppState {
    fn default() -> Self {
        let mut hardware = Hardware::default();
        let program: [u16; 16] = [
            15, 60040, 14, 64528, 15, 58114, 13, 64528, 15, 61576, 14, 64648, 2, 60039, 15, 60039,
        ];
        hardware.load_program(program.iter().map(|raw| Instruction::new(*raw)));

        AppState::Hardware(HardwareState { hardware: hardware })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Action {
    Quit,
}

pub struct EmulatorApp {
    state: AppState,
}

impl Default for EmulatorApp {
    fn default() -> Self {
        Self {
            state: Default::default(),
        }
    }
}

fn draw_shared(ctx: &egui::Context, action: &mut Option<Action>) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        // The top panel is often a good place for a menu bar:
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Quit").clicked() {
                    *action = Some(Action::Quit);
                }
            });
        });
    });
}

fn draw_hardware(state: &HardwareState, ctx: &egui::Context, action: &mut Option<Action>) {
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
                            strip.cell(|ui| {
                                ui.rom_grid("ROM", &state.hardware.rom, 0..i16::MAX, 1);
                            });
                            strip.cell(|ui| {
                                ui.ram_grid("RAM", &state.hardware.ram, 0..i16::MAX);
                            });
                        });
                });
            });
    });
}

fn draw_vm(state: &VMState, ctx: &egui::Context, action: &mut Option<Action>) {
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
                            strip.cell(|ui| {});
                            strip.strip(|builder| {
                                builder.sizes(Size::relative(1.0 / 6.0), 6).vertical(
                                    |mut strip| {
                                        strip.cell(|ui| {
                                            ui.ram_grid("Static", &state.vm.ram, 0..5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("Local", &state.vm.ram, 0..5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("Argument", &state.vm.ram, 0..5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("This", &state.vm.ram, 0..5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("That", &state.vm.ram, 0..5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("Temp", &state.vm.ram, 0..5);
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
                            strip.cell(|ui| {});
                            strip.strip(|builder| {
                                builder
                                    .size(Size::relative(0.5))
                                    .size(Size::remainder())
                                    .horizontal(|mut strip| {
                                        strip.cell(|ui| {
                                            ui.ram_grid("Global Stack", &state.vm.ram, 256..1024);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("RAM", &state.vm.ram, 0..i16::MAX);
                                        });
                                    });
                            });
                        });
                });
            });
    });
}

fn draw_start(ctx: &egui::Context, action: &mut Option<Action>) {}

fn reduce(state: &mut AppState, action: &Action) {
    match action {
        _ => {}
    }
}

trait EmulatorWidgets {
    fn ram_grid(&mut self, caption: &str, ram: &RAM, range: Range<i16>);
    fn rom_grid(&mut self, caption: &str, rom: &[Instruction; 32 * 1024], range: Range<i16>, highlight_address: i16);
}

impl EmulatorWidgets for egui::Ui {
    fn ram_grid(&mut self, caption: &str, ram: &RAM, range: Range<i16>) {
        self.vertical(|ui| {
            ui.label(caption);
            let text_style = egui::TextStyle::Body;
            let row_height = ui.text_style_height(&text_style);
            egui::ScrollArea::vertical().auto_shrink([false; 2]).id_source(&caption).show_rows(
                ui,
                row_height,
                range.len() + 1,
                |ui, row_range| {
                    egui::Grid::new(&caption).num_columns(2).striped(true).show(ui, |ui| {
                        for i in row_range {
                            let address = i as i16 + range.start;
                            ui.label(address.to_string());
                            ui.label(ram[address].to_string());
                            ui.end_row();
                        }
                    });
                },
            );
        });
    }

    fn rom_grid(&mut self, caption: &str, rom: &[Instruction; 32 * 1024], range: Range<i16>, highlight_address: i16) {
        self.vertical(|ui| {
            ui.label(caption);
            let text_style = egui::TextStyle::Body;
            let row_height = ui.text_style_height(&text_style);
            let width = ui.available_width();
            egui::ScrollArea::vertical().auto_shrink([false; 2]).id_source(&caption).show_rows(
                ui,
                row_height,
                range.len() + 1,
                |ui, row_range| {
                    egui::Grid::new(&caption).num_columns(2).striped(true).show(ui, |ui| {
                        for i in row_range {
                            let address = i as i16 + range.start;
                            if highlight_address == address {
                                let size = Vec2::new(width, row_height);
                                let rect = Rect::from_min_size(ui.cursor().min, size);
                                // let rect = rect.expand2(0.5 * ui.spacing(). * Vec2::Y);
                                let rect = rect.expand2(2.0 * Vec2::X); // HACK: just looks better with some spacing on the sides

                                ui.painter().rect_filled(rect, 2.0, Color32::YELLOW);
                            }
                            ui.label(address.to_string());
                            ui.label(rom[address as usize].to_string());
                            ui.end_row();
                        }
                    });
                },
            );
        });
    }
}

impl eframe::App for EmulatorApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());

        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        let mut action = None;
        draw_shared(ctx, &mut action);

        match &self.state {
            AppState::Hardware(state) => draw_hardware(state, ctx, &mut action),
            AppState::VM(state) => draw_vm(state, ctx, &mut action),
            AppState::Start => draw_start(ctx, &mut action),
        };

        if action == Some(Action::Quit) {
            frame.quit();
            return;
        }

        if let Some(action) = action {
            reduce(&mut self.state, &action);
        }
    }
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let mut native_options = eframe::NativeOptions::default();
    native_options.initial_window_size = Some(Vec2::new(1600.0, 1200.0));
    eframe::run_native(
        "Emulator",
        native_options,
        Box::new(|cc| {
            cc.egui_ctx.set_pixels_per_point(2.0);
            Box::new(EmulatorApp::default())
        }),
    );
}
