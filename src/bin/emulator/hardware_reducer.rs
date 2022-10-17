use crate::hardware_state::{BreakpointAction, HardwareState};

use nand2tetris::hardware::Breakpoint;

pub fn reduce_breakpoint_hardware(hardware_state: &mut HardwareState, action: &BreakpointAction) {
    match action {
        BreakpointAction::AddClicked => {
            hardware_state.hardware.add_breakpoint(&Breakpoint {
                var: hardware_state.selected_breakpoint_var,
                value: hardware_state.breakpoint_value,
            });
        }
        BreakpointAction::RemoveClicked(row_index) => {
            hardware_state.hardware.remove_breakpoint(*row_index);
        }
        BreakpointAction::VariableChanged(new_var) => {
            hardware_state.selected_breakpoint_var = *new_var;
        }
        BreakpointAction::ValueChanged(new_value) => {
            hardware_state.breakpoint_value = *new_value;
        }
    }
}
