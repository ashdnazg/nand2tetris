use std::collections::{HashMap, HashSet};

type RAM = [u16; 32 * 1024];

pub struct VM {
    current_file_name: String,
    current_command_index: usize,
    files: HashMap<String, File>,
    ram: RAM,
    file_name_to_static_segment: HashMap<String, usize>,
    call_stack: Vec<Frame>,
}

impl VM {
    pub fn new(files: Vec<(String, File)>) -> Self {
        let mut ram = [0; 32 * 1024];
        ram[0] = 256;

        let file_name_to_static_segment = Self::create_file_name_to_static_segment(&files);

        Self {
            current_file_name: "".to_string(),
            current_command_index: 0,
            files: files.into_iter().collect(),
            ram: ram,
            file_name_to_static_segment: file_name_to_static_segment,
            call_stack: Vec::new(),
        }
    }

    fn create_file_name_to_static_segment(files: &Vec<(String, File)>) -> HashMap<String, usize> {
        let mut map: HashMap<String, usize> = HashMap::new();
        let mut index = 0u8;
        for (file_name, file) in files {
            map.insert(file_name.clone(), index as usize);
            let static_vars: HashSet<u8> = file
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
                    } => Some(*offset as u8),
                    _ => None,
                })
                .collect();
            index += *static_vars.iter().max().unwrap_or(&0) as u8;
        }
        return map;
    }

    fn push(ram: &mut RAM, value: u16) {
        ram[ram[0] as usize] = value;
        ram[0] += 1;
    }

    fn pop(ram: &mut RAM) -> u16 {
        ram[0] -= 1;
        ram[ram[0] as usize]
    }

    fn stack_top(ram: &mut RAM) -> &mut u16 {
        &mut ram[(ram[0] - 1) as usize]
    }

    fn set(
        ram: &mut RAM,
        file_name_to_static_segment: &HashMap<String, usize>,
        current_file_name: &String,
        segment: PopSegment,
        offset: u16,
        value: u16,
    ) {
        match segment {
            PopSegment::Static => {
                let static_ram_offset =
                    Self::static_ram_offset(file_name_to_static_segment, current_file_name, offset);
                ram[static_ram_offset] = value;
            }
            PopSegment::Local => {
                ram[(ram[1] + offset) as usize] = value;
            }
            PopSegment::Argument => {
                ram[(ram[2] + offset) as usize] = value;
            }
            PopSegment::This => {
                ram[(ram[3] + offset) as usize] = value;
            }
            PopSegment::That => {
                ram[(ram[4] + offset) as usize] = value;
            }
            PopSegment::Temp => {
                ram[5 + offset as usize] = value;
            }
            PopSegment::Pointer => {
                ram[3 + offset as usize] = value;
            }
        }
    }

    fn get(
        ram: &RAM,
        file_name_to_static_segment: &HashMap<String, usize>,
        current_file_name: &String,
        segment: PushSegment,
        offset: u16,
    ) -> u16 {
        match segment {
            PushSegment::Constant => offset,
            PushSegment::Static => {
                let static_ram_offset =
                    Self::static_ram_offset(file_name_to_static_segment, current_file_name, offset);
                ram[static_ram_offset]
            }
            PushSegment::Local => ram[(ram[1] + offset) as usize],
            PushSegment::Argument => ram[(ram[2] + offset) as usize],
            PushSegment::This => ram[(ram[3] + offset) as usize],
            PushSegment::That => ram[(ram[4] + offset) as usize],
            PushSegment::Temp => ram[5 + offset as usize],
            PushSegment::Pointer => ram[3 + offset as usize],
        }
    }

    fn static_ram_offset(
        file_name_to_static_segment: &HashMap<String, usize>,
        current_file_name: &String,
        offset: u16,
    ) -> usize {
        file_name_to_static_segment[current_file_name] + (offset as usize)
    }

    fn step(&mut self) {
        match &self.files[&self.current_file_name].commands[self.current_command_index] {
            VMCommand::Add => {
                let y = Self::pop(&mut self.ram);
                *Self::stack_top(&mut self.ram) += y;
            }
            VMCommand::Push { segment, offset } => {
                let value = Self::get(
                    &self.ram,
                    &self.file_name_to_static_segment,
                    &self.current_file_name,
                    *segment,
                    *offset,
                );
                Self::push(&mut self.ram, value);
            }
            VMCommand::Pop { segment, offset } => {
                let value = Self::pop(&mut self.ram);
                Self::set(
                    &mut self.ram,
                    &self.file_name_to_static_segment,
                    &self.current_file_name,
                    *segment,
                    *offset,
                    value,
                );
            }
            VMCommand::Sub => {
                let y = Self::pop(&mut self.ram);
                *Self::stack_top(&mut self.ram) -= y;
            }
            VMCommand::Neg => {
                let y = Self::stack_top(&mut self.ram);
                *y = -(*y as i16) as u16;
            }
            VMCommand::Eq => {
                let y = Self::pop(&mut self.ram);
                let x = Self::stack_top(&mut self.ram);
                *x = (*x == y) as u16 * 0xFFFF;
            }
            VMCommand::Gt => {
                let y = Self::pop(&mut self.ram);
                let x = Self::stack_top(&mut self.ram);
                *x = (*x > y) as u16 * 0xFFFF;
            }
            VMCommand::Lt => {
                let y = Self::pop(&mut self.ram);
                let x = Self::stack_top(&mut self.ram);
                *x = (*x < y) as u16 * 0xFFFF;
            }
            VMCommand::And => {
                let y = Self::pop(&mut self.ram);
                *Self::stack_top(&mut self.ram) &= y;
            }
            VMCommand::Or => {
                let y = Self::pop(&mut self.ram);
                *Self::stack_top(&mut self.ram) |= y;
            }
            VMCommand::Not => {
                *Self::stack_top(&mut self.ram) ^= 0xFFFF;
            }
            VMCommand::Label { name: _ } => {}
            VMCommand::Goto { label_name } => {
                Self::goto(
                    &mut self.current_command_index,
                    &self.current_file_name,
                    &self.files,
                    label_name,
                );
            }
            VMCommand::IfGoto { label_name } => {
                let value = Self::pop(&mut self.ram);
                if value != 0 {
                    Self::goto(
                        &mut self.current_command_index,
                        &self.current_file_name,
                        &self.files,
                        label_name,
                    );
                }
            }
            VMCommand::Function {
                name: _,
                local_var_count,
            } => {
                for _ in 0..*local_var_count {
                    Self::push(&mut self.ram, 0);
                }
            }
            VMCommand::Call {
                function_name,
                argument_count,
            } => {
                let argument_segment = self.ram[0] - argument_count;
                Self::push(&mut self.ram, self.current_command_index as u16);
                for i in 1..=4 {
                    let value = self.ram[i];
                    Self::push(&mut self.ram, value);
                }
                let (file_name, actual_function_name) = function_name.split_once(".").unwrap();
                self.call_stack.push(Frame {
                    file_name: file_name.to_owned(),
                    function_name: actual_function_name.to_owned(),
                });
                let local_segment = self.ram[0];
                self.ram[1] = argument_segment;
                self.ram[2] = local_segment;
            }
            VMCommand::Return => todo!(),
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
            commands: Vec::new(),
            label_name_to_command_index: label_name_to_command_index,
            function_name_to_command_index: function_name_to_command_index,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum VMCommand {
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

impl VMCommand {
    fn is_nop(&self) -> bool {
        if let VMCommand::Label { name: _ }
        | VMCommand::Function {
            name: _,
            local_var_count: _,
        } = self
        {
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PushSegment {
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
enum PopSegment {
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
            Self::get(
                &self.ram,
                &self.file_name_to_static_segment,
                &self.current_file_name,
                segment,
                offset,
            )
        }

        fn test_set(&mut self, segment: PopSegment, offset: u16, value: u16) {
            Self::set(
                &mut self.ram,
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
}
