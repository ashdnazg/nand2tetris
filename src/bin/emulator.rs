// #![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

use std::ops::RangeInclusive;
use std::time::Instant;

use eframe::egui::{self, Key};
use eframe::emath::Rect;
use eframe::epaint::Vec2;

use egui_extras::TableBuilder;
use egui_extras::{Size, StripBuilder};

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
                        ivec2 coord = ivec2((v_pos + 1) * vec2(256.0, 128.0));
                        uint i_color = 1 - ((texelFetch(u_screen, coord / ivec2(8, 1) ,0).r >> (coord.x % 8)) & uint(1));
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

            let texture = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));

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

    fn paint(&self, gl: &glow::Context) {
        use glow::HasContext as _;
        unsafe {
            gl.use_program(Some(self.program));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "u_screen").as_ref(),
                0,
            );
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            gl.bind_texture(glow::TEXTURE_2D, None);
        }
    }
}

struct SharedState {
    desired_steps_per_second: u64,
    run_started: bool,
}

struct HardwareState {
    shared_state: SharedState,
    hardware: Hardware,
}

struct VMState {
    shared_state: SharedState,
    vm: VM,
}

enum AppState {
    Hardware(HardwareState),
    VM(VMState),
    Start,
}

trait CommonState {
    fn step(&mut self);
    fn shared_state(&self) -> &SharedState;
    fn shared_state_mut(&mut self) -> &mut SharedState;
    fn ram(&self) -> &RAM;
    fn ram_mut(&mut self) -> &mut RAM;
    fn reset(&mut self);
}

impl CommonState for HardwareState {
    fn step(&mut self) {
        self.hardware.step();
    }

    fn shared_state(&self) -> &SharedState {
        &self.shared_state
    }

    fn shared_state_mut(&mut self) -> &mut SharedState {
        &mut self.shared_state
    }

    fn ram(&self) -> &RAM {
        &self.hardware.ram
    }

    fn ram_mut(&mut self) -> &mut RAM {
        &mut self.hardware.ram
    }

    fn reset(&mut self) {
        self.hardware.reset();
    }
}

impl CommonState for VMState {
    fn step(&mut self) {
        self.vm.step();
    }

    fn shared_state(&self) -> &SharedState {
        &self.shared_state
    }

    fn shared_state_mut(&mut self) -> &mut SharedState {
        &mut self.shared_state
    }

    fn ram(&self) -> &RAM {
        &self.vm.ram
    }

    fn ram_mut(&mut self) -> &mut RAM {
        &mut self.vm.ram
    }

    fn reset(&mut self) {
        self.vm.reset();
    }
}

