use std::collections::{HashMap, HashSet};

use wast::{
    core::{
        BlockType, Export, ExportKind, FuncKind, FunctionType, Global, GlobalKind, GlobalType,
        InlineExport, Instruction, Local, MemArg, MemoryArg, ModuleField, TypeUse, ValType,
    },
    token::{Id, Index, Span},
};

use crate::{
    hardware::Word,
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

fn unary_stack_op(prefix: Vec<Instruction<'static>>, op: Vec<Instruction<'static>>) -> Vec<Instruction<'static>> {
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

fn binary_stack_op(prefix: Vec<Instruction<'static>>, op: Vec<Instruction<'static>>) -> Vec<Instruction<'static>> {
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
            wasm_instructions.extend(binary_stack_op(vec![], vec![
                Instruction::I32Add,
                Instruction::I32Extend16S,
            ]));
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
                        Instruction::I32Load(mem_offset_arg(*offset * 4))
                    ]);
                }
                PushSegment::Argument => {
                    wasm_instructions.extend(load_register(Register::ARG));
                    wasm_instructions.extend([
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::I32Load(mem_offset_arg(*offset * 4))
                    ]);
                }
                PushSegment::This => {
                    wasm_instructions.extend(load_register(Register::THIS));
                    wasm_instructions.extend([
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::I32Load(mem_offset_arg(*offset * 4))
                    ]);
                }
                PushSegment::That => {
                    wasm_instructions.extend(load_register(Register::THAT));
                    wasm_instructions.extend([
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::I32Load(mem_offset_arg(*offset * 4))
                    ]);
                }
                PushSegment::Temp => {
                    wasm_instructions.extend([
                        Instruction::I32Const(Register::TEMP(*offset).address() as i32 * 4),
                        Instruction::I32Load(mem_arg()),
                    ]);
                }
                PushSegment::Pointer => {
                    wasm_instructions.extend([
                        Instruction::I32Const((Register::THIS.address() + offset) as i32 * 4),
                        Instruction::I32Load(mem_arg()),
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
                    wasm_instructions.extend([
                        Instruction::I32Const((static_segment_start + offset) as i32 * 4),
                        Instruction::I32Load(mem_arg()),
                    ]);
                }
                PopSegment::Local => {
                    wasm_instructions.extend(load_register(Register::LCL));
                    wasm_instructions.extend([
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::I32Load(mem_offset_arg(*offset * 4))
                    ]);
                }
                PopSegment::Argument => {
                    wasm_instructions.extend(load_register(Register::ARG));
                    wasm_instructions.extend([
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::I32Load(mem_offset_arg(*offset * 4))
                    ]);
                }
                PopSegment::This => {
                    wasm_instructions.extend(load_register(Register::THIS));
                    wasm_instructions.extend([
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::I32Load(mem_offset_arg(*offset * 4))
                    ]);
                }
                PopSegment::That => {
                    wasm_instructions.extend(load_register(Register::THAT));
                    wasm_instructions.extend([
                        Instruction::I32Const(2),
                        Instruction::I32Shl,
                        Instruction::I32Load(mem_offset_arg(*offset * 4))
                    ]);
                }
                PopSegment::Temp => {
                    wasm_instructions.extend([
                        Instruction::I32Const(Register::TEMP(*offset).address() as i32 * 4),
                        Instruction::I32Load(mem_arg()),
                    ]);
                }
                PopSegment::Pointer => {
                    wasm_instructions.extend([
                        Instruction::I32Const((Register::THIS.address() + offset) as i32 * 4),
                        Instruction::I32Load(mem_arg()),
                    ]);
                }
            }

            wasm_instructions.extend([
                Instruction::LocalGet(index_sp()),
                Instruction::I32Const(1),
                Instruction::I32Sub,
                Instruction::LocalTee(index_sp()),
                Instruction::I32Const(2),
                Instruction::I32Shl,
                Instruction::I32Store(mem_arg()),
                Instruction::I32Const(Register::SP.address() as i32),
                Instruction::LocalGet(index_sp()),
                Instruction::I32Store(mem_arg()),
            ]);
        }
        VMCommand::Sub => {
            wasm_instructions.extend(binary_stack_op(vec![], vec![
                Instruction::I32Sub,
                Instruction::I32Extend16S,
            ]));
        }
        VMCommand::Neg => {
            wasm_instructions.extend(unary_stack_op(vec![Instruction::I32Const(0)], vec![
                Instruction::I32Sub,
                Instruction::I32Extend16S,
            ]));
        }
        VMCommand::Eq => {
            wasm_instructions.extend(binary_stack_op(vec![Instruction::I32Const(0)], vec![
                Instruction::I32Eq,
                Instruction::I32Sub,
            ]));
        }
        VMCommand::Gt => {
            wasm_instructions.extend(binary_stack_op(vec![Instruction::I32Const(0)], vec![
                Instruction::I32GtS,
                Instruction::I32Sub,
            ]));
        }
        VMCommand::Lt => {
            wasm_instructions.extend(binary_stack_op(vec![Instruction::I32Const(0)], vec![
                Instruction::I32LtS,
                Instruction::I32Sub,
            ]));
        }
        VMCommand::And => {
            wasm_instructions.extend(binary_stack_op(vec![], vec![Instruction::I32And]));
        }
        VMCommand::Or => {
            wasm_instructions.extend(binary_stack_op(vec![], vec![Instruction::I32Or]));
        }
        VMCommand::Not => {
            wasm_instructions.extend(unary_stack_op(vec![], vec![
                Instruction::I32Const(-1),
                Instruction::I32Xor,
            ]));
        }
        VMCommand::Label { .. } => {
            unreachable!("Labels should have been removed by now");
        }
        VMCommand::Goto { label_name } => {
            if matches!(jump_index, Index::Id(_)) {
                wasm_instructions.extend([
                    Instruction::I32Const(label_indices[label_name]),
                    Instruction::LocalSet(index_jump_target()),
                ]);
            }
            wasm_instructions.push(Instruction::Br(jump_index))
        }
        VMCommand::IfGoto { label_name } => {
            if matches!(jump_index, Index::Id(_)) {
                wasm_instructions.extend([
                    Instruction::I32Const(label_indices[label_name]),
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
                    Instruction::LocalGet(index_frame()),
                    Instruction::I32Const(2),
                    Instruction::I32Shl,
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
    for (i, command) in program
        .files
        .iter()
        .flat_map(|f| f.commands(&program.all_commands).iter())
        .enumerate()
    {
        match command {
            VMCommand::Label { name } => {
                label_indices.insert(name.clone(), case_index);
                if !case_starts.contains(&i) {
                    case_starts.insert(i);
                    case_index += 1;
                }
            }
            VMCommand::Function { name, .. } => {
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
        let jump_index = Index::Id(loop_id);

        if matches!(command, VMCommand::Label { .. }) {
            continue;
        }

        let instructions = command_to_wasm(
            command,
            index,
            jump_index,
            static_segment_start,
            &label_indices,
            &function_indices,
            &call_indices,
        );

        current_case.extend(instructions);

        if case_starts.contains(&(index + 1)) {
            cases.push(current_case);
            current_case = vec![];
        }
    }

    (cases, start_case_index.unwrap())
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
        .fields(globals(start_case_index).into_iter().map(ModuleField::Global).collect())
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
    std::fs::write("out.wasm", &unoptimized_data).unwrap();

    Ok(unoptimized_data)
}
