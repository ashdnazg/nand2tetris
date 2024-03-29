use std::{
    borrow::Borrow,
    ops::{Index, IndexMut},
};

use crate::hardware_parse::assemble_hack_file;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instruction {
    raw: u16,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DestinationRegisters {
    NoDestination,
    A,
    AM,
    AMD,
    AD,
    M,
    MD,
    D,
}

fn encode_jmp(jump_condition: JumpCondition) -> u16 {
    use JumpCondition::*;
    match jump_condition {
        NoJump => 0,
        JGT => 1,
        JEQ => 2,
        JGE => 3,
        JLT => 4,
        JNE => 5,
        JLE => 6,
        JMP => 7,
    }
}

fn encode_dst_registers(dst_registers: DestinationRegisters) -> u16 {
    use DestinationRegisters::*;
    match dst_registers {
        NoDestination => 0,
        M => 1,
        D => 2,
        MD => 3,
        A => 4,
        AM => 5,
        AD => 6,
        AMD => 7,
    }
}

impl Instruction {
    pub fn new(raw: u16) -> Instruction {
        Instruction { raw }
    }

    pub fn create(
        dst_registers: DestinationRegisters,
        calculation_value: u16,
        jump_condition: JumpCondition,
    ) -> Self {
        let encoded_dst = encode_dst_registers(dst_registers);
        let encoded_calculation = calculation_value;
        let encoded_jump = encode_jmp(jump_condition);

        Instruction {
            raw: (1 << 15) | (encoded_calculation << 6) | (encoded_dst << 3) | encoded_jump,
        }
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
                let dst = ["", "M=", "D=", "MD=", "A=", "AM=", "AD=", "AMD="]
                    [((self.raw >> 3) & 7) as usize];
                let jmp = ["", ";JGT", ";JEQ", ";JGE", ";JLT", ";JNE", ";JLE", ";JMP"]
                    [(self.raw & 7) as usize];
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JumpCondition {
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
    pub contents: Box<[i16; 32 * 1024]>,
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
    pub const SCREEN: i16 = 0x4000;
    pub const KBD: i16 = 0x6000;
    pub const SCREEN_ROW_LENGTH: i16 = 32;

    pub fn get_pixel(&self, x: i16, y: i16) -> bool {
        (self[Self::SCREEN + y * Self::SCREEN_ROW_LENGTH + x / (i16::BITS as i16)]
            & (1 << (x % (i16::BITS as i16))))
            != 0
    }

    pub fn set_pixel(&mut self, x: i16, y: i16, value: bool) {
        if value {
            self[Self::SCREEN + y * Self::SCREEN_ROW_LENGTH + x / (i16::BITS as i16)] |=
                1 << (x % (i16::BITS as i16));
        } else {
            self[Self::SCREEN + y * Self::SCREEN_ROW_LENGTH + x / (i16::BITS as i16)] &=
                !(1 << (x % (i16::BITS as i16)));
        }
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
    pub rom: Box<[Instruction; 32 * 1024]>,
    pub ram: RAM,
    pub breakpoints: Vec<Breakpoint>,
}

impl Default for Hardware {
    fn default() -> Self {
        Hardware {
            a: 0,
            d: 0,
            pc: 0,
            rom: Box::new([Instruction { raw: 0 }; 32 * 1024]),
            ram: RAM {
                contents: Box::new([0; 32 * 1024]),
            },
            breakpoints: vec![],
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

    pub fn get_breakpoint_var(&self, breakpoint_var: &BreakpointVar) -> i16 {
        match breakpoint_var {
            BreakpointVar::A => self.a,
            BreakpointVar::D => self.d,
            BreakpointVar::M => self.ram[self.a],
            BreakpointVar::PC => self.pc,
            BreakpointVar::Mem(address) => self.ram[*address],
        }
    }

    pub fn run(&mut self, step_count: u64) -> bool {
        for _ in 0..step_count {
            if self.step() {
                return true;
            }
        }

        false
    }

    pub fn step(&mut self) -> bool {
        let instruction = *self.current_instruction();
        match instruction.instruction_type() {
            InstructionType::A => {
                self.a = instruction.loaded_value();
                self.pc += 1;
            }
            InstructionType::C => {
                let result = self.compute(instruction);
                self.pc = if instruction.jump_condition().is_true(result) {
                    self.a
                } else {
                    self.pc + 1
                };
                self.set(instruction, result);
            }
        }

        for breakpoint in &self.breakpoints {
            if self.get_breakpoint_var(&breakpoint.var) == breakpoint.value {
                return true;
            }
        }

        false
    }

    pub fn from_file_contents(contents: &str) -> Self {
        let mut instance = Self::default();
        let instructions = assemble_hack_file(contents).unwrap().1;

        for (i, instruction) in instructions.into_iter().enumerate() {
            instance.rom[i] = instruction;
        }

        instance
    }

    pub fn from_hack_file_contents(contents: &str) -> Self {
        let mut instance = Self::default();

        for (i, raw) in contents
            .lines()
            .map(|l| u16::from_str_radix(l.trim(), 2).unwrap())
            .enumerate()
        {
            instance.rom[i] = Instruction { raw };
        }

        instance
    }

    pub fn load_program(&mut self, program: impl IntoIterator<Item = impl Borrow<Instruction>>) {
        self.rom.fill(Instruction { raw: 0 });
        for (i, instruction) in program.into_iter().enumerate() {
            self.rom[i] = *instruction.borrow();
        }
    }

    pub fn run_program(&mut self, program_length: usize) {
        while (self.pc as usize) < program_length - 1 {
            self.step();
        }
    }

    pub fn reset(&mut self) {
        *self = Hardware {
            rom: self.rom.clone(),
            breakpoints: self.breakpoints.clone(),
            ..Default::default()
        };
    }

    pub fn get_breakpoints(&self) -> &Vec<Breakpoint> {
        &self.breakpoints
    }

    pub fn add_breakpoint(&mut self, breakpoint: &Breakpoint) {
        self.breakpoints.push(breakpoint.clone())
    }

    pub fn remove_breakpoint(&mut self, index: usize) {
        self.breakpoints.remove(index);
    }
}

#[derive(Clone, PartialEq, Eq, Copy, Debug)]
pub enum BreakpointVar {
    A,
    D,
    M,
    PC,
    Mem(i16),
}

impl std::fmt::Display for BreakpointVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BreakpointVar::A => write!(f, "A"),
            BreakpointVar::D => write!(f, "D"),
            BreakpointVar::M => write!(f, "M"),
            BreakpointVar::PC => write!(f, "PC"),
            BreakpointVar::Mem(address) => write!(f, "RAM[{}]", address),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Breakpoint {
    pub var: BreakpointVar,
    pub value: i16,
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
    fn test_sub_minus_1() {
        let mut hardware = Hardware::default();
        hardware.rom[0] =
            Instruction::create(DestinationRegisters::D, 0x186, JumpCondition::NoJump);
        hardware.a = 1234;
        hardware.d = 2345;

        let mut expected_hardware = hardware.clone();
        hardware.step();

        expected_hardware.d = 1110;
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

    #[test]
    fn test_jump_setting_a() {
        let mut hardware = Hardware::default();
        hardware.load_program(&[Instruction::create(
            DestinationRegisters::A,
            0x01BF, // 1
            JumpCondition::JMP,
        )]);
        hardware.step();

        assert_eq!(hardware.pc, 0);
    }
}
