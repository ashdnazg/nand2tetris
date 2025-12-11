use std::borrow::Cow;

#[cfg(not(target_arch = "wasm32"))]
use wasmtime::{Engine, Func, Global, Instance, Linker, Memory, Module, Store};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::js_sys::{
    self, BigInt, Function, Int32Array, Object,
    WebAssembly::{self, Global, Instance},
};

pub enum Val {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

#[cfg(not(target_arch = "wasm32"))]
pub trait NonWasmSendSync: Send + Sync {}

#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + Sync> NonWasmSendSync for T {}

#[cfg(target_arch = "wasm32")]
pub trait NonWasmSendSync {}

#[cfg(target_arch = "wasm32")]
impl<T> NonWasmSendSync for T {}

pub trait AnyWasmHandle: Sized + NonWasmSendSync + 'static {
    type Global: 'static + NonWasmSendSync;
    type Memory: 'static + NonWasmSendSync;
    type Function: 'static + NonWasmSendSync;

    fn from_binary(binary: &[u8], callback: impl FnOnce(Self) + NonWasmSendSync + 'static);

    fn get_global(&mut self, name: &str) -> Option<Self::Global>;

    fn get_memory(&mut self, name: &str) -> Option<Self::Memory>;

    fn get_function(&mut self, name: &str) -> Option<Self::Function>;

    fn get_global_value_i32(&mut self, global: &Self::Global) -> i32;

    fn set_global_value_i32(&mut self, global: &Self::Global, value: i32);

    fn get_memory_at(&mut self, memory: &Self::Memory, address: usize) -> i32;

    fn set_memory_at(&mut self, memory: &Self::Memory, address: usize, value: i32);

    fn raw_memory(&mut self, memory: &Self::Memory) -> Cow<'_, [i32]>;

    fn fill_memory(&mut self, memory: &Self::Memory, value: i32);

    fn call_function<const A: usize, const R: usize>(
        &mut self,
        function: &Self::Function,
        args: &[Val; A],
        returns: &mut [Val; R],
    );
}

#[cfg(not(target_arch = "wasm32"))]
pub struct WasmtimeHandle {
    store: Store<()>,
    instance: Instance,
}

#[cfg(not(target_arch = "wasm32"))]
impl AnyWasmHandle for WasmtimeHandle {
    type Global = Global;
    type Memory = Memory;
    type Function = Func;

    fn from_binary(binary: &[u8], callback: impl FnOnce(Self) + Send + 'static) {
        let binary = binary.to_vec();
        // std::fs::write("unopt.wasm", &binary).unwrap();
        // let text = wasmprinter::print_bytes(&binary).unwrap();
        // std::fs::write("unopt.wat", &text).unwrap();
        #[cfg(not(test))]
        std::thread::spawn(move || {
            // let binary = unsafe {
            //     let b_module = binaryen_sys::BinaryenModuleReadWithFeatures(binary.as_ptr() as *mut _, binary.len(), binaryen_sys::BinaryenFeatureSignExt() | binaryen_sys::BinaryenFeatureBulkMemory());
            //     binaryen_sys::BinaryenModuleRunPassesWithSettings(
            //         b_module,
            //         std::ptr::null_mut(),
            //         0,
            //         2,
            //         4,
            //         0
            //     );

            //     let write_result =
            //     binaryen_sys::BinaryenModuleAllocateAndWrite(b_module, std::ptr::null());

            //     // Create a slice from the resulting array and then copy it in vector.
            //     let binary_buf = if write_result.binaryBytes == 0 {
            //         vec![]
            //     } else {
            //         std::slice::from_raw_parts(write_result.binary as *const u8, write_result.binaryBytes)
            //             .to_vec()
            //     };

            //     // This will free buffers in the write_result.
            //     binaryen_sys::BinaryenShimDisposeBinaryenModuleAllocateAndWriteResult(write_result);

            //     println!("{}", binary_buf.len());

            //     // std::fs::write("opt.wasm", &binary_buf).unwrap();
            //     // let text = wasmprinter::print_bytes(&binary_buf).unwrap();
            //     // std::fs::write("opt.wat", &text).unwrap();

            //     binary_buf
            // };
            let engine = Engine::default();
            let module = Module::from_binary(&engine, &binary).unwrap();
            let mut store = Store::new(&engine, ());
            let mut linker = Linker::new(&engine);
            linker
                .func_wrap("env", "print", |arg: i32| {
                    println!("WASM print: {}", arg);
                })
                .unwrap();
            let instance = linker.instantiate(&mut store, &module).unwrap();
            let handle = Self { store, instance };

            callback(handle);
        });
        #[cfg(test)]
        {
            let engine = Engine::default();
            let module = Module::from_binary(&engine, &binary).unwrap();
            let mut store = Store::new(&engine, ());
            let mut linker = Linker::new(&engine);
            linker
                .func_wrap("env", "print", |arg: i32| {
                    println!("WASM print: {}", arg);
                })
                .unwrap();
            let instance = linker.instantiate(&mut store, &module).unwrap();
            let handle = Self { store, instance };

            callback(handle);
        }
    }

