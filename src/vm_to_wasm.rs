use std::collections::{HashMap, HashSet};

use wast::{
    core::{
        BlockType, Export, ExportKind, FuncKind, FunctionType, Global, GlobalKind, GlobalType, Import, InlineExport, Instruction, ItemSig, Local, MemArg, MemoryArg, ModuleField, SelectTypes, TypeUse, ValType
    },
    token::{Id, Index, NameAnnotation, Span},
};

use crate::{
    hardware::{RAM, Word},
    vm::{Program, Register, VMCommand},
    wasm_utils::{ExpressionBuilder, FuncBuilder, ModuleBuilder, create_memory},
};

fn id_ticks() -> Id<'static> {
    Id::new("ticks", Span::from_offset(0))
}

fn id_sp() -> Id<'static> {
    Id::new("sp", Span::from_offset(0))
}

fn id_jump_target() -> Id<'static> {
    Id::new("jump_target", Span::from_offset(0))
}

fn id_temp() -> Id<'static> {
    Id::new("temp", Span::from_offset(0))
}

fn id_temp2() -> Id<'static> {
    Id::new("temp2", Span::from_offset(0))
}

fn id_screen_color() -> Id<'static> {
    Id::new("screen_color", Span::from_offset(0))
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

fn index_temp() -> Index<'static> {
    Index::Id(id_temp())
}

fn index_temp2() -> Index<'static> {
    Index::Id(id_temp2())
}

fn index_screen_color() -> Index<'static> {
    Index::Id(id_screen_color())
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
            id: Some(id_temp()),
            name: None,
            ty: ValType::I32,
        },
        Local {
            id: Some(id_temp2()),
            name: None,
            ty: ValType::I32,
        },
    ])
}

fn globals(start_case_index: i32) -> Vec<Global<'static>> {
    vec![
        Global {
            span: Span::from_offset(0),
            id: Some(id_jump_target()),
            name: None,
            exports: InlineExport { names: vec!["pc"] },
            ty: GlobalType {
                ty: ValType::I32,
                mutable: true,
                shared: false,
            },
            kind: GlobalKind::Inline(
                ExpressionBuilder::default()
                    .instr(Instruction::I32Const(start_case_index))
                    .build(),
            ),
        },
        Global {
            span: Span::from_offset(0),
            id: Some(id_screen_color()),
            name: None,
            exports: InlineExport { names: vec![] },
            ty: GlobalType {
                ty: ValType::I32,
                mutable: true,
                shared: false,
            },
            kind: GlobalKind::Inline(
                ExpressionBuilder::default()
                    .instr(Instruction::I32Const(0))
                    .build(),
            ),
        },
    ]
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
    [
        Instruction::I32Const(register.address() as i32 * 4),
        Instruction::I32Load(mem_arg()),
    ]
}

fn unary_stack_op(
    prefix: Vec<Instruction<'static>>,
    op: Vec<Instruction<'static>>,
) -> Vec<Instruction<'static>> {
    let mut ret = vec![
        Instruction::LocalGet(index_sp()),
        Instruction::I32Const(1),
        Instruction::I32Sub,
        Instruction::I32Const(2),
        Instruction::I32Shl,

    ];
    ret.extend(prefix);
    ret.extend([
        Instruction::LocalGet(index_sp()),
        Instruction::I32Const(1),
        Instruction::I32Sub,
        Instruction::I32Const(2),
        Instruction::I32Shl,
        Instruction::I32Load(mem_arg()),
    ]);

    ret.extend(op);

    ret.push(Instruction::I32Store(mem_arg()));

    ret
}

