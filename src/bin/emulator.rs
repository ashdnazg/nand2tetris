// #![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

use std::ops::Range;

use eframe::egui;
use eframe::emath::Rect;
use eframe::epaint::Color32;
use eframe::epaint::Vec2;

use egui_extras::{Size, StripBuilder};

use glow::NativeTexture;
use glow::PixelUnpackData;
use nand2tetris::hardware::*;
use nand2tetris::vm::*;

use egui::mutex::Mutex;
use std::sync::Arc;

struct Screen {
    program: glow::Program,
    vertex_array: glow::VertexArray,
    texture: glow::NativeTexture,
}

impl Screen {
    fn new(gl: &glow::Context) -> Self {
        use glow::HasContext as _;

        let shader_version = if cfg!(target_arch = "wasm32") {
            "#version 300 es"
        } else {
            "#version 410"
        };

        unsafe {
            let program = gl.create_program().expect("Cannot create program");

            let (vertex_shader_source, fragment_shader_source) = (
                r#"
                    const vec2 verts[4] = vec2[4](
                        vec2(1.0, 1.0),
                        vec2(-1.0, 1.0),
                        vec2(1.0, -1.0),
                        vec2(-1.0, -1.0)
                    );

                    out vec2 v_pos;
                    void main() {
                        gl_Position = vec4(verts[gl_VertexID], 0.0, 1.0);
                        v_pos = gl_Position.xy * vec2(1.0, -1.0);
                    }
                "#,
                r#"
                    precision mediump float;
                    uniform usampler2D u_screen;
                    in vec2 v_pos;
                    out vec4 out_color;
                    void main() {
                        ivec2 coord = ivec2((v_pos + 1) * vec2(128.0, 128.0));
                        uint i_color = (texelFetch(u_screen, coord / ivec2(8, 1) ,0).r >> (coord.x % 8)) & 1;
                        out_color = vec4(vec3(i_color), 1.0);
                    }
                "#,
            );

            let shader_sources = [
                (glow::VERTEX_SHADER, vertex_shader_source),
                (glow::FRAGMENT_SHADER, fragment_shader_source),
            ];

            let shaders: Vec<_> = shader_sources
                .iter()
                .map(|(shader_type, shader_source)| {
                    let shader = gl
                        .create_shader(*shader_type)
                        .expect("Cannot create shader");
                    gl.shader_source(shader, &format!("{}\n{}", shader_version, shader_source));
                    gl.compile_shader(shader);
                    if !gl.get_shader_compile_status(shader) {
                        panic!("{}", gl.get_shader_info_log(shader));
                    }
                    gl.attach_shader(program, shader);
                    shader
                })
                .collect();

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("{}", gl.get_program_info_log(program));
            }

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            let vertex_array = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");

            let mut buffer = vec![0u8; 64 * 256];
            for i in 0..256 {
                for j in 0..64 {
                    buffer[i * 64 + j] = (i & j) as u8;
                }
            }
            let texture = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::R8UI as i32,
                64,
                256,
                0,
                glow::RED_INTEGER,
                glow::UNSIGNED_BYTE,
                Some(&buffer),
            );
            println!("{}", gl.get_error());

            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
            gl.bind_texture(glow::TEXTURE_2D, None);

            Self {
                program,
                vertex_array,
                texture,
            }
        }
    }

    fn destroy(&self, gl: &glow::Context) {
        use glow::HasContext as _;
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vertex_array);
            gl.delete_texture(self.texture);
        }
    }

    fn paint(&self, gl: &glow::Context, screen_buffer: &[i16]) {
        use glow::HasContext as _;
        unsafe {
            // println!("start");
            // println!("{}", gl.get_error());
            // gl.clear_color(0.0, 0.0, 0.0, 1.0);
            // // println!("{}", gl.get_error());
            // gl.clear(glow::COLOR_BUFFER_BIT);
            // println!("{}", gl.get_error());
            gl.use_program(Some(self.program));
            // println!("{}", gl.get_error());
            gl.active_texture(glow::TEXTURE0);
            // println!("{}", gl.get_error());
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            // println!("{}", gl.get_error());
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::R8UI as i32,
                64,
                256,
                0,
                glow::RED_INTEGER,
                glow::UNSIGNED_BYTE,
                Some(screen_buffer.align_to::<u8>().1),
            );
            // println!("{}", gl.get_error());
            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "u_screen").as_ref(),
                0,
            );
            // println!("{}", gl.get_error());
            gl.bind_vertex_array(Some(self.vertex_array));
            // println!("{}", gl.get_error());
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            // println!("{}", gl.get_error());
            // println!("end");
        }
    }
}

