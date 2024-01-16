use std::path::PathBuf;

use crate::hardware::{BreakpointVar, Hardware, Instruction, RAM};

use super::common_state::CommonState;

pub struct HardwareState {
    pub selected_breakpoint_var: BreakpointVar,
    pub breakpoint_value: i16,
    pub breakpoints_open: bool,
    pub hardware: Hardware,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BreakpointAction {
    AddClicked,
    VariableChanged(BreakpointVar),
    ValueChanged(i16),
    RemoveClicked(usize),
}

impl Default for HardwareState {
    fn default() -> Self {
        let mut hardware = Hardware::default();
        let program: [u16; 29] = [
            16384, 60432, 16, 58248, 17, 60040, 24576, 64528, 12, 58114, 17, 61064, 17, 64528, 16,
            65000, 58120, 24576, 60560, 16, 62672, 4, 58115, 16384, 60432, 16, 58248, 4, 60039,
        ];
        hardware.load_program(program.iter().map(|raw| Instruction::new(*raw)));

        HardwareState {
            selected_breakpoint_var: BreakpointVar::A,
            breakpoint_value: 0,
            breakpoints_open: false,
            hardware,
        }
    }
}

impl HardwareState {
    pub fn from_file(path: &PathBuf) -> Self {
        HardwareState {
            selected_breakpoint_var: BreakpointVar::A,
            breakpoint_value: 0,
            breakpoints_open: false,
            hardware: Hardware::from_file(path),
        }
    }
}

impl CommonState for HardwareState {
    fn step(&mut self) -> bool {
        self.hardware.step()
    }

    fn run(&mut self, step_count: u64) -> bool {
        self.hardware.run(step_count)
    }

    fn ram(&self) -> &RAM {
        &self.hardware.ram
    }

    fn ram_mut(&mut self) -> &mut RAM {
        &mut self.hardware.ram
    }

    fn reset(&mut self) {
        self.hardware.reset();
    }
}
