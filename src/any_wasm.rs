use std::{borrow::Cow, cell::UnsafeCell};

#[cfg(not(target_arch = "wasm32"))]
use wasmtime::{Engine, Global, Instance, Linker, Memory, Module, Store, Func};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};

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

pub trait AnyWasmHandle: Sized + 'static {
    type Global: 'static;
    type Memory: 'static;
    type Function: 'static;

    fn from_binary(binary: &[u8], callback: impl FnOnce(Self) + 'static);

    fn get_global(&self, name: &str) -> Option<Self::Global>;

    fn get_memory(&self, name: &str) -> Option<Self::Memory>;

    fn get_function(&self, name: &str) -> Option<Self::Function>;

    fn get_global_value_i32(&self, global: &Self::Global) -> i32;

    fn set_global_value_i32(&self, global: &Self::Global, value: i32);

    fn get_memory_at(&self, memory: &Self::Memory, address: usize) -> i32;

    fn set_memory_at(&self, memory: &Self::Memory, address: usize, value: i32);

    fn raw_memory(&self, memory: &Self::Memory) -> Cow<'_, [i32]>;

    fn fill_memory(&self, memory: &Self::Memory, value: i32);

    fn call_function<const A: usize, const R: usize>(&self, function: &Self::Function, args: &[Val; A], returns: &mut [Val; R]);
}

#[cfg(not(target_arch = "wasm32"))]
pub struct WasmtimeHandle {
    store: UnsafeCell<Store<()>>,
    instance: Instance,
}

#[cfg(not(target_arch = "wasm32"))]
impl AnyWasmHandle for WasmtimeHandle {
    type Global = Global;
    type Memory = Memory;
    type Function = Func;

    fn from_binary(binary: &[u8], callback: impl FnOnce(Self)) {
        let engine = Engine::default();
        let module = Module::from_binary(&engine, binary).unwrap();
        let mut store = Store::new(&engine, ());
        let linker = Linker::new(&engine);
        let instance = linker.instantiate(&mut store, &module).unwrap();

        let handle = Self {
            store: UnsafeCell::new(store),
            instance,
        };

        callback(handle);
    }

    fn get_global(&self, name: &str) -> Option<Self::Global> {
        let store = unsafe { &mut *self.store.get() };

        self.instance.get_global(store, name)
    }

    fn get_memory(&self, name: &str) -> Option<Self::Memory> {
        let store = unsafe { &mut *self.store.get() };

        self.instance.get_memory(store, name)
    }

    fn get_function(&self, name: &str) -> Option<Self::Function> {
        let store = unsafe { &mut *self.store.get() };

        self.instance.get_func(store, name)
    }

    fn get_global_value_i32(&self, global: &Self::Global) -> i32 {
        let store = unsafe { &mut *self.store.get() };

        global.get(store).i32().unwrap()
    }

    fn set_global_value_i32(&self, global: &Self::Global, value: i32) {
        let store = unsafe { &mut *self.store.get() };

        global.set(store, value.into()).unwrap()
    }

    fn get_memory_at(&self, memory: &Self::Memory, address: usize) -> i32 {
        let store = unsafe { &mut *self.store.get() };

        let mut bytes = [0; 4];
        memory.read(store, address * 4, &mut bytes).unwrap();

        i32::from_le_bytes(bytes)
    }

    fn set_memory_at(&self, memory: &Self::Memory, address: usize, value: i32) {
        let store = unsafe { &mut *self.store.get() };

        memory.write(store, address * 4, &value.to_le_bytes()).unwrap();
    }

    fn fill_memory(&self, memory: &Self::Memory, value: i32) {
        let store = unsafe { &mut *self.store.get() };

        let data = memory.data_mut(store);
        let (_, ints, _) = unsafe { data.align_to_mut::<i32>() };

        ints[..crate::hardware::MEM_SIZE].fill(value);
    }

    fn raw_memory(&self, memory: &Self::Memory) -> Cow<'_, [i32]> {
        let store = unsafe { &mut *self.store.get() };

        let data = memory.data(&mut *store);
        let (_, ints, _) = unsafe { data.align_to::<i32>() };

