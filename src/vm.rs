use hashbrown::HashMap;
use std::{
    fs,
    ops::{Index, IndexMut, RangeInclusive},
    path::PathBuf,
};

use crate::{
    hardware::{RAM, Word},
    os::OS,
    vm_parse::parse_commands,
};

impl Index<Register> for RAM {
    type Output = Word;

    fn index(&self, index: Register) -> &Self::Output {
        &self[index.address()]
    }
}

impl IndexMut<Register> for RAM {
    fn index_mut(&mut self, index: Register) -> &mut Self::Output {
        &mut self[index.address()]
    }
}

impl Default for RAM {
    fn default() -> Self {
        let mut instance = Self {
            contents: Box::new([0; 32 * 1024]),
        };
        instance[Register::SP] = 256;

        instance
    }
}

impl RAM {
    fn new() -> Self {
        let mut instance = Self {
            contents: Box::new([0; 32 * 1024]),
        };
        instance[Register::SP] = 256;

        instance
    }

    pub fn push(&mut self, value: Word) {
        let sp = self[Register::SP];
        self[sp] = value;
        self[Register::SP] += 1;
    }

    fn pop(&mut self) -> Word {
        self[Register::SP] -= 1;
        self[self[Register::SP]]
    }

    fn stack_top(&mut self) -> &mut Word {
        let sp = self[Register::SP];
        &mut self[sp - 1]
    }

    pub fn set(&mut self, static_segment: Word, segment: PopSegment, offset: Word, value: Word) {
        match segment {
            PopSegment::Static => {
                self[static_segment + offset] = value;
            }
            PopSegment::Local => {
                let lcl = self[Register::LCL];
                self[lcl + offset] = value;
            }
            PopSegment::Argument => {
                let arg = self[Register::ARG];
                self[arg + offset] = value;
            }
            PopSegment::This => {
                let this = self[Register::THIS];
                self[this + offset] = value;
            }
            PopSegment::That => {
                let that = self[Register::THAT];
                self[that + offset] = value;
            }
            PopSegment::Temp => {
                self[Register::TEMP(offset)] = value;
            }
            PopSegment::Pointer => {
                self[Register::THIS.address() + offset] = value;
            }
        }
    }

