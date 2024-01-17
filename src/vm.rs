use hashbrown::{HashMap, HashSet};
use std::{
    fs,
    ops::{Index, IndexMut, RangeInclusive},
    path::Path,
};

use crate::{hardware::RAM, os::OS, vm_parse::commands};

impl Index<Register> for RAM {
    type Output = i16;

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

    pub fn push(&mut self, value: i16) {
        let sp = self[Register::SP];
        self[sp] = value;
        self[Register::SP] += 1;
    }

    fn pop(&mut self) -> i16 {
        self[Register::SP] -= 1;
        self[self[Register::SP]]
    }

    fn stack_top(&mut self) -> &mut i16 {
        let sp = self[Register::SP];
        &mut self[sp - 1]
    }

    fn set(&mut self, static_segment: i16, segment: PopSegment, offset: i16, value: i16) {
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

    pub fn get(&self, static_segment: i16, segment: PushSegment, offset: i16) -> i16 {
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

pub struct Program {
    pub files: HashMap<String, File>,
    pub file_name_to_static_segment: HashMap<String, RangeInclusive<i16>>,
}

pub struct RunState {
    pub current_file_name: String,
    pub current_command_index: usize,
    pub ram: RAM,
    pub os: OS,
    pub call_stack: Vec<Frame>,
}

pub struct VM {
    pub run_state: RunState,
    pub program: Program,
}

impl VM {
    pub fn from_dir(path: impl AsRef<Path>) -> Self {
        let paths = fs::read_dir(path).unwrap();
        let files = paths
            .map(|path| path.unwrap())
            .filter(|path| path.file_name().to_str().unwrap().ends_with(".vm"))
            .map(|path| {
                (
                    path.file_name().to_str().unwrap().to_owned(),
                    fs::read_to_string(path.path()).unwrap(),
                )
            })
            .collect();

        Self::from_file_contents(files)
    }

    pub fn from_file_contents(file_contents: Vec<(String, String)>) -> Self {
        Self::new(
            file_contents
                .into_iter()
                .map(|(name, contents)| {
                    (
                        name.rsplit_once('.').unwrap().0.to_owned(),
                        File::new(commands(&contents).unwrap().1),
                    )
                })
                .collect(),
        )
    }

    pub fn new(files: Vec<(String, File)>) -> Self {
        let file_name_to_static_segment = Self::create_file_name_to_static_segment(&files);
        let program = Program {
            files: files.into_iter().collect(),
            file_name_to_static_segment,
        };

        Self {
            program,
            run_state: RunState {
                current_file_name: "Sys".to_owned(),
                current_command_index: 0,
                ram: RAM::new(),
                os: Default::default(),
                call_stack: vec![Frame {
                    file_name: "Sys".to_owned(),
                    function_name: "Sys.init".to_owned(),
                }],
            },
        }
    }

    pub fn reset(&mut self) {
        *self = VM::new(self.program.files.clone().into_iter().collect());
    }

    fn create_file_name_to_static_segment(
        files: &Vec<(String, File)>,
    ) -> HashMap<String, RangeInclusive<i16>> {
        let mut map: HashMap<String, RangeInclusive<i16>> = HashMap::new();
        let mut index = 16i16;
        for (file_name, file) in files {
            let static_vars: HashSet<i16> = file
                .commands
                .iter()
                .filter_map(|cmd| match cmd {
                    VMCommand::Push {
                        segment: PushSegment::Static,
                        offset,
                    }
                    | VMCommand::Pop {
                        segment: PopSegment::Static,
                        offset,
                    } => Some(*offset + 1),
                    _ => None,
                })
                .collect();
            let num_vars = *static_vars.iter().max().unwrap_or(&0);
            map.insert(file_name.clone(), index..=(index + num_vars - 1));
            index += num_vars;
        }
        map
    }

    pub fn step(&mut self) {
        self.run(1)
    }

    pub fn run(&mut self, num_steps: u64) {
        let mut steps_remaining = num_steps;
        while steps_remaining > 0 {
            let commands = &self.program.files[&self.run_state.current_file_name].commands;
            let static_segment =
                &self.program.file_name_to_static_segment[&self.run_state.current_file_name];
            steps_remaining -= Self::run_commands(
                &mut self.run_state,
                commands,
                *static_segment.start(),
                &self.program.files,
                steps_remaining,
            );
        }
    }

    pub fn run_commands(
        run_state: &mut RunState,
        commands: &[VMCommand],
        static_segment: i16,
        files: &HashMap<String, File>,
        num_steps: u64,
    ) -> u64 {
        // if self.call_stack.last().unwrap().function_name.eq("Math.divide") {
        //     println!(
        //         "{:?} RAM[LCL1]:{:?} RAM[SP]:{:?} RAM[SP-1]:{:?} SP:{:?} LCL:{:?} ARG:{:?} THIS:{:?} THAT:{:?}",
        //         self.files[&self.current_file_name].commands[self.current_command_index],
        //         self.ram[self.ram[Register::LCL] + 1], self.ram[self.ram[Register::SP] -1], self.ram[self.ram[Register::SP] - 2], self.ram[Register::SP], self.ram[Register::LCL], self.ram[Register::ARG], self.ram[Register::THIS], self.ram[Register::THAT]
        //     );
        // }
        for steps_done in 1..=num_steps {
            match &commands[run_state.current_command_index] {
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
                    *y = -*y;
                    run_state.current_command_index += 1;
                }
                VMCommand::Eq => {
                    let y = run_state.ram.pop();
                    let x = run_state.ram.stack_top();
                    *x = -((*x == y) as i16);
                    run_state.current_command_index += 1;
                }
                VMCommand::Gt => {
                    let y = run_state.ram.pop();
                    let x = run_state.ram.stack_top();
                    *x = -((*x > y) as i16);
                    run_state.current_command_index += 1;
                }
                VMCommand::Lt => {
                    let y = run_state.ram.pop();
                    // let (this, that, wat) = (self.ram[Register::THIS], self.ram[Register::THAT], self.ram[self.ram[Register::THAT]]);
                    let x = run_state.ram.stack_top();
                    // println!("x:{:?} y: {:?} this: {:?} that: {:?}, ram[THAT]: {:?} ", x, y, this, that, wat);
                    *x = -((*x < y) as i16);
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
                        &run_state.current_file_name,
                        files,
                        &run_state.call_stack,
                        label_name,
                    );
                }
                VMCommand::IfGoto { label_name } => {
                    let value = run_state.ram.pop();
                    if value != 0 {
                        Self::goto(
                            &mut run_state.current_command_index,
                            &run_state.current_file_name,
                            files,
                            &run_state.call_stack,
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
                    let argument_segment = run_state.ram[Register::SP] - argument_count;
                    run_state
                        .ram
                        .push((run_state.current_command_index + 1) as i16);
                    for i in 1..=4 {
                        let value = run_state.ram[i];
                        run_state.ram.push(value);
                    }

                    // println!("{function_name}");

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
                        let (file_name, _) = function_name.split_once('.').unwrap();
                        run_state.call_stack.push(Frame {
                            file_name: file_name.to_owned(),
                            function_name: function_name.to_owned(),
                        });

                        run_state.current_file_name = file_name.to_owned();
                        run_state.current_command_index =
                            files[file_name].function_name_to_command_index[function_name];

                        return steps_done;
                    }
                }
                VMCommand::Return => {
                    // println!("return");
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
                    run_state.current_file_name =
                        run_state.call_stack.last().unwrap().file_name.clone();

                    return steps_done;
                }
            }
        }

        num_steps
    }

    fn goto(
        current_command_index: &mut usize,
        current_file_name: &String,
        files: &HashMap<String, File>,
        call_stack: &[Frame],
        label_name: &str,
    ) {
        *current_command_index = files[current_file_name].label_name_to_command_index[&(
            call_stack.last().unwrap().function_name.clone(),
            label_name.to_owned(),
        )];
    }
}

pub struct Frame {
    file_name: String,
    pub function_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionMetadata {
    pub argument_count: i16,
    pub local_var_count: i16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    pub commands: Vec<VMCommand>,
    label_name_to_command_index: HashMap<(String, String), usize>,
    function_name_to_command_index: HashMap<String, usize>,
    pub function_metadata: HashMap<String, FunctionMetadata>,
}

impl File {
    pub fn new(commands: Vec<VMCommand>) -> Self {
        let mut current_function: Option<String> = None;
        let mut current_local_var_count: i16 = 0;
        let mut max_argument_encountered: i16 = 0;
        let mut label_name_to_command_index: HashMap<(String, String), usize> = HashMap::new();
        let mut function_name_to_command_index: HashMap<String, usize> = HashMap::new();
        let mut function_metadata: HashMap<String, FunctionMetadata> = HashMap::new();
        for (i, command) in commands.iter().enumerate() {
            match command {
                VMCommand::Label { name } => {
                    label_name_to_command_index
                        .insert((current_function.clone().unwrap(), name.clone()), i);
                }
                VMCommand::Function {
                    name,
                    local_var_count,
                } => {
                    if let Some(previous_function) = current_function {
                        function_metadata.insert(
                            previous_function,
                            FunctionMetadata {
                                argument_count: max_argument_encountered,
                                local_var_count: current_local_var_count,
                            },
                        );
                    }
                    current_function = Some(name.clone());
                    current_local_var_count = *local_var_count;
                    function_name_to_command_index.insert(name.clone(), i);
                }
                VMCommand::Pop {
                    segment: PopSegment::Argument,
                    offset,
                }
                | VMCommand::Push {
                    segment: PushSegment::Argument,
                    offset,
                } => {
                    max_argument_encountered = i16::max(max_argument_encountered, *offset);
                }
                _ => {}
            }
        }
        if let Some(previous_function) = current_function {
            function_metadata.insert(
                previous_function,
                FunctionMetadata {
                    argument_count: max_argument_encountered,
                    local_var_count: current_local_var_count,
                },
            );
        }

        File {
            commands,
            label_name_to_command_index,
            function_name_to_command_index,
            function_metadata,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VMCommand {
    Add,
    Push {
        segment: PushSegment,
        offset: i16,
    },
    Pop {
        segment: PopSegment,
        offset: i16,
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
        local_var_count: i16,
    },
    Call {
        function_name: String,
        argument_count: i16,
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
    TEMP(i16),
}

impl Register {
    fn address(&self) -> i16 {
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

#[cfg(test)]
mod tests {
    use super::*;

    impl VM {
        fn test_get(&self, segment: PushSegment, offset: i16) -> i16 {
            self.run_state.ram.get(
                *self.program.file_name_to_static_segment[&self.run_state.current_file_name]
                    .start(),
                segment,
                offset,
            )
        }

        fn test_set(&mut self, segment: PopSegment, offset: i16, value: i16) {
            self.run_state.ram.set(
                *self.program.file_name_to_static_segment[&self.run_state.current_file_name]
                    .start(),
                segment,
                offset,
                value,
            )
        }

        fn test_instance() -> VM {
            let mut vm = VM::new(vec![(
                "foo".to_owned(),
                File::new(vec![VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 666,
                }]),
            )]);
            vm.run_state.current_file_name = "foo".to_owned();

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
        let files = vec![
            (
                "a".to_owned(),
                File::new(vec![
                    VMCommand::Push {
                        segment: PushSegment::Static,
                        offset: 0,
                    },
                    VMCommand::Pop {
                        segment: PopSegment::Static,
                        offset: 1,
                    },
                ]),
            ),
            (
                "b".to_owned(),
                File::new(vec![
                    VMCommand::Push {
                        segment: PushSegment::Static,
                        offset: 1,
                    },
                    VMCommand::Pop {
                        segment: PopSegment::Static,
                        offset: 0,
                    },
                ]),
            ),
        ];

        let mut vm = VM::new(files);
        vm.run_state.current_file_name = "a".to_owned();

        vm.test_set(PopSegment::Static, 0, 1337);
        vm.run_state.current_file_name = "b".to_owned();
        vm.test_set(PopSegment::Static, 1, 2337);

        assert_eq!(vm.test_get(PushSegment::Static, 0), 0);
        assert_eq!(vm.test_get(PushSegment::Static, 1), 2337);
        vm.run_state.current_file_name = "a".to_owned();
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
        let files = vec![(
            "a".to_owned(),
            File::new(vec![
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 2337,
                },
                VMCommand::Add,
            ]),
        )];

        let mut vm = VM::new(files);
        vm.run_state.current_file_name = "a".to_owned();
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.run_state.ram.stack_top(), 1337 + 2337);
    }

    #[test]
    fn test_sub() {
        let files = vec![(
            "a".to_owned(),
            File::new(vec![
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 2337,
                },
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Sub,
            ]),
        )];

        let mut vm = VM::new(files);
        vm.run_state.current_file_name = "a".to_owned();
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.run_state.ram.stack_top(), 2337 - 1337);
    }

    #[test]
    fn test_neg() {
        let files = vec![(
            "a".to_owned(),
            File::new(vec![
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Neg,
            ]),
        )];

        let mut vm = VM::new(files);
        vm.run_state.current_file_name = "a".to_owned();
        vm.step();
        vm.step();

        assert_eq!(*vm.run_state.ram.stack_top(), -1337);
    }

    #[test]
    fn test_eq() {
        let files = vec![(
            "a".to_owned(),
            File::new(vec![
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
            ]),
        )];

        let mut vm = VM::new(files);
        vm.run_state.current_file_name = "a".to_owned();
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
        let files = vec![(
            "a".to_owned(),
            File::new(vec![
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
            ]),
        )];

        let mut vm = VM::new(files);
        vm.run_state.current_file_name = "a".to_owned();
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
        let files = vec![(
            "a".to_owned(),
            File::new(vec![
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
            ]),
        )];

        let mut vm = VM::new(files);
        vm.run_state.current_file_name = "a".to_owned();
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
        let files = vec![(
            "a".to_owned(),
            File::new(vec![
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 2337,
                },
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::And,
            ]),
        )];

        let mut vm = VM::new(files);
        vm.run_state.current_file_name = "a".to_owned();
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.run_state.ram.stack_top(), 1337 & 2337);
    }

    #[test]
    fn test_or() {
        let files = vec![(
            "a".to_owned(),
            File::new(vec![
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 2337,
                },
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Or,
            ]),
        )];

        let mut vm = VM::new(files);
        vm.run_state.current_file_name = "a".to_owned();
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.run_state.ram.stack_top(), 1337 | 2337);
    }

    #[test]
    fn test_not() {
        let files = vec![(
            "a".to_owned(),
            File::new(vec![
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Not,
            ]),
        )];

        let mut vm = VM::new(files);
        vm.run_state.current_file_name = "a".to_owned();
        vm.step();
        vm.step();

        assert_eq!(*vm.run_state.ram.stack_top(), -1338);
    }

    #[test]
    fn test_label() {
        let files = vec![(
            "Sys".to_owned(),
            File::new(vec![
                VMCommand::Function {
                    name: "Sys.init".to_owned(),
                    local_var_count: 0,
                },
                VMCommand::Label {
                    name: "foo".to_owned(),
                },
            ]),
        )];

        let mut vm = VM::new(files.clone());
        let vm2 = VM::new(files);
        vm.step();
        vm.step();

        assert_eq!(vm.run_state.ram, vm2.run_state.ram);
    }

    #[test]
    fn test_goto() {
        let files = vec![(
            "Sys".to_owned(),
            File::new(vec![
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
            ]),
        )];

        let mut vm = VM::new(files.clone());
        let vm2 = VM::new(files);
        vm.step();
        vm.step();

        assert_eq!(vm.run_state.ram, vm2.run_state.ram);
        assert_eq!(vm.run_state.current_command_index, 3);
    }

    #[test]
    fn test_if_goto() {
        let files = vec![(
            "Sys".to_owned(),
            File::new(vec![
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
            ]),
        )];

        let mut vm = VM::new(files.clone());
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
        let files = vec![(
            "Sys".to_owned(),
            File::new(vec![
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
            ]),
        )];

        let mut vm = VM::new(files.clone());
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
