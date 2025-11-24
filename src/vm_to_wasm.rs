use hashbrown::HashMap;
use wast::{core::{Instruction, Local, MemArg, MemoryArg, ValType}, token::{Id, Index, Span}};

use crate::{hardware::Word, vm::{Register, VMCommand}};

fn id_ticks() -> Id<'static> {
    Id::new("ticks", Span::from_offset(0))
}

fn id_sp() -> Id<'static> {
    Id::new("sp", Span::from_offset(0))
}

fn id_jump_target() -> Id<'static> {
    Id::new("jump_target", Span::from_offset(0))
}

fn id_frame() -> Id<'static> {
    Id::new("frame", Span::from_offset(0))
}

fn index_ticks() -> Index<'static> {
    Index::Id(id_ticks())
}

fn index_sp() -> Index<'static> {
    Index::Id(id_sp())
}

fn index_jump_target() -> Index<'static> {
    Index::Id(id_jump_target())
}

fn index_frame() -> Index<'static> {
    Index::Id(id_frame())
}

fn locals() -> Box<[Local<'static>]> {
    Box::new([
        Local {
            id: Some(id_ticks()),
            name: None,
            ty: ValType::I32,
        },
        Local {
            id: Some(id_jump_target()),
            name: None,
            ty: ValType::I32,
        },
        Local {
            id: Some(id_sp()),
            name: None,
            ty: ValType::I32,
        },
        Local {
            id: Some(id_frame()),
            name: None,
            ty: ValType::I32,
        },
    ])
}

fn mem_arg() -> MemArg<'static> {
    MemArg {
        align: 4,
        offset: 0,
        memory: Index::Num(0, Span::from_offset(0)),
    }
}

fn mem_offset_arg(offset: Word) -> MemArg<'static> {
    MemArg {
        align: 4,
        offset: offset as u64 * 4,
        memory: Index::Num(0, Span::from_offset(0)),
    }
}

fn load_register(register: Register) -> [Instruction<'static>; 2] {
    [Instruction::I32Const(register.address() as i32),
    Instruction::I32Load(mem_arg())]
}

