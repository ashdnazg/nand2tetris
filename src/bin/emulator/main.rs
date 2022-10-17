// #![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

mod common_reducer;
mod hardware_reducer;
mod hardware_ui;
mod shared_ui;
mod hardware_state;
mod vm_state;
mod common_state;
mod vm_ui;

use eframe::egui;
use eframe::epaint::Vec2;

use egui::mutex::Mutex;
use std::sync::Arc;

use crate::common_state::{Action, PerformanceData, CommonState};
use crate::common_reducer::{reduce_common, steps_to_run};
use crate::hardware_reducer::reduce_breakpoint_hardware;
use crate::hardware_state::HardwareState;
use crate::vm_state::VMState;
use crate::vm_ui::draw_vm;
use crate::shared_ui::{Screen, StepRunnable, draw_shared};

enum AppState {
    Hardware(HardwareState),
    VM(VMState),
    Start,
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
                draw_shared(&state.shared_state(), ctx, &self.performance_data, &mut action)
            }
            AppState::VM(state) => {
                draw_shared(&state.shared_state(), ctx, &self.performance_data, &mut action)
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
