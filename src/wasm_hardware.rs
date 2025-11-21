use std::{cell::UnsafeCell, sync::Arc, vec};

use eframe::web_sys::console;
use wasm_bindgen::{JsCast as _, prelude::wasm_bindgen};
use wasm_bindgen_futures::js_sys::{self, Function, Int32Array, Object, WebAssembly::{self, Instance}};

use crate::{hardware::AnyHardware, hardware_parse::assemble_hack_file};

#[wasm_bindgen(raw_module = "https://cdn.jsdelivr.net/gh/AssemblyScript/binaryen.js@v124.0.0/index.js")]
extern "C" {
    type Binaryen;

    #[wasm_bindgen(thread_local_v2, js_name = default)]
    static BINARYEN: Binaryen;

    type FeaturesNs;

    type Features;

    type Module;

    #[wasm_bindgen(method, js_name = readBinary)]
    fn read_binary(this: &Binaryen, buffer_source: &[u8]) -> Module;

    #[wasm_bindgen(method, getter, js_name = Features)]
    fn features(this: &Binaryen) -> FeaturesNs;

    #[wasm_bindgen(method, getter, js_name = SignExt)]
    fn sign_ext(this: &FeaturesNs) -> Features;

    #[wasm_bindgen(method, js_name = setFeatures)]
    fn set_features(this: &Module, features: Features);

    #[wasm_bindgen(method, js_name = setOptimizeLevel)]
    fn set_optimize_level(this: &Binaryen, level: f64);

    #[wasm_bindgen(method, js_name = setShrinkLevel)]
    fn set_shrink_level(this: &Binaryen, level: f64);

    #[wasm_bindgen(method, js_name = setClosedWorld)]
    fn set_closed_world(this: &Binaryen, flag: bool);

    #[wasm_bindgen(method, js_name = setTrapsNeverHappen)]
    fn set_traps_never_happen(this: &Binaryen, flag: bool);

    #[wasm_bindgen(method)]
    fn optimize(this: &Module);

    #[wasm_bindgen(method, js_name = emitBinary)]
    fn emit_binary(this: &Module) -> Vec<u8>;
}

fn optimize(buffer_source: &[u8]) -> Vec<u8> {
    BINARYEN.with(|binaryen| {
        let module = binaryen.read_binary(&buffer_source);
        module.set_features(binaryen.features().sign_ext());
        binaryen.set_optimize_level(2.0);
        binaryen.set_shrink_level(2.0);
        binaryen.set_closed_world(true);
        binaryen.set_traps_never_happen(true);
        module.optimize();

        module.emit_binary()
    })
}

pub struct WasmHardware {
    rom: Box<[crate::hardware::Instruction; crate::hardware::MEM_SIZE]>,
    function: Arc<UnsafeCell<Option<Function>>>,
    memory: Arc<UnsafeCell<Option<Int32Array>>>,
}

impl WasmHardware {
    pub fn from_instructions(instructions: &[crate::hardware::Instruction]) -> Self {
        let unoptimized_wasm = crate::hack_to_wasm::hack_to_wasm(instructions, true).unwrap();
        // let optimized_wasm = optimize(&unoptimized_wasm);
        let promise = WebAssembly::instantiate_buffer(&unoptimized_wasm, &Object::new());
        let future = wasm_bindgen_futures::JsFuture::from(promise);
        let function = Arc::new(UnsafeCell::new(None));
        let memory = Arc::new(UnsafeCell::new(None));
        let function_clone = Arc::clone(&function);
        let memory_clone = Arc::clone(&memory);
        wasm_bindgen_futures::spawn_local(async move {
            match future.await {
                Ok(object) => {
                    let instance = js_sys::Reflect::get(&object, &"instance".into()).unwrap().dyn_into::<Instance>().unwrap();
                    let exports = instance.exports();
                    let func = js_sys::Reflect::get(&exports, &"foo".into()).unwrap().dyn_into::<Function>().unwrap();
                    let mem = js_sys::Reflect::get(&exports, &"memory".into()).unwrap().dyn_into::<js_sys::WebAssembly::Memory>().unwrap();
                    let typed_mem = Int32Array::new_with_byte_offset_and_length(&mem.buffer(), 0, 32768);
                    unsafe {
                        *function_clone.get() = Some(func);
                        *memory_clone.get() = Some(typed_mem);
                    }
                    console::log_1(&"Finished!".into());
                },
                Err(err) => {
                    console::log_1(&err);
                },
            }
        });

        let mut rom = Box::new([crate::hardware::Instruction::new(0); crate::hardware::MEM_SIZE]);
        for (i, instruction) in instructions.iter().enumerate() {
            rom[i] = *instruction;
        }

        WasmHardware {
            rom,
            function,
            memory,
        }
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
        let func_ptr = self.function.get();
        let function = unsafe { &*func_ptr };
        function.is_some()
    }

    fn rom(&self) -> &[crate::hardware::Instruction; crate::hardware::MEM_SIZE] {
        &self.rom
    }

    fn copy_ram(&self) -> crate::hardware::RAM {
        let ptr = self.memory.get();
        let memory = unsafe { (*ptr).as_ref().unwrap() };
        let mut target = vec![0i32; crate::hardware::MEM_SIZE];
        memory.copy_to(&mut target);
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
        0
    }

    fn d_mut(&mut self) -> &mut crate::hardware::Word {
        todo!()
    }

    fn d(&self) -> crate::hardware::Word {
        0
    }

    fn get_ram_value(&self, address: crate::hardware::Word) -> crate::hardware::Word {
        let ptr = self.memory.get();
        let memory = unsafe { (*ptr).as_ref().unwrap() };
        memory.get_index(address as u32) as crate::hardware::Word
    }

    fn set_ram_value(&mut self, address: crate::hardware::Word, value: crate::hardware::Word) {
        let ptr = self.memory.get();
        let memory = unsafe { (*ptr).as_ref().unwrap() };
        memory.set_index(address as u32, value as i32);
    }

    fn pc(&self) -> crate::hardware::Word {
        0
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
        let func_ptr = self.function.get();
        let function = unsafe { (*func_ptr).as_ref().unwrap() };
        _ = function.call1(&Object::new(), &(step_count as u32).into()).unwrap();

        false
    }

    fn reset(&mut self) {
        todo!();
    }
}