struct HardwareState {
    hardware: Hardware,
}

struct VMState {
    vm: VM,
}

enum AppState {
    Hardware(HardwareState),
    VM(VMState),
    Start,
}

impl Default for AppState {
    fn default() -> Self {
        let mut hardware = Hardware::default();
        let program: [u16; 29] = [
            16384, 60432, 16, 58248, 17, 60040, 24576, 64528, 12, 58114, 17, 61064, 17, 64528, 16,
            65000, 58120, 24576, 60560, 16, 62672, 4, 58115, 16384, 60432, 16, 58248, 4, 60039,
        ];
        hardware.load_program(program.iter().map(|raw| Instruction::new(*raw)));

        AppState::Hardware(HardwareState { hardware: hardware })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Action {
    Quit,
}

pub struct EmulatorApp {
    state: AppState,
    screen: Arc<Mutex<Screen>>,
}

impl EmulatorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            state: Default::default(),
            screen: Arc::new(Mutex::new(Screen::new(&cc.gl))),
        }
    }
}

fn draw_screen(ui: &mut egui::Ui, screen: &Arc<Mutex<Screen>>, ram: &RAM) {
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(512.0, 256.0), egui::Sense::drag());

    // Clone locals so we can move them into the paint callback:
    let screen = screen.clone();
    let screen_buffer: Vec<i16> = ram.contents
        [RAM::SCREEN as usize..(RAM::SCREEN + 256 * RAM::SCREEN_ROW_LENGTH) as usize]
        .into();

    let callback = egui::PaintCallback {
        rect,
        callback: std::sync::Arc::new(move |_info, render_ctx| {
            if let Some(painter) = render_ctx.downcast_ref::<egui_glow::Painter>() {
                screen.lock().paint(painter.gl(), &screen_buffer);
            } else {
                eprintln!("Can't do custom painting because we are not using a glow context");
            }
        }),
    };
    ui.painter().add(callback);
}

fn draw_shared(ctx: &egui::Context, action: &mut Option<Action>) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        // The top panel is often a good place for a menu bar:
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Quit").clicked() {
                    *action = Some(Action::Quit);
                }
            });
        });
    });
}

fn draw_hardware(
    state: &HardwareState,
    ctx: &egui::Context,
    action: &mut Option<Action>,
    screen: &Arc<Mutex<Screen>>,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        StripBuilder::new(ui)
            .size(Size::relative(0.5))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.strip(|builder| {
                    builder
                        .size(Size::relative(0.5))
                        .size(Size::remainder())
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.rom_grid("ROM", &state.hardware.rom, 0..i16::MAX, 1);
                            });
                            strip.cell(|ui| {
                                ui.ram_grid("RAM", &state.hardware.ram, 0..i16::MAX);
                            });
                        });
                });
                strip.cell(|ui| {
                    egui::Frame::canvas(ui.style()).show(ui, |ui| {
                        draw_screen(ui, screen, &state.hardware.ram);
                    });
                });
            });
    });
}

fn draw_vm(state: &VMState, ctx: &egui::Context, action: &mut Option<Action>) {
    egui::CentralPanel::default().show(ctx, |ui| {
        StripBuilder::new(ui)
            .size(Size::relative(0.5))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.strip(|builder| {
                    builder
                        .size(Size::relative(0.5))
                        .size(Size::remainder())
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {});
                            strip.strip(|builder| {
                                builder.sizes(Size::relative(1.0 / 6.0), 6).vertical(
                                    |mut strip| {
                                        strip.cell(|ui| {
                                            ui.ram_grid("Static", &state.vm.ram, 0..5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("Local", &state.vm.ram, 0..5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("Argument", &state.vm.ram, 0..5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("This", &state.vm.ram, 0..5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("That", &state.vm.ram, 0..5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("Temp", &state.vm.ram, 0..5);
                                        });
                                    },
                                );
                            });
                        });
                });

                strip.strip(|builder| {
                    builder
                        .size(Size::relative(0.5))
                        .size(Size::remainder())
                        .vertical(|mut strip| {
                            strip.cell(|ui| {});
                            strip.strip(|builder| {
                                builder
                                    .size(Size::relative(0.5))
                                    .size(Size::remainder())
                                    .horizontal(|mut strip| {
                                        strip.cell(|ui| {
                                            ui.ram_grid("Global Stack", &state.vm.ram, 256..1024);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("RAM", &state.vm.ram, 0..i16::MAX);
                                        });
                                    });
                            });
                        });
                });
            });
    });
}

