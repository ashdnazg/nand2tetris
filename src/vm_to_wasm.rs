use std::collections::{HashMap, HashSet};

use wast::{
    core::{
        BlockType, Export, ExportKind, Expression, FuncKind, FunctionType, Global, GlobalKind, GlobalType, Import, InlineExport, Instruction, ItemSig, Local, MemArg, MemoryArg, ModuleField, SelectTypes, TypeUse, ValType
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

fn id_lcl() -> Id<'static> {
    Id::new("lcl", Span::from_offset(0))
}

fn id_arg() -> Id<'static> {
    Id::new("arg", Span::from_offset(0))
}

fn id_this() -> Id<'static> {
    Id::new("this", Span::from_offset(0))
}

fn id_that() -> Id<'static> {
    Id::new("that", Span::from_offset(0))
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

fn id_temp3() -> Id<'static> {
    Id::new("temp3", Span::from_offset(0))
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

fn index_lcl() -> Index<'static> {
    Index::Id(id_lcl())
}

fn index_arg() -> Index<'static> {
    Index::Id(id_arg())
}

fn index_this() -> Index<'static> {
    Index::Id(id_this())
}

fn index_that() -> Index<'static> {
    Index::Id(id_that())
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

fn index_temp3() -> Index<'static> {
    Index::Id(id_temp3())
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
            id: Some(id_lcl()),
            name: None,
            ty: ValType::I32,
        },
        Local {
            id: Some(id_arg()),
            name: None,
            ty: ValType::I32,
        },
        Local {
            id: Some(id_this()),
            name: None,
            ty: ValType::I32,
        },
        Local {
            id: Some(id_that()),
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
        Local {
            id: Some(id_temp3()),
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
        Instruction::I32Const(4),
        Instruction::I32Sub,
    ];
    ret.extend(prefix);
    ret.extend([
        Instruction::LocalGet(index_sp()),
        Instruction::I32Const(4),
        Instruction::I32Sub,
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
        Instruction::LocalGet(index_sp()),
        Instruction::I32Const(4),
        Instruction::I32Sub,
        Instruction::LocalTee(index_sp()),
        Instruction::I32Const(4),
        Instruction::I32Sub,
    ];
    ret.extend(prefix);
    ret.extend([
        Instruction::LocalGet(index_sp()),
        Instruction::I32Const(4),
        Instruction::I32Sub,
        Instruction::I32Load(mem_arg()),
        Instruction::LocalGet(index_sp()),
        Instruction::I32Load(mem_arg()),
    ]);
    ret.extend(op);
    ret.push(Instruction::I32Store(mem_arg()));

    ret
}

fn prepare_on_stack1(stack_size: &mut usize, wasm_instructions: &mut Vec<Instruction<'static>>) {
    if *stack_size > 0 {
        *stack_size -= 1;
    } else {
        wasm_instructions.extend([
            Instruction::LocalGet(index_sp()),
            Instruction::I32Const(4),
            Instruction::I32Sub,
            Instruction::LocalTee(index_sp()),
            Instruction::I32Load(mem_arg()),
        ]);
    }
}

fn prepare_on_stack2(stack_size: &mut usize, wasm_instructions: &mut Vec<Instruction<'static>>) {
    match stack_size {
        0 => {
            wasm_instructions.extend([
                Instruction::LocalGet(index_sp()),
                Instruction::I32Const(8),
                Instruction::I32Sub,
                Instruction::LocalTee(index_sp()),
                Instruction::I32Load(mem_arg()),
                Instruction::LocalGet(index_sp()),
                Instruction::I32Load(mem_offset_arg(1)),
            ]);
        }
        1 => {
            wasm_instructions.extend([
                Instruction::LocalSet(index_temp()), // value
                Instruction::LocalGet(index_sp()),
                Instruction::I32Const(4),
                Instruction::I32Sub,
                Instruction::LocalTee(index_sp()),
                Instruction::I32Load(mem_arg()),
                Instruction::LocalGet(index_temp()), // value
            ]);

            *stack_size -= 1;
        }
        _ => {
            *stack_size -= 2;
        }
    }
}

fn drop_stack_to_ram(stack_size: &mut usize, wasm_instructions: &mut Vec<Instruction<'static>>) {
    for i in 0..*stack_size {
        wasm_instructions.extend([
            Instruction::LocalSet(index_temp()),
            Instruction::LocalGet(index_sp()),
            Instruction::LocalGet(index_temp()),
            Instruction::I32Store(mem_offset_arg((*stack_size - i - 1) as i16)),
        ]);
    }
    if *stack_size > 0 {
        wasm_instructions.extend([
            Instruction::LocalGet(index_sp()),
            Instruction::I32Const(*stack_size as i32 * 4),
            Instruction::I32Add,
            Instruction::LocalSet(index_sp()),
        ]);
        *stack_size = 0;
    }
}

fn command_to_wasm2(
    command: &VMCommand,
    index: usize,
    jump_index: Index<'static>,
    static_segment_start: Word,
    current_function_name: Option<&String>,
    label_indices: &HashMap<String, i32>,
    function_indices: &HashMap<String, i32>,
    call_indices: &HashMap<usize, i32>,
    stack_size: &mut usize,
) -> Vec<Instruction<'static>> {
    let mut wasm_instructions: Vec<Instruction<'static>> = vec![
        Instruction::I32Const(1),
        Instruction::LocalGet(index_ticks()),
        Instruction::I32Add,
        Instruction::LocalSet(index_ticks()),
    ];

    match command {
        VMCommand::Add => {
            prepare_on_stack2(stack_size, &mut wasm_instructions);
            wasm_instructions.extend([Instruction::I32Add, Instruction::I32Extend16S]);
            *stack_size += 1;
        }
        VMCommand::Push { segment, offset } => {
            *stack_size += 1;
            use crate::vm::PushSegment;

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
                    wasm_instructions.extend([
                        Instruction::LocalGet(index_lcl()),
                        Instruction::I32Load(mem_offset_arg(*offset)),
                    ]);
                }
                PushSegment::Argument => {
                    wasm_instructions.extend([
                        Instruction::LocalGet(index_arg()),
                        Instruction::I32Load(mem_offset_arg(*offset)),
                    ]);
                }
                PushSegment::This => {
                    wasm_instructions.extend([
                        Instruction::LocalGet(index_this()),
                        Instruction::I32Load(mem_offset_arg(*offset)),
                    ]);
                }
                PushSegment::That => {
                    wasm_instructions.extend([
                        Instruction::LocalGet(index_that()),
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
                    let local_index = match offset {
                        0 => index_this(),
                        1 => index_that(),
                        _ => panic!("Invalid offset for pointer pop"),
                    };
                    wasm_instructions.extend([
                        Instruction::LocalGet(local_index),
                        Instruction::I32Const(2),
                        Instruction::I32ShrS,
                    ]);
                }
            }
        }
        VMCommand::Pop { segment, offset } => {
            use crate::vm::PopSegment;

            prepare_on_stack1(stack_size, &mut wasm_instructions);

            if matches!(segment, PopSegment::Pointer) {
                let local_index = match offset {
                    0 => index_this(),
                    1 => index_that(),
                    _ => panic!("Invalid offset for pointer pop"),
                };
                wasm_instructions.extend([
                    Instruction::I32Const(2),
                    Instruction::I32Shl,
                    Instruction::LocalSet(local_index),
                ]);
                return wasm_instructions;
            }

            wasm_instructions.push(Instruction::LocalSet(index_temp())); // value

            match segment {
                PopSegment::Static => {
                    wasm_instructions
                        .extend([Instruction::I32Const(static_segment_start as i32 * 4)]);
                }
                PopSegment::Local => {
                    wasm_instructions.push(Instruction::LocalGet(index_lcl()));
                }
                PopSegment::Argument => {
                    wasm_instructions.push(Instruction::LocalGet(index_arg()));
                }
                PopSegment::This => {
                    wasm_instructions.push(Instruction::LocalGet(index_this()));
                }
                PopSegment::That => {
                    wasm_instructions.push(Instruction::LocalGet(index_that()));
                }
                PopSegment::Temp => {
                    wasm_instructions.extend([Instruction::I32Const(
                        Register::TEMP(0).address() as i32 * 4,
                    )]);
                }
                PopSegment::Pointer => {
                    unreachable!("Already handled");
                }
            }

            wasm_instructions.extend([
                Instruction::LocalGet(index_temp()), // value
                Instruction::I32Store(mem_offset_arg(*offset)),
            ]);
        }
        VMCommand::Sub => {
            prepare_on_stack2(stack_size, &mut wasm_instructions);
            wasm_instructions.extend([Instruction::I32Sub, Instruction::I32Extend16S]);
            *stack_size += 1;
        }
        VMCommand::Neg => {
            prepare_on_stack1(stack_size, &mut wasm_instructions);
            wasm_instructions.extend([
                Instruction::LocalSet(index_temp()), // value
                Instruction::I32Const(0),
                Instruction::LocalGet(index_temp()), // value
                Instruction::I32Sub,
                Instruction::I32Extend16S,
            ]);
            *stack_size += 1;
        }
        VMCommand::Eq => {
            prepare_on_stack2(stack_size, &mut wasm_instructions);
            wasm_instructions.extend([
                Instruction::I32Eq,
                Instruction::LocalSet(index_temp()), // value
                Instruction::I32Const(0),
                Instruction::LocalGet(index_temp()), // value
                Instruction::I32Sub,
            ]);
            *stack_size += 1;
        }
        VMCommand::Gt => {
            prepare_on_stack2(stack_size, &mut wasm_instructions);
            wasm_instructions.extend([
                Instruction::I32GtS,
                Instruction::LocalSet(index_temp()), // value
                Instruction::I32Const(0),
                Instruction::LocalGet(index_temp()), // value
                Instruction::I32Sub,
            ]);
            *stack_size += 1;
        }
        VMCommand::Lt => {
            prepare_on_stack2(stack_size, &mut wasm_instructions);
            wasm_instructions.extend([
                Instruction::I32LtS,
                Instruction::LocalSet(index_temp()), // value
                Instruction::I32Const(0),
                Instruction::LocalGet(index_temp()), // value
                Instruction::I32Sub,
            ]);
            *stack_size += 1;
        }
        VMCommand::And => {
            prepare_on_stack2(stack_size, &mut wasm_instructions);
            wasm_instructions.extend([Instruction::I32And, Instruction::I32Extend16S]);
            *stack_size += 1;
        }
        VMCommand::Or => {
            prepare_on_stack2(stack_size, &mut wasm_instructions);
            wasm_instructions.extend([Instruction::I32Or, Instruction::I32Extend16S]);
            *stack_size += 1;
        }
        VMCommand::Not => {
            prepare_on_stack1(stack_size, &mut wasm_instructions);
            wasm_instructions.extend([Instruction::I32Const(-1), Instruction::I32Xor]);
            *stack_size += 1;
        }
        VMCommand::Label { .. } => {
        }
        VMCommand::Goto { label_name } => {
            drop_stack_to_ram(stack_size, &mut wasm_instructions);
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

            if *stack_size > 1 {
                drop_stack_to_ram(stack_size, &mut wasm_instructions);
            }
            prepare_on_stack1(stack_size, &mut wasm_instructions);

            wasm_instructions.extend([
                Instruction::I32Const(0),
                Instruction::I32Ne,
                Instruction::BrIf(jump_index),
            ]);
        }
        VMCommand::Function {
            local_var_count, ..
        } => match local_var_count {
            0 => {}
            1 => {
                wasm_instructions.extend([
                    Instruction::LocalGet(index_sp()),
                    Instruction::I32Const(0),
                    Instruction::I32Store(mem_arg()),
                    Instruction::LocalGet(index_sp()),
                    Instruction::I32Const(4),
                    Instruction::I32Add,
                    Instruction::LocalSet(index_sp()),
                ]);
            }
            _ => {
                wasm_instructions.extend([
                    Instruction::LocalGet(index_sp()),
                    Instruction::I32Const(0),
                    Instruction::I32Const(*local_var_count as i32 * 4),
                    Instruction::MemoryFill(MemoryArg {
                        mem: Index::Num(0, Span::from_offset(0)),
                    }),
                    Instruction::LocalGet(index_sp()),
                    Instruction::I32Const(*local_var_count as i32 * 4),
                    Instruction::I32Add,
                    Instruction::LocalSet(index_sp()),
                ]);
            }
        },
        VMCommand::Call {
            function_name,
            argument_count,
        } => {
            match function_name.as_str() {
                "Math.multiply" => {
                    prepare_on_stack2(stack_size, &mut wasm_instructions);
                    wasm_instructions.extend([Instruction::I32Mul, Instruction::I32Extend16S]);
                    *stack_size += 1;
                }
                "Math.divide" => {
                    prepare_on_stack2(stack_size, &mut wasm_instructions);
                    wasm_instructions.extend([Instruction::I32DivS, Instruction::I32Extend16S]);
                    *stack_size += 1;
                }
                "Screen.clearScreen" => {
                    wasm_instructions.extend([
                        Instruction::I32Const(RAM::SCREEN as i32 * 4),
                        Instruction::I32Const(0),
                        Instruction::I32Const((RAM::KBD - RAM::SCREEN) as i32 * 4),
                        Instruction::MemoryFill(MemoryArg {
                            mem: Index::Num(0, Span::from_offset(0)),
                        }),
                        Instruction::I32Const(0),
                    ]);
                    *stack_size += 1;
                }
                "Screen.setColor" => {
                    prepare_on_stack1(stack_size, &mut wasm_instructions);
                    wasm_instructions.extend([
                        Instruction::GlobalSet(index_screen_color()),
                        Instruction::I32Const(0),
                    ]);
                    *stack_size += 1;
                }
                "Screen.drawPixel" => {
                    prepare_on_stack2(stack_size, &mut wasm_instructions);
                    wasm_instructions.extend([
                        Instruction::I32Const(RAM::SCREEN_ROW_LENGTH as i32 * 4),
                        Instruction::I32Mul,
                        Instruction::I32Const(RAM::SCREEN as i32 * 4),
                        Instruction::I32Add,
                        Instruction::LocalSet(index_temp()), // address
                        Instruction::LocalTee(index_temp2()), // x
                        Instruction::I32Const((Word::BITS as i32).ilog2() as i32),
                        Instruction::I32ShrU,
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::LocalGet(index_temp()), // address
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
                        Instruction::I32Extend16S,
                        Instruction::I32Store(mem_arg()),
                        Instruction::I32Const(0),
                    ]);
                    *stack_size += 1;
                }
                "Memory.init" => {
                    let heap_start = RAM::HEAP as i32;
                    let heap_end = RAM::SCREEN as i32;

                    wasm_instructions.extend([
                        Instruction::I32Const(heap_start * 4),
                        // Free head
                        Instruction::I32Const(heap_start * 4 + 4),
                        Instruction::I32Store(mem_arg()),

                        Instruction::I32Const(heap_start * 4 + 4),
                        // Initial node size
                        Instruction::I32Const(heap_end - heap_start - 2),
                        Instruction::I32Store(mem_arg()),

                        Instruction::I32Const(heap_start * 4 + 8),
                        // End of list
                        Instruction::I32Const(0),
                        Instruction::I32Store(mem_arg()),
                        Instruction::I32Const(0),
                    ]);
                    *stack_size += 1;
                }
                "Memory.alloc" | "Array.new" => {
                    let heap_start = RAM::HEAP as i32;

                    let continue_id = Id::new("alloc_continue", Span::from_offset(0));

                    prepare_on_stack1(stack_size, &mut wasm_instructions);

                    wasm_instructions.extend([
                        Instruction::LocalSet(index_temp()), // size

                        Instruction::I32Const(heap_start * 4 - 4),
                        Instruction::LocalTee(index_temp3()), // prev
                        Instruction::I32Load(mem_offset_arg(1)),
                        Instruction::LocalSet(index_temp2()), // address
                        Instruction::Loop(Box::new(BlockType {
                            label: Some(continue_id),
                            label_name: None,
                            ty: TypeUse {
                                index: None,
                                inline: None,
                            },
                        })),

                        Instruction::LocalGet(index_temp2()), // address
                        Instruction::I32Load(mem_arg()),
                        Instruction::LocalGet(index_temp()), // size
                        Instruction::I32Eq,
                        // Did we find a node with the right size?
                        Instruction::If(Box::new(BlockType {
                            label: None,
                            label_name: None,
                            ty: TypeUse {
                                index: None,
                                inline: None,
                            },
                        })),
                            Instruction::LocalGet(index_temp3()), // prev
                            Instruction::LocalGet(index_temp2()), // address
                            Instruction::I32Load(mem_offset_arg(1)),
                            Instruction::I32Store(mem_offset_arg(1)),
                        Instruction::Else(None),
                            Instruction::LocalGet(index_temp2()), // address
                            Instruction::I32Load(mem_offset_arg(1)),
                            Instruction::I32Eqz,
                            // Are we at the last node and therefore we have to split it?
                            Instruction::If(Box::new(BlockType {
                                label: None,
                                label_name: None,
                                ty: TypeUse {
                                    index: None,
                                    inline: None,
                                },
                            })),

                                Instruction::LocalGet(index_temp3()), // prev

                                Instruction::LocalGet(index_temp2()), // address
                                Instruction::LocalGet(index_temp()), // size
                                Instruction::I32Const(2),
                                Instruction::I32Shl,
                                Instruction::I32Add,
                                Instruction::LocalTee(index_temp3()), // address + size * 4
                                Instruction::I32Const(4),
                                Instruction::I32Add,

                                Instruction::I32Store(mem_offset_arg(1)),

                                Instruction::LocalGet(index_temp3()), // address + size * 4

                                Instruction::LocalGet(index_temp2()), // address
                                Instruction::I32Load(mem_arg()),
                                Instruction::LocalGet(index_temp()), // size
                                Instruction::I32Sub,
                                Instruction::I32Const(1),
                                Instruction::I32Sub,

                                Instruction::I32Store(mem_offset_arg(1)),

                                Instruction::LocalGet(index_temp3()), // address + size * 4

                                // If we split, we know it's the last node
                                Instruction::I32Const(0),

                                Instruction::I32Store(mem_offset_arg(2)),

                                Instruction::LocalGet(index_temp2()), // address
                                Instruction::LocalGet(index_temp()), // size
                                Instruction::I32Store(mem_arg()),

                            Instruction::Else(None),
                                Instruction::LocalGet(index_temp2()), // address
                                Instruction::LocalTee(index_temp3()), // prev
                                Instruction::I32Load(mem_offset_arg(1)),
                                Instruction::LocalSet(index_temp2()), // address
                                Instruction::Br(Index::Id(continue_id)),
                            Instruction::End(None),
                        Instruction::End(None),
                        Instruction::End(None),

                        Instruction::LocalGet(index_temp2()), // address
                        Instruction::I32Const(2),
                        Instruction::I32ShrU,
                        Instruction::I32Const(1),
                        Instruction::I32Add,
                    ]);

                    *stack_size += 1;
                }
                "Memory.deAlloc" | "Array.dispose" => {
                    prepare_on_stack1(stack_size, &mut wasm_instructions);
                    // Fragmented AF
                    wasm_instructions.extend([
                        Instruction::I32Const(1),
                        Instruction::I32Sub,
                        Instruction::I32Const(2),
                        Instruction::I32Shl,

                        Instruction::LocalTee(index_temp2()), // address
                        Instruction::I32Const(RAM::HEAP as i32 * 4),
                        Instruction::I32Load(mem_arg()),
                        Instruction::I32Store(mem_offset_arg(1)),

                        Instruction::I32Const(RAM::HEAP as i32 * 4),
                        Instruction::LocalGet(index_temp2()), // address
                        Instruction::I32Store(mem_arg()),

                        Instruction::I32Const(0),
                    ]);
                    *stack_size += 1;
                }
                _ => {
                    drop_stack_to_ram(stack_size, &mut wasm_instructions);

                    wasm_instructions.extend([
                        Instruction::LocalGet(index_sp()),
                        Instruction::I32Const(call_indices[&(index + 1)]),
                        Instruction::I32Store(mem_arg()),
                    ]);

                    wasm_instructions.extend([
                        Instruction::LocalGet(index_sp()),
                        Instruction::LocalGet(index_lcl()),
                        Instruction::I32Store(mem_offset_arg(Register::LCL.address())),
                        Instruction::LocalGet(index_sp()),
                        Instruction::LocalGet(index_arg()),
                        Instruction::I32Store(mem_offset_arg(Register::ARG.address())),
                        Instruction::LocalGet(index_sp()),
                        Instruction::LocalGet(index_this()),
                        Instruction::I32Store(mem_offset_arg(Register::THIS.address())),
                        Instruction::LocalGet(index_sp()),
                        Instruction::LocalGet(index_that()),
                        Instruction::I32Store(mem_offset_arg(Register::THAT.address())),
                    ]);

                    wasm_instructions.push(Instruction::LocalGet(index_sp()));

                    if *argument_count > 0 {
                        wasm_instructions.extend([
                            Instruction::I32Const(*argument_count as i32 * 4),
                            Instruction::I32Sub,
                        ]);
                    }

                    wasm_instructions.push(Instruction::LocalSet(index_arg()));

                    wasm_instructions.extend([
                        Instruction::LocalGet(index_sp()),
                        Instruction::I32Const(20),
                        Instruction::I32Add,
                        Instruction::LocalTee(index_sp()),
                        Instruction::LocalSet(index_lcl()),
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
            prepare_on_stack1(stack_size, &mut wasm_instructions);
            wasm_instructions.push(Instruction::LocalSet(index_temp2())); // return value

            // Store frame pointer and put return address in jump target
            wasm_instructions.extend([
                Instruction::LocalGet(index_lcl()),
                Instruction::I32Const(20),
                Instruction::I32Sub,
                Instruction::LocalTee(index_temp()), // frame
                Instruction::I32Load(mem_arg()),
                Instruction::LocalSet(index_jump_target()),
            ]);

            // Move return value to beginning of argument segment
            wasm_instructions.extend([
                Instruction::LocalGet(index_arg()),
                Instruction::LocalGet(index_temp2()), // return value
                Instruction::I32Store(mem_arg()),
            ]);

            // Set stack pointer to after return value
            wasm_instructions.extend([
                Instruction::LocalGet(index_arg()),
                Instruction::I32Const(4),
                Instruction::I32Add,
                Instruction::LocalSet(index_sp()),
            ]);

            // Restore frame
            wasm_instructions.extend([
                Instruction::LocalGet(index_temp()), // frame
                Instruction::I32Load(mem_offset_arg(Register::LCL.address())),
                Instruction::LocalSet(index_lcl()),
                Instruction::LocalGet(index_temp()), // frame
                Instruction::I32Load(mem_offset_arg(Register::ARG.address())),
                Instruction::LocalSet(index_arg()),
                Instruction::LocalGet(index_temp()), // frame
                Instruction::I32Load(mem_offset_arg(Register::THIS.address())),
                Instruction::LocalSet(index_this()),
                Instruction::LocalGet(index_temp()), // frame
                Instruction::I32Load(mem_offset_arg(Register::THAT.address())),
                Instruction::LocalSet(index_that()),
            ]);

            wasm_instructions.push(Instruction::Br(jump_index))
        }
    }

    wasm_instructions
}

fn program_to_dynamic_cases(
    program: &Program,
    loop_id: Id<'static>,
) -> (Vec<Vec<Instruction<'static>>>, i32) {
    let mut label_indices = HashMap::new();
    let mut function_indices = HashMap::new();
    let mut call_indices = HashMap::new();
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
                    i as i32,
                );
            }
            VMCommand::Function { name, .. } => {
                current_function_name = Some(name);
                if name == "Sys.init" {
                    start_case_index = Some(i as i32);
                }
                function_indices.insert(name.clone(), i as i32);
            }
            VMCommand::Call { .. } => {
                call_indices.insert(i + 1, i as i32 + 1);
            }
            _ => {}
        }
    }

    let mut cases = vec![];
    let mut current_case = vec![];
    let mut stack_size = 0;

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
        let jump_index = Index::Id(loop_id);

        let instructions = command_to_wasm2(
            command,
            index,
            jump_index,
            static_segment_start,
            current_function_name,
            &label_indices,
            &function_indices,
            &call_indices,
            &mut stack_size,
        );

        current_case.extend(instructions);

        drop_stack_to_ram(&mut stack_size, &mut current_case);
        current_case.extend([
            Instruction::I32Const(index as i32 + 1),
            Instruction::LocalSet(index_jump_target()),
            Instruction::Br(Index::Id(loop_id)),
        ]);
        cases.push(current_case);
        current_case = vec![];
    }

    (cases, start_case_index.unwrap_or(0))
}