    fn get_global(&mut self, name: &str) -> Option<Self::Global> {
        self.instance.get_global(&mut self.store, name)
    }

    fn get_memory(&mut self, name: &str) -> Option<Self::Memory> {
        self.instance.get_memory(&mut self.store, name)
    }

    fn get_function(&mut self, name: &str) -> Option<Self::Function> {
        self.instance.get_func(&mut self.store, name)
    }

    fn get_global_value_i32(&mut self, global: &Self::Global) -> i32 {
        global.get(&mut self.store).i32().unwrap()
    }

    fn set_global_value_i32(&mut self, global: &Self::Global, value: i32) {
        global.set(&mut self.store, value.into()).unwrap()
    }

    fn get_memory_at(&mut self, memory: &Self::Memory, address: usize) -> i32 {
        let mut bytes = [0; 4];
        memory
            .read(&mut self.store, address * 4, &mut bytes)
            .unwrap();

        i32::from_le_bytes(bytes)
    }

    fn set_memory_at(&mut self, memory: &Self::Memory, address: usize, value: i32) {
        memory
            .write(&mut self.store, address * 4, &value.to_le_bytes())
            .unwrap();
    }

    fn fill_memory(&mut self, memory: &Self::Memory, value: i32) {
        let data = memory.data_mut(&mut self.store);
        let (_, ints, _) = unsafe { data.align_to_mut::<i32>() };

        ints[..crate::hardware::MEM_SIZE].fill(value);
    }

    fn raw_memory(&mut self, memory: &Self::Memory) -> Cow<'_, [i32]> {
        let data = memory.data(&mut self.store);
        let (_, ints, _) = unsafe { data.align_to::<i32>() };

        Cow::Borrowed(ints)
    }

