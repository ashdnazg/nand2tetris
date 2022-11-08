use sdl2::event::Event;
use sdl2::keyboard::{Mod, Scancode};
use sdl2::pixels::Color;
use sdl2::rect::Point;
use std::fs;
use std::time::Instant;

use nand2tetris::vm::*;
use nand2tetris::vm_parse::*;

fn keyboard_value_from_scancode(scancode: Scancode, keymod: Mod) -> i16 {
    match scancode {
        Scancode::Space => 32,
        Scancode::Return => 128,
        Scancode::Backspace => 129,
        Scancode::Left => 130,
        Scancode::Up => 131,
        Scancode::Right => 132,
        Scancode::Down => 133,
        Scancode::Home => 134,
        Scancode::End => 135,
        Scancode::PageUp => 136,
        Scancode::PageDown => 137,
        Scancode::Insert => 138,
        Scancode::Delete => 139,
        Scancode::Escape => 140,
        Scancode::F1 => 141,
        Scancode::F2 => 142,
        Scancode::F3 => 143,
        Scancode::F4 => 144,
        Scancode::F5 => 145,
        Scancode::F6 => 146,
        Scancode::F7 => 147,
        Scancode::F8 => 148,
        Scancode::F9 => 149,
        Scancode::F10 => 150,
        Scancode::F11 => 151,
        Scancode::F12 => 152,
        _ => {
            let name = scancode.name();
            if name.len() == 1 && name.is_ascii() {
                let mut value = name.as_bytes()[0];
                if !keymod.contains(Mod::LSHIFTMOD) && !keymod.contains(Mod::RSHIFTMOD) {
                    value.make_ascii_lowercase();
                }
                value as i16
            } else {
                0
            }
        }
    }
}

fn run_vm() -> Result<(), String> {
    let paths = fs::read_dir("../hackenstein3DVM").unwrap();
    let files: Vec<(String, File)> = paths
        .map(|path| path.unwrap())
        .filter(|path| path.file_name().to_str().unwrap().ends_with(".vm"))
        .map(|path| {
            (
                path.file_name()
                    .to_str()
                    .unwrap()
                    .split_once('.')
                    .unwrap()
                    .0
                    .to_owned(),
                File::new(
                    commands(fs::read_to_string(path.path()).unwrap().as_str())
                        .unwrap()
                        .1,
                ),
            )
        })
        .collect();

    let mut vm = VM::new(files);

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("rust-sdl2 demo: Video", 512, 256)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    let mut event_pump = sdl_context.event_pump()?;

    let mut last_frame_time = Instant::now();
    let mut steps_ran = 0;
    let mut points: Vec<Point> = vec![];
    let mut present_time = 0f64;
    let mut polling_time = 0f64;
    let mut pre_hardware = Instant::now();
    let mut num_events = 0;
    'running: loop {
        let current_time = Instant::now();
        if (current_time - last_frame_time).as_secs_f64() * 60.0 > 1.0 {
            let hardware_time = (Instant::now() - pre_hardware).as_secs_f64();
            println!("steps_ran: {}, present_time: {}, polling_time: {}, hardware_time: {}, num_events: {}", steps_ran, present_time, polling_time, hardware_time, num_events);
            steps_ran = 0;
            last_frame_time = current_time;
            canvas.set_draw_color(Color::RGB(255, 255, 255));
            canvas.clear();

            canvas.set_draw_color(Color::RGB(0, 0, 0));
            points.clear();
            for x in 0..512 {
                for y in 0..256 {
                    if vm.run_state.ram.get_pixel(x, y) {
                        points.push(Point::new(x as i32, y as i32))
                    }
                }
            }
            if !points.is_empty() {
                canvas.draw_points(points.as_slice())?;
            }
            let pre_present = Instant::now();
            canvas.present();
            present_time = (Instant::now() - pre_present).as_secs_f64();
            num_events = 0;
            let iter = event_pump.poll_iter();
            let pre_polling = Instant::now();
            for event in iter {
                num_events += 1;
                match event {
                    Event::Quit { .. } => break 'running,
                    Event::KeyDown {
                        scancode: Some(scancode),
                        keymod,
                        ..
                    } => {
                        let keyboard_value = keyboard_value_from_scancode(scancode, keymod);
                        vm.run_state.ram.set_keyboard(keyboard_value);
                    }
                    Event::KeyUp { .. } => {
                        vm.run_state.ram.set_keyboard(0);
                    }
                    _ => {}
                }
            }
            polling_time = (Instant::now() - pre_polling).as_secs_f64();
            pre_hardware = Instant::now();
        }

        for _ in 0..1000 {
            vm.step();
        }
        steps_ran += 1000;
    }

    Ok(())
}

fn main() -> Result<(), String> {
    run_vm()?;

    Ok(())
}
