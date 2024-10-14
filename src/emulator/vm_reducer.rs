use super::vm_state::VMState;

pub fn reduce_vm_file_selected(vm_state: &mut VMState, selected_file: &str) {
    selected_file.clone_into(&mut vm_state.selected_file);
}

use super::{
    common_state::{Breakpoint, BreakpointAction},
    hardware_state::HardwareState,
};

pub fn reduce_breakpoint_vm(vm_state: &mut VMState, action: &BreakpointAction) {
    match action {
        BreakpointAction::AddClicked => {
            vm_state.vm.add_breakpoint(&vm_state.selected_breakpoint);
        }
        BreakpointAction::RemoveClicked(row_index) => {
            vm_state.vm.remove_breakpoint(*row_index);
        }
        BreakpointAction::BreakpointChanged(Breakpoint::VM(new_breakpoint)) => {
            vm_state.selected_breakpoint = new_breakpoint.clone();
        }
        BreakpointAction::BreakpointChanged(Breakpoint::Hardware(_)) => {
            panic!("Invalid action {action:?} in VM state");
        }
    }
}
