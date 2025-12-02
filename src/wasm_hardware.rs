use std::{cell::OnceCell, rc::Rc};

use crate::any_wasm::{AnyWasmHandle, Val};

use crate::hardware::Word;
use crate::{hardware::AnyHardware, hardware_parse::assemble_hack_file};

#[cfg(not(target_arch = "wasm32"))]
pub type WasmHardware = GenericWasmHardware<crate::any_wasm::WasmtimeHandle>;

#[cfg(target_arch = "wasm32")]
pub type WasmHardware = GenericWasmHardware<crate::any_wasm::JsWasmHandle>;

struct State<H: AnyWasmHandle> {
    handle: H,
    function: H::Function,
    memory: H::Memory,
    a: H::Global,
    d: H::Global,
    pc: H::Global,
}

pub struct GenericWasmHardware<H: AnyWasmHandle> {
    rom: Box<[crate::hardware::Instruction; crate::hardware::MEM_SIZE]>,
    state: Rc<OnceCell<State<H>>>,
}

impl<H: AnyWasmHandle> GenericWasmHardware<H> {
    pub fn from_instructions(instructions: &[crate::hardware::Instruction]) -> Self {
        let unoptimized_wasm = crate::hack_to_wasm::hack_to_wasm(instructions, true).unwrap();
        let state = Rc::new(OnceCell::new());
        let state_clone = Rc::clone(&state);

        H::from_binary(&unoptimized_wasm, move |handle| {
            let function = handle.get_function("run").unwrap();
            let pc = handle.get_global("pc").unwrap();
            let a = handle.get_global("a").unwrap();
            let d = handle.get_global("d").unwrap();
            let memory = handle.get_memory("memory").unwrap();

            state_clone
                .set(State {
                    handle,
                    function,
                    memory,
                    a,
                    d,
                    pc,
                })
                .ok()
                .unwrap();
        });

        let mut rom = Box::new([crate::hardware::Instruction::new(0); crate::hardware::MEM_SIZE]);
        for (i, instruction) in instructions.iter().enumerate() {
            rom[i] = *instruction;
        }

        Self { rom, state }
    }

    pub fn from_file_contents(contents: &str) -> Self {
        let instructions = assemble_hack_file(contents).unwrap().1;

        Self::from_instructions(&instructions)
    }

    pub fn from_hack_file_contents(contents: &str) -> Self {
        let instructions = contents
            .lines()
            .map(|l| crate::hardware::UWord::from_str_radix(l.trim(), 2).unwrap())
            .map(crate::hardware::Instruction::new)
            .collect::<Vec<_>>();

        Self::from_instructions(&instructions)
    }
}

impl AnyHardware for WasmHardware {
    fn is_ready(&self) -> bool {
        self.state.get().is_some()
    }

    fn rom(&self) -> &[crate::hardware::Instruction; crate::hardware::MEM_SIZE] {
        &self.rom
    }

    fn copy_ram(&self) -> crate::hardware::RAM {
        let state = self.state.get().unwrap();
        let data = state.handle.raw_memory(&state.memory);

        let mut ram = crate::hardware::RAM::default();
        for (i, r) in ram
            .contents
            .iter_mut()
            .enumerate()
            .take(crate::hardware::MEM_SIZE)
        {
            *r = data[i] as crate::hardware::Word;
        }
        ram
    }

    fn a_mut(&mut self) -> &mut crate::hardware::Word {
        todo!()
    }

    fn a(&self) -> crate::hardware::Word {
        let state = self.state.get().unwrap();

        state.handle.get_global_value_i32(&state.a) as Word
    }

    fn d_mut(&mut self) -> &mut crate::hardware::Word {
        todo!()
    }

    fn d(&self) -> crate::hardware::Word {
        let state = self.state.get().unwrap();

        state.handle.get_global_value_i32(&state.d) as Word
    }

    fn get_ram_value(&self, address: crate::hardware::Word) -> crate::hardware::Word {
        let state = self.state.get().unwrap();

        state.handle.get_memory_at(&state.memory, address as usize) as Word
    }

    fn set_ram_value(&mut self, address: crate::hardware::Word, value: crate::hardware::Word) {
        let state = self.state.get().unwrap();

        state
            .handle
            .set_memory_at(&state.memory, address as usize, value as i32);
    }

    fn pc(&self) -> crate::hardware::Word {
        let state = self.state.get().unwrap();

        state.handle.get_global_value_i32(&state.pc) as Word
    }

    fn step(&mut self) -> bool {
        todo!()
    }

    fn load_program(&mut self, program: &[crate::hardware::Instruction]) {
        *self = Self::from_instructions(program)
    }

    fn run_program(&mut self) {
        todo!();
    }

    fn run(&mut self, step_count: u64) -> bool {
        let state = self.state.get().unwrap();

        let mut returns = [Val::I32(0)];
        state.handle.call_function(
            &state.function,
            &[Val::I32(step_count as i32)],
            &mut returns,
        );
        let [Val::I32(ticks)] = returns else {
            panic!("Return type changed");
        };

        false
    }

    fn reset(&mut self) {
        let state = self.state.get().unwrap();

        state.handle.set_global_value_i32(&state.a, 0);
        state.handle.set_global_value_i32(&state.d, 0);
        state.handle.set_global_value_i32(&state.pc, 0);
        state.handle.fill_memory(&state.memory, 0);
    }
}