fn program_to_static_cases(
    program: &Program,
    loop_id: Id<'static>,
) -> (Vec<Vec<Instruction<'static>>>, i32, Vec<i32>) {
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
                if !case_starts.contains(&i) {
                    case_starts.insert(i);
                    case_index += 1;
                }
                label_indices.insert(
                    format!("{}.{}", current_function_name.unwrap(), name),
                    case_index - 1,
                );
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
    let mut stack_size = 0;

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
                if let Some(target_index) = function_indices.get(function_name)
                    && *target_index > cases.len() as i32
                {
                    jump_index = Index::Num(
                        (target_index - cases.len() as i32) as u32 - 1,
                        Span::from_offset(0),
                    );
                }
            }
            _ => {}
        }

        let instructions = command_to_wasm2(
            command,
            index,
            jump_index,
            static_segment_start,
            current_function_name,
            &label_indices,
            &function_indices,
            &call_indices,
            &mut stack_size,
        );

        current_case.extend(instructions);


        if case_starts.contains(&(index + 1)) {
            drop_stack_to_ram(&mut stack_size, &mut current_case);
            cases.push(current_case);
            current_case = vec![];
        }
    }

    let mut case_starts: Vec<_> = case_starts.into_iter().map(|i| i as i32).collect();
    case_starts.sort();

    (cases, start_case_index.unwrap_or(0), case_starts)
}

