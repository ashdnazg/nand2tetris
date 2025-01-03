use std::{
    borrow::Borrow,
    ops::{Index, IndexMut},
};

#[cfg(not(feature = "bit32"))]
pub type Word = i16;
#[cfg(not(feature = "bit32"))]
pub type UWord = u16;

#[cfg(feature = "bit32")]
pub type Word = i32;
#[cfg(feature = "bit32")]
pub type UWord = u32;

use crate::hardware_parse::assemble_hack_file;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instruction {
    raw: UWord,
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

fn encode_jmp(jump_condition: JumpCondition) -> UWord {
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

fn encode_dst_registers(dst_registers: DestinationRegisters) -> UWord {
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
    pub fn new(raw: UWord) -> Instruction {
        Instruction { raw }
    }

    pub fn from_legacy(legacy_raw: u16) -> Instruction {
        let raw =
            (legacy_raw as UWord >> 15) << (Word::BITS - 1) | (legacy_raw & !(1 << 15)) as UWord;

        Instruction { raw }
    }

    pub fn create(
        dst_registers: DestinationRegisters,
        calculation_value: UWord,
        jump_condition: JumpCondition,
    ) -> Self {
        let encoded_dst = encode_dst_registers(dst_registers);
        let encoded_calculation = calculation_value;
        let encoded_jump = encode_jmp(jump_condition);

        Instruction {
            raw: (1 << (UWord::BITS - 1))
                | (encoded_calculation << 6)
                | (encoded_dst << 3)
                | encoded_jump,
        }
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.instruction_type() {
            InstructionType::A => write!(f, "@{}", self.raw),
            InstructionType::C => {
                let op_name = self.op_name();
                let dst = ["", "M=", "D=", "MD=", "A=", "AM=", "AD=", "AMD="]
                    [((self.raw >> 3) & 7) as usize];
                let jmp = ["", ";JGT", ";JEQ", ";JGE", ";JLT", ";JNE", ";JLE", ";JMP"]
                    [(self.raw & 7) as usize];
                write!(f, "{}{}{}", dst, op_name, jmp)
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
pub enum InstructionType {
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
    fn flag(&self, pos: u32) -> bool {
        self.raw & (1 << pos) != 0
    }

    pub fn instruction_type(&self) -> InstructionType {
        if self.flag(UWord::BITS - 1) {
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

    pub fn dst_has_a(&self) -> bool {
        self.flag(5)
    }

    pub fn dst_has_d(&self) -> bool {
        self.flag(4)
    }

    pub fn dst_has_m(&self) -> bool {
        self.flag(3)
    }

    pub fn loaded_value(&self) -> Word {
        self.raw as Word
    }

    pub fn jump_condition(&self) -> JumpCondition {
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

    pub fn op_name(&self) -> &'static str {
        match (self.raw >> 6) & 0x01FF {
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
            0x0182 => "D+A",
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
        }
    }
}

impl JumpCondition {
    fn is_true(&self, value: Word) -> bool {
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
pub const MEM_SIZE: usize = 32 * 1024;

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RAM {
    pub contents: Box<[Word; MEM_SIZE]>,
}

impl Index<Word> for RAM {
    type Output = Word;

    fn index(&self, index: Word) -> &Self::Output {
        &self.contents[index as usize]
    }
}

impl IndexMut<Word> for RAM {
    fn index_mut(&mut self, index: Word) -> &mut Self::Output {
        &mut self.contents[index as usize]
    }
}

impl RAM {
    pub const SCREEN: Word = (MEM_SIZE / 2) as Word;
    pub const KBD: Word = Self::SCREEN + Self::SCREEN_ROW_LENGTH * 256;
    pub const SCREEN_ROW_LENGTH: Word = 512 / Word::BITS as Word;

    pub fn get_pixel(&self, x: Word, y: Word) -> bool {
        (self[Self::SCREEN + y * Self::SCREEN_ROW_LENGTH + x / (Word::BITS as Word)]
            & (1 << (x % (Word::BITS as Word))))
            != 0
    }

    pub fn set_pixel(&mut self, x: Word, y: Word, value: bool) {
        if value {
            self[Self::SCREEN + y * Self::SCREEN_ROW_LENGTH + x / (Word::BITS as Word)] |=
                1 << (x % (Word::BITS as Word));
        } else {
            self[Self::SCREEN + y * Self::SCREEN_ROW_LENGTH + x / (Word::BITS as Word)] &=
                !(1 << (x % (Word::BITS as Word)));
        }
    }

    pub fn set_keyboard(&mut self, value: Word) {
        self[Self::KBD] = value;
    }
}

pub trait Emulator {
    fn a_mut(&mut self) -> &mut Word;
    fn a(&self) -> Word;
    fn d_mut(&mut self) -> &mut Word;
    fn d(&self) -> Word;
    fn get_ram_value(&self, address: Word) -> Word;
    fn set_ram_value(&mut self, address: Word, value: Word);
    fn pc(&self) -> Word;
    fn step(&mut self) -> bool;
    fn load_program(&mut self, program: impl IntoIterator<Item = impl Borrow<Instruction>>);
    fn run_program(&mut self);
    fn reset(&mut self);
}

impl Emulator for Hardware {
    fn a_mut(&mut self) -> &mut Word {
        &mut self.a
    }

    fn a(&self) -> Word {
        self.a
    }

    fn d_mut(&mut self) -> &mut Word {
        &mut self.d
    }

    fn d(&self) -> Word {
        self.d
    }

    fn pc(&self) -> Word {
        self.pc
    }

    fn get_ram_value(&self, address: Word) -> Word {
        self.ram[address]
    }

    fn set_ram_value(&mut self, address: Word, value: Word) {
        self.ram[address] = value;
    }

    fn step(&mut self) -> bool {
        self.ticks += 1;
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

    fn load_program(&mut self, program: impl IntoIterator<Item = impl Borrow<Instruction>>) {
        self.rom.fill(Instruction { raw: 0 });
        self.length = 0;
        for (i, instruction) in program.into_iter().enumerate() {
            self.rom[i] = *instruction.borrow();
            self.length += 1;
        }
    }

    fn run_program(&mut self) {
        while (self.pc as usize) < self.length {
            self.step();
        }
    }

    fn reset(&mut self) {
        *self = Hardware {
            rom: self.rom.clone(),
            breakpoints: self.breakpoints.clone(),
            length: self.length,
            ..Default::default()
        };
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Hardware {
    pub a: Word,
    pub d: Word,
    pub pc: Word,
    pub rom: Box<[Instruction; MEM_SIZE]>,
    pub ram: RAM,
    pub breakpoints: Vec<Breakpoint>,
    pub length: usize,
    pub ticks: u64,
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
            length: 32 * 1024,
            ticks: 0,
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
            .field("ticks", &self.ticks)
            .finish()
    }
}

impl Hardware {
    fn m_mut(&mut self) -> &mut Word {
        &mut self.ram[self.a]
    }

    fn m(&self) -> &Word {
        &self.ram[self.a]
    }

    fn current_instruction(&self) -> &Instruction {
        &self.rom[self.pc as usize]
    }

    fn set(&mut self, instruction: Instruction, value: Word) {
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

    fn y_register_value(&self, y_register: YRegister) -> Word {
        match y_register {
            YRegister::A => self.a,
            YRegister::M => *self.m(),
        }
    }

    fn compute(&self, instruction: Instruction) -> Word {
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

    pub fn get_breakpoint_var(&self, breakpoint_var: &BreakpointVar) -> Word {
        match breakpoint_var {
            BreakpointVar::A => self.a,
            BreakpointVar::D => self.d,
            BreakpointVar::M => self.ram[self.a],
            BreakpointVar::PC => self.pc,
            BreakpointVar::RAM(address) => self.ram[*address],
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

    pub fn from_file_contents(contents: &str) -> Self {
        let mut instance = Self::default();
        let instructions = assemble_hack_file(contents).unwrap().1;

        instance.length = instructions.len();
        for (i, instruction) in instructions.into_iter().enumerate() {
            instance.rom[i] = instruction;
        }

        instance
    }

    pub fn from_hack_file_contents(contents: &str) -> Self {
        let mut instance = Self::default();

        for (i, raw) in contents
            .lines()
            .map(|l| UWord::from_str_radix(l.trim(), 2).unwrap())
            .enumerate()
        {
            instance.rom[i] = Instruction { raw };
        }

        instance
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
    RAM(Word),
}

impl std::fmt::Display for BreakpointVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BreakpointVar::A => write!(f, "A"),
            BreakpointVar::D => write!(f, "D"),
            BreakpointVar::M => write!(f, "M"),
            BreakpointVar::PC => write!(f, "PC"),
            BreakpointVar::RAM(address) => write!(f, "RAM[{}]", address),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Breakpoint {
    pub var: BreakpointVar,
    pub value: Word,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_increment_hardware() {
        let mut hardware = Hardware::default();
        test_increment(&mut hardware);
    }

    fn test_increment(emulator: &mut impl Emulator) {
        emulator.load_program([Instruction::from_legacy(59344)]);
        *emulator.d_mut() = 1337;

        emulator.step();

        assert_eq!(emulator.d(), 1338);
    }

    #[test]
    fn test_add_hardware() {
        let mut hardware = Hardware::default();
        test_add(&mut hardware);
    }

    fn test_add(emulator: &mut impl Emulator) {
        emulator.load_program([Instruction::from_legacy(57488)]);
        *emulator.d_mut() = 1337;
        *emulator.a_mut() = 1337;

        emulator.step();

        assert_eq!(emulator.d(), 1337 * 2);
    }

    #[test]
    fn test_sub_minus_1_hardware() {
        let mut hardware = Hardware::default();
        test_sub_minus_1(&mut hardware);
    }

    fn test_sub_minus_1(emulator: &mut impl Emulator) {
        emulator.load_program([Instruction::create(
            DestinationRegisters::D,
            0x186,
            JumpCondition::NoJump,
        )]);
        *emulator.a_mut() = 1234;
        *emulator.d_mut() = 2345;

        emulator.step();

        assert_eq!(emulator.d(), 1110);
    }

    #[test]
    fn test_zero_hardware() {
        let mut hardware = Hardware::default();
        test_zero(&mut hardware);
    }

    fn test_zero(emulator: &mut impl Emulator) {
        emulator.load_program([Instruction::from_legacy(60048)]);
        *emulator.d_mut() = 1337;

        emulator.step();

        assert_eq!(emulator.d(), 0);
    }

    #[test]
    fn test_load_hardware() {
        let mut hardware = Hardware::default();
        test_load(&mut hardware);
    }

    fn test_load(emulator: &mut impl Emulator) {
        emulator.load_program([Instruction::from_legacy(1337)]);

        emulator.step();

        assert_eq!(emulator.a(), 1337);
    }

    #[test]
    fn test_integration_hardware() {
        let mut hardware = Hardware::default();
        test_integration(&mut hardware);
    }

    fn test_integration(emulator: &mut impl Emulator) {
        let program = [
            15, 60040, 14, 64528, 15, 58114, 13, 64528, 15, 61576, 14, 64648, 2, 60039,
        ];
        emulator.load_program(program.iter().copied().map(Instruction::from_legacy));

        emulator.set_ram_value(13, 34);
        emulator.set_ram_value(14, 12);

        emulator.run_program();

        assert_eq!(emulator.get_ram_value(15), 34 * 12);
    }

    #[test]
    fn test_jump_setting_a_hardware() {
        let mut hardware = Hardware::default();
        test_jump_setting_a(&mut hardware);
    }

    fn test_jump_setting_a(emulator: &mut impl Emulator) {
        emulator.load_program(&[Instruction::create(
            DestinationRegisters::A,
            0x01BF, // 1
            JumpCondition::JMP,
        )]);
        *emulator.a_mut() = 0;

        emulator.step();

        assert_eq!(emulator.pc(), 0);
    }
}
