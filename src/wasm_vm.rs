use std::cell::OnceCell;
use std::rc::Rc;

use crate::any_wasm::{AnyWasmHandle, Val};

use crate::{hardware::Word, vm::Program};

use crate::vm::{VM, VMCommand};

#[cfg(not(target_arch = "wasm32"))]
pub type WasmVm = GenericWasmVm<crate::any_wasm::WasmtimeHandle>;

#[cfg(target_arch = "wasm32")]
pub type WasmVm = GenericWasmVm<crate::any_wasm::JsWasmHandle>;

struct State<H: AnyWasmHandle> {
    handle: H,
    function: H::Function,
    memory: H::Memory,
    pc: H::Global,
    start_pc: i32,
}

// #[derive(Debug)]
pub struct GenericWasmVm<H: AnyWasmHandle> {
    pub program: Program,
    state: Rc<OnceCell<State<H>>>,
}

impl<H: AnyWasmHandle> GenericWasmVm<H> {
    pub fn from_program(program: Program) -> Self {
        let unoptimized_wasm = crate::vm_to_wasm::vm_to_wasm(&program, true).unwrap();
        let state = Rc::new(OnceCell::new());
        let state_clone = Rc::clone(&state);

        H::from_binary(&unoptimized_wasm, move |handle| {
            let function = handle.get_function("run").unwrap();
            let pc = handle.get_global("pc").unwrap();
            let start_pc = handle.get_global_value_i32(&pc);

            let memory = handle.get_memory("memory").unwrap();

            handle.set_memory_at(&memory, 0, 256);

            state_clone.set(State { handle, function, memory, pc, start_pc }).ok().unwrap();
        });

        Self { program, state }
    }

    pub fn is_ready(&self) -> bool {
        self.state.get().is_some()
    }

    pub fn from_file_contents(contents: Vec<(String, String)>) -> Self {
        let vm = VM::from_file_contents(contents);

        Self::from_program(vm.program)
    }

    pub fn from_all_file_commands(all_file_commands: Vec<(String, Vec<VMCommand>)>) -> Self {
        let vm = VM::from_all_file_commands(all_file_commands);

        Self::from_program(vm.program)
    }

    pub fn current_file_name(&self) -> &str {
        let state = self.state.get().unwrap();
        let pc = state.handle.get_global_value_i32(&state.pc) as usize;
        let file = self.program.files.iter().take_while(|f| f.starting_command_index <= pc).last().unwrap();

        &file.name
    }

    pub fn current_file_index(&self) -> usize {
        self.program.file_name_to_index[self.current_file_name()]
    }

    pub fn run(&mut self, step_count: u64) -> bool {
        let state = self.state.get().unwrap();

        let mut returns = [Val::I32(0)];
        state.handle.call_function(&state.function, &[Val::I32(step_count as i32)], &mut returns);
        let [Val::I32(ticks)] = returns else {
            panic!("Return type changed");
        };
        // println!("ticks: {}", ticks);


        // for _ in 0..step_count {
        //     println!("{:?}", self.program.all_commands[self.reference_vm.run_state.current_command_index]);
        //     let ticks = self.function.call(&mut *store, 1).unwrap();
        //     println!("ticks: {}", ticks);
        //     self.reference_vm.run(ticks as u64);
        //     let ram = self.copy_ram();
        //     let mut fail: bool = false;
        //     if ram.contents != self.reference_vm.run_state.ram.contents {
        //         for i in 0..crate::hardware::MEM_SIZE {
        //             if ram.contents[i] != self.reference_vm.run_state.ram.contents[i] {
        //                 println!(
        //                     "RAM mismatch at address {}: wasm = {}, reference = {}",
        //                     i, ram.contents[i], self.reference_vm.run_state.ram.contents[i]
        //                 );
        //             }
        //         }
        //         fail = true;
        //     }
        //     let pc = self.pc.get(&mut *store).unwrap_i32() as usize;
        //     if pc != self.reference_vm.run_state.current_command_index {
        //         println!(
        //             "PC mismatch: wasm pc = {}, reference pc = {}",
        //             pc,
        //             self.reference_vm.run_state.current_command_index
        //         );
        //         fail = true;
        //     }

        //     if fail {
        //         panic!("VM state mismatch detected");
        //     }
        // }

        false
    }

