use std::ops::{Index, IndexMut};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instruction {
    raw: u16,
}

impl Instruction {
    pub fn new(raw: u16) -> Instruction {
        Instruction { raw }
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.instruction_type() {
            InstructionType::A => write!(f, "@{}", self.raw),
            InstructionType::C => {
                let op = match (self.raw >> 6) & 0x01FF {
                    0x01AA => "0",
                    0x01BF => "1",
                    0x01BA => "-1",
                    0x018C => "D",
                    0x01B0 => "A",
                    0x01F0 => "M",
                    0x018D => "!D",
                    0x01B1 => "!A",
                    0x01F1 => "!M",
                    0x018F => "-D",
                    0x01B3 => "-A",
                    0x01F3 => "-M",
                    0x019F => "D+1",
                    0x01B7 => "A+1",
                    0x01F7 => "M+1",
                    0x018E => "D-1",
                    0x01B2 => "A-1",
                    0x01F2 => "M-1",
                    0x0182 => "A+D",
                    0x01C2 => "D+M",
                    0x0193 => "D-A",
                    0x0187 => "A-D",
                    0x01D3 => "D-M",
                    0x01C7 => "M-D",
                    0x0180 => "A&D",
                    0x01C0 => "D&M",
                    0x0195 => "A|D",
                    0x01D5 => "D|M",
                    _ => "???",
                };
                let dst = ["", "M = ","D = ", "MD = ", "A = ", "AM = ", "AD = ", "AMD = "][((self.raw >> 3) & 7) as usize];
                let jmp = ["", ";JGT", ";JEQ", ";JGE", ";JLT", ";JNE", ";JLE", ";JMP"][(self.raw & 7) as usize];
                write!(f, "{}{}{}", dst, op, jmp)
            }
        }
    }
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

#[allow(clippy::upper_case_acronyms)]
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

    fn loaded_value(&self) -> i16 {
        self.raw as i16
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
    fn is_true(&self, value: i16) -> bool {
        match self {
            JumpCondition::NoJump => false,
            JumpCondition::JGT => value > 0,
            JumpCondition::JEQ => value == 0,
            JumpCondition::JGE => value >= 0,
            JumpCondition::JLT => value < 0,
            JumpCondition::JNE => value != 0,
            JumpCondition::JLE => value <= 0,
            JumpCondition::JMP => true,
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RAM {
    pub contents: [i16; 32 * 1024],
}

impl Index<i16> for RAM {
    type Output = i16;

    fn index(&self, index: i16) -> &Self::Output {
        &self.contents[index as usize]
    }
}

impl IndexMut<i16> for RAM {
    fn index_mut(&mut self, index: i16) -> &mut Self::Output {
        &mut self.contents[index as usize]
    }
}

impl RAM {
    const SCREEN: i16 = 0x4000;
    const KBD: i16 = 0x6000;
    const SCREEN_ROW_LENGTH: i16 = 32;

    pub fn get_pixel(&self, x: i16, y: i16) -> bool {
        (self[Self::SCREEN + y * Self::SCREEN_ROW_LENGTH + x / (i16::BITS as i16)]
            & (1 << (x % (i16::BITS as i16))))
            != 0
    }

    pub fn set_keyboard(&mut self, value: i16) {
        self[Self::KBD] = value;
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Hardware {
    pub a: i16,
    pub d: i16,
    pub pc: i16,
    pub rom: [Instruction; 32 * 1024],
    pub ram: RAM,
}

impl Default for Hardware {
    fn default() -> Self {
        Hardware {
            a: 0,
            d: 0,
            pc: 0,
            rom: [Instruction { raw: 0 }; 32 * 1024],
            ram: RAM {
                contents: [0; 32 * 1024],
            },
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
    fn m_mut(&mut self) -> &mut i16 {
        &mut self.ram[self.a]
    }

    fn m(&self) -> &i16 {
        &self.ram[self.a]
    }

    fn current_instruction(&self) -> &Instruction {
        &self.rom[self.pc as usize]
    }

    fn set(&mut self, instruction: Instruction, value: i16) {
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

    fn y_register_value(&self, y_register: YRegister) -> i16 {
        match y_register {
            YRegister::A => self.a,
            YRegister::M => *self.m(),
        }
    }

    fn compute(&self, instruction: Instruction) -> i16 {
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
            Operator::And => x & y,
            Operator::Add => x.wrapping_add(y),
        };

        if instruction.negate_out() {
            !computation_result
        } else {
            computation_result
        }
    }

    pub fn step(&mut self) {
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

    pub fn load_program<I: Iterator<Item = Instruction>>(&mut self, program: I) {
        self.rom = [Instruction { raw: 0 }; 32 * 1024];
        for (i, instruction) in program.enumerate() {
            self.rom[i] = instruction;
        }
    }

    pub fn run_program(&mut self, program_length: usize) {
        while (self.pc as usize) < program_length - 1 {
            self.step();
        }
    }
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
        hardware.load_program(program.iter().map(|raw| Instruction::new(*raw)));

        hardware.ram[13] = 34;
        hardware.ram[14] = 12;

        hardware.run_program(program.len());

        assert_eq!(hardware.ram[15], 34 * 12);
    }
}