fn binary_stack_op(
    prefix: Vec<Instruction<'static>>,
    op: Vec<Instruction<'static>>,
) -> Vec<Instruction<'static>> {
    let mut ret = vec![
        Instruction::I32Const(Register::SP.address() as i32),
        Instruction::LocalGet(index_sp()),
        Instruction::I32Const(1),
        Instruction::I32Sub,
        Instruction::LocalTee(index_sp()),
        Instruction::I32Store(mem_arg()),
        Instruction::LocalGet(index_sp()),
        Instruction::I32Const(1),
        Instruction::I32Sub,
        Instruction::I32Const(2),
        Instruction::I32Shl,
    ];
    ret.extend(prefix);
    ret.extend([
        Instruction::LocalGet(index_sp()),
        Instruction::I32Const(1),
        Instruction::I32Sub,
        Instruction::I32Const(2),
        Instruction::I32Shl,
        Instruction::I32Load(mem_arg()),
        Instruction::LocalGet(index_sp()),
        Instruction::I32Const(2),
        Instruction::I32Shl,
        Instruction::I32Load(mem_arg()),
    ]);
    ret.extend(op);
    ret.push(Instruction::I32Store(mem_arg()));

    ret
}

fn command_to_wasm(
    command: &VMCommand,
    index: usize,
    jump_index: Index<'static>,
    static_segment_start: Word,
    current_function_name: Option<&String>,
    label_indices: &HashMap<String, i32>,
    function_indices: &HashMap<String, i32>,
    call_indices: &HashMap<usize, i32>,
) -> Vec<Instruction<'static>> {
    let mut wasm_instructions: Vec<Instruction<'static>> = vec![
        Instruction::I32Const(1),
        Instruction::LocalGet(index_ticks()),
        Instruction::I32Add,
        Instruction::LocalSet(index_ticks()),
    ];

    match command {
        VMCommand::Add => {
            wasm_instructions.extend(binary_stack_op(
                vec![],
                vec![Instruction::I32Add, Instruction::I32Extend16S],
            ));
        }
        VMCommand::Push { segment, offset } => {
            use crate::vm::PushSegment;
            wasm_instructions.extend([
                Instruction::LocalGet(index_sp()),
                Instruction::I32Const(2),
                Instruction::I32Shl,
            ]);

            match segment {
                PushSegment::Constant => {
                    wasm_instructions.push(Instruction::I32Const(*offset as i32));
                }
                PushSegment::Static => {
                    wasm_instructions.extend([
                        Instruction::I32Const((static_segment_start + offset) as i32 * 4),
                        Instruction::I32Load(mem_arg()),
                    ]);
                }
                PushSegment::Local => {
                    wasm_instructions.extend(load_register(Register::LCL));
                    wasm_instructions.extend([
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::I32Load(mem_offset_arg(*offset)),
                    ]);
                }
                PushSegment::Argument => {
                    wasm_instructions.extend(load_register(Register::ARG));
                    wasm_instructions.extend([
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::I32Load(mem_offset_arg(*offset)),
                    ]);
                }
                PushSegment::This => {
                    wasm_instructions.extend(load_register(Register::THIS));
                    wasm_instructions.extend([
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::I32Load(mem_offset_arg(*offset)),
                    ]);
                }
                PushSegment::That => {
                    wasm_instructions.extend(load_register(Register::THAT));
                    wasm_instructions.extend([
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::I32Load(mem_offset_arg(*offset)),
                    ]);
                }
                PushSegment::Temp => {
                    wasm_instructions.extend([
                        Instruction::I32Const(Register::TEMP(*offset).address() as i32 * 4),
                        Instruction::I32Load(mem_offset_arg(*offset)),
                    ]);
                }
                PushSegment::Pointer => {
                    wasm_instructions.extend([
                        Instruction::I32Const((Register::THIS.address() + offset) as i32 * 4),
                        Instruction::I32Load(mem_offset_arg(*offset)),
                    ]);
                }
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
        }
        VMCommand::Pop { segment, offset } => {
            use crate::vm::PopSegment;

            match segment {
                PopSegment::Static => {
                    wasm_instructions
                        .extend([Instruction::I32Const(static_segment_start as i32 * 4)]);
                }
                PopSegment::Local => {
                    wasm_instructions.extend(load_register(Register::LCL));
                    wasm_instructions.extend([Instruction::I32Const(2), Instruction::I32Shl]);
                }
                PopSegment::Argument => {
                    wasm_instructions.extend(load_register(Register::ARG));
                    wasm_instructions.extend([Instruction::I32Const(2), Instruction::I32Shl]);
                }
                PopSegment::This => {
                    wasm_instructions.extend(load_register(Register::THIS));
                    wasm_instructions.extend([Instruction::I32Const(2), Instruction::I32Shl]);
                }
                PopSegment::That => {
                    wasm_instructions.extend(load_register(Register::THAT));
                    wasm_instructions.extend([Instruction::I32Const(2), Instruction::I32Shl]);
                }
                PopSegment::Temp => {
                    wasm_instructions.extend([Instruction::I32Const(
                        Register::TEMP(0).address() as i32 * 4,
                    )]);
                }
                PopSegment::Pointer => {
                    wasm_instructions
                        .extend([Instruction::I32Const(Register::THIS.address() as i32 * 4)]);
                }
            }

            wasm_instructions.extend([
                Instruction::LocalGet(index_sp()),
                Instruction::I32Const(1),
                Instruction::I32Sub,
                Instruction::LocalTee(index_sp()),
                Instruction::I32Const(2),
                Instruction::I32Shl,
                Instruction::I32Load(mem_arg()),
                Instruction::I32Store(mem_offset_arg(*offset)),
                Instruction::I32Const(Register::SP.address() as i32),
                Instruction::LocalGet(index_sp()),
                Instruction::I32Store(mem_arg()),
            ]);
        }
        VMCommand::Sub => {
            wasm_instructions.extend(binary_stack_op(
                vec![],
                vec![Instruction::I32Sub, Instruction::I32Extend16S],
            ));
        }
        VMCommand::Neg => {
            wasm_instructions.extend(unary_stack_op(
                vec![Instruction::I32Const(0)],
                vec![Instruction::I32Sub, Instruction::I32Extend16S],
            ));
        }
        VMCommand::Eq => {
            wasm_instructions.extend(binary_stack_op(
                vec![Instruction::I32Const(0)],
                vec![Instruction::I32Eq, Instruction::I32Sub],
            ));
        }
        VMCommand::Gt => {
            wasm_instructions.extend(binary_stack_op(
                vec![Instruction::I32Const(0)],
                vec![Instruction::I32GtS, Instruction::I32Sub],
            ));
        }
        VMCommand::Lt => {
            wasm_instructions.extend(binary_stack_op(
                vec![Instruction::I32Const(0)],
                vec![Instruction::I32LtS, Instruction::I32Sub],
            ));
        }
        VMCommand::And => {
            wasm_instructions.extend(binary_stack_op(vec![], vec![Instruction::I32And]));
        }
        VMCommand::Or => {
            wasm_instructions.extend(binary_stack_op(vec![], vec![Instruction::I32Or]));
        }
        VMCommand::Not => {
            wasm_instructions.extend(unary_stack_op(
                vec![],
                vec![Instruction::I32Const(-1), Instruction::I32Xor],
            ));
        }
        VMCommand::Label { .. } => {
            // unreachable!("Labels should have been removed by now");
        }
        VMCommand::Goto { label_name } => {
            if matches!(jump_index, Index::Id(_)) {
                wasm_instructions.extend([
                    Instruction::I32Const(
                        label_indices
                            [&format!("{}.{}", current_function_name.unwrap(), label_name)],
                    ),
                    Instruction::LocalSet(index_jump_target()),
                ]);
            }
            wasm_instructions.push(Instruction::Br(jump_index))
        }
        VMCommand::IfGoto { label_name } => {
            if matches!(jump_index, Index::Id(_)) {
                wasm_instructions.extend([
                    Instruction::I32Const(
                        label_indices
                            [&format!("{}.{}", current_function_name.unwrap(), label_name)],
                    ),
                    Instruction::LocalSet(index_jump_target()),
                ]);
            }
            wasm_instructions.extend([
                Instruction::I32Const(Register::SP.address() as i32),
                Instruction::LocalGet(index_sp()),
                Instruction::I32Const(1),
                Instruction::I32Sub,
                Instruction::LocalTee(index_sp()),
                Instruction::I32Store(mem_arg()),
                Instruction::LocalGet(index_sp()),
                Instruction::I32Const(2),
                Instruction::I32Shl,
                Instruction::I32Load(mem_arg()),
                Instruction::I32Const(0),
                Instruction::I32Ne,
                Instruction::BrIf(jump_index),
            ]);
        }
        VMCommand::Function {
            local_var_count, ..
        } => {
            wasm_instructions.extend([
                Instruction::LocalGet(index_sp()),
                Instruction::I32Const(2),
                Instruction::I32Shl,
                Instruction::I32Const(0),
                Instruction::I32Const(*local_var_count as i32 * 4),
                Instruction::MemoryFill(MemoryArg {
                    mem: Index::Num(0, Span::from_offset(0)),
                }),
                Instruction::I32Const(Register::SP.address() as i32 * 4),
                Instruction::LocalGet(index_sp()),
                Instruction::I32Const(*local_var_count as i32),
                Instruction::I32Add,
                Instruction::LocalTee(index_sp()),
                Instruction::I32Store(mem_arg()),
            ]);
        }
        VMCommand::Call {
            function_name,
            argument_count,
        } => {
            match function_name.as_str() {
                "Math.multiply" => {
                    wasm_instructions.extend(binary_stack_op(
                        vec![],
                        vec![Instruction::I32Mul, Instruction::I32Extend16S],
                    ));
                }
                "Math.divide" => {
                    wasm_instructions.extend(binary_stack_op(
                        vec![],
                        vec![Instruction::I32DivS, Instruction::I32Extend16S],
                    ));
                }
                "Screen.setColor" => {
                    wasm_instructions.extend([
                        Instruction::LocalGet(index_sp()),
                        Instruction::I32Const(1),
                        Instruction::I32Sub,
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::I32Load(mem_arg()),
                        Instruction::GlobalSet(index_screen_color())
                    ]);
                }
                "Screen.drawPixel" => {
                    wasm_instructions.extend([
                        Instruction::I32Const(Register::SP.address() as i32 * 4),
                        Instruction::LocalGet(index_sp()),
                        Instruction::I32Const(1),
                        Instruction::I32Sub,
                        Instruction::LocalTee(index_sp()),
                        Instruction::I32Store(mem_arg()),

                        Instruction::I32Const(RAM::SCREEN as i32 * 4),
                        Instruction::LocalGet(index_sp()),
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::I32Load(mem_arg()),
                        Instruction::I32Const(RAM::SCREEN_ROW_LENGTH as i32 * 4),
                        Instruction::I32Mul,
                        Instruction::I32Add,

                        Instruction::LocalGet(index_sp()),
                        Instruction::I32Const(1),
                        Instruction::I32Sub,
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::I32Load(mem_arg()),
                        Instruction::LocalTee(index_temp2()), // x
                        Instruction::I32Const((Word::BITS as i32).ilog2() as i32),
                        Instruction::I32ShrU,
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::I32Add,
                        Instruction::LocalTee(index_temp()), // address
                        Instruction::LocalGet(index_temp()), // address
                        Instruction::I32Load(mem_arg()),

                        Instruction::I32Const(1),
                        Instruction::LocalGet(index_temp2()), // x
                        Instruction::I32Const((1 << (Word::BITS as i32).ilog2() as i32) - 1),
                        Instruction::I32And,
                        Instruction::I32Shl,
                        Instruction::LocalTee(index_temp2()), // bitmask
                        Instruction::I32Const(-1),
                        Instruction::I32Xor,
                        Instruction::I32And,


                        Instruction::GlobalGet(index_screen_color()),
                        Instruction::LocalGet(index_temp2()), // bitmask
                        Instruction::I32And,
                        Instruction::I32Or,

                        Instruction::I32Store(mem_arg()),
                    ]);
                }
                "Memory.init" => {
                    let heap_start = 0x800;
                    let heap_end = RAM::SCREEN as i32;

                    wasm_instructions.extend([
                        Instruction::I32Const(heap_start * 4),
                        Instruction::I32Const(heap_end - heap_start),
                        Instruction::I32Store(mem_arg()),

                        Instruction::I32Const(Register::SP.address() as i32 * 4),
                        Instruction::LocalGet(index_sp()),
                        Instruction::I32Const(1),
                        Instruction::I32Add,
                        Instruction::LocalTee(index_sp()),
                        Instruction::I32Store(mem_arg()),
                    ]);
                }
                "Memory.alloc" | "Array.new" => {
                    let heap_start = 0x800;
                    let heap_end = RAM::SCREEN as i32;

                    let continue_id = Id::new("alloc_continue", Span::from_offset(0));
                    // let break_id = Id::new("alloc_break", Span::from_offset(0));

                    wasm_instructions.extend([
                        Instruction::LocalGet(index_sp()),
                        Instruction::I32Const(1),
                        Instruction::I32Sub,
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::I32Load(mem_arg()),
                        Instruction::I32Const(1),
                        Instruction::I32Add,
                        Instruction::LocalSet(index_temp()), // size + 1

                        Instruction::I32Const(heap_start * 4),
                        Instruction::LocalSet(index_temp2()), // address

                        // Instruction::Block(Box::new(BlockType { label: Some(break_id.clone()), label_name: None, ty: TypeUse { index: None, inline: None } })),
                        Instruction::Loop(Box::new(BlockType { label: Some(continue_id.clone()), label_name: None, ty: TypeUse { index: None, inline: None } })),
                        // Instruction::LocalGet(index_temp2()), // address
                        // Instruction::Call(Index::Id(Id::new("print", Span::from_offset(0)))), // DEBUG

                        Instruction::LocalGet(index_temp2()), // address
                        Instruction::I32Load(mem_arg()),
                        Instruction::LocalGet(index_temp()),
                        Instruction::I32LtS,

                        Instruction::If(Box::new(BlockType { label: None, label_name: None, ty: TypeUse { index: None, inline: None } })),

                            Instruction::LocalGet(index_temp2()), // address
                            Instruction::LocalGet(index_temp2()), // address
                            Instruction::I32Load(mem_arg()),

                            Instruction::I32Const(0),
                            Instruction::LocalGet(index_temp2()), // address
                            Instruction::I32Load(mem_arg()),
                            Instruction::I32Sub,

                            Instruction::I32Const(0),
                            Instruction::LocalGet(index_temp2()), // address
                            Instruction::I32Load(mem_arg()),
                            Instruction::I32LtS,
                            Instruction::Select(SelectTypes { tys: None }),

                            Instruction::I32Const(2),
                            Instruction::I32Shl,
                            Instruction::I32Add,
                            Instruction::LocalTee(index_temp2()), // address
                            Instruction::I32Const(heap_end * 4),
                            Instruction::I32GeS,
                            Instruction::If(Box::new(BlockType { label: None, label_name: None, ty: TypeUse { index: None, inline: None } })),
                                Instruction::Unreachable,
                            Instruction::End(None),
                            Instruction::Br(Index::Id(continue_id)),

                        Instruction::End(None),

                        Instruction::LocalGet(index_temp()), // size + 1
                        Instruction::LocalGet(index_temp2()), // address
                        Instruction::I32Load(mem_arg()),
                        Instruction::I32Eq,
                        Instruction::If(Box::new(BlockType { label: None, label_name: None, ty: TypeUse { index: None, inline: None } })),
                            Instruction::LocalGet(index_temp2()), // address
                            Instruction::I32Const(0),
                            Instruction::LocalGet(index_temp()), // size + 1
                            Instruction::I32Sub,
                            Instruction::I32Store(mem_arg()),
                        Instruction::Else(None),
                            Instruction::LocalGet(index_temp2()),
                            Instruction::LocalGet(index_temp()), // size + 1
                            Instruction::I32Const(2),
                            Instruction::I32Shl,
                            Instruction::I32Add,

                            Instruction::LocalGet(index_temp2()), // address
                            Instruction::I32Load(mem_arg()),
                            Instruction::LocalGet(index_temp()), // size + 1
                            Instruction::I32Sub,

                            Instruction::I32Store(mem_arg()),

                            Instruction::LocalGet(index_temp2()), // address
                            Instruction::I32Const(0),
                            Instruction::LocalGet(index_temp()), // size + 1
                            Instruction::I32Sub,
                            Instruction::I32Store(mem_arg()),
                        Instruction::End(None),

                        Instruction::LocalGet(index_sp()),
                        Instruction::I32Const(1),
                        Instruction::I32Sub,
                        Instruction::I32Const(2),
                        Instruction::I32Shl,

                        Instruction::LocalGet(index_temp2()),
                        Instruction::I32Const(2),
                        Instruction::I32ShrU,
                        Instruction::I32Const(1),
                        Instruction::I32Add,

                        Instruction::I32Store(mem_arg()),

                        Instruction::End(None),
                        // Instruction::End(None)
                    ]);
                }
                "Memory.deAlloc" | "Array.dispose" => {
                    // Fragmented AF
                    wasm_instructions.extend([
                        Instruction::LocalGet(index_sp()),
                        Instruction::I32Const(1),
                        Instruction::I32Sub,
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::I32Load(mem_arg()),
                        Instruction::I32Const(1),
                        Instruction::I32Sub,
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::LocalTee(index_temp2()), // address
                        Instruction::I32Const(0),
                        Instruction::LocalGet(index_temp2()), // address
                        Instruction::I32Load(mem_arg()),
                        Instruction::I32Sub,
                        Instruction::I32Store(mem_arg()),
                    ]);
                }
                _ => {
                    wasm_instructions.extend([
                        Instruction::LocalGet(index_sp()),
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::I32Const(call_indices[&(index + 1)]),
                        Instruction::I32Store(mem_arg()),
                    ]);

                    for i in 1..=4 {
                        wasm_instructions.extend([
                            Instruction::LocalGet(index_sp()),
                            Instruction::I32Const(2),
                            Instruction::I32Shl,
                            Instruction::I32Const(i as i32 * 4),
                            Instruction::I32Load(mem_arg()),
                            Instruction::I32Store(mem_offset_arg(i)),
                        ]);
                    }

                    wasm_instructions.extend([
                        Instruction::I32Const(Register::ARG.address() as i32 * 4),
                        Instruction::LocalGet(index_sp()),
                        Instruction::I32Const(*argument_count as i32),
                        Instruction::I32Sub,
                        Instruction::I32Store(mem_arg()),
                    ]);

                    wasm_instructions.extend([
                        Instruction::I32Const(Register::SP.address() as i32 * 4),
                        Instruction::LocalGet(index_sp()),
                        Instruction::I32Const(5),
                        Instruction::I32Add,
                        Instruction::LocalTee(index_sp()),
                        Instruction::I32Store(mem_arg()),
                    ]);

                    wasm_instructions.extend([
                        Instruction::I32Const(Register::LCL.address() as i32 * 4),
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
                }
            }
        }
        VMCommand::Return => {
            // Store frame pointer and put return address in jump target
            wasm_instructions.extend(load_register(Register::LCL));
            wasm_instructions.extend([
                Instruction::I32Const(5),
                Instruction::I32Sub,
                Instruction::I32Const(2),
                Instruction::I32Shl,
                Instruction::LocalTee(index_temp()), // frame
                Instruction::I32Load(mem_arg()),
                Instruction::LocalSet(index_jump_target()),
            ]);

            // Move return value to beginning of argument segment
            wasm_instructions.extend(load_register(Register::ARG));
            wasm_instructions.extend([
                Instruction::I32Const(2),
                Instruction::I32Shl,
                Instruction::LocalGet(index_sp()),
                Instruction::I32Const(1),
                Instruction::I32Sub,
                Instruction::I32Const(2),
                Instruction::I32Shl,
                Instruction::I32Load(mem_arg()),
                Instruction::I32Store(mem_arg()),
            ]);

            // Set stack pointer to after return value
            wasm_instructions.push(Instruction::I32Const(Register::SP.address() as i32 * 4));
            wasm_instructions.extend(load_register(Register::ARG));
            wasm_instructions.extend([
                Instruction::I32Const(1),
                Instruction::I32Add,
                Instruction::LocalTee(index_sp()),
                Instruction::I32Store(mem_arg()),
            ]);

            // Restore frame
            for i in 1..=4 {
                wasm_instructions.extend([
                    Instruction::I32Const(i as i32 * 4),
                    Instruction::LocalGet(index_temp()), // frame
                    Instruction::I32Load(mem_offset_arg(i)),
                    Instruction::I32Store(mem_arg()),
                ]);
            }

            wasm_instructions.push(Instruction::Br(jump_index))
        }
    }

    wasm_instructions
}

fn program_to_static_cases(
    program: &Program,
    loop_id: Id<'static>,
) -> (Vec<Vec<Instruction<'static>>>, i32) {
    let mut label_indices = HashMap::new();
    let mut function_indices = HashMap::new();
    let mut call_indices = HashMap::new();
    let mut case_index = 0;
    let mut case_starts = HashSet::new();
    let mut start_case_index = None;
    let mut current_function_name = None;
    for (i, command) in program
        .files
        .iter()
        .flat_map(|f| f.commands(&program.all_commands).iter())
        .enumerate()
    {
        match command {
            VMCommand::Label { name } => {
                label_indices.insert(
                    format!("{}.{}", current_function_name.unwrap(), name),
                    case_index,
                );
                if !case_starts.contains(&i) {
                    case_starts.insert(i);
                    case_index += 1;
                }
            }
            VMCommand::Function { name, .. } => {
                current_function_name = Some(name);
                if name == "Sys.init" {
                    start_case_index = Some(case_index);
                }
                function_indices.insert(name.clone(), case_index);
                case_starts.insert(i);
                case_index += 1;
            }
            VMCommand::Call { .. } => {
                call_indices.insert(i + 1, case_index);
                case_starts.insert(i + 1);
                case_index += 1;
            }
            _ => {}
        }
    }
    case_starts.insert(program.all_commands.len());

    let mut cases = vec![];
    let mut current_case = vec![];

    for (index, (static_segment_start, command)) in program
        .files
        .iter()
        .flat_map(|f| {
            f.commands(&program.all_commands)
                .iter()
                .map(|command| (*f.static_segment.start(), command))
        })
        .enumerate()
    {
        if let VMCommand::Function { name, .. } = command {
            current_function_name = Some(name);
        }
        let mut jump_index = Index::Id(loop_id);

        match command {
            VMCommand::Goto { label_name } | VMCommand::IfGoto { label_name } => {
                let target_index = label_indices
                    [&format!("{}.{}", current_function_name.unwrap(), label_name)]
                    as i32;
                if target_index > cases.len() as i32 {
                    jump_index = Index::Num(
                        (target_index - cases.len() as i32) as u32 - 1,
                        Span::from_offset(0),
                    );
                }
            }
            VMCommand::Call { function_name, .. } => {
                if let Some(target_index) = function_indices.get(function_name) && *target_index > cases.len() as i32 {
                    jump_index = Index::Num(
                        (target_index - cases.len() as i32) as u32 - 1,
                        Span::from_offset(0),
                    );
                }
            }
            _ => {}
        }

        let instructions = command_to_wasm(
            command,
            index,
            jump_index,
            static_segment_start,
            current_function_name,
            &label_indices,
            &function_indices,
            &call_indices,
        );

        current_case.extend(instructions);

        // current_case.extend([
        //     Instruction::I32Const(index as i32 + 1),
        //     Instruction::LocalSet(index_jump_target()),
        //     Instruction::Br(Index::Id(loop_id)),
        // ]);

        if case_starts.contains(&(index + 1)) {
            cases.push(current_case);
            current_case = vec![];
        }
    }

    (cases, start_case_index.unwrap_or(0))
}

pub fn vm_to_wasm(program: &Program, with_limit: bool) -> Result<Vec<u8>, String> {
    let loop_id = Id::new("loop", Span::from_offset(0));

    let (cases, start_case_index) = program_to_static_cases(program, loop_id);
    let case_count = cases.len();

    let memory_id = Id::new("memory", Span::from_offset(0));

    let expression = ExpressionBuilder::default()
        .instr(Instruction::GlobalGet(index_jump_target()))
        .instr(Instruction::LocalSet(index_jump_target()))
        .instr(Instruction::I32Const(Register::SP.address() as i32))
        .instr(Instruction::I32Load(mem_arg()))
        .instr(Instruction::LocalSet(index_sp()))
        .with_loop(loop_id, |mut builder| {
            if with_limit {
                builder = builder
                    .instr(Instruction::LocalGet(index_ticks()))
                    .instr(Instruction::LocalGet(Index::Num(0, Span::from_offset(0))))
                    .instr(Instruction::I32GeU)
                    .instr(Instruction::If(Box::new(BlockType {
                        label: None,
                        label_name: None,
                        ty: TypeUse {
                            index: None,
                            inline: None,
                        },
                    })))
                    .instr(Instruction::LocalGet(index_jump_target()))
                    .instr(Instruction::GlobalSet(index_jump_target()))
                    .instr(Instruction::LocalGet(index_ticks()))
                    .instr(Instruction::Return)
                    .instr(Instruction::End(None));
            }
            builder.switch(index_jump_target(), cases, vec![], HashMap::new())
        })
        .instr(Instruction::I32Const(case_count as i32))
        .instr(Instruction::GlobalSet(index_jump_target()))
        .instr(Instruction::LocalGet(index_ticks()))
        .build();

    let params = if with_limit {
        vec![(None, None, ValType::I32)]
    } else {
        vec![]
    };

    let mut m = ModuleBuilder::default()
        // .field(ModuleField::Import(Import { span: Span::from_offset(0), module: "env", field: "print", item: ItemSig { span: Span::from_offset(0), id: Some(Id::new("print", Span::from_offset(0))), name: None, kind: wast::core::ItemKind::Func(TypeUse { index: None, inline: Some(FunctionType { params: Box::new([(None, None, ValType::I32)]), results: Box::new([]) }) }) } }))
        .fields(
            globals(start_case_index)
                .into_iter()
                .map(ModuleField::Global)
                .collect(),
        )
        .field(ModuleField::Memory(create_memory(memory_id, 32768)))
        .field(ModuleField::Export(Export {
            span: Span::from_offset(0),
            name: "memory",
            kind: ExportKind::Memory,
            item: Index::Id(memory_id),
        }))
        .field(ModuleField::Func(
            FuncBuilder::default()
                .export("run")
                .kind(FuncKind::Inline {
                    locals: locals(),
                    expression,
                })
                .ty(TypeUse {
                    index: None,
                    inline: Some(FunctionType {
                        params: params.into(),
                        results: [ValType::I32].into(),
                    }),
                })
                .build(),
        ))
        .build();
    let unoptimized_data = m.encode().map_err(|e| e.to_string())?;

    // std::fs::write("wasm.out", &unoptimized_data).unwrap();

    Ok(unoptimized_data)
}
