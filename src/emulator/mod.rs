mod common_reducer;
mod common_state;
mod hardware_reducer;
mod hardware_state;
mod hardware_ui;
mod instant;
mod shared_ui;
mod vm_reducer;
mod vm_state;
mod vm_ui;

use common_state::SharedState;
use eframe::egui;

use egui::mutex::Mutex;
use std::sync::Arc;

use common_reducer::reduce;
use common_reducer::steps_to_run;
use common_state::{Action, AppState, PerformanceData, StepRunnable};
use shared_ui::{draw_shared, Screen};
use vm_ui::draw_vm;

pub struct EmulatorApp {
    performance_data: PerformanceData,
    shared_state: SharedState,
    state: AppState,
    screen: Arc<Mutex<Screen>>,
}

impl EmulatorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            performance_data: Default::default(),
            shared_state: Default::default(),
            state: Default::default(),
            screen: Arc::new(Mutex::new(Screen::new(cc.gl.as_ref().unwrap()))),
        }
    }
}

impl eframe::App for EmulatorApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let mut action = None;
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                reduce(self, &Action::FilesDropped(i.raw.dropped_files.clone()));
            }
        });

        draw_shared(
            &self.shared_state,
            ctx,
            &self.performance_data,
            !matches!(self.state, AppState::Start),
            &mut action,
        );

        let last_frame_time = frame.info().cpu_usage.unwrap_or(1.0 / 60.0);
        let steps_to_run = steps_to_run(
            self.shared_state.desired_steps_per_second,
            last_frame_time,
            &mut self.performance_data,
            self.shared_state.run_started,
            &action,
        );

        let key_down = if ctx.memory(|m| m.focus().is_none()) {
            ctx.input(|i| i.keys_down.iter().cloned().next())
        } else {
            None
        };

        match &mut self.state {
            AppState::Hardware(state) => {
                self.shared_state.run_started &=
                    state.run_steps(steps_to_run, key_down, ctx.input(|i| i.modifiers));
            }
            AppState::VM(state) => {
                self.shared_state.run_started &=
                    state.run_steps(steps_to_run, key_down, ctx.input(|i| i.modifiers));
            }
            _ => {}
        }

        ctx.request_repaint();

        match &self.state {
            AppState::Hardware(state) => {
                state.draw(ctx, &mut action, &self.screen, frame);
            }
            AppState::VM(state) => draw_vm(state, ctx, &mut action, &self.screen, frame),
            AppState::Start => {}
        };

        if matches!(action, Some(Action::Quit)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        if let Some(action) = action {
            reduce(self, &action);
        }
    }

    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        if let Some(context) = gl {
            self.screen.lock().destroy(context);
        }
    }
}
