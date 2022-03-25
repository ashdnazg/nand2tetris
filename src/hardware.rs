use std::ops::{Index, IndexMut};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instruction {
    raw: u16,
}

impl Instruction {
    pub fn new(raw: u16) -> Instruction {
        Instruction { raw: raw }
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RAM {
    pub contents: [u16; 32 * 1024],
}

impl Index<u16> for RAM {
    type Output = u16;

    fn index(&self, index: u16) -> &Self::Output {
        &self.contents[index as usize]
    }
}

impl IndexMut<u16> for RAM {
    fn index_mut(&mut self, index: u16) -> &mut Self::Output {
        &mut self.contents[index as usize]
    }
}

impl RAM {
    const SCREEN: u16 = 0x4000;
    const KBD: u16 = 0x6000;

    pub fn get_pixel(&self, x: u16, y: u16) -> bool {
        (self[Self::SCREEN + y * 32 + x / 16] & (1 << (x % 16))) != 0
    }

    // pub fn set_pixel(&mut self, x: u16, y: u16, value: bool) {
    //     self[Self::SCREEN + y * 32 + x / 16] |= 1 << (x % 16);
    //     self[Self::SCREEN + y * 32 + x / 16] ^= (!value as u16) << (x % 16);
    // }

    pub fn set_keyboard(&mut self, value: u16) {
        self[Self::KBD] = value;
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Hardware {
    pub a: u16,
    pub d: u16,
    pub pc: u16,
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
    fn m_mut(&mut self) -> &mut u16 {
        &mut self.ram[self.a]
    }

    fn m(&self) -> &u16 {
        &self.ram[self.a]
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

    fn run_program(&mut self, program_length: usize) {
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
        hardware.load_program(program.iter().map(|raw| Instruction { raw: *raw }));

        hardware.ram[13] = 34;
        hardware.ram[14] = 12;

        hardware.run_program(program.len());

        assert_eq!(hardware.ram[15], 34 * 12);
    }
}
