use nand2tetris::hardware::RAM;
use nand2tetris::vm::VM;

use crate::common_state::{CommonState, SharedState};

pub struct VMState {
    pub shared_state: SharedState,
    pub vm: VM,
}

impl CommonState for VMState {
    fn step(&mut self) -> bool {
        self.vm.step();
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