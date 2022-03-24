use std::{
    collections::{HashMap, HashSet},
    ops::{Index, IndexMut},
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct RAM {
    contents: [u16; 32 * 1024],
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

impl Index<Register> for RAM {
    type Output = u16;

    fn index(&self, index: Register) -> &Self::Output {
        &self.contents[index.address() as usize]
    }
}

impl IndexMut<Register> for RAM {
    fn index_mut(&mut self, index: Register) -> &mut Self::Output {
        &mut self.contents[index.address() as usize]
    }
}

impl RAM {
    fn new() -> Self {
        let mut instance = Self {
            contents: [0; 32 * 1024],
        };
        instance[Register::SP] = 256;

        instance
    }

    fn push(&mut self, value: u16) {
        let sp = self[Register::SP];
        self[sp] = value;
        self[Register::SP] += 1;
    }

    fn pop(&mut self) -> u16 {
        self[Register::SP] -= 1;
        self[self[Register::SP]]
    }

    fn stack_top(&mut self) -> &mut u16 {
        let sp = self[Register::SP];
        &mut self[sp - 1]
    }

    fn set(
        &mut self,
        file_name_to_static_segment: &HashMap<String, u16>,
        current_file_name: &String,
        segment: PopSegment,
        offset: u16,
        value: u16,
    ) {
        match segment {
            PopSegment::Static => {
                self[file_name_to_static_segment[current_file_name] + offset] = value;
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

    fn get(
        &self,
        file_name_to_static_segment: &HashMap<String, u16>,
        current_file_name: &String,
        segment: PushSegment,
        offset: u16,
    ) -> u16 {
        match segment {
            PushSegment::Constant => offset,
            PushSegment::Static => self[file_name_to_static_segment[current_file_name] + offset],
            PushSegment::Local => self[self[Register::LCL] + offset],
            PushSegment::Argument => self[self[Register::ARG] + offset],
            PushSegment::This => self[self[Register::THIS] + offset],
            PushSegment::That => self[self[Register::THAT] + offset],
            PushSegment::Temp => self[Register::TEMP(offset)],
            PushSegment::Pointer => self[Register::THIS.address() + offset],
        }
    }
}

pub struct VM {
    current_file_name: String,
    current_command_index: usize,
    files: HashMap<String, File>,
    ram: RAM,
    file_name_to_static_segment: HashMap<String, u16>,
    call_stack: Vec<Frame>,
}

impl VM {
    pub fn new(files: Vec<(String, File)>) -> Self {
        let file_name_to_static_segment = Self::create_file_name_to_static_segment(&files);

        Self {
            current_file_name: "".to_owned(),
            current_command_index: 0,
            files: files.into_iter().collect(),
            ram: RAM::new(),
            file_name_to_static_segment,
            call_stack: Vec::new(),
        }
    }

    fn create_file_name_to_static_segment(files: &Vec<(String, File)>) -> HashMap<String, u16> {
        let mut map: HashMap<String, u16> = HashMap::new();
        let mut index = 0u16;
        for (file_name, file) in files {
            map.insert(file_name.clone(), index);
            let static_vars: HashSet<u16> = file
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
            index += *static_vars.iter().max().unwrap_or(&0);
        }
        map
    }

    fn step(&mut self) {
        match &self.files[&self.current_file_name].commands[self.current_command_index] {
            VMCommand::Add => {
                let y = self.ram.pop();
                *self.ram.stack_top() += y;
                self.current_command_index += 1;
            }
            VMCommand::Push { segment, offset } => {
                let value = self.ram.get(
                    &self.file_name_to_static_segment,
                    &self.current_file_name,
                    *segment,
                    *offset,
                );
                self.ram.push(value);
                self.current_command_index += 1;
            }
            VMCommand::Pop { segment, offset } => {
                let value = self.ram.pop();
                self.ram.set(
                    &self.file_name_to_static_segment,
                    &self.current_file_name,
                    *segment,
                    *offset,
                    value,
                );
                self.current_command_index += 1;
            }
            VMCommand::Sub => {
                let y = self.ram.pop();
                *self.ram.stack_top() -= y;
                self.current_command_index += 1;
            }
            VMCommand::Neg => {
                let y = self.ram.stack_top();
                *y = -(*y as i16) as u16;
                self.current_command_index += 1;
            }
            VMCommand::Eq => {
                let y = self.ram.pop();
                let x = self.ram.stack_top();
                *x = (*x == y) as u16 * u16::MAX;
                self.current_command_index += 1;
            }
            VMCommand::Gt => {
                let y = self.ram.pop();
                let x = self.ram.stack_top();
                *x = (*x > y) as u16 * u16::MAX;
                self.current_command_index += 1;
            }
            VMCommand::Lt => {
                let y = self.ram.pop();
                let x = self.ram.stack_top();
                *x = (*x < y) as u16 * u16::MAX;
                self.current_command_index += 1;
            }
            VMCommand::And => {
                let y = self.ram.pop();
                *self.ram.stack_top() &= y;
                self.current_command_index += 1;
            }
            VMCommand::Or => {
                let y = self.ram.pop();
                *self.ram.stack_top() |= y;
                self.current_command_index += 1;
            }
            VMCommand::Not => {
                *self.ram.stack_top() ^= u16::MAX;
                self.current_command_index += 1;
            }
            VMCommand::Label { name: _ } => {
                self.current_command_index += 1;
            }
            VMCommand::Goto { label_name } => {
                Self::goto(
                    &mut self.current_command_index,
                    &self.current_file_name,
                    &self.files,
                    label_name,
                );
            }
            VMCommand::IfGoto { label_name } => {
                let value = self.ram.pop();
                if value != 0 {
                    Self::goto(
                        &mut self.current_command_index,
                        &self.current_file_name,
                        &self.files,
                        label_name,
                    );
                } else {
                    self.current_command_index += 1;
                }
            }
            VMCommand::Function {
                name: _,
                local_var_count,
            } => {
                for _ in 0..*local_var_count {
                    self.ram.push(0);
                }
                self.current_command_index += 1;
            }
            VMCommand::Call {
                function_name,
                argument_count,
            } => {
                let argument_segment = self.ram[Register::SP] - argument_count;
                self.ram.push((self.current_command_index + 1) as u16);
                for i in 1..=4 {
                    let value = self.ram[i];
                    self.ram.push(value);
                }
                let (file_name, actual_function_name) = function_name.split_once('.').unwrap();
                self.call_stack.push(Frame {
                    file_name: file_name.to_owned(),
                    function_name: actual_function_name.to_owned(),
                });
                let local_segment = self.ram[Register::SP];
                println!("argument segment: {:?}", argument_segment);
                self.ram[Register::LCL] = local_segment;
                self.ram[Register::ARG] = argument_segment;
                self.current_file_name = file_name.to_owned();
                self.current_command_index =
                    self.files[file_name].function_name_to_command_index[function_name];
            }
            VMCommand::Return => {
                let frame = self.ram[Register::LCL];
                self.current_command_index = self.ram[frame - 5] as usize;
                let return_value = self.ram.pop();
                println!("return value: {:?}", return_value);
                self.ram.set(
                    &self.file_name_to_static_segment,
                    &self.current_file_name,
                    PopSegment::Argument,
                    0,
                    return_value,
                );
                self.ram[Register::SP] = self.ram[Register::ARG] + 1;
                for i in 1..=4 {
                    self.ram[i] = self.ram[frame - 5 + i];
                }
                self.call_stack.pop();
                self.current_file_name = self.call_stack.last().unwrap().file_name.clone();
            }
        }
    }

    fn goto(
        current_command_index: &mut usize,
        current_file_name: &String,
        files: &HashMap<String, File>,
        label_name: &String,
    ) {
        *current_command_index = files[current_file_name].label_name_to_command_index[label_name];
    }
}

struct Frame {
    file_name: String,
    function_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    commands: Vec<VMCommand>,
    label_name_to_command_index: HashMap<String, usize>,
    function_name_to_command_index: HashMap<String, usize>,
}

impl File {
    fn new(commands: Vec<VMCommand>) -> Self {
        let label_name_to_command_index = commands
            .iter()
            .enumerate()
            .filter_map(|(i, command)| {
                if let VMCommand::Label { name } = command {
                    Some((name.clone(), i))
                } else {
                    None
                }
            })
            .collect();

        let function_name_to_command_index = commands
            .iter()
            .enumerate()
            .filter_map(|(i, command)| {
                if let VMCommand::Function {
                    name,
                    local_var_count: _,
                } = command
                {
                    Some((name.clone(), i))
                } else {
                    None
                }
            })
            .collect();

        File {
            commands,
            label_name_to_command_index,
            function_name_to_command_index,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VMCommand {
    Add,
    Push {
        segment: PushSegment,
        offset: u16,
    },
    Pop {
        segment: PopSegment,
        offset: u16,
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
        local_var_count: u16,
    },
    Call {
        function_name: String,
        argument_count: u16,
    },
    Return,
}

enum Register {
    SP,
    LCL,
    ARG,
    THIS,
    THAT,
    TEMP(u16),
}

impl Register {
    fn address(&self) -> u16 {
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

mod tests {
    use super::*;

    impl VM {
        fn test_get(&self, segment: PushSegment, offset: u16) -> u16 {
            self.ram.get(
                &self.file_name_to_static_segment,
                &self.current_file_name,
                segment,
                offset,
            )
        }

        fn test_set(&mut self, segment: PopSegment, offset: u16, value: u16) {
            self.ram.set(
                &self.file_name_to_static_segment,
                &self.current_file_name,
                segment,
                offset,
                value,
            )
        }
    }

    #[test]
    fn test_constant() {
        let vm = VM::new(vec![]);

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
        vm.current_file_name = "a".to_owned();

        vm.test_set(PopSegment::Static, 0, 1337);
        vm.current_file_name = "b".to_owned();
        vm.test_set(PopSegment::Static, 1, 2337);

        assert_eq!(vm.test_get(PushSegment::Static, 0), 0);
        assert_eq!(vm.test_get(PushSegment::Static, 1), 2337);
        vm.current_file_name = "a".to_owned();
        assert_eq!(vm.test_get(PushSegment::Static, 0), 1337);
        assert_eq!(vm.test_get(PushSegment::Static, 1), 0);
    }

    #[test]
    fn test_local() {
        let mut vm = VM::new(vec![]);
        vm.ram[1] = 1337;

        vm.test_set(PopSegment::Local, 0, 2337);
        vm.test_set(PopSegment::Local, 3, 3337);

        assert_eq!(vm.test_get(PushSegment::Local, 0), 2337);
        assert_eq!(vm.test_get(PushSegment::Local, 3), 3337);
        assert_eq!(vm.ram[1337], 2337);
        assert_eq!(vm.ram[1340], 3337);
    }

    #[test]
    fn test_argument() {
        let mut vm = VM::new(vec![]);
        vm.ram[2] = 1337;

        vm.test_set(PopSegment::Argument, 0, 2337);
        vm.test_set(PopSegment::Argument, 3, 3337);

        assert_eq!(vm.test_get(PushSegment::Argument, 0), 2337);
        assert_eq!(vm.test_get(PushSegment::Argument, 3), 3337);
        assert_eq!(vm.ram[1337], 2337);
        assert_eq!(vm.ram[1340], 3337);
    }

    #[test]
    fn test_this() {
        let mut vm = VM::new(vec![]);
        vm.ram[3] = 1337;

        vm.test_set(PopSegment::This, 0, 2337);
        vm.test_set(PopSegment::This, 3, 3337);

        assert_eq!(vm.test_get(PushSegment::This, 0), 2337);
        assert_eq!(vm.test_get(PushSegment::This, 3), 3337);
        assert_eq!(vm.ram[1337], 2337);
        assert_eq!(vm.ram[1340], 3337);
    }

    #[test]
    fn test_that() {
        let mut vm = VM::new(vec![]);
        vm.ram[4] = 1337;

        vm.test_set(PopSegment::That, 0, 2337);
        vm.test_set(PopSegment::That, 3, 3337);

        assert_eq!(vm.test_get(PushSegment::That, 0), 2337);
        assert_eq!(vm.test_get(PushSegment::That, 3), 3337);
        assert_eq!(vm.ram[1337], 2337);
        assert_eq!(vm.ram[1340], 3337);
    }

    #[test]
    fn test_temp() {
        let mut vm = VM::new(vec![]);

        vm.test_set(PopSegment::Temp, 0, 1337);
        vm.test_set(PopSegment::Temp, 3, 2337);

        assert_eq!(vm.test_get(PushSegment::Temp, 0), 1337);
        assert_eq!(vm.test_get(PushSegment::Temp, 3), 2337);
        assert_eq!(vm.ram[5], 1337);
        assert_eq!(vm.ram[8], 2337);
    }

    #[test]
    fn test_pointer() {
        let mut vm = VM::new(vec![]);

        vm.test_set(PopSegment::Pointer, 0, 1337);
        vm.test_set(PopSegment::Pointer, 1, 2337);

        assert_eq!(vm.test_get(PushSegment::Pointer, 0), 1337);
        assert_eq!(vm.ram[3], 1337);
        assert_eq!(vm.test_get(PushSegment::Pointer, 1), 2337);
        assert_eq!(vm.ram[4], 2337);
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
        vm.current_file_name = "a".to_owned();
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.ram.stack_top(), 1337 + 2337);
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
        vm.current_file_name = "a".to_owned();
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.ram.stack_top(), 2337 - 1337);
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
        vm.current_file_name = "a".to_owned();
        vm.step();
        vm.step();

        assert_eq!(*vm.ram.stack_top(), 64199);
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
        vm.current_file_name = "a".to_owned();
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.ram.stack_top(), 0);

        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.ram.stack_top(), 0xFFFF);
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
        vm.current_file_name = "a".to_owned();
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.ram.stack_top(), 0);

        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.ram.stack_top(), 0xFFFF);
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
        vm.current_file_name = "a".to_owned();
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.ram.stack_top(), 0);

        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.ram.stack_top(), 0xFFFF);
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
        vm.current_file_name = "a".to_owned();
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.ram.stack_top(), 1337 & 2337);
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
        vm.current_file_name = "a".to_owned();
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(*vm.ram.stack_top(), 1337 | 2337);
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
        vm.current_file_name = "a".to_owned();
        vm.step();
        vm.step();

        assert_eq!(*vm.ram.stack_top(), 64198);
    }

    #[test]
    fn test_label() {
        let files = vec![(
            "a".to_owned(),
            File::new(vec![VMCommand::Label {
                name: "foo".to_owned(),
            }]),
        )];

        let mut vm = VM::new(files.clone());
        let vm2 = VM::new(files);
        vm.current_file_name = "a".to_owned();
        vm.step();

        assert_eq!(vm.ram, vm2.ram);
    }

    #[test]
    fn test_goto() {
        let files = vec![(
            "a".to_owned(),
            File::new(vec![
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
        vm.current_file_name = "a".to_owned();
        vm.step();

        assert_eq!(vm.ram, vm2.ram);
        assert_eq!(vm.current_command_index, 2);
    }

    #[test]
    fn test_if_goto() {
        let files = vec![(
            "a".to_owned(),
            File::new(vec![
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
        vm.current_file_name = "a".to_owned();
        vm.step();
        vm.step();

        assert_eq!(vm.current_command_index, 2);

        vm.step();
        vm.step();

        assert_eq!(vm.current_command_index, 5);
    }

    #[test]
    fn test_call_return() {
        let files = vec![(
            "a".to_owned(),
            File::new(vec![
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 1337,
                },
                VMCommand::Call {
                    function_name: "a.foo".to_owned(),
                    argument_count: 1,
                },
                VMCommand::Label {
                    name: "nop".to_owned(),
                },
                VMCommand::Function {
                    name: "a.foo".to_owned(),
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
        vm.call_stack.push(Frame {
            file_name: "a".to_owned(),
            function_name: "bar".to_owned(),
        });
        vm.current_file_name = "a".to_owned();
        vm.step();
        vm.step();
        vm.step();
        vm.step();
        vm.step();

        assert_eq!(vm.current_command_index, 2);
        assert_eq!(*vm.ram.stack_top(), 2337);
    }
}
