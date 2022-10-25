use nand2tetris::hardware::{BreakpointVar, Hardware, Instruction};

use crate::common_state::SharedState;

pub struct HardwareState {
    pub shared_state: SharedState,
    pub selected_breakpoint_var: BreakpointVar,
    pub breakpoint_value: i16,
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
            shared_state: SharedState::default(),
            selected_breakpoint_var: BreakpointVar::A,
            breakpoint_value: 0,
            hardware,
        }
    }
}
