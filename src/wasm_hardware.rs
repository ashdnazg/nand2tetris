use std::{cell::UnsafeCell, sync::Arc, vec};

use eframe::web_sys::console;
use wasm_bindgen::{JsCast as _, prelude::wasm_bindgen};
use wasm_bindgen_futures::js_sys::{
    self, Function, Int32Array, Object,
    WebAssembly::{self, Global, Instance},
};

use crate::{hardware::AnyHardware, hardware_parse::assemble_hack_file};

struct State {
    function: Function,
    memory: Int32Array,
    a: Global,
    d: Global,
    pc: Global,
}

pub struct WasmHardware {
    rom: Box<[crate::hardware::Instruction; crate::hardware::MEM_SIZE]>,
    state: Arc<UnsafeCell<Option<State>>>,
}

impl WasmHardware {
    pub fn from_instructions(instructions: &[crate::hardware::Instruction]) -> Self {
        let unoptimized_wasm = crate::hack_to_wasm::hack_to_wasm(instructions, true).unwrap();
        let promise = WebAssembly::instantiate_buffer(&unoptimized_wasm, &Object::new());
        let future = wasm_bindgen_futures::JsFuture::from(promise);
        let state = Arc::new(UnsafeCell::new(None));
        let state_clone = Arc::clone(&state);
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
                    unsafe {
                        *state_clone.get() = Some(new_state);
                    }
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
            .map(|raw| crate::hardware::Instruction::new(raw))
            .collect::<Vec<_>>();

        Self::from_instructions(&instructions)
    }
}

impl AnyHardware for WasmHardware {
    fn is_ready(&self) -> bool {
        let state_ptr = self.state.get();
        let state = unsafe { &*state_ptr };
        state.is_some()
    }

    fn rom(&self) -> &[crate::hardware::Instruction; crate::hardware::MEM_SIZE] {
        &self.rom
    }

    fn copy_ram(&self) -> crate::hardware::RAM {
        let state_ptr = self.state.get();
        let state = unsafe { (*state_ptr).as_ref().unwrap() };

        let mut target = vec![0i32; crate::hardware::MEM_SIZE];
        state.memory.copy_to(&mut target);
        let mut ram = crate::hardware::RAM::default();
        for i in 0..crate::hardware::MEM_SIZE {
            ram.contents[i] = target[i] as crate::hardware::Word;
        }
        ram
    }

    fn a_mut(&mut self) -> &mut crate::hardware::Word {
        todo!()
    }

    fn a(&self) -> crate::hardware::Word {
        let state_ptr = self.state.get();
        let state = unsafe { (*state_ptr).as_ref().unwrap() };

        state.a.value().as_f64().unwrap() as crate::hardware::Word
    }

    fn d_mut(&mut self) -> &mut crate::hardware::Word {
        todo!()
    }

    fn d(&self) -> crate::hardware::Word {
        let state_ptr = self.state.get();
        let state = unsafe { (*state_ptr).as_ref().unwrap() };

        state.d.value().as_f64().unwrap() as crate::hardware::Word
    }

    fn get_ram_value(&self, address: crate::hardware::Word) -> crate::hardware::Word {
        let state_ptr = self.state.get();
        let state = unsafe { (*state_ptr).as_ref().unwrap() };

        state.memory.get_index(address as u32) as crate::hardware::Word
    }

    fn set_ram_value(&mut self, address: crate::hardware::Word, value: crate::hardware::Word) {
        let state_ptr = self.state.get();
        let state = unsafe { (*state_ptr).as_ref().unwrap() };

        state.memory.set_index(address as u32, value as i32);
    }

    fn pc(&self) -> crate::hardware::Word {
        let state_ptr = self.state.get();
        let state = unsafe { (*state_ptr).as_ref().unwrap() };

        state.pc.value().as_f64().unwrap() as crate::hardware::Word
    }

    fn step(&mut self) -> bool {
        todo!()
    }

    fn load_program(&mut self, program: &[crate::hardware::Instruction]) {
        *self = Self::from_instructions(&program)
    }

    fn run_program(&mut self) {
        todo!();
    }

    fn run(&mut self, step_count: u64) -> bool {
        let state_ptr = self.state.get();
        let state = unsafe { (*state_ptr).as_ref().unwrap() };

        _ = state
            .function
            .call1(&Object::new(), &(step_count as u32).into())
            .unwrap();

        false
    }

    fn reset(&mut self) {
        let state_ptr = self.state.get();
        let state = unsafe { (*state_ptr).as_ref().unwrap() };

        state.a.set_value(&0.into());
        state.d.set_value(&0.into());
        state.pc.set_value(&0.into());
        state.memory.fill(0, 0, crate::hardware::MEM_SIZE as u32);
    }
}
