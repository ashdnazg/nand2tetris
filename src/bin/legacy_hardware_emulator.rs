use sdl2::event::Event;
use sdl2::keyboard::{Mod, Scancode};
use sdl2::pixels::Color;
use sdl2::rect::Point;
use std::time::Instant;

use nand2tetris::hardware::*;

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

fn run_hardware() -> Result<(), String> {
    let mut hardware = Hardware::default();

    let program: [u16; 29] = [
        16384, 60432, 16, 58248, 17, 60040, 24576, 64528, 12, 58114, 17, 61064, 17, 64528, 16,
        65000, 58120, 24576, 60560, 16, 62672, 4, 58115, 16384, 60432, 16, 58248, 4, 60039,
    ];
    hardware.load_program(program.iter().map(|raw| Instruction::new(*raw)));

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
                    if hardware.ram.get_pixel(x, y) {
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
                        hardware.ram.set_keyboard(keyboard_value);
                    }
                    Event::KeyUp { .. } => {
                        hardware.ram.set_keyboard(0);
                    }
                    _ => {}
                }
            }
            polling_time = (Instant::now() - pre_polling).as_secs_f64();
            pre_hardware = Instant::now();
        }
        for _ in 0..1000 {
            hardware.step();
        }
        steps_ran += 1000;
    }

    Ok(())
}

fn main() -> Result<(), String> {
    run_hardware()?;

    Ok(())
}