fn unary_stack_op(op: Vec<Instruction<'static>>) -> Vec<Instruction<'static>> {
    let mut ret = vec![
        Instruction::LocalGet(index_sp()),
        Instruction::I32Const(1),
        Instruction::I32Sub,
        Instruction::I32Load(mem_arg()),
    ];

    ret.extend(op);

    ret.extend([
        Instruction::LocalGet(index_sp()),
        Instruction::I32Const(1),
        Instruction::I32Sub,
        Instruction::I32Store(mem_arg()),
    ]);

    ret
}

fn binary_stack_op(op: Vec<Instruction<'static>>) -> Vec<Instruction<'static>> {
    let mut ret = vec![
        Instruction::I32Const(Register::SP.address() as i32),

        Instruction::LocalGet(index_sp()),
        Instruction::I32Const(1),
        Instruction::I32Sub,
        Instruction::LocalTee(index_sp()),

        Instruction::I32Store(mem_arg()),

        Instruction::LocalGet(index_sp()),
        Instruction::I32Load(mem_arg()),

        Instruction::LocalGet(index_sp()),
        Instruction::I32Const(1),
        Instruction::I32Sub,
        Instruction::I32Load(mem_arg()),
    ];
    ret.extend(op);
    ret.extend([
        Instruction::LocalGet(index_sp()),
        Instruction::I32Const(1),
        Instruction::I32Sub,
        Instruction::I32Store(mem_arg()),
    ]);

    ret
}

fn command_to_wasm(
    command: &VMCommand,
    index: i32,
    jump_index: Index<'static>,
    static_segment_start: Word,
    label_indices: &HashMap<String, i32>,
    function_indices: &HashMap<String, i32>,
) -> Vec<Instruction<'static>> {
    let mut wasm_instructions: Vec<Instruction<'static>> = vec![
        Instruction::I32Const(1),
        Instruction::LocalGet(index_ticks()),
        Instruction::I32Add,
        Instruction::LocalSet(index_ticks()),
    ];

    match command {
        VMCommand::Add => {
            wasm_instructions.extend(binary_stack_op(vec![
                Instruction::I32Add,
                Instruction::I32Extend16S,
            ]));
        },
        VMCommand::Push { segment, offset } => {
            use crate::vm::PushSegment;
            wasm_instructions.push(Instruction::LocalGet(index_sp()));

            match segment {
                PushSegment::Constant => {
                    wasm_instructions.extend([
                        Instruction::I32Const(*offset as i32),
                        Instruction::I32Load(mem_arg()),
                    ]);
                },
                PushSegment::Static => {
                    wasm_instructions.extend([
                        Instruction::I32Const((static_segment_start + offset) as i32),
                        Instruction::I32Load(mem_arg()),
                    ]);
                },
                PushSegment::Local => {
                    wasm_instructions.extend(load_register(Register::LCL));
                    wasm_instructions.push(Instruction::I32Load(mem_offset_arg(*offset)));
                },
                PushSegment::Argument => {
                    wasm_instructions.extend(load_register(Register::ARG));
                    wasm_instructions.push(Instruction::I32Load(mem_offset_arg(*offset)));
                },
                PushSegment::This => {
                    wasm_instructions.extend(load_register(Register::THIS));
                    wasm_instructions.push(Instruction::I32Load(mem_offset_arg(*offset)));
                }
                PushSegment::That => {
                    wasm_instructions.extend(load_register(Register::THAT));
                    wasm_instructions.push(Instruction::I32Load(mem_offset_arg(*offset)));
                }
                PushSegment::Temp => {
                    wasm_instructions.extend([
                        Instruction::I32Const(Register::TEMP(*offset).address() as i32),
                        Instruction::I32Load(mem_arg())
                    ]);
                }
                PushSegment::Pointer => {
                    wasm_instructions.extend([
                        Instruction::I32Const((Register::THIS.address() + offset) as i32),
                        Instruction::I32Load(mem_arg()),
                    ]);
                },
            }

            wasm_instructions.extend([
                Instruction::I32Store(mem_arg()),

                Instruction::I32Const(Register::SP.address() as i32),

                Instruction::LocalGet(index_sp()),
                Instruction::I32Const(1),
                Instruction::I32Add,
                Instruction::LocalTee(index_sp()),

                Instruction::I32Store(mem_arg()),
            ]);
        },
        VMCommand::Pop { segment, offset } => {
            use crate::vm::PopSegment;

            match segment {
                PopSegment::Static => {
                    wasm_instructions.extend([
                        Instruction::I32Const((static_segment_start + offset) as i32),
                        Instruction::I32Load(mem_arg()),
                    ]);
                },
                PopSegment::Local => {
                    wasm_instructions.extend(load_register(Register::LCL));
                    wasm_instructions.push(Instruction::I32Load(mem_offset_arg(*offset)));
                },
                PopSegment::Argument => {
                    wasm_instructions.extend(load_register(Register::ARG));
                    wasm_instructions.push(Instruction::I32Load(mem_offset_arg(*offset)));
                },
                PopSegment::This => {
                    wasm_instructions.extend(load_register(Register::THIS));
                    wasm_instructions.push(Instruction::I32Load(mem_offset_arg(*offset)));
                }
                PopSegment::That => {
                    wasm_instructions.extend(load_register(Register::THAT));
                    wasm_instructions.push(Instruction::I32Load(mem_offset_arg(*offset)));
                }
                PopSegment::Temp => {
                    wasm_instructions.extend([
                        Instruction::I32Const(Register::TEMP(*offset).address() as i32),
                        Instruction::I32Load(mem_arg())
                    ]);
                }
                PopSegment::Pointer => {
                    wasm_instructions.extend([
                        Instruction::I32Const((Register::THIS.address() + offset) as i32),
                        Instruction::I32Load(mem_arg()),
                    ]);
                },
            }

            wasm_instructions.extend([
                Instruction::LocalGet(index_sp()),
                Instruction::I32Const(1),
                Instruction::I32Sub,
                Instruction::LocalTee(index_sp()),
                Instruction::I32Store(mem_arg()),

                Instruction::I32Const(Register::SP.address() as i32),
                Instruction::LocalGet(index_sp()),
                Instruction::I32Store(mem_arg()),
            ]);
        },
        VMCommand::Sub => {
            wasm_instructions.extend(binary_stack_op(vec![
                Instruction::I32Sub,
                Instruction::I32Extend16S,
            ]));
        },
        VMCommand::Neg => {
            wasm_instructions.push(Instruction::I32Const(0));
            wasm_instructions.extend(unary_stack_op(vec![
                Instruction::I32Sub,
                Instruction::I32Extend16S,
            ]));
        },
        VMCommand::Eq => {
            wasm_instructions.push(Instruction::I32Const(0));
            wasm_instructions.extend(binary_stack_op(vec![
                Instruction::I32Eq,
                Instruction::I32Sub,
            ]));
        },
        VMCommand::Gt => {
            wasm_instructions.push(Instruction::I32Const(0));
            wasm_instructions.extend(binary_stack_op(vec![
                Instruction::I32GtS,
                Instruction::I32Sub,
            ]));
        }
        VMCommand::Lt => {
            wasm_instructions.push(Instruction::I32Const(0));
            wasm_instructions.extend(binary_stack_op(vec![
                Instruction::I32LtS,
                Instruction::I32Sub,
            ]));
        }
        VMCommand::And => {
            wasm_instructions.extend(binary_stack_op(vec![
                Instruction::I32And
            ]));
        },
        VMCommand::Or => {
            wasm_instructions.extend(binary_stack_op(vec![
                Instruction::I32Or,
            ]));
        },
        VMCommand::Not => {
            wasm_instructions.extend(unary_stack_op(vec![
                Instruction::I32Const(-1),
                Instruction::I32Xor,
            ]));
        },
        VMCommand::Label { .. } => {
            unreachable!("Labels should have been removed by now");
        },
        VMCommand::Goto { label_name } => {
            if matches!(jump_index, Index::Id(_)) {
                wasm_instructions.extend([
                    Instruction::I32Const(label_indices[label_name]),
                    Instruction::LocalSet(index_jump_target()),
                ]);
            }
            wasm_instructions.push(Instruction::Br(jump_index))
        },
        VMCommand::IfGoto { label_name } => {
            if matches!(jump_index, Index::Id(_)) {
                wasm_instructions.extend([
                    Instruction::I32Const(label_indices[label_name]),
                    Instruction::LocalSet(index_jump_target()),
                ]);
            }
            wasm_instructions.extend([
                Instruction::I32Const(0),
                Instruction::I32Ne,
                Instruction::BrIf(jump_index)
            ]);
        },
        VMCommand::Function { local_var_count, .. } => {
            wasm_instructions.extend([
                Instruction::LocalGet(index_sp()),
                Instruction::I32Const(0),
                Instruction::I32Const(*local_var_count as i32 * 4),
                Instruction::MemoryFill(MemoryArg { mem: Index::Num(0, Span::from_offset(0)) }),

                Instruction::I32Const(Register::SP.address() as i32),
                Instruction::LocalGet(index_sp()),
                Instruction::I32Const(*local_var_count as i32),
                Instruction::I32Add,
                Instruction::LocalTee(index_sp()),
                Instruction::I32Store(mem_arg()),
            ]);
        },
        VMCommand::Call { function_name, argument_count } => {
            wasm_instructions.extend([
                Instruction::LocalGet(index_sp()),
                Instruction::I32Const(index + 1),
                Instruction::I32Store(mem_arg()),
            ]);

            for i in 1..=4 {
                wasm_instructions.extend([
                    Instruction::LocalGet(index_sp()),
                    Instruction::I32Const(i as i32),
                    Instruction::I32Load(mem_arg()),
                    Instruction::I32Store(mem_offset_arg(i)),
                ]);
            }

            wasm_instructions.extend([
                Instruction::I32Const(Register::ARG.address() as i32),

                Instruction::LocalGet(index_sp()),
                Instruction::I32Const(*argument_count as i32),
                Instruction::I32Sub,

                Instruction::I32Store(mem_arg()),
            ]);


            wasm_instructions.extend([
                Instruction::I32Const(Register::SP.address() as i32),

                Instruction::LocalGet(index_sp()),
                Instruction::I32Const(5),
                Instruction::I32Add,
                Instruction::LocalTee(index_sp()),

                Instruction::I32Store(mem_arg()),
            ]);

            wasm_instructions.extend([
                Instruction::I32Const(Register::LCL.address() as i32),

                Instruction::LocalGet(index_sp()),

                Instruction::I32Store(mem_arg()),
            ]);

            if matches!(jump_index, Index::Id(_)) {
                wasm_instructions.extend([
                    Instruction::I32Const(function_indices[function_name]),
                    Instruction::LocalSet(index_jump_target()),
                ]);
            }
            wasm_instructions.push(Instruction::Br(jump_index))
        },
        VMCommand::Return => {
            // Store frame pointer and put return address in jump target
            wasm_instructions.extend(load_register(Register::LCL));
            wasm_instructions.extend([
                Instruction::I32Const(5),
                Instruction::I32Sub,
                Instruction::LocalTee(index_frame()),

                Instruction::I32Load(mem_arg()),
                Instruction::LocalSet(index_jump_target()),
            ]);

            // Move return value to beginning of argument segment
            wasm_instructions.extend(load_register(Register::ARG));
            wasm_instructions.extend([
                Instruction::LocalGet(index_sp()),
                Instruction::I32Const(1),
                Instruction::I32Sub,
                Instruction::I32Load(mem_arg()),

                Instruction::I32Store(mem_arg()),
            ]);

            // Set stack pointer to after return value
            wasm_instructions.push(Instruction::I32Const(Register::SP.address() as i32));
            wasm_instructions.extend(load_register(Register::ARG));
            wasm_instructions.extend([
                Instruction::I32Const(1),
                Instruction::I32Add,
                Instruction::LocalTee(index_sp()),

                Instruction::I32Store(mem_arg())
            ]);

            // Restore frame
            for i in 1..=4 {
                wasm_instructions.extend([
                    Instruction::I32Const(i as i32),
                    Instruction::LocalGet(index_frame()),
                    Instruction::I32Load(mem_offset_arg(i)),
                    Instruction::I32Store(mem_arg()),
                ]);
            }

            wasm_instructions.push(Instruction::Br(jump_index))
        },
    }

    wasm_instructions
}


pub fn hack_to_wasm(
    instructions: &[VMCommand],
    with_limit: bool,
) -> Result<Vec<u8>, String> {

    todo!()
}
