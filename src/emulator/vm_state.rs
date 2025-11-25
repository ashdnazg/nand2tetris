use crate::vm::{Breakpoint, VM};

use super::common_state::CommonState;

#[cfg(not(target_arch = "wasm32"))]
type VMImpl = crate::wasm_vm2::WasmVm;

#[cfg(target_arch = "wasm32")]
type VMImpl = crate::wasm_vm::WasmVm;

pub struct VMState {
    pub vm: VMImpl,
    pub selected_file: String,
    pub selected_breakpoint: Breakpoint,
}

impl VMState {
    pub fn from_file_contents(file_contents: Vec<(String, String)>) -> Self {
        let vm = VMImpl::from_file_contents(file_contents);
        let selected_file = "Sys".to_owned(); //vm.current_file_name().to_owned();
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
        self.vm.set_ram_value(address, value);
    }

    fn reset(&mut self) {
        self.vm.reset();
    }
}
