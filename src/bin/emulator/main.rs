// #![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

mod hardware_ui;
mod shared_ui;

use eframe::egui;
use eframe::epaint::Vec2;

use egui_extras::{Size, StripBuilder};

use nand2tetris::hardware::*;
use nand2tetris::vm::*;

use egui::mutex::Mutex;
use hardware_ui::*;
use shared_ui::*;
use std::sync::Arc;

struct VMState {
    shared_state: SharedState,
    vm: VM,
}

enum AppState {
    Hardware(HardwareState),
    VM(VMState),
    Start,
}

impl CommonState for VMState {
    fn step(&mut self) -> bool {
        self.vm.step();
        false
    }

    fn shared_state(&self) -> &SharedState {
        &self.shared_state
    }

    fn shared_state_mut(&mut self) -> &mut SharedState {
        &mut self.shared_state
    }

    fn ram(&self) -> &RAM {
        &self.vm.ram
    }

    fn ram_mut(&mut self) -> &mut RAM {
        &mut self.vm.ram
    }

    fn reset(&mut self) {
        self.vm.reset();
    }
}

impl Default for AppState {
    fn default() -> Self {
        AppState::Hardware(Default::default())
    }
}

pub struct EmulatorApp {
    performance_data: PerformanceData,
    state: AppState,
    screen: Arc<Mutex<Screen>>,
}

impl EmulatorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            performance_data: Default::default(),
            state: Default::default(),
            screen: Arc::new(Mutex::new(Screen::new(&cc.gl.as_ref().unwrap()))),
        }
    }
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

fn draw_start(ctx: &egui::Context, action: &mut Option<Action>) {}

fn reduce(state: &mut AppState, action: &Action) {
    match action {
        Action::Common(common_action) => match state {
            AppState::Hardware(hardware_state) => reduce_common(hardware_state, common_action),
            AppState::VM(vm_state) => reduce_common(vm_state, common_action),
            AppState::Start => panic!(
                "Received common action {:?} when in state AppState::Start",
                common_action
            ),
        },
        Action::Breakpoint(breakpoint_action) => match state {
            AppState::Hardware(hardware_state) => {
                reduce_breakpoint_hardware(hardware_state, breakpoint_action)
            }
            AppState::VM(vm_state) => todo!(),
            AppState::Start => todo!(),
        },
        Action::Quit => todo!(),
    }
}

impl eframe::App for EmulatorApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let mut action = None;
        match &self.state {
            AppState::Hardware(state) => {
                state
                    .shared_state()
                    .draw(ctx, &self.performance_data, &mut action)
            }
            AppState::VM(state) => {
                state
                    .shared_state()
                    .draw(ctx, &self.performance_data, &mut action)
            }
            AppState::Start => {}
        };

        let last_frame_time = frame.info().cpu_usage.unwrap_or(1.0 / 60.0);
        let steps_to_run = match &self.state {
            AppState::Hardware(state) => steps_to_run(
                state.shared_state().desired_steps_per_second,
                last_frame_time,
                &mut self.performance_data,
                state,
                &action,
            ),
            AppState::VM(state) => steps_to_run(
                state.shared_state().desired_steps_per_second,
                last_frame_time,
                &mut self.performance_data,
                state,
                &action,
            ),
            _ => 0,
        };

        let key_down = ctx.input().keys_down.iter().cloned().next();

        match &mut self.state {
            AppState::Hardware(state) => {
                state.run_steps(steps_to_run, key_down);
            }
            AppState::VM(state) => {
                state.run_steps(steps_to_run, key_down);
            }
            _ => {}
        }

        ctx.request_repaint();

        match &self.state {
            AppState::Hardware(state) => {
                state.draw(ctx, &mut action, &self.screen, &frame);
            }
            AppState::VM(state) => draw_vm(state, ctx, &mut action),
            AppState::Start => draw_start(ctx, &mut action),
        };

        if action == Some(Action::Quit) {
            frame.close();
            return;
        }

        if let Some(action) = action {
            reduce(&mut self.state, &action);
        }
    }

    fn on_exit(&mut self, gl: Option<&glow::Context>) {
        if let Some(context) = gl {
            self.screen.lock().destroy(context);
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
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Box::new(EmulatorApp::new(&cc))
        }),
    );
}