    pub fn get_ram_value(&self, address: Word) -> Word {
        let state = self.state.get().unwrap();

        state.handle.get_memory_at(&state.memory, address as usize) as Word
    }

    pub fn set_ram_value(&mut self, address: Word, value: Word) {
        let state = self.state.get().unwrap();

        state.handle.set_memory_at(&state.memory, address as usize, value as i32);
    }

    pub fn reset(&mut self) {
        let state = self.state.get().unwrap();

        state.handle.fill_memory(&state.memory, 0);
        state.handle.set_memory_at(&state.memory, 0, 256);
        state.handle.set_global_value_i32(&state.pc, state.start_pc);
    }

    pub fn copy_ram(&self) -> crate::hardware::RAM {
        let state = self.state.get().unwrap();
        let data = state.handle.raw_memory(&state.memory);

        let mut ram = crate::hardware::RAM::default();
        for (i, r) in ram.contents.iter_mut().enumerate().take(crate::hardware::MEM_SIZE) {
            *r = data[i] as crate::hardware::Word;
        }
        ram
    }
}

mod tests {
    use crate::{vm::{PopSegment, PushSegment, Register}};

    use super::*;

    impl WasmVm {
        fn test_get(&self, segment: PushSegment, offset: Word) -> Word {
            let static_segment = *self.program.files[self.current_file_index()]
                    .static_segment
                    .start();
            match segment {
                PushSegment::Constant => offset,
                PushSegment::Static => self.get_ram_value(static_segment + offset),
                PushSegment::Local => self.get_ram_value(self.get_ram_value(Register::LCL.address()) + offset),
                PushSegment::Argument => self.get_ram_value(self.get_ram_value(Register::ARG.address()) + offset),
                PushSegment::This => self.get_ram_value(self.get_ram_value(Register::THIS.address()) + offset),
                PushSegment::That => self.get_ram_value(self.get_ram_value(Register::THAT.address()) + offset),
                PushSegment::Temp => self.get_ram_value(Register::TEMP(offset).address()),
                PushSegment::Pointer => self.get_ram_value(Register::THIS.address() + offset),
            }
        }

        fn test_set(&mut self, segment: PopSegment, offset: Word, value: Word) {
            let static_segment = *self.program.files[self.current_file_index()]
                    .static_segment
                    .start();
            match segment {
                PopSegment::Static => self.set_ram_value(static_segment + offset, value),
                PopSegment::Local => self.set_ram_value(self.get_ram_value(Register::LCL.address()) + offset, value),
                PopSegment::Argument => self.set_ram_value(self.get_ram_value(Register::ARG.address()) + offset, value),
                PopSegment::This => self.set_ram_value(self.get_ram_value(Register::THIS.address()) + offset, value),
                PopSegment::That => self.set_ram_value(self.get_ram_value(Register::THAT.address()) + offset, value),
                PopSegment::Temp => self.set_ram_value(Register::TEMP(offset).address(), value),
                PopSegment::Pointer => self.set_ram_value(Register::THIS.address() + offset, value),
            }
        }

        fn set_current_file(&mut self, file_name: &str) {
            let state = self.state.get().unwrap();
            let file_start = self.program.files[self.program.file_name_to_index[file_name]].starting_command_index;
            state.handle.set_global_value_i32(&state.pc, file_start as i32);
        }

        fn stack_top(&self) -> Word {
            let sp = self.get_ram_value(Register::SP.address());
            self.get_ram_value(sp - 1)
        }

        fn test_instance() -> Self {
            let vm = Self::from_all_file_commands(vec![(
                "foo".to_owned(),
                vec![VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 666,
                }],
            )]);
            let state = vm.state.get().unwrap();
            state.handle.set_global_value_i32(&state.pc, 0);