fn draw_start(ctx: &egui::Context, action: &mut Option<Action>) {}

fn reduce(state: &mut AppState, action: &Action) {
    match action {
        _ => {}
    }
}

trait EmulatorWidgets {
    fn ram_grid(&mut self, caption: &str, ram: &RAM, range: Range<i16>);
    fn rom_grid(
        &mut self,
        caption: &str,
        rom: &[Instruction; 32 * 1024],
        range: Range<i16>,
        highlight_address: i16,
    );
}

impl EmulatorWidgets for egui::Ui {
    fn ram_grid(&mut self, caption: &str, ram: &RAM, range: Range<i16>) {
        self.vertical(|ui| {
            ui.label(caption);
            let text_style = egui::TextStyle::Body;
            let row_height = ui.text_style_height(&text_style);
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .id_source(&caption)
                .show_rows(ui, row_height, range.len() + 1, |ui, row_range| {
                    egui::Grid::new(&caption)
                        .num_columns(2)
                        .striped(true)
                        .show(ui, |ui| {
                            for i in row_range {
                                let address = i as i16 + range.start;
                                ui.label(address.to_string());
                                ui.label(ram[address].to_string());
                                ui.end_row();
                            }
                        });
                });
        });
    }

    fn rom_grid(
        &mut self,
        caption: &str,
        rom: &[Instruction; 32 * 1024],
        range: Range<i16>,
        highlight_address: i16,
    ) {
        self.vertical(|ui| {
            ui.label(caption);
            let text_style = egui::TextStyle::Body;
            let row_height = ui.text_style_height(&text_style);
            let width = ui.available_width();
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .id_source(&caption)
                .show_rows(ui, row_height, range.len() + 1, |ui, row_range| {
                    egui::Grid::new(&caption)
                        .num_columns(2)
                        .striped(true)
                        .show(ui, |ui| {
                            for i in row_range {
                                let address = i as i16 + range.start;
                                if highlight_address == address {
                                    let size = Vec2::new(width, row_height);
                                    let rect = Rect::from_min_size(ui.cursor().min, size);
                                    // let rect = rect.expand2(0.5 * ui.spacing(). * Vec2::Y);
                                    let rect = rect.expand2(2.0 * Vec2::X); // HACK: just looks better with some spacing on the sides

                                    ui.painter().rect_filled(rect, 2.0, Color32::YELLOW);
                                }
                                ui.label(address.to_string());
                                ui.label(rom[address as usize].to_string());
                                ui.end_row();
                            }
                        });
                });
        });
    }
}

impl eframe::App for EmulatorApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui
        let mut action = None;
        draw_shared(ctx, &mut action);

        match &mut self.state {
            AppState::Hardware(state) => {
                state.hardware.ram.set_keyboard(0);
                ctx.input().keys_down.iter().take(1).for_each(|_| {
                    state.hardware.ram.set_keyboard(32);
                });
                for _ in 0..1000 {
                    state.hardware.step();
                }
            },
            _ => {},
        };
        ctx.request_repaint();

        match &self.state {
            AppState::Hardware(state) => draw_hardware(state, ctx, &mut action, &self.screen),
            AppState::VM(state) => draw_vm(state, ctx, &mut action),
            AppState::Start => draw_start(ctx, &mut action),
        };

        if action == Some(Action::Quit) {
            frame.quit();
            return;
        }

        if let Some(action) = action {
            reduce(&mut self.state, &action);
        }
    }

    fn on_exit(&mut self, gl: &glow::Context) {
        self.screen.lock().destroy(gl);
    }
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let mut native_options = eframe::NativeOptions::default();
    native_options.initial_window_size = Some(Vec2::new(1600.0, 1200.0));
    eframe::run_native(
        "Emulator",
        native_options,
        Box::new(|cc| {
            cc.egui_ctx.set_pixels_per_point(2.0);
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Box::new(EmulatorApp::new(&cc))
        }),
    );
}