pub fn expression_from_cases(loop_id: Id<'static>, cases: Vec<Vec<Instruction<'static>>>, with_limit: bool) -> Expression<'static> {
    let case_count = cases.len();
    ExpressionBuilder::default()
        .instr(Instruction::GlobalGet(index_jump_target()))
        .instr(Instruction::LocalSet(index_jump_target()))
        .instr(Instruction::I32Const(Register::SP.address() as i32 * 4))
        .instr(Instruction::I32Load(mem_arg()))
        .instr(Instruction::I32Const(2))
        .instr(Instruction::I32Shl)
        .instr(Instruction::LocalSet(index_sp()))
        .instr(Instruction::I32Const(Register::LCL.address() as i32 * 4))
        .instr(Instruction::I32Load(mem_arg()))
        .instr(Instruction::I32Const(2))
        .instr(Instruction::I32Shl)
        .instr(Instruction::LocalSet(index_lcl()))
        .instr(Instruction::I32Const(Register::ARG.address() as i32 * 4))
        .instr(Instruction::I32Load(mem_arg()))
        .instr(Instruction::I32Const(2))
        .instr(Instruction::I32Shl)
        .instr(Instruction::LocalSet(index_arg()))
        .instr(Instruction::I32Const(Register::THIS.address() as i32 * 4))
        .instr(Instruction::I32Load(mem_arg()))
        .instr(Instruction::I32Const(2))
        .instr(Instruction::I32Shl)
        .instr(Instruction::LocalSet(index_this()))
        .instr(Instruction::I32Const(Register::THAT.address() as i32 * 4))
        .instr(Instruction::I32Load(mem_arg()))
        .instr(Instruction::I32Const(2))
        .instr(Instruction::I32Shl)
        .instr(Instruction::LocalSet(index_that()))
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
                    .instr(Instruction::I32Const(Register::SP.address() as i32 * 4))
                    .instr(Instruction::LocalGet(index_sp()))
                    .instr(Instruction::I32Const(2))
                    .instr(Instruction::I32ShrU)
                    .instr(Instruction::I32Store(mem_arg()))
                    .instr(Instruction::I32Const(Register::LCL.address() as i32 * 4))
                    .instr(Instruction::LocalGet(index_lcl()))
                    .instr(Instruction::I32Const(2))
                    .instr(Instruction::I32ShrU)
                    .instr(Instruction::I32Store(mem_arg()))
                    .instr(Instruction::I32Const(Register::ARG.address() as i32 * 4))
                    .instr(Instruction::LocalGet(index_arg()))
                    .instr(Instruction::I32Const(2))
                    .instr(Instruction::I32ShrU)
                    .instr(Instruction::I32Store(mem_arg()))
                    .instr(Instruction::I32Const(Register::THIS.address() as i32 * 4))
                    .instr(Instruction::LocalGet(index_this()))
                    .instr(Instruction::I32Const(2))
                    .instr(Instruction::I32ShrU)
                    .instr(Instruction::I32Store(mem_arg()))
                    .instr(Instruction::I32Const(Register::THAT.address() as i32 * 4))
                    .instr(Instruction::LocalGet(index_that()))
                    .instr(Instruction::I32Const(2))
                    .instr(Instruction::I32ShrU)
                    .instr(Instruction::I32Store(mem_arg()))
                    .instr(Instruction::Return)
                    .instr(Instruction::End(None));
            }
            builder.switch(index_jump_target(), cases, vec![
                Instruction::Unreachable
            ], HashMap::new())
        })
        .instr(Instruction::I32Const(case_count as i32))
        .instr(Instruction::GlobalSet(index_jump_target()))
        .instr(Instruction::I32Const(Register::SP.address() as i32 * 4))
        .instr(Instruction::LocalGet(index_sp()))
        .instr(Instruction::I32Const(2))
        .instr(Instruction::I32ShrU)
        .instr(Instruction::I32Store(mem_arg()))
        .instr(Instruction::I32Const(Register::LCL.address() as i32 * 4))
        .instr(Instruction::LocalGet(index_lcl()))
        .instr(Instruction::I32Const(2))
        .instr(Instruction::I32ShrU)
        .instr(Instruction::I32Store(mem_arg()))
        .instr(Instruction::I32Const(Register::ARG.address() as i32 * 4))
        .instr(Instruction::LocalGet(index_arg()))
        .instr(Instruction::I32Const(2))
        .instr(Instruction::I32ShrU)
        .instr(Instruction::I32Store(mem_arg()))
        .instr(Instruction::I32Const(Register::THIS.address() as i32 * 4))
        .instr(Instruction::LocalGet(index_this()))
        .instr(Instruction::I32Const(2))
        .instr(Instruction::I32ShrU)
        .instr(Instruction::I32Store(mem_arg()))
        .instr(Instruction::I32Const(Register::THAT.address() as i32 * 4))
        .instr(Instruction::LocalGet(index_that()))
        .instr(Instruction::I32Const(2))
        .instr(Instruction::I32ShrU)
        .instr(Instruction::I32Store(mem_arg()))
        .instr(Instruction::LocalGet(index_ticks()))
        .build()
}

