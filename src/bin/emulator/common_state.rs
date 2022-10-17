use nand2tetris::hardware::RAM;
use std::time::Instant;
use crate::hardware_state::BreakpointAction;

pub trait CommonState {
    fn step(&mut self) -> bool;
    fn shared_state(&self) -> &SharedState;
    fn shared_state_mut(&mut self) -> &mut SharedState;
    fn ram(&self) -> &RAM;
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
pub enum Action {
    Breakpoint(BreakpointAction),
    Common(CommonAction),
    Quit,
}

pub struct PerformanceData {
    pub steps_during_last_frame: u64,
    pub total_steps: u64,
    pub run_start: Option<Instant>,
    pub previous_desired_steps_per_second: u64,
}

impl Default for PerformanceData {
    fn default() -> Self {
        PerformanceData {
            steps_during_last_frame: 0,
            total_steps: 0,
            run_start: None,
            previous_desired_steps_per_second: 0,
        }
    }
}

pub struct SharedState {
    pub desired_steps_per_second: u64,
    pub run_started: bool,
    pub breakpoints_open: bool,
}