        Cow::Borrowed(ints)
    }

    fn call_function<const A: usize, const R: usize>(&self, function: &Self::Function, args: &[Val; A], returns: &mut [Val; R]) {
        let store = unsafe { &mut *self.store.get() };

        let params = args.iter().map(|arg| match arg {
            &Val::I32(i) => i.into(),
            &Val::I64(i) => i.into(),
            &Val::F32(f) => f.into(),
            &Val::F64(f) => f.into(),
        }).collect::<Vec<_>>();

        let mut results = [wasmtime::Val::I32(0); R];

        function.call(store, &params, &mut results).unwrap();

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

        Some(js_sys::Reflect::get(&exports, &name.into()).ok()?.dyn_into::<T>().unwrap())
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
impl AnyWasmHandle for JsWasmHandle {
    type Global = Global;
    type Memory = Int32Array;
    type Function = Function;

    fn from_binary(binary: &[u8], callback: impl FnOnce(Self) + 'static) {
        let promise = WebAssembly::instantiate_buffer(binary, &Object::new());
        let future = wasm_bindgen_futures::JsFuture::from(promise);
        wasm_bindgen_futures::spawn_local(async move {
            let object = future.await.unwrap();
            let instance = js_sys::Reflect::get(&object, &"instance".into())
                .unwrap()
                .dyn_into::<Instance>()
                .unwrap();
            let handle = Self { instance };
            callback(handle);
        });
    }

    fn get_global(&self, name: &str) -> Option<Self::Global> {
        self.get_export(name)
    }

    fn get_memory(&self, name: &str) -> Option<Self::Memory> {
        use wasm_bindgen_futures::js_sys::WebAssembly::Memory;

        self.get_export::<Memory>(name).map(|mem| Int32Array::new_with_byte_offset_and_length(&mem.buffer(), 0, crate::hardware::MEM_SIZE as u32))
    }

    fn get_function(&self, name: &str) -> Option<Self::Function> {
        self.get_export(name)
    }

    fn get_global_value_i32(&self, global: &Self::Global) -> i32 {
        global.value().as_f64().unwrap() as i32
    }

    fn set_global_value_i32(&self, global: &Self::Global, value: i32) {
        global.set_value(&value.into());
    }

    fn get_memory_at(&self, memory: &Self::Memory, address: usize) -> i32 {
        memory.get_index(address as u32)
    }

    fn set_memory_at(&self, memory: &Self::Memory, address: usize, value: i32) {
        memory.set_index(address as u32, value);
    }

    fn raw_memory(&self, memory: &Self::Memory) -> Cow<'_, [i32]> {
        let mut dest = vec![0i32; crate::hardware::MEM_SIZE];
        memory.copy_to(&mut dest);

        Cow::Owned(dest)
    }

    fn fill_memory(&self, memory: &Self::Memory, value: i32) {
        memory.fill(value, 0, crate::hardware::MEM_SIZE as u32);
    }

    fn call_function<const A: usize, const R: usize>(&self, function: &Self::Function, args: &[Val; A], returns: &mut [Val; R]) {
        let ret = match args.as_slice() {
            [] => function.call0(&Object::new()).unwrap(),
            [arg1] => function.call1(&Object::new(), &arg1.into()).unwrap(),
            [arg1, arg2] => function.call2(&Object::new(), &arg1.into(), &arg2.into()).unwrap(),
            [arg1, arg2, arg3] => function.call3(&Object::new(), &arg1.into(), &arg2.into(), &arg3.into()).unwrap(),
            [arg1, arg2, arg3, arg4] => function.call4(&Object::new(), &arg1.into(), &arg2.into(), &arg3.into(), &arg4.into()).unwrap(),
            [arg1, arg2, arg3, arg4, arg5] => function.call5(&Object::new(), &arg1.into(), &arg2.into(), &arg3.into(), &arg4.into(), &arg5.into()).unwrap(),
            [arg1, arg2, arg3, arg4, arg5, arg6] => function.call6(&Object::new(), &arg1.into(), &arg2.into(), &arg3.into(), &arg4.into(), &arg5.into(), &arg6.into()).unwrap(),
            [arg1, arg2, arg3, arg4, arg5, arg6, arg7] => function.call7(&Object::new(), &arg1.into(), &arg2.into(), &arg3.into(), &arg4.into(), &arg5.into(), &arg6.into(), &arg7.into()).unwrap(),
            [arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8] => function.call8(&Object::new(), &arg1.into(), &arg2.into(), &arg3.into(), &arg4.into(), &arg5.into(), &arg6.into(), &arg7.into(), &arg8.into()).unwrap(),
            [arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9] => function.call9(&Object::new(), &arg1.into(), &arg2.into(), &arg3.into(), &arg4.into(), &arg5.into(), &arg6.into(), &arg7.into(), &arg8.into(), &arg9.into()).unwrap(),
            _ => panic!("Too many arguments to call_function: {A}"),
        };

        let set_ret = |ret: &mut Val, value: JsValue| {
            match ret {
                Val::I32(i) => *i = value.as_f64().unwrap() as i32,
                Val::I64(i) => *i = value.dyn_into::<BigInt>().unwrap().try_into().unwrap(),
                Val::F32(f) => *f = value.as_f64().unwrap() as f32,
                Val::F64(f) => *f = value.as_f64().unwrap() as f64,
            }
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
