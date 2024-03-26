use super::vm_state::VMState;

pub fn reduce_vm_file_selected(vm_state: &mut VMState, selected_file: &str) {
    selected_file.clone_into(&mut vm_state.selected_file);
}
