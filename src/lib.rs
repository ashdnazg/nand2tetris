pub(crate) mod characters;

#[cfg(target_arch = "wasm32")]
pub mod hack_to_wasm;
pub mod hardware;
pub mod hardware_parse;
mod os;
pub(crate) mod parse_utils;
pub mod vm;
pub mod vm_parse;

pub mod any_wasm;

pub mod vm_to_wasm;
#[cfg(target_arch = "wasm32")]
pub mod wasm_hardware;
#[cfg(target_arch = "wasm32")]
pub mod wasm_vm;
#[cfg(not(target_arch = "wasm32"))]
pub mod wasm_vm2;
pub mod wasm_utils;

#[cfg(feature = "emulator")]
pub mod emulator;
