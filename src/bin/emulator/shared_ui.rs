use egui::mutex::Mutex;
use std::{ops::RangeInclusive, sync::Arc, time::Instant};

use eframe::{
    egui::{self, Slider},
    epaint::Rect,
};
use egui_extras::{Size, TableBuilder};
use nand2tetris::hardware::{Instruction, RAM};

use crate::common_state::{Action, CommonAction, PerformanceData, SharedState, UIStyle};

pub struct Screen {
    program: glow::Program,
    vertex_array: glow::VertexArray,
    texture: glow::NativeTexture,
}

impl Screen {
    pub fn new(gl: &glow::Context) -> Self {
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

    pub fn destroy(&self, gl: &glow::Context) {
        use glow::HasContext as _;
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vertex_array);
            gl.delete_texture(self.texture);
        }
    }

    pub fn paint(&self, gl: &glow::Context) {
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

pub fn draw_screen(
    ui: &mut egui::Ui,
    screen: &Arc<Mutex<Screen>>,
    ram: &RAM,
    frame: &eframe::Frame,
) {
    let rect = Rect::from_min_size(ui.cursor().min, egui::Vec2::new(512.0, 256.0));

    // Clone locals so we can move them into the paint callback:
    let screen = screen.clone();
    let screen_buffer =
        &ram.contents[RAM::SCREEN as usize..(RAM::SCREEN + 256 * RAM::SCREEN_ROW_LENGTH) as usize];

    unsafe {
        use glow::HasContext as _;
        let context = frame.gl().unwrap();

        context.active_texture(glow::TEXTURE0);
        let guard = screen.lock();
        context.bind_texture(glow::TEXTURE_2D, Some(guard.texture));
        context.tex_image_2d(
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
        context.bind_texture(glow::TEXTURE_2D, None);
    }

    let cb = egui_glow::CallbackFn::new(move |_info, painter| {
        screen.lock().paint(painter.gl());
    });

    let callback = egui::PaintCallback {
        rect,
        callback: Arc::new(cb),
    };
    ui.painter().add(callback);
}

pub fn draw_shared(
    state: &SharedState,
    ctx: &egui::Context,
    performance_data: &PerformanceData,
    is_top_bar_enabled: bool,
    action: &mut Option<Action>,
) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        // The top panel is often a good place for a menu bar:
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Load VM Folder").clicked() {
                    let mut dialog = rfd::FileDialog::new();
                    if let Ok(current_dir) = std::env::current_dir() {
                        dialog = dialog.set_directory(current_dir);
                    }
                    if let Some(path) = dialog.pick_folder() {
                        *action = Some(Action::FolderPicked(path));
                        ui.close_menu();
                    }
                }
                if ui.button("Load Hack File").clicked() {
                    let mut dialog = rfd::FileDialog::new();
                    if let Ok(current_dir) = std::env::current_dir() {
                        dialog = dialog.set_directory(current_dir);
                    }
                    if let Some(path) = dialog.add_filter("Hack", &[&"asm"]).pick_file() {
                        *action = Some(Action::FilePicked(path));
                        ui.close_menu();
                    }
                }
                if ui.button("Quit").clicked() {
                    *action = Some(Action::Quit);
                }
            });
        });
        ui.separator();
        ui.add_enabled_ui(is_top_bar_enabled, |ui| {
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
                if ui.button("Breakpoints").clicked() {
                    *action = Some(Action::Common(CommonAction::BreakpointsClicked));
                }

                let mut new_steps_per_second = state.desired_steps_per_second;
                ui.vertical(|ui| {
                    // let height = ui.text_style_height(&egui::TextStyle::Body);
                    let old_size = ui.spacing_mut().interact_size.x;
                    ui.spacing_mut().interact_size.x = 100.0;
                    ui.add(
                        Slider::new(&mut new_steps_per_second, 0..=1000000000).logarithmic(true),
                    );
                    ui.spacing_mut().interact_size.x = old_size;
                });
                if new_steps_per_second != state.desired_steps_per_second {
                    *action = Some(Action::Common(CommonAction::SpeedSliderMoved(
                        new_steps_per_second,
                    )))
                }
                if let Some(run_start) = performance_data.run_start {
                    let run_time = (Instant::now() - run_start).as_secs_f64();
                    let steps_per_second = performance_data.total_steps as f64 / run_time;
                    ui.label((steps_per_second.round() as u64).to_string());
                }
            });
        });
    });
}

pub trait EmulatorWidgets {
    fn ram_grid(&mut self, caption: &str, ram: &RAM, range: &RangeInclusive<i16>, style: UIStyle);
    fn rom_grid(
        &mut self,
        caption: &str,
        rom: &[Instruction; 32 * 1024],
        range: &RangeInclusive<i16>,
        highlight_address: i16,
    );
}

impl EmulatorWidgets for egui::Ui {
    fn ram_grid(&mut self, caption: &str, ram: &RAM, range: &RangeInclusive<i16>, style: UIStyle) {
        self.push_id(caption, |ui| {
            ui.label(caption);
            let header_height = ui.text_style_height(&egui::TextStyle::Body);
            let row_height = ui.text_style_height(&egui::TextStyle::Monospace);

            TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Size::initial(45.0).at_least(45.0))
                .column(Size::remainder().at_least(40.0))
                .header(header_height, |mut header| {
                    if style == UIStyle::Hardware {
                        header.col(|ui| {
                            ui.label("Address");
                        });
                        header.col(|ui| {
                            ui.label("Value");
                        });
                    }
                })
                .body(|body| {
                    body.rows(row_height, range.len(), |row_index, mut row| {
                        row.col(|ui| {
                            ui.monospace(row_index.to_string());
                        });
                        row.col(|ui| {
                            ui.monospace(ram[row_index as i16 + range.start()].to_string());
                        });
                    });
                });
        });
    }

    fn rom_grid(
        &mut self,
        caption: &str,
        rom: &[Instruction; 32 * 1024],
        range: &RangeInclusive<i16>,
        highlight_address: i16,
    ) {
        self.push_id(caption, |ui| {
            ui.label(caption);
            let header_height = ui.text_style_height(&egui::TextStyle::Body);
            let row_height = ui.text_style_height(&egui::TextStyle::Monospace);

            TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
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
