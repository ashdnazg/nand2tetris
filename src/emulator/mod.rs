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
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use common_reducer::reduce;
use common_reducer::steps_to_run;
use common_state::{Action, AppState, PerformanceData, StepRunnable};
use shared_ui::{draw_shared, Screen};
use vm_ui::draw_vm;

use crate::emulator::hardware_state::HardwareState;

use self::vm_state::VMState;

pub struct EmulatorApp {
    performance_data: PerformanceData,
    shared_state: SharedState,
    state: AppState,
    screen: Arc<Mutex<Screen>>,
    async_actions: (Sender<Action>, Receiver<Action>),
}

impl EmulatorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            performance_data: Default::default(),
            shared_state: Default::default(),
            state: Default::default(),
            screen: Arc::new(Mutex::new(Screen::new(cc.gl.as_ref().unwrap()))),
            async_actions: channel(),
        }
    }
}

impl eframe::App for EmulatorApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                reduce(self, &Action::FilesDropped(i.raw.dropped_files.clone()));
            }
        });

        while let Ok(action) = self.async_actions.1.try_recv() {
            reduce(self, &action);
        }

        let mut action = None;

        draw_shared(
            &self.shared_state,
            ctx,
            &self.performance_data,
            !matches!(self.state, AppState::Start),
            &mut action,
            &self.async_actions.0,
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
            AppState::Start => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.vertical(|ui| {
                        fn file_contents_from_dir(dir: &include_dir::Dir) -> Vec<(String, String)> {
                            dir.files()
                                .map(|f| {
                                    (
                                        f.path().file_name().unwrap().to_str().unwrap().to_owned(),
                                        f.contents_utf8().unwrap().to_owned(),
                                    )
                                })
                                .collect()
                        }
                        if ui.button("VM Example 1: Ray Tracer").clicked() {
                            let file_contents = file_contents_from_dir(&include_dir::include_dir!(
                                "$CARGO_MANIFEST_DIR/Raytracer"
                            ));
                            self.state = AppState::VM(VMState::from_file_contents(file_contents));
                            self.shared_state = Default::default();
                        }
                        if ui.button("VM Example 2: Hackenstein").clicked() {
                            let file_contents = file_contents_from_dir(&include_dir::include_dir!(
                                "$CARGO_MANIFEST_DIR/hackenstein3DVM"
                            ));
                            self.state = AppState::VM(VMState::from_file_contents(file_contents));
                            self.shared_state = Default::default();
                        }
                        if ui.button("VM Example 3: Dino").clicked() {
                            let file_contents = file_contents_from_dir(&include_dir::include_dir!(
                                "$CARGO_MANIFEST_DIR/Dino"
                            ));
                            self.state = AppState::VM(VMState::from_file_contents(file_contents));
                            self.shared_state = Default::default();
                        }
                        if ui.button("VM Example 4: 2048").clicked() {
                            let file_contents = file_contents_from_dir(&include_dir::include_dir!(
                                "$CARGO_MANIFEST_DIR/2048"
                            ));
                            self.state = AppState::VM(VMState::from_file_contents(file_contents));
                            self.shared_state = Default::default();
                        }
                        if ui.button("VM Example 5: Ray Marcher").clicked() {
                            let file_contents = file_contents_from_dir(&include_dir::include_dir!(
                                "$CARGO_MANIFEST_DIR/Raymarcher"
                            ));
                            self.state = AppState::VM(VMState::from_file_contents(file_contents));
                            self.shared_state = Default::default();
                        }
                        if ui.button("Hack Example: Ray Marcher").clicked() {
                            let file_contents = include_str!("../../r_soj.hack");
                            self.state = AppState::Hardware(
                                HardwareState::from_hack_file_contents(file_contents),
                            );
                            self.shared_state = Default::default();
                        }
                    });
                });
            }
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
