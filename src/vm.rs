use std::collections::HashMap;

struct VM {
    current_file_name: String,
    current_command_index: usize,
    files: HashMap<String, File>,
    ram: [u16; 32 * 1024],
}

impl VM {
    fn new(files: Vec<(String, File)>) -> Self {
        let mut ram = [0; 32 * 1024];
        ram[0] = 256;

        Self {
            current_file_name: "Main".to_string(),
            current_command_index: Default::default(),
            files: files.into_iter().collect(),
            ram: ram,
        }
    }
}

impl VM {
    fn push(&mut self, value: u16) {
        self.ram[self.ram[0] as usize] = value;
        self.ram[0] += 1;
    }

    fn pop(&mut self) -> u16 {
        self.ram[0] -= 1;
        self.ram[self.ram[0] as usize]
    }

    fn set(&mut self, segment: PopSegment, offset: u16) {
        match segment {
            PopSegment::Static => todo!(),
            PopSegment::Local => todo!(),
            PopSegment::Argument => todo!(),
            PopSegment::This => todo!(),
            PopSegment::That => todo!(),
            PopSegment::Temp => todo!(),
            PopSegment::Pointer => todo!(),
        }
    }
}

struct File {
    commands: Vec<VMCommand>,
}

enum VMCommand {
    Add,
    Push { segment: PushSegment, offset: u16 },
    Pop { segment: PopSegment, offset: u16 },
    Sub,
    Neg,
    Eq,
    Gt,
    Lt,
    And,
    Or,
    Not,
    Label { name: String },
    Goto { label_name: String },
    IfGoto { label_name: String },
    Function { name: String, local_var_count: u16 },
    Call { function_name: String, argument_count: u16 },
    Return
}

enum PushSegment {
    Constant,
    Static,
    Local,
    Argument,
    This,
    That,
    Temp,
    Pointer
}

enum PopSegment {
    Static,
    Local,
    Argument,
    This,
    That,
    Temp,
    Pointer
}