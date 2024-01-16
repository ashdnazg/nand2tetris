#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size(eframe::epaint::Vec2::new(1600.0, 1200.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Emulator",
        native_options,
        Box::new(|cc| {
            cc.egui_ctx.set_pixels_per_point(1.0);
            cc.egui_ctx.set_visuals(eframe::egui::Visuals::dark());
            Box::new(nand2tetris::emulator::EmulatorApp::new(cc))
        }),
    )
    .unwrap();
}