pub fn vm_to_wasm(program: &Program, with_limit: bool) -> Result<(Vec<u8>, Vec<i32>), String> {
    let loop_id = Id::new("loop", Span::from_offset(0));

    let (static_cases, static_start_case_index, case_starts) = program_to_static_cases(program, loop_id);
    // let (dynamic_cases, dynamic_start_case_index) = program_to_dynamic_cases(program, loop_id);
    // assert_eq!(case_starts[static_start_case_index as usize], dynamic_start_case_index);

    let static_expression = expression_from_cases(loop_id, static_cases, with_limit);
    // let dynamic_expression = expression_from_cases(loop_id, dynamic_cases, with_limit);

    let memory_id = Id::new("memory", Span::from_offset(0));

    let params = if with_limit {
        vec![(None, None, ValType::I32)]
    } else {
        vec![]
    };

    let mut m = ModuleBuilder::default()
        // .field(ModuleField::Import(Import { span: Span::from_offset(0), module: "env", field: "print", item: ItemSig { span: Span::from_offset(0), id: Some(Id::new("print", Span::from_offset(0))), name: None, kind: wast::core::ItemKind::Func(TypeUse { index: None, inline: Some(FunctionType { params: Box::new([(None, None, ValType::I32)]), results: Box::new([]) }) }) } }))
        .fields(
            globals(static_start_case_index)
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
                    expression: static_expression,
                })
                .ty(TypeUse {
                    index: None,
                    inline: Some(FunctionType {
                        params: params.clone().into(),
                        results: [ValType::I32].into(),
                    }),
                })
                .build(),
        ))
        // .field(ModuleField::Func(
        //     FuncBuilder::default()
        //         .export("run_slow")
        //         .kind(FuncKind::Inline {
        //             locals: locals(),
        //             expression: dynamic_expression,
        //         })
        //         .ty(TypeUse {
        //             index: None,
        //             inline: Some(FunctionType {
        //                 params: params.into(),
        //                 results: [ValType::I32].into(),
        //             }),
        //         })
        //         .build(),
        // ))
        .build();
    let unoptimized_data = m.encode().map_err(|e| e.to_string())?;

    println!("{}", unoptimized_data.len());

    // std::fs::write("wasm.out", &unoptimized_data).unwrap();

    Ok((unoptimized_data, case_starts))
}
