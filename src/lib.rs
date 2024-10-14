pub(crate) mod characters;
pub mod hardware;
pub mod hardware_parse;
mod os;
pub(crate) mod parse_utils;
pub mod vm;
pub mod vm_parse;

#[cfg(feature = "emulator")]
pub mod emulator;

#[cfg(all(target_arch = "wasm32", feature = "emulator"))]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(|cc| Box::new(emulator::EmulatorApp::new(cc))),
            )
            .await
            .expect("failed to start eframe");
    });
}