    fn call_function<const A: usize, const R: usize>(
        &mut self,
        function: &Self::Function,
        args: &[Val; A],
        returns: &mut [Val; R],
    ) {
        let params = args
            .iter()
            .map(|arg| match *arg {
                Val::I32(i) => i.into(),
                Val::I64(i) => i.into(),
                Val::F32(f) => f.into(),
                Val::F64(f) => f.into(),
            })
            .collect::<Vec<_>>();

        let mut results = [wasmtime::Val::I32(0); R];

        function
            .call(&mut self.store, &params, &mut results)
            .unwrap();

        for (index, result) in results.iter().enumerate() {
            match &mut returns[index] {
                Val::I32(i) => *i = result.i32().unwrap(),
                Val::I64(i) => *i = result.i64().unwrap(),
                Val::F32(f) => *f = result.f32().unwrap(),
                Val::F64(f) => *f = result.f64().unwrap(),
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub struct JsWasmHandle {
    instance: Instance,
}

#[cfg(target_arch = "wasm32")]
impl JsWasmHandle {
    fn get_export<T: JsCast>(&self, name: &str) -> Option<T> {
        let exports = self.instance.exports();

        Some(
            js_sys::Reflect::get(&exports, &name.into())
                .ok()?
                .dyn_into::<T>()
                .unwrap(),
        )
    }
}

#[cfg(target_arch = "wasm32")]
impl From<&Val> for JsValue {
    fn from(val: &Val) -> Self {
        match *val {
            Val::I32(i) => i.into(),
            Val::I64(i) => i.into(),
            Val::F32(f) => f.into(),
            Val::F64(f) => f.into(),
        }
    }
}


#[cfg(target_arch = "wasm32")]
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

    #[wasm_bindgen(method, getter, js_name = BulkMemory)]
    fn bulk_memory(this: &FeaturesNs) -> Features;

    #[wasm_bindgen(method, getter, js_name = All)]
    fn all(this: &FeaturesNs) -> Features;

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

#[cfg(target_arch = "wasm32")]
impl AnyWasmHandle for JsWasmHandle {
    type Global = Global;
    type Memory = Int32Array;
    type Function = Function;

    fn from_binary(binary: &[u8], callback: impl FnOnce(Self) + 'static) {
        let binary = binary.to_vec();

        wasm_bindgen_futures::spawn_local(async move {
            // let binary = BINARYEN.with(|binaryen| {
            //     let module = binaryen.read_binary(&binary);
            //     module.set_features(binaryen.features().all());
            //     binaryen.set_optimize_level(4.0);
            //     binaryen.set_shrink_level(2.0);
            //     binaryen.set_closed_world(true);
            //     binaryen.set_traps_never_happen(true);
            //     module.optimize();

            //     module.emit_binary()
            // });
            let promise = WebAssembly::instantiate_buffer(&binary, &Object::new());
            let future = wasm_bindgen_futures::JsFuture::from(promise);
            let object = future.await.unwrap();
            let instance = js_sys::Reflect::get(&object, &"instance".into())
                .unwrap()
                .dyn_into::<Instance>()
                .unwrap();
            let handle = Self { instance };
            callback(handle);
        });
    }

    fn get_global(&mut self, name: &str) -> Option<Self::Global> {
        self.get_export(name)
    }

    fn get_memory(&mut self, name: &str) -> Option<Self::Memory> {
        use wasm_bindgen_futures::js_sys::WebAssembly::Memory;

        self.get_export::<Memory>(name).map(|mem| {
            Int32Array::new_with_byte_offset_and_length(
                &mem.buffer(),
                0,
                crate::hardware::MEM_SIZE as u32,
            )
        })
    }

    fn get_function(&mut self, name: &str) -> Option<Self::Function> {
        self.get_export(name)
    }

    fn get_global_value_i32(&mut self, global: &Self::Global) -> i32 {
        global.value().as_f64().unwrap() as i32
    }

    fn set_global_value_i32(&mut self, global: &Self::Global, value: i32) {
        global.set_value(&value.into());
    }

    fn get_memory_at(&mut self, memory: &Self::Memory, address: usize) -> i32 {
        memory.get_index(address as u32)
    }

    fn set_memory_at(&mut self, memory: &Self::Memory, address: usize, value: i32) {
        memory.set_index(address as u32, value);
    }

    fn raw_memory(&mut self, memory: &Self::Memory) -> Cow<'_, [i32]> {
        let mut dest = vec![0i32; crate::hardware::MEM_SIZE];
        memory.copy_to(&mut dest);

        Cow::Owned(dest)
    }

    fn fill_memory(&mut self, memory: &Self::Memory, value: i32) {
        memory.fill(value, 0, crate::hardware::MEM_SIZE as u32);
    }

    fn call_function<const A: usize, const R: usize>(
        &mut self,
        function: &Self::Function,
        args: &[Val; A],
        returns: &mut [Val; R],
    ) {
        let ret = match args.as_slice() {
            [] => function.call0(&Object::new()).unwrap(),
            [arg1] => function.call1(&Object::new(), &arg1.into()).unwrap(),
            [arg1, arg2] => function
                .call2(&Object::new(), &arg1.into(), &arg2.into())
                .unwrap(),
            [arg1, arg2, arg3] => function
                .call3(&Object::new(), &arg1.into(), &arg2.into(), &arg3.into())
                .unwrap(),
            [arg1, arg2, arg3, arg4] => function
                .call4(
                    &Object::new(),
                    &arg1.into(),
                    &arg2.into(),
                    &arg3.into(),
                    &arg4.into(),
                )
                .unwrap(),
            [arg1, arg2, arg3, arg4, arg5] => function
                .call5(
                    &Object::new(),
                    &arg1.into(),
                    &arg2.into(),
                    &arg3.into(),
                    &arg4.into(),
                    &arg5.into(),
                )
                .unwrap(),
            [arg1, arg2, arg3, arg4, arg5, arg6] => function
                .call6(
                    &Object::new(),
                    &arg1.into(),
                    &arg2.into(),
                    &arg3.into(),
                    &arg4.into(),
                    &arg5.into(),
                    &arg6.into(),
                )
                .unwrap(),
            [arg1, arg2, arg3, arg4, arg5, arg6, arg7] => function
                .call7(
                    &Object::new(),
                    &arg1.into(),
                    &arg2.into(),
                    &arg3.into(),
                    &arg4.into(),
                    &arg5.into(),
                    &arg6.into(),
                    &arg7.into(),
                )
                .unwrap(),
            [arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8] => function
                .call8(
                    &Object::new(),
                    &arg1.into(),
                    &arg2.into(),
                    &arg3.into(),
                    &arg4.into(),
                    &arg5.into(),
                    &arg6.into(),
                    &arg7.into(),
                    &arg8.into(),
                )
                .unwrap(),
            [arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9] => function
                .call9(
                    &Object::new(),
                    &arg1.into(),
                    &arg2.into(),
                    &arg3.into(),
                    &arg4.into(),
                    &arg5.into(),
                    &arg6.into(),
                    &arg7.into(),
                    &arg8.into(),
                    &arg9.into(),
                )
                .unwrap(),
            _ => panic!("Too many arguments to call_function: {A}"),
        };

        let set_ret = |ret: &mut Val, value: JsValue| match ret {
            Val::I32(i) => *i = value.as_f64().unwrap() as i32,
            Val::I64(i) => *i = value.dyn_into::<BigInt>().unwrap().try_into().unwrap(),
            Val::F32(f) => *f = value.as_f64().unwrap() as f32,
            Val::F64(f) => *f = value.as_f64().unwrap() as f64,
        };

        match R {
            0 => {}
            1 => set_ret(&mut returns[0], ret),
            _ => {
                let array = ret.dyn_into::<js_sys::Array>().unwrap();
                for (i, ret) in returns.iter_mut().enumerate() {
                    set_ret(ret, array.get(i as u32));
                }
            }
        }
    }
}
