pub(crate) mod characters;

pub mod hack_to_wasm;
pub mod hardware;
pub mod hardware_parse;
mod os;
pub(crate) mod parse_utils;
pub mod vm;
pub mod vm_parse;

pub mod any_wasm;

pub mod vm_to_wasm;
pub mod wasm_hardware;
// #[cfg(target_arch = "wasm32")]
// pub mod wasm_vm;
// #[cfg(not(target_arch = "wasm32"))]
pub mod wasm_utils;
pub mod wasm_vm;

#[cfg(feature = "emulator")]
pub mod emulator;
