use crate::vm::{Breakpoint, VM};

use super::common_state::CommonState;

pub struct VMState {
    pub vm: VM,
    pub selected_file: String,
    pub selected_breakpoint: Breakpoint,
}

impl VMState {
    pub fn from_file_contents(file_contents: Vec<(String, String)>) -> Self {
        let vm = VM::from_file_contents(file_contents);
        let selected_file = vm.program.files[vm.run_state.current_file_index]
            .name
            .clone();
        let selected_breakpoint = Breakpoint::SP(0);
        VMState {
            vm,
            selected_file,
            selected_breakpoint,
        }
    }
}

impl CommonState for VMState {
    fn run(&mut self, step_count: u64) -> bool {
        self.vm.run(step_count);
        false
    }

    fn set_ram_value(&mut self, address: i16, value: i16) {
        self.vm.run_state.ram[address] = value;
    }

    fn reset(&mut self) {
        self.vm.reset();
    }
}