impl Default for AppState {
    fn default() -> Self {
        let mut hardware = Hardware::default();
        let program: [u16; 29] = [
            16384, 60432, 16, 58248, 17, 60040, 24576, 64528, 12, 58114, 17, 61064, 17, 64528, 16,
            65000, 58120, 24576, 60560, 16, 62672, 4, 58115, 16384, 60432, 16, 58248, 4, 60039,
        ];
        hardware.load_program(program.iter().map(|raw| Instruction::new(*raw)));

        let shared_state = SharedState {
            desired_steps_per_second: 1_000_000,
            run_started: false,
        };

        AppState::Hardware(HardwareState {
            shared_state,
            hardware,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CommonAction {
    StepClicked,
    RunClicked,
    PauseClicked,
    ResetClicked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Action {
    Common(CommonAction),
    Quit,
}

struct PerformanceData {
    steps_during_last_frame: u64,
    total_steps: u64,
    run_start: Option<Instant>,
}

pub struct EmulatorApp {
    performance_data: PerformanceData,
    state: AppState,
    screen: Arc<Mutex<Screen>>,
}

impl EmulatorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let performance_data = PerformanceData {
            steps_during_last_frame: 0,
            total_steps: 0,
            run_start: None,
        };
        Self {
            performance_data,
            state: Default::default(),
            screen: Arc::new(Mutex::new(Screen::new(&cc.gl))),
        }
    }
}

fn draw_screen(ui: &mut egui::Ui, screen: &Arc<Mutex<Screen>>, ram: &RAM, frame: &eframe::Frame) {
    let rect = Rect::from_min_size(ui.cursor().min, egui::Vec2::new(512.0, 256.0));

    // Clone locals so we can move them into the paint callback:
    let screen = screen.clone();
    let screen_buffer =
        &ram.contents[RAM::SCREEN as usize..(RAM::SCREEN + 256 * RAM::SCREEN_ROW_LENGTH) as usize];

    unsafe {
        use glow::HasContext as _;
        frame.gl().active_texture(glow::TEXTURE0);
        let guard = screen.lock();
        frame
            .gl()
            .bind_texture(glow::TEXTURE_2D, Some(guard.texture));
        frame.gl().tex_image_2d(
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
        frame.gl().bind_texture(glow::TEXTURE_2D, None);
    }

    let callback = egui::PaintCallback {
        rect,
        callback: std::sync::Arc::new(move |_info, render_ctx| {
            if let Some(painter) = render_ctx.downcast_ref::<egui_glow::Painter>() {
                screen.lock().paint(painter.gl());
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
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Step").clicked() {
                *action = Some(Action::Common(CommonAction::StepClicked));
            }
            if ui.button("Run").clicked() {
                *action = Some(Action::Common(CommonAction::RunClicked));
            }
            if ui.button("Pause").clicked() {
                *action = Some(Action::Common(CommonAction::PauseClicked));
            }
            if ui.button("Reset").clicked() {
                *action = Some(Action::Common(CommonAction::ResetClicked));
            }
        });
    });
}

fn draw_hardware(
    state: &HardwareState,
    ctx: &egui::Context,
    action: &mut Option<Action>,
    screen: &Arc<Mutex<Screen>>,
    frame: &eframe::Frame,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        StripBuilder::new(ui)
            .size(Size::remainder())
            .size(Size::exact(512.0))
            .horizontal(|mut strip| {
                strip.strip(|builder| {
                    builder
                        .size(Size::initial(140.0).at_least(140.0))
                        .size(Size::initial(110.0).at_least(110.0))
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.rom_grid(
                                    "ROM",
                                    &state.hardware.rom,
                                    0..=i16::MAX,
                                    state.hardware.pc,
                                );
                            });
                            strip.cell(|ui| {
                                ui.ram_grid("RAM", &state.hardware.ram, 0..=i16::MAX);
                            });
                        });
                });
                strip.cell(|ui| {
                    ui.allocate_ui(Vec2::new(512.0, 256.0), |ui| {
                        egui::Frame::canvas(ui.style()).show(ui, |ui| {
                            draw_screen(ui, screen, &state.hardware.ram, frame);
                        });
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
                                            ui.ram_grid("Static", &state.vm.ram, 0..=5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("Local", &state.vm.ram, 0..=5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("Argument", &state.vm.ram, 0..=5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("This", &state.vm.ram, 0..=5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("That", &state.vm.ram, 0..=5);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("Temp", &state.vm.ram, 0..=5);
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
                                            ui.ram_grid("Global Stack", &state.vm.ram, 256..=1024);
                                        });
                                        strip.cell(|ui| {
                                            ui.ram_grid("RAM", &state.vm.ram, 0..=i16::MAX);
                                        });
                                    });
                            });
                        });
                });
            });
    });
}

fn draw_start(ctx: &egui::Context, action: &mut Option<Action>) {}

fn reduce_common(state: &mut impl CommonState, action: &CommonAction) {
    match action {
        CommonAction::StepClicked => {}
        CommonAction::RunClicked => {
            state.shared_state_mut().run_started = true;
        }
        CommonAction::PauseClicked => {
            state.shared_state_mut().run_started = false;
        }
        CommonAction::ResetClicked => {
            state.reset();
            state.shared_state_mut().run_started = false;
        }
    }
}

fn reduce(state: &mut AppState, action: &Action) {
    match action {
        Action::Common(common_action) => match state {
            AppState::Hardware(hardware_state) => reduce_common(hardware_state, common_action),
            AppState::VM(vm_state) => reduce_common(vm_state, common_action),
            AppState::Start => panic!(
                "Received common action {:?} when in state AppState::Start",
                common_action
            ),
        },
        Action::Quit => todo!(),
    }
}

trait EmulatorWidgets {
    fn ram_grid(&mut self, caption: &str, ram: &RAM, range: RangeInclusive<i16>);
    fn rom_grid(
        &mut self,
        caption: &str,
        rom: &[Instruction; 32 * 1024],
        range: RangeInclusive<i16>,
        highlight_address: i16,
    );
}

