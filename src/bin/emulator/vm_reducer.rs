use crate::vm_state::VMState;

pub fn reduce_vm_file_selected(vm_state: &mut VMState, selected_file: &str) {
    vm_state.selected_file = selected_file.to_owned();
}
