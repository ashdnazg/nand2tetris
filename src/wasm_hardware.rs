use std::{cell::OnceCell, rc::Rc, vec};

use eframe::web_sys::console;
use wasm_bindgen::JsCast as _;
use wasm_bindgen_futures::js_sys::{
    self, Function, Int32Array, Object,
    WebAssembly::{self, Global, Instance},
};

use crate::{hardware::AnyHardware, hardware_parse::assemble_hack_file};

#[derive(Debug)]
struct State {
    function: Function,
    memory: Int32Array,
    a: Global,
    d: Global,
    pc: Global,
}

#[derive(Debug)]
pub struct WasmHardware {
    rom: Box<[crate::hardware::Instruction; crate::hardware::MEM_SIZE]>,
    state: Rc<OnceCell<State>>,
}

impl WasmHardware {
    pub fn from_instructions(instructions: &[crate::hardware::Instruction]) -> Self {
        let unoptimized_wasm = crate::hack_to_wasm::hack_to_wasm(instructions, true).unwrap();
        let promise = WebAssembly::instantiate_buffer(&unoptimized_wasm, &Object::new());
        let future = wasm_bindgen_futures::JsFuture::from(promise);
        let state = Rc::new(OnceCell::new());
        let state_clone = Rc::clone(&state);
        wasm_bindgen_futures::spawn_local(async move {
            match future.await {
                Ok(object) => {
                    let instance = js_sys::Reflect::get(&object, &"instance".into())
                        .unwrap()
                        .dyn_into::<Instance>()
                        .unwrap();
                    let exports = instance.exports();
                    let function = js_sys::Reflect::get(&exports, &"run".into())
                        .unwrap()
                        .dyn_into::<Function>()
                        .unwrap();
                    let raw_memory = js_sys::Reflect::get(&exports, &"memory".into())
                        .unwrap()
                        .dyn_into::<js_sys::WebAssembly::Memory>()
                        .unwrap();
                    let memory =
                        Int32Array::new_with_byte_offset_and_length(&raw_memory.buffer(), 0, 32768);
                    let a = js_sys::Reflect::get(&exports, &"a".into())
                        .unwrap()
                        .dyn_into::<Global>()
                        .unwrap();
                    let d = js_sys::Reflect::get(&exports, &"d".into())
                        .unwrap()
                        .dyn_into::<Global>()
                        .unwrap();
                    let pc = js_sys::Reflect::get(&exports, &"pc".into())
                        .unwrap()
                        .dyn_into::<Global>()
                        .unwrap();

                    let new_state = State {
                        function,
                        memory,
                        a,
                        d,
                        pc,
                    };
                    state_clone.set(new_state).unwrap();
                    console::log_1(&"Finished!".into());
                }
                Err(err) => {
                    console::log_1(&err);
                }
            }
        });

        let mut rom = Box::new([crate::hardware::Instruction::new(0); crate::hardware::MEM_SIZE]);
        for (i, instruction) in instructions.iter().enumerate() {
            rom[i] = *instruction;
        }

        WasmHardware { rom, state }
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

        let mut dest = vec![0i32; crate::hardware::MEM_SIZE];
        state.memory.copy_to(&mut dest);
        let mut ram = crate::hardware::RAM::default();
        for (i, value) in dest.into_iter().enumerate().take(crate::hardware::MEM_SIZE) {
            ram.contents[i] = value as crate::hardware::Word;
        }
        ram
    }

    fn a_mut(&mut self) -> &mut crate::hardware::Word {
        todo!()
    }

    fn a(&self) -> crate::hardware::Word {
        let state = self.state.get().unwrap();

        state.a.value().as_f64().unwrap() as crate::hardware::Word
    }

    fn d_mut(&mut self) -> &mut crate::hardware::Word {
        todo!()
    }

    fn d(&self) -> crate::hardware::Word {
        let state = self.state.get().unwrap();

        state.d.value().as_f64().unwrap() as crate::hardware::Word
    }

    fn get_ram_value(&self, address: crate::hardware::Word) -> crate::hardware::Word {
        let state = self.state.get().unwrap();

        state.memory.get_index(address as u32) as crate::hardware::Word
    }

    fn set_ram_value(&mut self, address: crate::hardware::Word, value: crate::hardware::Word) {
        let state = self.state.get().unwrap();

        state.memory.set_index(address as u32, value as i32);
    }

    fn pc(&self) -> crate::hardware::Word {
        let state = self.state.get().unwrap();

        state.pc.value().as_f64().unwrap() as crate::hardware::Word
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

        _ = state
            .function
            .call1(&Object::new(), &(step_count as u32).into())
            .unwrap();

        false
    }

    fn reset(&mut self) {
        let state = self.state.get().unwrap();

        state.a.set_value(&0.into());
        state.d.set_value(&0.into());
        state.pc.set_value(&0.into());
        state.memory.fill(0, 0, crate::hardware::MEM_SIZE as u32);
    }
}