impl EmulatorWidgets for egui::Ui {
    fn ram_grid(&mut self, caption: &str, ram: &RAM, range: RangeInclusive<i16>) {
        self.push_id(caption, |ui| {
            ui.label(caption);
            let header_height = ui.text_style_height(&egui::TextStyle::Body);
            let row_height = ui.text_style_height(&egui::TextStyle::Monospace);

            TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right().with_cross_align(egui::Align::Center))
                .column(Size::initial(45.0).at_least(45.0))
                .column(Size::remainder().at_least(40.0))
                .header(header_height, |mut header| {
                    header.col(|ui| {
                        ui.label("Address");
                    });
                    header.col(|ui| {
                        ui.label("Value");
                    });
                })
                .body(|body| {
                    body.rows(row_height, range.len(), |row_index, mut row| {
                        row.col(|ui| {
                            ui.monospace(row_index.to_string());
                        });
                        row.col(|ui| {
                            ui.monospace(ram[row_index as i16].to_string());
                        });
                    });
                });
        });
    }

    fn rom_grid(
        &mut self,
        caption: &str,
        rom: &[Instruction; 32 * 1024],
        range: RangeInclusive<i16>,
        highlight_address: i16,
    ) {
        self.push_id(caption, |ui| {
            ui.label(caption);
            let header_height = ui.text_style_height(&egui::TextStyle::Body);
            let row_height = ui.text_style_height(&egui::TextStyle::Monospace);

            TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right().with_cross_align(egui::Align::Center))
                .column(Size::initial(45.0).at_least(45.0))
                .column(Size::remainder().at_least(70.0))
                .header(header_height, |mut header| {
                    header.col(|ui| {
                        ui.label("Address");
                    });
                    header.col(|ui| {
                        ui.label("Instruction");
                    });
                })
                .body(|body| {
                    body.rows(row_height, range.len(), |row_index, mut row| {
                        row.col(|ui| {
                            if row_index == highlight_address as usize {
                                let rect = ui.max_rect();
                                let rect =
                                    rect.expand2(egui::vec2(ui.spacing().item_spacing.x, 0.0));

                                ui.painter()
                                    .rect_filled(rect, 0.0, ui.visuals().selection.bg_fill);
                            }
                            ui.monospace(row_index.to_string());
                        });
                        row.col(|ui| {
                            if row_index == highlight_address as usize {
                                let rect = ui.max_rect();
                                let rect =
                                    rect.expand2(egui::vec2(ui.spacing().item_spacing.x, 0.0));

                                ui.painter()
                                    .rect_filled(rect, 0.0, ui.visuals().selection.bg_fill);
                            }
                            ui.monospace(rom[row_index].to_string());
                        });
                    });
                });
        });
    }
}

fn steps_to_run(
    desired_steps_per_second: u64,
    last_frame_time: f32,
    performance_data: &mut PerformanceData,
    state: &impl CommonState,
    ctx: &egui::Context,
) -> u64 {
    if !state.shared_state().run_started {
        performance_data.run_start = None;
        performance_data.steps_during_last_frame = 0;
        performance_data.total_steps = 0;
        return 0;
    }

    let run_start = performance_data.run_start.get_or_insert(Instant::now());

    let run_time = (Instant::now() - *run_start).as_secs_f64();
    let wanted_steps = (desired_steps_per_second as f64 * run_time) as u64;
    let mut steps_to_run = wanted_steps - performance_data.total_steps;

    if performance_data.steps_during_last_frame > 0 {
        steps_to_run = u64::min(
            steps_to_run,
            ((performance_data.steps_during_last_frame as f64) / (last_frame_time as f64 * 60.0))
                as u64,
        );
    }

    performance_data.steps_during_last_frame = steps_to_run;
    performance_data.total_steps += steps_to_run;

    return steps_to_run;
}

fn run_steps(state: &mut impl CommonState, steps_to_run: u64, key_down: Option<Key>) {
    if steps_to_run > 0 {
        state.ram_mut().set_keyboard(0);
        if let Some(_) = key_down {
            state.ram_mut().set_keyboard(32);
        }

        for _ in 0..steps_to_run {
            state.step();
        }
    }
}

impl eframe::App for EmulatorApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let mut action = None;
        draw_shared(ctx, &mut action);

        let steps_to_run = if action == Some(Action::Common(CommonAction::StepClicked)) {
            1
        } else {
            match &self.state {
                AppState::Hardware(state) => steps_to_run(
                    state.shared_state.desired_steps_per_second,
                    frame.info().cpu_usage.unwrap_or(1.0 / 60.0),
                    &mut self.performance_data,
                    state,
                    ctx,
                ),
                AppState::VM(state) => steps_to_run(
                    state.shared_state.desired_steps_per_second,
                    frame.info().cpu_usage.unwrap_or(1.0 / 60.0),
                    &mut self.performance_data,
                    state,
                    ctx,
                ),
                _ => 0,
            }
        };

        let key_down = ctx.input().keys_down.iter().cloned().next();

        match &mut self.state {
            AppState::Hardware(state) => {
                run_steps(state, steps_to_run, key_down);
            }
            AppState::VM(state) => {
                run_steps(state, steps_to_run, key_down);
            }
            _ => {}
        }

        ctx.request_repaint();

        match &self.state {
            AppState::Hardware(state) => {
                draw_hardware(state, ctx, &mut action, &self.screen, &frame)
            }
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
