use super::hardware_state::HardwareState;
use super::instant::Instant;
use super::vm_state::VMState;
use crate::{
    hardware::{self, Word, RAM},
    vm,
};
use eframe::egui::{DroppedFile, Key, Modifiers};

#[allow(clippy::large_enum_variant)]
#[derive(Default)]
pub enum AppState {
    Hardware(HardwareState),
    VM(VMState),
    #[default]
    Start,
}

#[derive(PartialEq)]
pub enum UIStyle {
    Hardware,
    VM,
}

pub trait CommonState {
    fn run(&mut self, step_count: u64) -> bool;
    fn ram_mut(&mut self) -> &mut RAM;
    fn reset(&mut self);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommonAction {
    StepClicked,
    RunClicked,
    PauseClicked,
    ResetClicked,
    BreakpointsClicked,
    BreakpointsClosed,
    SpeedSliderMoved(u64),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Breakpoint {
    Hardware(hardware::Breakpoint),
    VM(vm::Breakpoint),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BreakpointAction {
    AddClicked,
    BreakpointChanged(Breakpoint),
    RemoveClicked(usize),
}

#[derive(Debug)]
pub enum Action {
    FilesPicked(Vec<(String, String)>),
    FilePicked { name: String, contents: String },
    FilesDropped(Vec<DroppedFile>),
    Breakpoint(BreakpointAction),
    Common(CommonAction),
    VMFileSelected(String),
    CloseFile,
    Quit,
}

#[derive(Default)]
pub struct PerformanceData {
    pub steps_during_last_frame: u64,
    pub total_steps: u64,
    pub run_start: Option<Instant>,
    pub previous_desired_steps_per_second: u64,
}

pub struct SharedState {
    pub desired_steps_per_second: u64,
    pub run_started: bool,
    pub scroll_once: bool,
    pub breakpoints_open: bool,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            desired_steps_per_second: 500000,
            run_started: false,
            scroll_once: true,
            breakpoints_open: false,
        }
    }
}

pub trait StepRunnable {
    fn run_steps(&mut self, steps_to_run: u64, key_down: Option<Key>, modifiers: Modifiers)
        -> bool;
}

impl<T: CommonState> StepRunnable for T {
    fn run_steps(
        &mut self,
        steps_to_run: u64,
        key_down: Option<Key>,
        modifiers: Modifiers,
    ) -> bool {
        if steps_to_run > 0 {
            let keyboard_value = keyboard_value_from_key(key_down, modifiers);
            self.ram_mut().set_keyboard(keyboard_value);

            if self.run(steps_to_run) {
                return false;
            }
        }
        true
    }
}

fn keyboard_value_from_key(key: Option<Key>, modifiers: Modifiers) -> Word {
    let mut value = match key {
        Some(Key::ArrowDown) => 133,
        Some(Key::ArrowLeft) => 130,
        Some(Key::ArrowRight) => 132,
        Some(Key::ArrowUp) => 131,
        Some(Key::Escape) => 140,
        Some(Key::Tab) => todo!(),
        Some(Key::Backspace) => 129,
        Some(Key::Enter) => 128,
        Some(Key::Space) => 32,
        Some(Key::Insert) => 138,
        Some(Key::Delete) => 139,
        Some(Key::Home) => 134,
        Some(Key::End) => 135,
        Some(Key::PageUp) => 136,
        Some(Key::PageDown) => 137,
        Some(Key::F1) => 141,
        Some(Key::F2) => 142,
        Some(Key::F3) => 143,
        Some(Key::F4) => 144,
        Some(Key::F5) => 145,
        Some(Key::F6) => 146,
        Some(Key::F7) => 147,
        Some(Key::F8) => 148,
        Some(Key::F9) => 149,
        Some(Key::F10) => 150,
        Some(Key::F11) => 151,
        Some(Key::F12) => 152,
        Some(Key::Num0) => 48,
        Some(Key::Num1) => 49,
        Some(Key::Num2) => 50,
        Some(Key::Num3) => 51,
        Some(Key::Num4) => 52,
        Some(Key::Num5) => 53,
        Some(Key::Num6) => 54,
        Some(Key::Num7) => 55,
        Some(Key::Num8) => 56,
        Some(Key::Num9) => 57,
        Some(Key::A) => 65,
        Some(Key::B) => 66,
        Some(Key::C) => 67,
        Some(Key::D) => 68,
        Some(Key::E) => 69,
        Some(Key::F) => 70,
        Some(Key::G) => 71,
        Some(Key::H) => 72,
        Some(Key::I) => 73,
        Some(Key::J) => 74,
        Some(Key::K) => 75,
        Some(Key::L) => 76,
        Some(Key::M) => 77,
        Some(Key::N) => 78,
        Some(Key::O) => 79,
        Some(Key::P) => 80,
        Some(Key::Q) => 81,
        Some(Key::R) => 82,
        Some(Key::S) => 83,
        Some(Key::T) => 84,
        Some(Key::U) => 85,
        Some(Key::V) => 86,
        Some(Key::W) => 87,
        Some(Key::X) => 88,
        Some(Key::Y) => 89,
        Some(Key::Z) => 90,
        _ => 0,
    };
    if (65..=90).contains(&value) && !modifiers.shift {
        value += 32;
    }

    value
}