    pub fn get(&self, static_segment: Word, segment: PushSegment, offset: Word) -> Word {
        match segment {
            PushSegment::Constant => offset,
            PushSegment::Static => self[static_segment + offset],
            PushSegment::Local => self[self[Register::LCL] + offset],
            PushSegment::Argument => self[self[Register::ARG] + offset],
            PushSegment::This => self[self[Register::THIS] + offset],
            PushSegment::That => self[self[Register::THAT] + offset],
            PushSegment::Temp => self[Register::TEMP(offset)],
            PushSegment::Pointer => self[Register::THIS.address() + offset],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Program {
    pub all_commands: Vec<VMCommand>,
    pub function_name_to_index: HashMap<String, usize>,
    pub function_metadata: Vec<FunctionMetadata>,
    pub file_name_to_index: HashMap<String, usize>,
    pub files: Vec<File>,
}

#[derive(Clone)]
pub struct RunState {
    pub current_file_index: usize,
    pub current_command_index: usize,
    pub ram: RAM,
    pub os: OS,
    pub call_stack: Vec<Frame>,
    pub breakpoints: Vec<Breakpoint>,
    pub func_stats: HashMap<String, u64>,
}

#[derive(Clone)]
pub struct VM {
    pub run_state: RunState,
    pub program: Program,
}

impl VM {
    pub fn from_paths(paths: &[PathBuf]) -> Self {
        let files = paths
            .iter()
            .map(|path| {
                (
                    path.file_name().unwrap().to_str().unwrap().to_owned(),
                    fs::read_to_string(path).unwrap(),
                )
            })
            .collect();

        Self::from_file_contents(files)
    }

    pub fn from_file_contents(file_contents: Vec<(String, String)>) -> Self {
        Self::from_all_file_commands(
            file_contents
                .into_iter()
                .map(|(name, contents)| {
                    (
                        name.rsplit_once('.').unwrap().0.to_owned(),
                        parse_commands(&contents).unwrap().1,
                    )
                })
                .collect(),
        )
    }

    pub fn from_all_file_commands(all_file_commands: Vec<(String, Vec<VMCommand>)>) -> Self {
        let mut all_commands = vec![];
        let mut file_name_to_index = HashMap::new();
        let mut files = vec![];
        let mut next_static_index = 16;
        let mut function_name_to_index = HashMap::new();
        let mut function_metadata = vec![];
        for (name, file_commands) in all_file_commands.into_iter() {
            let file_index = files.len();
            let file = File::new(
                &name,
                file_index,
                &file_commands,
                all_commands.len(),
                next_static_index,
                &mut function_name_to_index,
                &mut function_metadata,
            );
            next_static_index = *file.static_segment.end() + 1;
            file_name_to_index.insert(name, file_index);
            files.push(file);
            all_commands.extend(file_commands);
        }

        let program = Program {
            all_commands,
            function_name_to_index,
            function_metadata,
            file_name_to_index,
            files: files.into_iter().collect(),
        };

        Self::new(program)
    }

    pub fn new(program: Program) -> Self {
        let current_file_index = *program.file_name_to_index.get("Sys").unwrap_or(&0);
        let current_command_index = program.files[current_file_index].starting_command_index;
        let function_index = *program.function_name_to_index.get("Sys.init").unwrap_or(&0);
        Self {
            program,
            run_state: RunState {
                current_file_index,
                current_command_index,
                ram: RAM::new(),
                os: Default::default(),
                call_stack: vec![Frame { function_index }],
                breakpoints: vec![],
                func_stats: HashMap::new(),
            },
        }
    }

    pub fn reset(&mut self) {
        for (key, value) in self.run_state.func_stats.iter() {
            println!("Function {} was called {} times", key, value);
        }
        *self = VM::new(self.program.clone());
    }

    pub fn step(&mut self) {
        self.run(1)
    }

    pub fn run(&mut self, num_steps: u64) {
        let files = &self.program.files;
        let run_state = &mut self.run_state;

        let mut static_segment = *files[run_state.current_file_index].static_segment.start();
        for _ in 0..num_steps {
            match &self.program.all_commands[run_state.current_command_index] {
                VMCommand::Add => {
                    let y = run_state.ram.pop();
                    *run_state.ram.stack_top() = run_state.ram.stack_top().wrapping_add(y);
                    run_state.current_command_index += 1;
                }
                VMCommand::Push { segment, offset } => {
                    let value = run_state.ram.get(static_segment, *segment, *offset);
                    run_state.ram.push(value);
                    run_state.current_command_index += 1;
                }
                VMCommand::Pop { segment, offset } => {
                    let value = run_state.ram.pop();
                    run_state.ram.set(static_segment, *segment, *offset, value);
                    run_state.current_command_index += 1;
                }
                VMCommand::Sub => {
                    let y = run_state.ram.pop();
                    *run_state.ram.stack_top() = run_state.ram.stack_top().wrapping_sub(y);
                    run_state.current_command_index += 1;
                }
                VMCommand::Neg => {
                    let y = run_state.ram.stack_top();
                    *y = y.wrapping_neg();
                    run_state.current_command_index += 1;
                }
                VMCommand::Eq => {
                    let y = run_state.ram.pop();
                    let x = run_state.ram.stack_top();
                    *x = -((*x == y) as Word);
                    run_state.current_command_index += 1;
                }
                VMCommand::Gt => {
                    let y = run_state.ram.pop();
                    let x = run_state.ram.stack_top();
                    *x = -((*x > y) as Word);
                    run_state.current_command_index += 1;
                }
                VMCommand::Lt => {
                    let y = run_state.ram.pop();
                    let x = run_state.ram.stack_top();
                    *x = -((*x < y) as Word);
                    run_state.current_command_index += 1;
                }
                VMCommand::And => {
                    let y = run_state.ram.pop();
                    *run_state.ram.stack_top() &= y;
                    run_state.current_command_index += 1;
                }
                VMCommand::Or => {
                    let y = run_state.ram.pop();
                    *run_state.ram.stack_top() |= y;
                    run_state.current_command_index += 1;
                }
                VMCommand::Not => {
                    *run_state.ram.stack_top() ^= -1;
                    run_state.current_command_index += 1;
                }
                VMCommand::Label { name: _ } => {
                    run_state.current_command_index += 1;
                }
                VMCommand::Goto { label_name } => {
                    Self::goto(
                        &mut run_state.current_command_index,
                        &self.program.function_metadata
                            [run_state.call_stack.last().unwrap().function_index],
                        label_name,
                    );
                }
                VMCommand::IfGoto { label_name } => {
                    let value = run_state.ram.pop();
                    if value != 0 {
                        Self::goto(
                            &mut run_state.current_command_index,
                            &self.program.function_metadata
                                [run_state.call_stack.last().unwrap().function_index],
                            label_name,
                        );
                    } else {
                        run_state.current_command_index += 1;
                    }
                }
                VMCommand::Function {
                    name: _,
                    local_var_count,
                } => {
                    for _ in 0..*local_var_count {
                        run_state.ram.push(0);
                    }
                    run_state.current_command_index += 1;
                }
                VMCommand::Call {
                    function_name,
                    argument_count,
                } => {
                    run_state.func_stats
                        .entry(function_name.clone())
                        .and_modify(|c| *c += 1)
                        .or_insert(1);
                    let argument_segment = run_state.ram[Register::SP] - argument_count;
                    run_state
                        .ram
                        .push((run_state.current_command_index + 1) as Word);
                    for i in 1..=4 {
                        let value = run_state.ram[i];
                        run_state.ram.push(value);
                    }

                    let local_segment = run_state.ram[Register::SP];
                    run_state.ram[Register::LCL] = local_segment;
                    run_state.ram[Register::ARG] = argument_segment;
                    if run_state.call_os(function_name) {
                        let frame = run_state.ram[Register::LCL];
                        run_state.current_command_index = run_state.ram[frame - 5] as usize;
                        let return_value = run_state.ram.pop();
                        run_state
                            .ram
                            .set(static_segment, PopSegment::Argument, 0, return_value);
                        run_state.ram[Register::SP] = run_state.ram[Register::ARG] + 1;
                        for i in 1..=4 {
                            run_state.ram[i] = run_state.ram[frame - 5 + i];
                        }
                    } else {
                        let function_index = self.program.function_name_to_index[function_name];
                        let function_metadata = &self.program.function_metadata[function_index];
                        let file_index = function_metadata.file_index;

                        run_state.current_command_index = function_metadata.command_index;
                        run_state.current_file_index = file_index;
                        static_segment = *files[file_index].static_segment.start();

                        run_state.call_stack.push(Frame { function_index });
                    }
                }
                VMCommand::Return => {
                    let frame = run_state.ram[Register::LCL];
                    run_state.current_command_index = run_state.ram[frame - 5] as usize;
                    let return_value = run_state.ram.pop();
                    run_state
                        .ram
                        .set(static_segment, PopSegment::Argument, 0, return_value);
                    run_state.ram[Register::SP] = run_state.ram[Register::ARG] + 1;
                    for i in 1..=4 {
                        run_state.ram[i] = run_state.ram[frame - 5 + i];
                    }
                    run_state.call_stack.pop();

                    let last_frame = run_state.call_stack.last().unwrap();
                    let file_index =
                        self.program.function_metadata[last_frame.function_index].file_index;
                    run_state.current_file_index = file_index;
                    static_segment = *files[file_index].static_segment.start();
                }
            }
        }
    }

    fn goto(
        current_command_index: &mut usize,
        function_metadata: &FunctionMetadata,
        label_name: &str,
    ) {
        *current_command_index = function_metadata.label_name_to_command_index[label_name]
    }

    pub fn get_breakpoints(&self) -> &Vec<Breakpoint> {
        &self.run_state.breakpoints
    }

    pub fn add_breakpoint(&mut self, breakpoint: &Breakpoint) {
        self.run_state.breakpoints.push(breakpoint.clone())
    }

    pub fn remove_breakpoint(&mut self, index: usize) {
        self.run_state.breakpoints.remove(index);
    }

    pub fn is_ready(&self) -> bool {
        true
    }

    pub fn copy_ram(&self) -> RAM {
        self.run_state.ram.clone()
    }

    pub fn set_ram_value(&mut self, address: i16, value: i16) {
        self.run_state.ram[address as Word] = value as Word;
    }

    pub fn get_ram_value(&self, address: i16) -> i16 {
        self.run_state.ram[address as Word] as i16
    }

    pub fn current_file_index(&self) -> usize {
        self.run_state.current_file_index
    }

    pub fn current_command_index(&self) -> usize {
        self.run_state.current_command_index
    }

    pub fn current_file_name(&self) -> &str {
        let index = self.current_file_index();

        &self.program.files[index].name
    }
}

#[derive(Clone)]
pub struct Frame {
    pub function_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionMetadata {
    pub argument_count: Word,
    pub local_var_count: Word,
    pub command_index: usize,
    file_index: usize,
    label_name_to_command_index: HashMap<String, usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    pub name: String,
    pub starting_command_index: usize,
    command_count: usize,
    pub static_segment: RangeInclusive<Word>,
}

impl File {
    fn new(
        name: &str,
        file_index: usize,
        commands: &[VMCommand],
        starting_command_index: usize,
        static_segment_start: Word,
        function_name_to_index: &mut HashMap<String, usize>,
        function_metadata: &mut Vec<FunctionMetadata>,
    ) -> Self {
        let mut max_static_index: Word = static_segment_start - 1;
        for (i, command) in commands.iter().enumerate() {
            match command {
                VMCommand::Label { name } => {
                    function_metadata
                        .last_mut()
                        .unwrap()
                        .label_name_to_command_index
                        .insert(name.clone(), starting_command_index + i);
                }
                VMCommand::Function {
                    name,
                    local_var_count,
                } => {
                    function_name_to_index.insert(name.clone(), function_metadata.len());
                    function_metadata.push(FunctionMetadata {
                        argument_count: 0,
                        local_var_count: *local_var_count,
                        command_index: starting_command_index + i,
                        file_index,
                        label_name_to_command_index: HashMap::new(),
                    });
                }
                VMCommand::Pop {
                    segment: PopSegment::Argument,
                    offset,
                }
                | VMCommand::Push {
                    segment: PushSegment::Argument,
                    offset,
                } => {
                    let metadata = function_metadata.last_mut().unwrap();
                    metadata.argument_count = Word::max(metadata.argument_count, *offset);
                }
                VMCommand::Push {
                    segment: PushSegment::Static,
                    offset,
                }
                | VMCommand::Pop {
                    segment: PopSegment::Static,
                    offset,
                } => {
                    max_static_index = max_static_index.max(static_segment_start + *offset);
                }
                _ => {}
            }
        }

        File {
            name: name.to_owned(),
            starting_command_index,
            static_segment: static_segment_start..=max_static_index,
            command_count: commands.len(),
        }
    }

    pub fn commands<'a>(&self, all_commands: &'a [VMCommand]) -> &'a [VMCommand] {
        &all_commands[self.starting_command_index..self.starting_command_index + self.command_count]
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VMCommand {
    Add,
    Push {
        segment: PushSegment,
        offset: Word,
    },
    Pop {
        segment: PopSegment,
        offset: Word,
    },
    Sub,
    Neg,
    Eq,
    Gt,
    Lt,
    And,
    Or,
    Not,
    Label {
        name: String,
    },
    Goto {
        label_name: String,
    },
    IfGoto {
        label_name: String,
    },
    Function {
        name: String,
        local_var_count: Word,
    },
    Call {
        function_name: String,
        argument_count: Word,
    },
    Return,
}

impl std::fmt::Display for VMCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VMCommand::Add => write!(f, "add"),
            VMCommand::Push { segment, offset } => write!(f, "push {segment} {offset}"),
            VMCommand::Pop { segment, offset } => write!(f, "pop {segment} {offset}"),
            VMCommand::Sub => write!(f, "sub"),
            VMCommand::Neg => write!(f, "neg"),
            VMCommand::Eq => write!(f, "eq"),
            VMCommand::Gt => write!(f, "gt"),
            VMCommand::Lt => write!(f, "lt"),
            VMCommand::And => write!(f, "and"),
            VMCommand::Or => write!(f, "or"),
            VMCommand::Not => write!(f, "not"),
            VMCommand::Label { name } => write!(f, "label {name}"),
            VMCommand::Goto { label_name } => write!(f, "goto {label_name}"),
            VMCommand::IfGoto { label_name } => write!(f, "if-goto {label_name}"),
            VMCommand::Function {
                name,
                local_var_count,
            } => write!(f, "function {name} {local_var_count}"),
            VMCommand::Call {
                function_name,
                argument_count,
            } => write!(f, "call {function_name} {argument_count}"),
            VMCommand::Return => write!(f, "return"),
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
pub enum Register {
    SP,
    LCL,
    ARG,
    THIS,
    THAT,
    TEMP(Word),
}

impl Register {
    pub fn address(&self) -> Word {
        match self {
            Register::SP => 0,
            Register::LCL => 1,
            Register::ARG => 2,
            Register::THIS => 3,
            Register::THAT => 4,
            Register::TEMP(offset) => 5 + *offset,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PushSegment {
    Constant,
    Static,
    Local,
    Argument,
    This,
    That,
    Temp,
    Pointer,
}

impl std::fmt::Display for PushSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            PushSegment::Constant => "constant",
            PushSegment::Static => "static",
            PushSegment::Local => "local",
            PushSegment::Argument => "argument",
            PushSegment::This => "this",
            PushSegment::That => "that",
            PushSegment::Temp => "temp",
            PushSegment::Pointer => "pointer",
        };
        write!(f, "{name}")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopSegment {
    Static,
    Local,
    Argument,
    This,
    That,
    Temp,
    Pointer,
}

impl std::fmt::Display for PopSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            PopSegment::Static => "static",
            PopSegment::Local => "local",
            PopSegment::Argument => "argument",
            PopSegment::This => "this",
            PopSegment::That => "that",
            PopSegment::Temp => "temp",
            PopSegment::Pointer => "pointer",
        };
        write!(f, "{name}")
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Breakpoint {
    SP(Word),
    CurrentFunction(String),
    Line {
        file_name: String,
        line_number: Word,
    },
    RAM {
        address: Word,
        value: Word,
    },
    LCL(Word),
    Local {
        offset: Word,
        value: Word,
    },
    ARG(Word),
    Argument {
        offset: Word,
        value: Word,
    },
    This(Word),
    ThisPointer {
        offset: Word,
        value: Word,
    },
    That(Word),
    ThatPointer {
        offset: Word,
        value: Word,
    },
    Temp {
        offset: Word,
        value: Word,
    },
}

impl Breakpoint {
    pub fn variable_name(&self) -> String {
        match self {
            Breakpoint::SP(_) => "SP".to_owned(),
            Breakpoint::CurrentFunction(_) => "Current Function".to_owned(),
            Breakpoint::Line {
                file_name: _,
                line_number: _,
            } => "Line".to_owned(),
            Breakpoint::RAM { address, .. } => format!("RAM[{address}]"),
            Breakpoint::LCL(_) => "LCL".to_owned(),
            Breakpoint::Local { offset, .. } => format!("LCL[{offset}]"),
            Breakpoint::ARG(_) => "ARG".to_owned(),
            Breakpoint::Argument { offset, .. } => format!("ARG[{offset}]"),
            Breakpoint::This(_) => "THIS".to_owned(),
            Breakpoint::ThisPointer { offset, .. } => format!("THIS[{offset}]"),
            Breakpoint::That(_) => "THAT".to_owned(),
            Breakpoint::ThatPointer { offset, .. } => format!("THIS[{offset}]"),
            Breakpoint::Temp { offset, .. } => format!("TEMP[{offset}]"),
        }
    }

    pub fn value(&self) -> String {
        match self {
            Breakpoint::SP(value)
            | Breakpoint::RAM { value, .. }
            | Breakpoint::LCL(value)
            | Breakpoint::Local { value, .. }
            | Breakpoint::ARG(value)
            | Breakpoint::Argument { value, .. }
            | Breakpoint::This(value)
            | Breakpoint::ThisPointer { value, .. }
            | Breakpoint::That(value)
            | Breakpoint::ThatPointer { value, .. }
            | Breakpoint::Temp { value, .. } => value.to_string(),
            Breakpoint::Line {
                file_name,
                line_number,
            } => format!("{file_name}:{line_number}"),
            Breakpoint::CurrentFunction(function_name) => function_name.clone(),
        }
    }

    pub fn address(&self) -> Option<Word> {
        match self {
            Breakpoint::SP(_) => None,
            Breakpoint::CurrentFunction(_) => None,
            Breakpoint::Line { .. } => None,
            Breakpoint::RAM { address, .. } => Some(*address),
            Breakpoint::LCL(_) => None,
            Breakpoint::Local { offset, .. } => Some(*offset),
            Breakpoint::ARG(_) => None,
            Breakpoint::Argument { offset, .. } => Some(*offset),
            Breakpoint::This(_) => None,
            Breakpoint::ThisPointer { offset, .. } => Some(*offset),
            Breakpoint::That(_) => None,
            Breakpoint::ThatPointer { offset, .. } => Some(*offset),
            Breakpoint::Temp { offset, .. } => Some(*offset),
        }
    }

    pub fn change_address(&mut self, new_address: Word) {
        match self {
            Breakpoint::RAM { address, .. } => *address = new_address,
            Breakpoint::Local { offset, .. } => *offset = new_address,
            Breakpoint::Argument { offset, .. } => *offset = new_address,
            Breakpoint::ThisPointer { offset, .. } => *offset = new_address,
            Breakpoint::ThatPointer { offset, .. } => *offset = new_address,
            Breakpoint::Temp { offset, .. } => *offset = new_address,
            _ => panic!("Tried to change address of a non-address breakpoint"),
        }
    }

    pub fn change_value(&mut self, new_value: String) {
        match self {
            Breakpoint::SP(value)
            | Breakpoint::RAM { value, .. }
            | Breakpoint::LCL(value)
            | Breakpoint::Local { value, .. }
            | Breakpoint::ARG(value)
            | Breakpoint::Argument { value, .. }
            | Breakpoint::This(value)
            | Breakpoint::ThisPointer { value, .. }
            | Breakpoint::That(value)
            | Breakpoint::ThatPointer { value, .. }
            | Breakpoint::Temp { value, .. } => {
                if let Ok(int_value) = new_value.parse::<Word>() {
                    *value = int_value;
                }
            }
            Breakpoint::CurrentFunction(value) => *value = new_value,
            _ => panic!("Tried to change value of a non-value breakpoint"),
        }
    }
}

// impl std::fmt::Display for Breakpoint {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Breakpoint::SP(_) => write!(f, "SP"),
//             Breakpoint::CurrentFunction(_) => write!(f, "Current Function"),
//             Breakpoint::Line { file_name, line_number } => write!(f, "Line"),
//             Breakpoint::RAM { address, value } => write!(f, "RAM"),
//             Breakpoint::LCL(_) => write!(f, "LCL"),
//             Breakpoint::Local { offset, value } => write!(f, "LCL[{}]", offset),
//             Breakpoint::ARG(_) => write!(f, "ARG"),
//             Breakpoint::Argument { offset, value } => write!(f, "Argument"),
//             Breakpoint::This(_) => write!(f, "THIS"),
//             Breakpoint::ThisPointer { offset, value } => todo!(),
//             Breakpoint::That(_) => todo!(),
//             Breakpoint::ThatPointer { offset, value } => todo!(),
//             Breakpoint::Temp { offset, value } => todo!(),
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    impl VM {
        fn test_get(&self, segment: PushSegment, offset: Word) -> Word {
            self.run_state.ram.get(
                *self.program.files[self.run_state.current_file_index]
                    .static_segment
                    .start(),
                segment,
                offset,
            )
        }

        fn test_set(&mut self, segment: PopSegment, offset: Word, value: Word) {
            self.run_state.ram.set(
                *self.program.files[self.run_state.current_file_index]
                    .static_segment
                    .start(),
                segment,
                offset,
                value,
            )
        }

        fn test_instance() -> VM {
            let mut vm = VM::from_all_file_commands(vec![(
                "foo".to_owned(),
                vec![VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 666,
                }],
            )]);
            vm.run_state.current_file_index = 0;

            vm
        }
    }

    #[test]
    fn test_constant() {
        let vm = VM::test_instance();

        assert_eq!(vm.test_get(PushSegment::Constant, 1337), 1337);
    }

    #[test]
    fn test_static() {
        let all_file_commands = vec![
            (
                "a".to_owned(),
                vec![
                    VMCommand::Push {
                        segment: PushSegment::Static,
                        offset: 0,
                    },
                    VMCommand::Pop {
                        segment: PopSegment::Static,
                        offset: 1,
                    },
                ],
            ),
            (
                "b".to_owned(),
                vec![
                    VMCommand::Push {
                        segment: PushSegment::Static,
                        offset: 1,
                    },
                    VMCommand::Pop {
                        segment: PopSegment::Static,
                        offset: 0,
                    },
                ],
            ),
        ];

        let mut vm = VM::from_all_file_commands(all_file_commands);
        vm.run_state.current_file_index = vm.program.file_name_to_index["a"];

        vm.test_set(PopSegment::Static, 0, 1337);
        vm.run_state.current_file_index = vm.program.file_name_to_index["b"];
        vm.test_set(PopSegment::Static, 1, 2337);

        assert_eq!(vm.test_get(PushSegment::Static, 0), 0);
        assert_eq!(vm.test_get(PushSegment::Static, 1), 2337);
        vm.run_state.current_file_index = vm.program.file_name_to_index["a"];
        assert_eq!(vm.test_get(PushSegment::Static, 0), 1337);
        assert_eq!(vm.test_get(PushSegment::Static, 1), 0);
    }

    #[test]
    fn test_local() {
        let mut vm = VM::test_instance();
        vm.run_state.ram[1] = 1337;

        vm.test_set(PopSegment::Local, 0, 2337);
        vm.test_set(PopSegment::Local, 3, 3337);

        assert_eq!(vm.test_get(PushSegment::Local, 0), 2337);
        assert_eq!(vm.test_get(PushSegment::Local, 3), 3337);
        assert_eq!(vm.run_state.ram[1337], 2337);
        assert_eq!(vm.run_state.ram[1340], 3337);
    }

    #[test]
    fn test_argument() {
        let mut vm = VM::test_instance();
        vm.run_state.ram[2] = 1337;

        vm.test_set(PopSegment::Argument, 0, 2337);
        vm.test_set(PopSegment::Argument, 3, 3337);

        assert_eq!(vm.test_get(PushSegment::Argument, 0), 2337);
        assert_eq!(vm.test_get(PushSegment::Argument, 3), 3337);
        assert_eq!(vm.run_state.ram[1337], 2337);
        assert_eq!(vm.run_state.ram[1340], 3337);
    }

    #[test]
    fn test_this() {
        let mut vm = VM::test_instance();
        vm.run_state.ram[3] = 1337;

        vm.test_set(PopSegment::This, 0, 2337);
        vm.test_set(PopSegment::This, 3, 3337);

        assert_eq!(vm.test_get(PushSegment::This, 0), 2337);
        assert_eq!(vm.test_get(PushSegment::This, 3), 3337);
        assert_eq!(vm.run_state.ram[1337], 2337);
        assert_eq!(vm.run_state.ram[1340], 3337);
    }

    #[test]
    fn test_that() {
        let mut vm = VM::test_instance();
        vm.run_state.ram[4] = 1337;

        vm.test_set(PopSegment::That, 0, 2337);
        vm.test_set(PopSegment::That, 3, 3337);

        assert_eq!(vm.test_get(PushSegment::That, 0), 2337);
        assert_eq!(vm.test_get(PushSegment::That, 3), 3337);
        assert_eq!(vm.run_state.ram[1337], 2337);
        assert_eq!(vm.run_state.ram[1340], 3337);
    }

    #[test]
    fn test_temp() {
        let mut vm = VM::test_instance();

        vm.test_set(PopSegment::Temp, 0, 1337);
        vm.test_set(PopSegment::Temp, 3, 2337);

        assert_eq!(vm.test_get(PushSegment::Temp, 0), 1337);
        assert_eq!(vm.test_get(PushSegment::Temp, 3), 2337);
        assert_eq!(vm.run_state.ram[5], 1337);
        assert_eq!(vm.run_state.ram[8], 2337);
    }

    #[test]
    fn test_pointer() {
        let mut vm = VM::test_instance();

        vm.test_set(PopSegment::Pointer, 0, 1337);
        vm.test_set(PopSegment::Pointer, 1, 2337);

        assert_eq!(vm.test_get(PushSegment::Pointer, 0), 1337);
        assert_eq!(vm.run_state.ram[3], 1337);
        assert_eq!(vm.test_get(PushSegment::Pointer, 1), 2337);
        assert_eq!(vm.run_state.ram[4], 2337);
    }

    #[test]
    fn test_add() {
        let all_file_commands = vec![(
            "a".to_owned(),
            vec![
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 2337,
                },
                VMCommand::Add,
            ],
        )];

        let mut vm = VM::from_all_file_commands(all_file_commands);
        vm.run_state.current_file_index = vm.program.file_name_to_index["a"];
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.run_state.ram.stack_top(), 1337 + 2337);
    }

    #[test]
    fn test_sub() {
        let all_file_commands = vec![(
            "a".to_owned(),
            vec![
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 2337,
                },
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Sub,
            ],
        )];

        let mut vm = VM::from_all_file_commands(all_file_commands);
        vm.run_state.current_file_index = vm.program.file_name_to_index["a"];
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.run_state.ram.stack_top(), 2337 - 1337);
    }

    #[test]
    fn test_neg() {
        let all_file_commands = vec![(
            "a".to_owned(),
            vec![
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Neg,
            ],
        )];

        let mut vm = VM::from_all_file_commands(all_file_commands);
        vm.run_state.current_file_index = vm.program.file_name_to_index["a"];
        vm.step();
        vm.step();

        assert_eq!(*vm.run_state.ram.stack_top(), -1337);
    }

    #[test]
    fn test_eq() {
        let all_file_commands = vec![(
            "a".to_owned(),
            vec![
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 2337,
                },
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Eq,
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Eq,
            ],
        )];

        let mut vm = VM::from_all_file_commands(all_file_commands);
        vm.run_state.current_file_index = vm.program.file_name_to_index["a"];
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.run_state.ram.stack_top(), 0);

        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.run_state.ram.stack_top(), -1);
    }

    #[test]
    fn test_gt() {
        let all_file_commands = vec![(
            "a".to_owned(),
            vec![
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 2337,
                },
                VMCommand::Gt,
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 2337,
                },
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Gt,
            ],
        )];

        let mut vm = VM::from_all_file_commands(all_file_commands);
        vm.run_state.current_file_index = vm.program.file_name_to_index["a"];
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.run_state.ram.stack_top(), 0);

        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.run_state.ram.stack_top(), -1);
    }

    #[test]
    fn test_lt() {
        let all_file_commands = vec![(
            "a".to_owned(),
            vec![
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 2337,
                },
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Lt,
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 2337,
                },
                VMCommand::Lt,
            ],
        )];

        let mut vm = VM::from_all_file_commands(all_file_commands);
        vm.run_state.current_file_index = vm.program.file_name_to_index["a"];
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.run_state.ram.stack_top(), 0);

        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.run_state.ram.stack_top(), -1);
    }

    #[test]
    fn test_and() {
        let all_file_commands = vec![(
            "a".to_owned(),
            vec![
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 2337,
                },
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::And,
            ],
        )];

        let mut vm = VM::from_all_file_commands(all_file_commands);
        vm.run_state.current_file_index = vm.program.file_name_to_index["a"];
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.run_state.ram.stack_top(), 1337 & 2337);
    }

    #[test]
    fn test_or() {
        let all_file_commands = vec![(
            "a".to_owned(),
            vec![
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 2337,
                },
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Or,
            ],
        )];

        let mut vm = VM::from_all_file_commands(all_file_commands);
        vm.run_state.current_file_index = vm.program.file_name_to_index["a"];
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.run_state.ram.stack_top(), 1337 | 2337);
    }

    #[test]
    fn test_not() {
        let all_file_commands = vec![(
            "a".to_owned(),
            vec![
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Not,
            ],
        )];

        let mut vm = VM::from_all_file_commands(all_file_commands);
        vm.run_state.current_file_index = vm.program.file_name_to_index["a"];
        vm.step();
        vm.step();

        assert_eq!(*vm.run_state.ram.stack_top(), -1338);
    }

    #[test]
    fn test_label() {
        let all_file_commands = vec![(
            "Sys".to_owned(),
            vec![
                VMCommand::Function {
                    name: "Sys.init".to_owned(),
                    local_var_count: 0,
                },
                VMCommand::Label {
                    name: "foo".to_owned(),
                },
            ],
        )];

        let mut vm = VM::from_all_file_commands(all_file_commands.clone());
        let vm2 = VM::from_all_file_commands(all_file_commands);
        vm.step();
        vm.step();

        assert_eq!(vm.run_state.ram, vm2.run_state.ram);
    }

    #[test]
    fn test_goto() {
        let all_file_commands = vec![(
            "Sys".to_owned(),
            vec![
                VMCommand::Function {
                    name: "Sys.init".to_owned(),
                    local_var_count: 0,
                },
                VMCommand::Goto {
                    label_name: "foo".to_owned(),
                },
                VMCommand::Label {
                    name: "bar".to_owned(),
                },
                VMCommand::Label {
                    name: "foo".to_owned(),
                },
            ],
        )];

        let mut vm = VM::from_all_file_commands(all_file_commands.clone());
        let vm2 = VM::from_all_file_commands(all_file_commands);
        vm.step();
        vm.step();

        assert_eq!(vm.run_state.ram, vm2.run_state.ram);
        assert_eq!(vm.run_state.current_command_index, 3);
    }

    #[test]
    fn test_if_goto() {
        let all_file_commands = vec![(
            "Sys".to_owned(),
            vec![
                VMCommand::Function {
                    name: "Sys.init".to_owned(),
                    local_var_count: 0,
                },
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 0,
                },
                VMCommand::IfGoto {
                    label_name: "bar".to_owned(),
                },
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1,
                },
                VMCommand::IfGoto {
                    label_name: "foo".to_owned(),
                },
                VMCommand::Label {
                    name: "bar".to_owned(),
                },
                VMCommand::Label {
                    name: "foo".to_owned(),
                },
            ],
        )];

        let mut vm = VM::from_all_file_commands(all_file_commands.clone());
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(vm.run_state.current_command_index, 3);

        vm.step();
        vm.step();

        assert_eq!(vm.run_state.current_command_index, 6);
    }

    #[test]
    fn test_call_return() {
        let all_file_commands = vec![(
            "Sys".to_owned(),
            vec![
                VMCommand::Function {
                    name: "Sys.init".to_owned(),
                    local_var_count: 0,
                },
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Call {
                    function_name: "Sys.foo".to_owned(),
                    argument_count: 1,
                },
                VMCommand::Label {
                    name: "nop".to_owned(),
                },
                VMCommand::Function {
                    name: "Sys.foo".to_owned(),
                    local_var_count: 1,
                },
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 2337,
                },
                VMCommand::Return,
            ],
        )];

        let mut vm = VM::from_all_file_commands(all_file_commands.clone());
        vm.step();
        vm.step();
        vm.step();
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(vm.run_state.current_command_index, 3);
        assert_eq!(*vm.run_state.ram.stack_top(), 2337);
    }
}
