use std::path::PathBuf;

use nand2tetris::hardware::RAM;
use nand2tetris::vm::VM;

use crate::common_state::CommonState;

pub struct VMState {
    pub vm: VM,
    pub selected_file: String,
}

impl VMState {
    pub fn from_dir(path_buf: &PathBuf) -> Self {
        let vm = VM::from_dir(path_buf);
        let selected_file = vm.run_state.current_file_name.clone();
        VMState { vm, selected_file }
    }
}

impl CommonState for VMState {
    fn step(&mut self) -> bool {
        self.vm.step();
        false
    }

    fn run(&mut self, step_count: u64) -> bool {
        self.vm.run(step_count);
        false
    }

    fn ram(&self) -> &RAM {
        &self.vm.run_state.ram
    }

    fn ram_mut(&mut self) -> &mut RAM {
        &mut self.vm.run_state.ram
    }

    fn reset(&mut self) {
        self.vm.reset();
    }
}
