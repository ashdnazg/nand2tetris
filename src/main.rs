extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::{Scancode, Mod};
use sdl2::pixels::Color;
use std::time::{Duration, Instant};
use sdl2::rect::Point;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Instruction {
    raw: u16,
}

enum YRegister {
    A,
    M,
}

enum Operator {
    And,
    Add,
}

#[derive(PartialEq)]
enum InstructionType {
    A,
    C,
}

enum JumpCondition {
    NoJump,
    JGT,
    JEQ,
    JGE,
    JLT,
    JNE,
    JLE,
    JMP,
}

impl Instruction {
    fn flag(&self, pos: i32) -> bool {
        self.raw & (1 << pos) != 0
    }

    fn instruction_type(&self) -> InstructionType {
        if self.flag(15) {
            InstructionType::C
        } else {
            InstructionType::A
        }
    }

    fn y_register(&self) -> YRegister {
        if self.flag(12) {
            YRegister::M
        } else {
            YRegister::A
        }
    }

    fn zero_x(&self) -> bool {
        self.flag(11)
    }

    fn negate_x(&self) -> bool {
        self.flag(10)
    }

    fn zero_y(&self) -> bool {
        self.flag(9)
    }

    fn negate_y(&self) -> bool {
        self.flag(8)
    }

    fn operator(&self) -> Operator {
        if self.flag(7) {
            Operator::Add
        } else {
            Operator::And
        }
    }

    fn negate_out(&self) -> bool {
        self.flag(6)
    }

    fn dst_has_a(&self) -> bool {
        self.flag(5)
    }

    fn dst_has_d(&self) -> bool {
        self.flag(4)
    }

    fn dst_has_m(&self) -> bool {
        self.flag(3)
    }

    fn loaded_value(&self) -> u16 {
        self.raw & 0x7FFF
    }

    fn jump_condition(&self) -> JumpCondition {
        match self.raw & 7 {
            0 => JumpCondition::NoJump,
            1 => JumpCondition::JGT,
            2 => JumpCondition::JEQ,
            3 => JumpCondition::JGE,
            4 => JumpCondition::JLT,
            5 => JumpCondition::JNE,
            6 => JumpCondition::JLE,
            7 => JumpCondition::JMP,
            _ => unreachable!(),
        }
    }
}

impl JumpCondition {
    fn is_true(&self, value: u16) -> bool {
        let signed_value = value as i16;
        match self {
            JumpCondition::NoJump => false,
            JumpCondition::JGT => signed_value > 0,
            JumpCondition::JEQ => signed_value == 0,
            JumpCondition::JGE => signed_value >= 0,
            JumpCondition::JLT => signed_value < 0,
            JumpCondition::JNE => signed_value != 0,
            JumpCondition::JLE => signed_value <= 0,
            JumpCondition::JMP => true,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
struct Hardware {
    a: u16,
    d: u16,
    pc: u16,
    rom: [Instruction; 32 * 1024],
    ram: [u16; 32 * 1024],
}

impl Default for Hardware {
    fn default() -> Self {
        Hardware {
            a: 0,
            d: 0,
            pc: 0,
            rom: [Instruction { raw: 0 }; 32 * 1024],
            ram: [0; 32 * 1024],
        }
    }
}

impl std::fmt::Debug for Hardware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hardware")
            .field("a", &self.a)
            .field("d", &self.d)
            .field("pc", &self.pc)
            .field("m", &self.m())
            .field("current instruction", &self.rom[self.pc as usize])
            .finish()
    }
}

impl Hardware {
    fn m_mut(&mut self) -> &mut u16 {
        &mut self.ram[self.a as usize]
    }

    fn m(&self) -> &u16 {
        &self.ram[self.a as usize]
    }

    fn current_instruction(&self) -> &Instruction {
        &self.rom[self.pc as usize]
    }

    fn set(&mut self, instruction: Instruction, value: u16) {
        if instruction.dst_has_m() {
            *self.m_mut() = value;
        }
        if instruction.dst_has_a() {
            self.a = value;
        }
        if instruction.dst_has_d() {
            self.d = value;
        }
    }

    fn y_register_value(&self, y_register: YRegister) -> u16 {
        match y_register {
            YRegister::A => self.a,
            YRegister::M => *self.m(),
        }
    }

    fn compute(&self, instruction: Instruction) -> u16 {
        let mut x = self.d;
        let mut y = self.y_register_value(instruction.y_register());
        if instruction.zero_x() {
            x = 0;
        }
        if instruction.negate_x() {
            x = !x;
        }
        if instruction.zero_y() {
            y = 0;
        }
        if instruction.negate_y() {
            y = !y;
        }

        let computation_result = match instruction.operator() {
            Operator::And => {
                x & y
            }
            Operator::Add => {
                x.wrapping_add(y)
            }
        };

        if instruction.negate_out() {
            !computation_result
        } else {
            computation_result
        }
    }

