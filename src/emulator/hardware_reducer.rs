use super::{
    common_state::{Breakpoint, BreakpointAction},
    hardware_state::HardwareState,
};

pub fn reduce_breakpoint_hardware(hardware_state: &mut HardwareState, action: &BreakpointAction) {
    match action {
        BreakpointAction::AddClicked => {
            // hardware_state
            //     .hardware
            //     .add_breakpoint(&hardware_state.selected_breakpoint);
        }
        BreakpointAction::RemoveClicked(row_index) => {
            // hardware_state.hardware.remove_breakpoint(*row_index);
        }
        BreakpointAction::BreakpointChanged(Breakpoint::Hardware(new_breakpoint)) => {
            hardware_state.selected_breakpoint = new_breakpoint.clone();
        }
        BreakpointAction::BreakpointChanged(Breakpoint::VM(_)) => {
            panic!("Invalid action {action:?} in hardware state");
        }
    }
}
