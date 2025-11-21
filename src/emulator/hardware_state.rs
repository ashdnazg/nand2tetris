use crate::{
    hardware::{AnyHardware, Breakpoint, BreakpointVar, Hardware, Instruction, RAM, UWord},
    wasm_hardware::WasmHardware,
};

use super::common_state::CommonState;

pub struct HardwareState {
    pub selected_breakpoint: Breakpoint,
    pub hardware: Box<dyn AnyHardware>,
}

impl Default for HardwareState {
    fn default() -> Self {
        let mut hardware = Hardware::default();
        let program: [UWord; 29] = [
            16384, 60432, 16, 58248, 17, 60040, 24576, 64528, 12, 58114, 17, 61064, 17, 64528, 16,
            65000, 58120, 24576, 60560, 16, 62672, 4, 58115, 16384, 60432, 16, 58248, 4, 60039,
        ];
        hardware.load_program(
            &program
                .iter()
                .map(|raw| Instruction::new(*raw))
                .collect::<Vec<_>>(),
        );

        HardwareState {
            selected_breakpoint: Breakpoint {
                var: BreakpointVar::A,
                value: 0,
            },
            hardware: Box::new(hardware),
        }
    }
}

impl HardwareState {
    pub fn from_file_contents(contents: &str) -> Self {
        HardwareState {
            selected_breakpoint: Breakpoint {
                var: BreakpointVar::A,
                value: 0,
            },
            hardware: Box::new(WasmHardware::from_file_contents(contents)),
        }
    }

    pub fn from_hack_file_contents(contents: &str) -> Self {
        HardwareState {
            selected_breakpoint: Breakpoint {
                var: BreakpointVar::A,
                value: 0,
            },
            hardware: Box::new(WasmHardware::from_hack_file_contents(contents)),
        }
    }
}

impl CommonState for HardwareState {
    fn run(&mut self, step_count: u64) -> bool {
        self.hardware.run(step_count)
    }

    fn set_ram_value(&mut self, address: i16, value: i16) {
        self.hardware.set_ram_value(address, value);
    }

    fn reset(&mut self) {
        self.hardware.reset();
    }
}
