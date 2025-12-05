use super::instant::Instant;
use crate::{
    hardware::{Instruction, RAM, Word},
    vm::{Program, RunState},
};
use core::slice;
use eframe::{
    egui::{self, Slider},
    epaint::Rect,
    glow::{self, PixelUnpackData},
};
use egui_extras::{Column, TableBuilder};
use futures::future::join_all;
use std::{future::Future, sync::mpsc::Sender};
use std::{ops::RangeInclusive, sync::Arc};

use super::common_state::{Action, CommonAction, PerformanceData, SharedState, UIStyle};

pub struct Screen {
    program: glow::Program,
    vertex_array: glow::VertexArray,
    texture: glow::Texture,
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
                    uniform lowp usampler2D u_screen;
                    in vec2 v_pos;
                    out vec4 out_color;
                    void main() {
                        ivec2 coord = ivec2((v_pos + 1.0) * vec2(256.0, 128.0));
                        uint i_color = uint(1) - ((texelFetch(u_screen, coord / ivec2(8, 1) ,0).r >> (coord.x % 8)) & uint(1));
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

pub fn draw_screen(ui: &mut egui::Ui, screen: &Arc<Screen>, ram: &RAM, frame: &eframe::Frame) {
    let rect = Rect::from_min_size(
        ui.cursor().min,
        egui::Vec2::new(ui.available_width(), ui.available_height()),
    );

    // Clone locals so we can move them into the paint callback:
    let screen = screen.clone();
    // let screen_ptr = ram.contents[RAM::SCREEN as usize..(RAM::SCREEN + 256 * RAM::SCREEN_ROW_LENGTH) as usize].as_ptr() as usize;
    let screen_vec = ram.contents
        [RAM::SCREEN as usize..(RAM::SCREEN + 256 * RAM::SCREEN_ROW_LENGTH) as usize]
        .to_vec();
    // let screen_slice = &ram.contents[RAM::SCREEN as usize..(RAM::SCREEN + 256 * RAM::SCREEN_ROW_LENGTH) as usize];

    unsafe {
        use glow::HasContext as _;
        let context = frame.gl().unwrap();
        context.active_texture(glow::TEXTURE0);
        context.bind_texture(glow::TEXTURE_2D, Some(screen.texture));
        context.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::R8UI as i32,
            64,
            256,
            0,
            glow::RED_INTEGER,
            glow::UNSIGNED_BYTE,
            // Some(screen_slice.align_to().1),
            PixelUnpackData::Slice(Some(screen_vec.align_to().1)),
            // Some(screen_vec.align_to().1),
            // Some(&*std::ptr::slice_from_raw_parts(screen_ptr as *const u8, 64*256))
        );
        context.bind_texture(glow::TEXTURE_2D, None);
    }

    let cb = eframe::egui_glow::CallbackFn::new(move |_info, painter| {
        // let context = painter.gl();
        // unsafe {
        //     use glow::HasContext as _;
        //     context.active_texture(glow::TEXTURE0);
        //     context.bind_texture(glow::TEXTURE_2D, Some(screen.texture));
        //     context.tex_image_2d(
        //         glow::TEXTURE_2D,
        //         0,
        //         glow::R8UI as i32,
        //         64,
        //         256,
        //         0,
        //         glow::RED_INTEGER,
        //         glow::UNSIGNED_BYTE,
        //         Some(screen_vec.align_to().1),
        //         // Some(std::slice::from_raw_parts(screen_ptr as *const u8, 64*256))
        //     );
        //     context.bind_texture(glow::TEXTURE_2D, None);
        // }

        screen.paint(painter.gl());
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
    is_file_loaded: bool,
    action: &mut Option<Action>,
    async_actions_sender: &Sender<Action>,
) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        // The top panel is often a good place for a menu bar:
        // #[cfg(not(target_arch = "wasm32"))]
        {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Load VM Files").clicked() {
                        ui.close();
                        let mut dialog = rfd::AsyncFileDialog::new();
                        if let Ok(current_dir) = std::env::current_dir() {
                            dialog = dialog.set_directory(current_dir);
                        }
                        let task = dialog.add_filter("VM", &[&"vm"]).pick_files();
                        let ctx = ctx.clone();
                        let async_actions_sender = async_actions_sender.clone();
                        execute(async move {
                            if let Some(files) = task.await {
                                let file_contents_futures: Vec<_> = files
                                    .iter()
                                    .map(|f| async {
                                        (f.file_name(), String::from_utf8(f.read().await).unwrap())
                                    })
                                    .collect();
                                let file_contents = join_all(file_contents_futures).await;
                                let _ =
                                    async_actions_sender.send(Action::FilesPicked(file_contents));
                                ctx.request_repaint();
                            }
                        });
                    }
                    if ui.button("Load Hack File").clicked() {
                        ui.close();
                        let mut dialog = rfd::AsyncFileDialog::new();
                        if let Ok(current_dir) = std::env::current_dir() {
                            dialog = dialog.set_directory(current_dir);
                        }
                        let task = dialog.add_filter("Hack", &[&"asm", &"hack"]).pick_file();
                        let ctx = ctx.clone();
                        let async_actions_sender = async_actions_sender.clone();
                        execute(async move {
                            if let Some(file) = task.await {
                                let contents = String::from_utf8(file.read().await).unwrap();
                                let name = file.file_name();
                                let _ = async_actions_sender
                                    .send(Action::FilePicked { name, contents });
                                ctx.request_repaint();
                            }
                        });
                    }

                    ui.add_enabled_ui(is_file_loaded, |ui| {
                        if ui.button("Close File(s)").clicked() {
                            ui.close();
                            *action = Some(Action::CloseFile)
                        }
                    });

                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.button("Quit").clicked() {
                        *action = Some(Action::Quit);
                    }
                });
            });
        }
        ui.separator();
        ui.add_enabled_ui(is_file_loaded, |ui| {
            ui.horizontal_wrapped(|ui| {
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
                // if ui.button("Breakpoints").clicked() {
                //     *action = Some(Action::Common(CommonAction::BreakpointsClicked));
                // }

                let mut new_steps_per_second = state.desired_steps_per_second;
                let height = ui.text_style_height(&egui::TextStyle::Body);
                ui.allocate_ui_with_layout(
                    [320.0, height].into(),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.label("Steps per second:");
                        ui.scope(|ui| {
                            ui.spacing_mut().interact_size.x = 100.0;
                            ui.add_sized(
                                [200.0, height],
                                Slider::new(&mut new_steps_per_second, 0..=10000000000)
                                    .logarithmic(true),
                            );
                        })
                    },
                );

                if is_file_loaded && new_steps_per_second != state.desired_steps_per_second {
                    *action = Some(Action::Common(CommonAction::SpeedSliderMoved(
                        new_steps_per_second,
                    )))
                }
                if let Some(run_start) = performance_data.run_start {
                    let run_time = (Instant::now() - run_start).as_secs_f64();
                    let steps_per_second = performance_data.total_steps as f64 / run_time;
                    ui.label("Actual:");
                    ui.label(
                        (steps_per_second.round() as u64)
                            .to_string()
                            .as_bytes()
                            .rchunks(3)
                            .rev()
                            .map(std::str::from_utf8)
                            .collect::<Result<Vec<&str>, _>>()
                            .unwrap()
                            .join(","),
                    );
                }
            });
        });
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    std::thread::spawn(move || futures::executor::block_on(f));
}

