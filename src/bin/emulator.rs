#![forbid(unsafe_code)]
// #![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

use std::ops::Range;

use eframe::egui::Layout;
use eframe::emath::Align;
use eframe::epaint::Vec2;
use eframe::epi::App;
use eframe::{egui, epi};

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
        AppState::Hardware(HardwareState {
            hardware: Default::default(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Action {
    Quit,
}

pub struct EmulatorApp {
    // Example stuff:
    label: String,

    // this how you opt-out of serialization of a member
    #[cfg_attr(feature = "persistence", serde(skip))]
    value: f32,

    state: AppState,
}

impl Default for EmulatorApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
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
    egui::SidePanel::right("right_panel").show(ctx, |ui| {
        let mut max_rect = ui.max_rect();
        max_rect.set_height(max_rect.height() / 2.0);
        max_rect = max_rect.translate(Vec2 {
            x: 0.0,
            y: max_rect.height(),
        });
        let mut bottom_right_ui = ui.child_ui(max_rect, Layout::left_to_right());
        bottom_right_ui.horizontal_top(|bottom_right_ui| {
            bottom_right_ui.memory_grid(
                "ram",
                &state.hardware.ram,
                0..100,
            );
            bottom_right_ui.memory_grid("stack", &state.hardware.ram, 256..1024);
        });
    });
}

fn draw_vm(state: &VMState, ctx: &egui::Context, action: &mut Option<Action>) {
    egui::SidePanel::right("right_panel").show(ctx, |ui| {
        let mut max_rect = ui.max_rect();
        max_rect.set_height(max_rect.height() / 2.0);
        ui.child_ui(max_rect, Layout::top_down(Align::LEFT))
            .memory_grid("bob2", &state.vm.ram, 0..100);
    });
}

fn draw_start(ctx: &egui::Context, action: &mut Option<Action>) {}

fn reduce(state: &mut AppState, action: &Action) {
    match action {
        _ => {}
    }
}

trait EmulatorWidgets {
    fn memory_grid(&mut self, id_source: impl std::hash::Hash, ram: &RAM, range: Range<i16>);
}

impl EmulatorWidgets for egui::Ui {
    fn memory_grid(&mut self, id_source: impl std::hash::Hash, ram: &RAM, range: Range<i16>) {
        let text_style = egui::TextStyle::Body;
        let row_height = self.text_style_height(&text_style);
        egui::ScrollArea::vertical().id_source(&id_source).show_rows(self, row_height, range.len(), |ui, row_range| {
            egui::Grid::new(&id_source).striped(true).show(ui, |ui| {
                for i in row_range {
                    let address = i as i16 + range.start;
                    ui.label(address.to_string());
                    ui.label(ram[address].to_string());
                    ui.end_row();
                }
            });
        });
    }
}

impl epi::App for EmulatorApp {
    fn name(&self) -> &str {
        "eframe template"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        _ctx: &egui::Context,
        _frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
    }

    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
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
    let app = EmulatorApp::default();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
