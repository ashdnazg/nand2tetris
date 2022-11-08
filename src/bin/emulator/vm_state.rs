use nand2tetris::hardware::RAM;
use nand2tetris::vm::VM;

use crate::common_state::{CommonState, SharedState};

pub struct VMState {
    pub shared_state: SharedState,
    pub vm: VM,
}

impl Default for VMState {
    fn default() -> Self {
        Self {
            shared_state: Default::default(),
            vm: VM::from_dir("hackenstein3DVM"),
        }
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

    fn shared_state(&self) -> &SharedState {
        &self.shared_state
    }

    fn shared_state_mut(&mut self) -> &mut SharedState {
        &mut self.shared_state
    }

    fn ram(&self) -> &RAM {
        &self.vm.ram
    }

    fn ram_mut(&mut self) -> &mut RAM {
        &mut self.vm.ram
    }

    fn reset(&mut self) {
        self.vm.reset();
    }
}