    fn step(&mut self) {
        let instruction = *self.current_instruction();
        match instruction.instruction_type() {
            InstructionType::A => {
                self.a = instruction.loaded_value();
                self.pc += 1;
            }
            InstructionType::C => {
                let result = self.compute(instruction);
                self.set(instruction, result);
                self.pc = if instruction.jump_condition().is_true(result) {
                    self.a
                } else {
                    self.pc + 1
                }
            }
        }
    }

    fn load_program<I: Iterator<Item=Instruction>>(&mut self, program: I) {
        self.rom = [Instruction { raw: 0 }; 32 * 1024];
        for (i, instruction) in program.enumerate() {
            self.rom[i] = instruction;
        }
    }

    fn run_program(&mut self, program_length: usize) {
        while (self.pc as usize) < program_length - 1 {
            self.step();
        }
    }

    fn get_pixel(&self, x: usize, y: usize) -> bool {
        (self.ram[16384 + y * 32 + x / 16] & (1 << (x % 16))) != 0
    }

    fn set_pixel(&mut self, x: usize, y: usize, value: bool) {
        self.ram[16384 + y * 32 + x / 16] |= 1 << (x % 16);
        self.ram[16384 + y * 32 + x / 16] ^= (!value as u16) << (x % 16);
    }

    fn set_keyboard(&mut self, value: u16) {
        self.ram[24576] = value;
    }
}

fn keyboard_value_from_scancode(scancode: Scancode, keymod: Mod) -> u16 {
    match scancode {
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
                value as u16
            } else {
                0
            }
        }
    }
}

fn main() -> Result<(), String>{
    let mut hardware = Hardware::default();

    let program: [u16; 29] = [16384,60432,16,58248,17,60040,24576,64528,12,58114,17,61064,17,64528,16,65000,58120,24576,60560,16,62672,4,58115,16384,60432,16,58248,4,60039];
    hardware.load_program(program.iter().map(|raw| Instruction { raw: *raw }));

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
    'running: loop {
        let current_time = Instant::now();
        if (current_time - last_frame_time).as_secs_f64() * 60.0 > 1.0 {
            println!("{:?}, {}", current_time, steps_ran);
            steps_ran = 0;
            last_frame_time = current_time;
            canvas.set_draw_color(Color::RGB(255, 255, 255));
            canvas.clear();

            canvas.set_draw_color(Color::RGB(0, 0, 0));
            points.clear();
            for x in 0..512 {
                for y in 0..256 {
                    if hardware.get_pixel(x, y) {
                        points.push(Point::new(x as i32, y as i32))
                    }
                }
            }
            if !points.is_empty() {
                canvas.draw_points(points.as_slice())?;
            }
            canvas.present();

            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => break 'running,
                    Event::KeyDown { scancode: Some(scancode), keymod, .. } => {
                        let keyboard_value = keyboard_value_from_scancode(scancode, keymod);
                        hardware.set_keyboard(keyboard_value);
                    },
                    Event::KeyUp { .. } => {
                        hardware.set_keyboard(0);
                    },
                    _ => {}
                }
            }
        }
        hardware.step();
        steps_ran += 1;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_increment() {
        let mut hardware = Hardware::default();
        hardware.rom[0] = Instruction { raw: 59344 };
        hardware.d = 1337;

        let mut expected_hardware = hardware.clone();
        hardware.step();

        expected_hardware.d = 1338;
        expected_hardware.pc = 1;
        assert_eq!(hardware, expected_hardware);
    }

    #[test]
    fn test_add() {
        let mut hardware = Hardware::default();
        hardware.rom[0] = Instruction { raw: 57488 };
        hardware.a = 1337;
        hardware.d = 1337;

        let mut expected_hardware = hardware.clone();
        hardware.step();

        expected_hardware.d = 1337 * 2;
        expected_hardware.pc = 1;
        assert_eq!(hardware, expected_hardware);
    }

    #[test]
    fn test_zero() {
        let mut hardware = Hardware::default();
        hardware.rom[0] = Instruction { raw: 60048 };
        hardware.d = 1337;

        let mut expected_hardware = hardware.clone();
        hardware.step();

        expected_hardware.d = 0;
        expected_hardware.pc = 1;
        assert_eq!(hardware, expected_hardware);
    }

    #[test]
    fn test_load() {
        let mut hardware = Hardware::default();
        hardware.rom[0] = Instruction { raw: 1337 };

        let mut expected_hardware = hardware.clone();
        hardware.step();

        expected_hardware.a = 1337;
        expected_hardware.pc = 1;
        assert_eq!(hardware, expected_hardware);
    }

    #[test]
    fn test_integration() {
        let mut hardware = Hardware::default();
        let program: [u16; 16] = [
            15, 60040, 14, 64528, 15, 58114, 13, 64528, 15, 61576, 14, 64648, 2, 60039, 15, 60039,
        ];
        hardware.load_program(program.iter().map(|raw| Instruction { raw: *raw }));

        hardware.ram[13] = 34;
        hardware.ram[14] = 12;

        hardware.run_program(program.len());

        assert_eq!(hardware.ram[15], 34 * 12);
    }
}