            vm
        }
    }

    #[test]
    fn test_constant() {
        let vm = WasmVm::test_instance();

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

        let mut vm = WasmVm::from_all_file_commands(all_file_commands);
        vm.set_current_file("a");

        vm.test_set(PopSegment::Static, 0, 1337);
        vm.set_current_file("b");
        vm.test_set(PopSegment::Static, 1, 2337);

        assert_eq!(vm.test_get(PushSegment::Static, 0), 0);
        assert_eq!(vm.test_get(PushSegment::Static, 1), 2337);
        vm.set_current_file("a");
        assert_eq!(vm.test_get(PushSegment::Static, 0), 1337);
        assert_eq!(vm.test_get(PushSegment::Static, 1), 0);
    }

    #[test]
    fn test_local() {
        let mut vm = WasmVm::test_instance();
        vm.set_ram_value(1, 1337);

        vm.test_set(PopSegment::Local, 0, 2337);
        vm.test_set(PopSegment::Local, 3, 3337);

        assert_eq!(vm.test_get(PushSegment::Local, 0), 2337);
        assert_eq!(vm.test_get(PushSegment::Local, 3), 3337);
        assert_eq!(vm.get_ram_value(1337), 2337);
        assert_eq!(vm.get_ram_value(1340), 3337);
    }

    #[test]
    fn test_argument() {
        let mut vm = WasmVm::test_instance();
        vm.set_ram_value(2, 1337);

        vm.test_set(PopSegment::Argument, 0, 2337);
        vm.test_set(PopSegment::Argument, 3, 3337);

        assert_eq!(vm.test_get(PushSegment::Argument, 0), 2337);
        assert_eq!(vm.test_get(PushSegment::Argument, 3), 3337);
        assert_eq!(vm.get_ram_value(1337), 2337);
        assert_eq!(vm.get_ram_value(1340), 3337);
    }

    #[test]
    fn test_this() {
        let mut vm = WasmVm::test_instance();
        vm.set_ram_value(3, 1337);

        vm.test_set(PopSegment::This, 0, 2337);
        vm.test_set(PopSegment::This, 3, 3337);

        assert_eq!(vm.test_get(PushSegment::This, 0), 2337);
        assert_eq!(vm.test_get(PushSegment::This, 3), 3337);
        assert_eq!(vm.get_ram_value(1337), 2337);
        assert_eq!(vm.get_ram_value(1340), 3337);
    }

    #[test]
    fn test_that() {
        let mut vm = WasmVm::test_instance();
        vm.set_ram_value(4, 1337);

        vm.test_set(PopSegment::That, 0, 2337);
        vm.test_set(PopSegment::That, 3, 3337);

        assert_eq!(vm.test_get(PushSegment::That, 0), 2337);
        assert_eq!(vm.test_get(PushSegment::That, 3), 3337);
        assert_eq!(vm.get_ram_value(1337), 2337);
        assert_eq!(vm.get_ram_value(1340), 3337);
    }

    #[test]
    fn test_temp() {
        let mut vm = WasmVm::test_instance();

        vm.test_set(PopSegment::Temp, 0, 1337);
        vm.test_set(PopSegment::Temp, 3, 2337);

        assert_eq!(vm.test_get(PushSegment::Temp, 0), 1337);
        assert_eq!(vm.test_get(PushSegment::Temp, 3), 2337);
        assert_eq!(vm.get_ram_value(5), 1337);
        assert_eq!(vm.get_ram_value(8), 2337);
    }

    #[test]
    fn test_pointer() {
        let mut vm = WasmVm::test_instance();

        vm.test_set(PopSegment::Pointer, 0, 1337);
        vm.test_set(PopSegment::Pointer, 1, 2337);

        assert_eq!(vm.test_get(PushSegment::Pointer, 0), 1337);
        assert_eq!(vm.get_ram_value(3), 1337);
        assert_eq!(vm.test_get(PushSegment::Pointer, 1), 2337);
        assert_eq!(vm.get_ram_value(4), 2337);
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

        let mut vm = WasmVm::from_all_file_commands(all_file_commands);

        vm.set_current_file("a");
        vm.run(3);
        assert_eq!(vm.stack_top(), 1337 + 2337);
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

        let mut vm = WasmVm::from_all_file_commands(all_file_commands);

        vm.set_current_file("a");
        vm.run(3);
        assert_eq!(vm.stack_top(), 2337 - 1337);
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

        let mut vm = WasmVm::from_all_file_commands(all_file_commands);
        vm.set_current_file("a");
        vm.run(2);

        assert_eq!(vm.stack_top(), -1337);
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
                VMCommand::Eq
            ],
        )];

        let mut vm = WasmVm::from_all_file_commands(all_file_commands);
        vm.set_current_file("a");
        vm.run(3);

        assert_eq!(vm.stack_top(), 0);

        let all_file_commands = vec![(
            "a".to_owned(),
            vec![
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

        let mut vm = WasmVm::from_all_file_commands(all_file_commands);
        vm.set_current_file("a");
        vm.run(3);

        assert_eq!(vm.stack_top(), -1);
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
                VMCommand::Gt
            ],
        )];

        let mut vm = WasmVm::from_all_file_commands(all_file_commands);
        vm.set_current_file("a");
        vm.run(3);

        assert_eq!(vm.stack_top(), 0);

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
                VMCommand::Gt,
            ],
        )];

        let mut vm = WasmVm::from_all_file_commands(all_file_commands);
        vm.set_current_file("a");
        vm.run(3);

        assert_eq!(vm.stack_top(), -1);
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
                VMCommand::Lt
            ],
        )];

        let mut vm = WasmVm::from_all_file_commands(all_file_commands);
        vm.set_current_file("a");
        vm.run(3);

        assert_eq!(vm.stack_top(), 0);

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
                VMCommand::Lt,
            ],
        )];

        let mut vm = WasmVm::from_all_file_commands(all_file_commands);
        vm.set_current_file("a");
        vm.run(3);

        assert_eq!(vm.stack_top(), -1);
    }

    #[test]
    fn test_and() {
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
                VMCommand::And,
            ],
        )];

        let mut vm = WasmVm::from_all_file_commands(all_file_commands);

        vm.set_current_file("a");
        vm.run(3);
        assert_eq!(vm.stack_top(), 1337 & 2337);
    }

    #[test]
    fn test_or() {
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
                VMCommand::Or,
            ],
        )];

        let mut vm = WasmVm::from_all_file_commands(all_file_commands);

        vm.set_current_file("a");
        vm.run(3);
        assert_eq!(vm.stack_top(), 1337 | 2337);
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

        let mut vm = WasmVm::from_all_file_commands(all_file_commands);
        vm.set_current_file("a");
        vm.run(2);

        assert_eq!(vm.stack_top(), -1338);
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

        let mut vm = WasmVm::from_all_file_commands(all_file_commands.clone());
        let vm2 = WasmVm::from_all_file_commands(all_file_commands);
        vm.run(2);

        assert_eq!(vm.copy_ram(), vm2.copy_ram());
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
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 5
                },
                VMCommand::Label {
                    name: "foo".to_owned(),
                },
            ],
        )];

        let mut vm = WasmVm::from_all_file_commands(all_file_commands.clone());
        vm.set_current_file("Sys");
        let vm2 = WasmVm::from_all_file_commands(all_file_commands);
        vm.run(5);

        assert_eq!(vm.copy_ram(), vm2.copy_ram());
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
                VMCommand::Push {
                    segment: PushSegment::Constant,
                    offset: 5
                },
                VMCommand::Label {
                    name: "foo".to_owned(),
                },
            ],
        )];

        let mut vm = WasmVm::from_all_file_commands(all_file_commands.clone());
        vm.set_current_file("Sys");
        vm.run(6);

        assert_eq!(vm.stack_top(), 0);
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
                VMCommand::Goto {
                    label_name: "nop".to_owned(),
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

        let mut vm = WasmVm::from_all_file_commands(all_file_commands.clone());
        vm.set_ram_value(1, 1);
        vm.set_ram_value(2, 2);
        vm.set_ram_value(3, 3);
        vm.set_ram_value(4, 4);
        vm.set_current_file("Sys");
        for _ in 0..15 {
            vm.run(1);
            let state = vm.state.get().unwrap();
            let pc = state.handle.get_global_value_i32(&state.pc);
            let ram = vm.copy_ram();
            println!("pc: {} ", pc);
            println!("{:?} ", &ram.contents[0..6]);
            println!("{:?} ", &ram.contents[255..265]);
        }

        assert_eq!(vm.stack_top(), 2337);
        assert_eq!(vm.get_ram_value(1), 1);
        assert_eq!(vm.get_ram_value(2), 2);
        assert_eq!(vm.get_ram_value(3), 3);
        assert_eq!(vm.get_ram_value(4), 4);
    }
}