#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

pub trait EmulatorWidgets {
    fn ram_grid(
        &mut self,
        caption: &str,
        ram: &RAM,
        range: &RangeInclusive<Word>,
        style: UIStyle,
        highlight_address: Option<Word>,
        scroll_to_row: bool,
    );
    fn rom_grid(
        &mut self,
        caption: &str,
        rom: &[Instruction; 32 * 1024],
        range: &RangeInclusive<Word>,
        highlight_address: Word,
        scroll_to_row: bool,
    );
    fn vm_grid(
        &mut self,
        program: &Program,
        current_file_index: usize,
        current_command_index: usize,
        selected_file: &mut String,
        scroll_to_row: bool,
    );
}

impl EmulatorWidgets for egui::Ui {
    fn ram_grid(
        &mut self,
        caption: &str,
        ram: &RAM,
        range: &RangeInclusive<Word>,
        style: UIStyle,
        highlight_address: Option<Word>,
        scroll_to_address: bool,
    ) {
        self.push_id(caption, |ui| {
            ui.vertical(|ui| {
                ui.label(caption);
                let header_height = ui.text_style_height(&egui::TextStyle::Body);
                let row_height = ui.text_style_height(&egui::TextStyle::Monospace);

                let available_height = ui.available_height();
                let mut builder = TableBuilder::new(ui)
                    .animate_scrolling(false)
                    .auto_shrink(false)
                    .min_scrolled_height(header_height + row_height)
                    .max_scroll_height(available_height);

                if scroll_to_address && let Some(address) = highlight_address {
                    builder = builder.scroll_to_row((address - range.start()) as usize, None);
                }

                builder
                    .striped(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::initial(45.0).at_least(45.0))
                    .column(Column::remainder().at_least(40.0))
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
                        if range.end() < range.start() {
                            return;
                        }
                        body.rows(
                            row_height,
                            *range.end() as usize - *range.start() as usize,
                            |mut row| {
                                let row_index = row.index();
                                row.set_selected(
                                    highlight_address.map(|addr| addr as usize)
                                        == Some(row_index + *range.start() as usize),
                                );
                                row.col(|ui| {
                                    ui.monospace(row_index.to_string());
                                });
                                row.col(|ui| {
                                    ui.monospace(
                                        ram[row_index as Word + range.start()].to_string(),
                                    );
                                });
                            },
                        );
                    });
            });
        });
    }

    fn rom_grid(
        &mut self,
        caption: &str,
        rom: &[Instruction; 32 * 1024],
        range: &RangeInclusive<Word>,
        highlight_address: Word,
        scroll_to_address: bool,
    ) {
        self.push_id(caption, |ui| {
            ui.vertical(|ui| {
                ui.label(caption);
                let header_height = ui.text_style_height(&egui::TextStyle::Body);
                let row_height = ui.text_style_height(&egui::TextStyle::Monospace);

                let available_height = ui.available_height();
                let mut builder = TableBuilder::new(ui)
                    .animate_scrolling(false)
                    .min_scrolled_height(header_height + row_height)
                    .max_scroll_height(available_height);

                if scroll_to_address {
                    builder = builder.scroll_to_row(highlight_address as usize, None);
                }

                builder
                    .auto_shrink(false)
                    .striped(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::initial(45.0).at_least(45.0))
                    .column(Column::remainder().at_least(70.0))
                    .header(header_height, |mut header| {
                        header.col(|ui| {
                            ui.label("Address");
                        });
                        header.col(|ui| {
                            ui.label("Instruction");
                        });
                    })
                    .body(|body| {
                        body.rows(
                            row_height,
                            *range.end() as usize - *range.start() as usize,
                            |mut row| {
                                let row_index = row.index();
                                row.set_selected(row_index == highlight_address as usize);
                                row.col(|ui| {
                                    ui.monospace(row_index.to_string());
                                });
                                row.col(|ui| {
                                    ui.monospace(rom[row_index].to_string());
                                });
                            },
                        );
                    });
            });
        });
    }

    fn vm_grid(
        &mut self,
        program: &Program,
        current_file_index: usize,
        current_command_index: usize,
        selected_file: &mut String,
        scroll_to_row: bool,
    ) {
        self.push_id("VM", |ui| {
            ui.vertical(|ui| {
                if scroll_to_row {
                    selected_file.clone_from(&program.files[current_file_index].name);
                }
                egui::ComboBox::from_id_salt("VM combo")
                    .selected_text(&*selected_file)
                    .show_ui(ui, |ui| {
                        for file_name in program.files.iter().map(|f| &f.name) {
                            ui.selectable_value(selected_file, file_name.clone(), file_name);
                        }
                    });
                let header_height = ui.text_style_height(&egui::TextStyle::Body);
                let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
                let file_index = program.file_name_to_index[selected_file];
                let file = &program.files[file_index];
                let commands = file.commands(&program.all_commands);

                let available_height = ui.available_height();
                let mut builder = TableBuilder::new(ui)
                    .animate_scrolling(false)
                    .min_scrolled_height(header_height + row_height)
                    .max_scroll_height(available_height);

                if scroll_to_row {
                    builder = builder.scroll_to_row(
                        current_command_index - file.starting_command_index,
                        None,
                    );
                }

                builder
                    .auto_shrink(false)
                    .striped(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::initial(45.0).at_least(45.0))
                    .column(Column::remainder().at_least(70.0))
                    .header(header_height, |mut header| {
                        header.col(|ui| {
                            ui.label("Line");
                        });
                        header.col(|ui| {
                            ui.label("Command");
                        });
                    })
                    .body(|body| {
                        body.rows(row_height, commands.len(), |mut row| {
                            let row_index = row.index();
                            let is_highlighted = file_index == current_file_index
                                && row_index
                                    == current_command_index
                                        - file.starting_command_index;
                            row.set_selected(is_highlighted);
                            row.col(|ui| {
                                ui.monospace(row_index.to_string());
                            });
                            row.col(|ui| {
                                ui.monospace(commands[row_index].to_string());
                            });
                        });
                    });
            });
        });
    }
}
