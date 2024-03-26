use crate::hardware::RAM;
use crate::vm::VM;

use super::common_state::CommonState;

pub struct VMState {
    pub vm: VM,
    pub selected_file: String,
}

impl VMState {
    pub fn from_file_contents(file_contents: Vec<(String, String)>) -> Self {
        let vm = VM::from_file_contents(file_contents);
        let selected_file = vm.program.files[vm.run_state.current_file_index]
            .name
            .clone();
        VMState { vm, selected_file }
    }
}

impl CommonState for VMState {
    fn run(&mut self, step_count: u64) -> bool {
        self.vm.run(step_count);
        false
    }

    fn ram_mut(&mut self) -> &mut RAM {
        &mut self.vm.run_state.ram
    }

    fn reset(&mut self) {
        self.vm.reset();
    }
}
