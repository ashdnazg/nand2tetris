use std::cell::UnsafeCell;

use wasmtime::{Engine, Global, Instance, Linker, Memory, Module, Store, Func};

trait AnyWasmHandle {
    type Global: 'static;
    type Memory: 'static;
    type Function: 'static;

    fn from_binary(binary: &[u8]) -> Self;

    fn is_ready(&self) -> bool;

    fn get_global(&self, name: &str) -> Option<Self::Global>;

    fn get_memory(&self, name: &str) -> Option<Self::Memory>;

    fn get_function(&self, name: &str) -> Option<Self::Function>;

    fn get_global_value_i32(&self, global: &Self::Global) -> i32;

    fn set_global_value_i32(&self, global: &Self::Global, value: i32);

    fn get_memory_at(&self, memory: &Self::Memory, address: u32) -> i32;

    fn set_memory_at(&self, memory: &Self::Memory, address: u32, value: i32);
}

struct WasmtimeHandle {
    store: UnsafeCell<Store<()>>,
    instance: Instance,
}

impl AnyWasmHandle for WasmtimeHandle {
    type Global = Global;
    type Memory = Memory;
    type Function = Func;

    fn from_binary(binary: &[u8]) -> Self {
        let engine = Engine::default();
        let module = Module::from_binary(&engine, binary).unwrap();
        let mut store = Store::new(&engine, ());
        let linker = Linker::new(&engine);
        let instance = linker.instantiate(&mut store, &module).unwrap();

        Self {
            store: UnsafeCell::new(store),
            instance,
        }
    }

    fn is_ready(&self) -> bool {
        true
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

    fn get_memory_at(&self, memory: &Self::Memory, address: u32) -> i32 {
        let store = unsafe { &mut *self.store.get() };

        let mut bytes = [0; 4];
        memory.read(store, address as usize * 4, &mut bytes).unwrap();

        i32::from_le_bytes(bytes)
    }

    fn set_memory_at(&self, memory: &Self::Memory, address: u32, value: i32) {
        let store = unsafe { &mut *self.store.get() };

        memory.write(store, address as usize * 4, &value.to_le_bytes()).unwrap();
    }
}
