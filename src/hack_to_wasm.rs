use std::collections::{HashMap, HashSet};

use wast::{
    core::{
        BlockType, Export, ExportKind, FuncKind,
        FunctionType, Global, GlobalKind, GlobalType, InlineExport, Instruction, Local,
        MemArg, ModuleField, TypeUse, ValType,
    },
    token::{Id, Index, Span},
};

use crate::wasm_utils::{ExpressionBuilder, FuncBuilder, ModuleBuilder, create_memory};

fn id_a() -> Id<'static> {
    Id::new("A", Span::from_offset(0))
}

fn id_d() -> Id<'static> {
    Id::new("D", Span::from_offset(0))
}

fn id_ticks() -> Id<'static> {
    Id::new("ticks", Span::from_offset(0))
}

fn id_jump_target() -> Id<'static> {
    Id::new("jump_target", Span::from_offset(0))
}

fn id_ram_address() -> Id<'static> {
    Id::new("ram_address", Span::from_offset(0))
}

fn id_result() -> Id<'static> {
    Id::new("result", Span::from_offset(0))
}

fn index_a() -> Index<'static> {
    Index::Id(id_a())
}

fn index_d() -> Index<'static> {
    Index::Id(id_d())
}

fn index_ticks() -> Index<'static> {
    Index::Id(id_ticks())
}

fn index_jump_target() -> Index<'static> {
    Index::Id(id_jump_target())
}

fn index_ram_address() -> Index<'static> {
    Index::Id(id_ram_address())
}

fn index_result() -> Index<'static> {
    Index::Id(id_result())
}

fn locals() -> Box<[Local<'static>]> {
    Box::new([
        Local {
            id: Some(id_a()),
            name: None,
            ty: ValType::I32,
        },
        Local {
            id: Some(id_d()),
            name: None,
            ty: ValType::I32,
        },
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
            id: Some(id_ram_address()),
            name: None,
            ty: ValType::I32,
        },
        Local {
            id: Some(id_result()),
            name: None,
            ty: ValType::I32,
        },
    ])
}

fn globals() -> Vec<Global<'static>> {
    vec![
        Global {
            span: Span::from_offset(0),
            id: Some(id_a()),
            name: None,
            exports: InlineExport { names: vec!["a"] },
            ty: GlobalType {
                ty: ValType::I32,
                mutable: true,
                shared: false,
            },
            kind: GlobalKind::Inline(
                ExpressionBuilder::default()
                    .instr(Instruction::I32Const(1337))
                    .build(),
            ),
        },
        Global {
            span: Span::from_offset(0),
            id: Some(id_d()),
            name: None,
            exports: InlineExport { names: vec!["d"] },
            ty: GlobalType {
                ty: ValType::I32,
                mutable: true,
                shared: false,
            },
            kind: GlobalKind::Inline(
                ExpressionBuilder::default()
                    .instr(Instruction::I32Const(1337))
                    .build(),
            ),
        },
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
                    .instr(Instruction::I32Const(0))
                    .build(),
            ),
        },
    ]
}

fn mem_arg_m() -> MemArg<'static> {
    MemArg {
        align: 4,
        offset: 0,
        memory: Index::Num(0, Span::from_offset(0)),
    }
}

fn hack_instr_to_wasm(
    hack_instr: &crate::hardware::Instruction,
    jump_index: Index<'static>,
) -> Vec<Instruction<'static>> {
    let mut wasm_instructions: Vec<Instruction<'static>> = vec![
        Instruction::I32Const(1),
        Instruction::LocalGet(index_ticks()),
        Instruction::I32Add,
        Instruction::LocalSet(index_ticks()),
    ];

    if hack_instr.instruction_type() == crate::hardware::InstructionType::A {
        wasm_instructions.push(Instruction::I32Const(hack_instr.loaded_value() as i32));
        wasm_instructions.push(Instruction::LocalSet(index_a()));

        return wasm_instructions;
    }

    if matches!(jump_index, Index::Id(_))
        && hack_instr.jump_condition() != crate::hardware::JumpCondition::NoJump
    {
        wasm_instructions.push(Instruction::LocalGet(index_a()));
        wasm_instructions.push(Instruction::LocalSet(index_jump_target()));
    }

    let mut result_users = hack_instr.dst_has_a() as u8
        + hack_instr.dst_has_d() as u8
        + hack_instr.dst_has_m() as u8
        + (hack_instr.jump_condition() != JumpCondition::NoJump
            && hack_instr.jump_condition() != JumpCondition::JMP) as u8;

    if result_users > 0 {
        let load_m_address = [
            Instruction::LocalGet(index_a()),
            Instruction::I32Const(2),
            Instruction::I32Shl,
        ];
        if hack_instr.dst_has_m() {
            wasm_instructions.extend(load_m_address.clone());
            if hack_instr.op_name().contains('M') {
                wasm_instructions.push(Instruction::LocalTee(index_ram_address()));
            }
        }

        let load_m = if hack_instr.dst_has_m() {
            vec![
                Instruction::LocalGet(index_ram_address()),
                Instruction::I32Load(mem_arg_m()),
            ]
        } else {
            load_m_address
                .into_iter()
                .chain(std::iter::once(Instruction::I32Load(mem_arg_m())))
                .collect()
        };

        match hack_instr.op_name() {
            "0" => wasm_instructions.push(Instruction::I32Const(0)),
            "1" => wasm_instructions.push(Instruction::I32Const(1)),
            "-1" => wasm_instructions.push(Instruction::I32Const(-1)),
            "D" => wasm_instructions.push(Instruction::LocalGet(index_d())),
            "!D" => {
                wasm_instructions.push(Instruction::I32Const(-1));
                wasm_instructions.push(Instruction::LocalGet(index_d()));
                wasm_instructions.push(Instruction::I32Xor);
            }
            "-D" => {
                wasm_instructions.push(Instruction::I32Const(0));
                wasm_instructions.push(Instruction::LocalGet(index_d()));
                wasm_instructions.push(Instruction::I32Sub);
                wasm_instructions.push(Instruction::I32Extend16S);
            }
            "D+1" => {
                wasm_instructions.push(Instruction::LocalGet(index_d()));
                wasm_instructions.push(Instruction::I32Const(1));
                wasm_instructions.push(Instruction::I32Add);
                wasm_instructions.push(Instruction::I32Extend16S);
            }
            "D-1" => {
                wasm_instructions.push(Instruction::LocalGet(index_d()));
                wasm_instructions.push(Instruction::I32Const(1));
                wasm_instructions.push(Instruction::I32Sub);
                wasm_instructions.push(Instruction::I32Extend16S);
            }
            "A" => wasm_instructions.push(Instruction::LocalGet(index_a())),
            "!A" => {
                wasm_instructions.push(Instruction::I32Const(-1));
                wasm_instructions.push(Instruction::LocalGet(index_a()));
                wasm_instructions.push(Instruction::I32Xor);
            }
            "-A" => {
                wasm_instructions.push(Instruction::I32Const(0));
                wasm_instructions.push(Instruction::LocalGet(index_a()));
                wasm_instructions.push(Instruction::I32Sub);
                wasm_instructions.push(Instruction::I32Extend16S);
            }
            "A+1" => {
                wasm_instructions.push(Instruction::LocalGet(index_a()));
                wasm_instructions.push(Instruction::I32Const(1));
                wasm_instructions.push(Instruction::I32Add);
                wasm_instructions.push(Instruction::I32Extend16S);
            }
            "A-1" => {
                wasm_instructions.push(Instruction::LocalGet(index_a()));
                wasm_instructions.push(Instruction::I32Const(1));
                wasm_instructions.push(Instruction::I32Sub);
                wasm_instructions.push(Instruction::I32Extend16S);
            }
            "D+A" => {
                wasm_instructions.push(Instruction::LocalGet(index_d()));
                wasm_instructions.push(Instruction::LocalGet(index_a()));
                wasm_instructions.push(Instruction::I32Add);
                wasm_instructions.push(Instruction::I32Extend16S);
            }
            "D-A" => {
                wasm_instructions.push(Instruction::LocalGet(index_d()));
                wasm_instructions.push(Instruction::LocalGet(index_a()));
                wasm_instructions.push(Instruction::I32Sub);
                wasm_instructions.push(Instruction::I32Extend16S);
            }
            "A-D" => {
                wasm_instructions.push(Instruction::LocalGet(index_a()));
                wasm_instructions.push(Instruction::LocalGet(index_d()));
                wasm_instructions.push(Instruction::I32Sub);
                wasm_instructions.push(Instruction::I32Extend16S);
            }
            "A&D" => {
                wasm_instructions.push(Instruction::LocalGet(index_d()));
                wasm_instructions.push(Instruction::LocalGet(index_a()));
                wasm_instructions.push(Instruction::I32And);
            }
            "A|D" => {
                wasm_instructions.push(Instruction::LocalGet(index_d()));
                wasm_instructions.push(Instruction::LocalGet(index_a()));
                wasm_instructions.push(Instruction::I32Or);
            }
            "M" => {
                wasm_instructions.extend(load_m);
            }
            "!M" => {
                wasm_instructions.extend(load_m);
                wasm_instructions.push(Instruction::I32Const(-1));
                wasm_instructions.push(Instruction::I32Xor);
            }
            "-M" => {
                wasm_instructions.push(Instruction::I32Const(0));
                wasm_instructions.extend(load_m);
                wasm_instructions.push(Instruction::I32Sub);
                wasm_instructions.push(Instruction::I32Extend16S);
            }
            "M+1" => {
                wasm_instructions.extend(load_m);
                wasm_instructions.push(Instruction::I32Const(1));
                wasm_instructions.push(Instruction::I32Add);
                wasm_instructions.push(Instruction::I32Extend16S);
            }
            "M-1" => {
                wasm_instructions.extend(load_m);
                wasm_instructions.push(Instruction::I32Const(1));
                wasm_instructions.push(Instruction::I32Sub);
                wasm_instructions.push(Instruction::I32Extend16S);
            }
            "D+M" => {
                wasm_instructions.push(Instruction::LocalGet(index_d()));
                wasm_instructions.extend(load_m);
                wasm_instructions.push(Instruction::I32Add);
                wasm_instructions.push(Instruction::I32Extend16S);
            }
            "D-M" => {
                wasm_instructions.push(Instruction::LocalGet(index_d()));
                wasm_instructions.extend(load_m);
                wasm_instructions.push(Instruction::I32Sub);
                wasm_instructions.push(Instruction::I32Extend16S);
            }
            "M-D" => {
                wasm_instructions.extend(load_m);
                wasm_instructions.push(Instruction::LocalGet(index_d()));
                wasm_instructions.push(Instruction::I32Sub);
                wasm_instructions.push(Instruction::I32Extend16S);
            }
            "D&M" => {
                wasm_instructions.push(Instruction::LocalGet(index_d()));
                wasm_instructions.extend(load_m);
                wasm_instructions.push(Instruction::I32And);
            }
            "D|M" => {
                wasm_instructions.push(Instruction::LocalGet(index_d()));
                wasm_instructions.extend(load_m);
                wasm_instructions.push(Instruction::I32Or);
            }
            _ => panic!("Unknown instruction: {}", hack_instr.op_name()),
        }
    }

    let mut set_local = |index| {
        if result_users > 1 {
            wasm_instructions.push(Instruction::LocalTee(index));
        } else {
            wasm_instructions.push(Instruction::LocalSet(index));
        }

        result_users -= 1;
    };

    if hack_instr.dst_has_a() {
        set_local(index_a());
    }

    if hack_instr.dst_has_d() {
        set_local(index_d());
    }

    if hack_instr.dst_has_m() {
        if result_users > 1 {
            wasm_instructions.push(Instruction::LocalTee(index_result()));
        }
        wasm_instructions.push(Instruction::I32Store(mem_arg_m()));
        if result_users > 1 {
            wasm_instructions.push(Instruction::LocalGet(index_result()));
        }
    }

    use crate::hardware::JumpCondition;
    match hack_instr.jump_condition() {
        JumpCondition::JMP => wasm_instructions.push(Instruction::Br(jump_index)),
        JumpCondition::JNE => {
            wasm_instructions.push(Instruction::I32Const(0));
            wasm_instructions.push(Instruction::I32Ne);
            wasm_instructions.push(Instruction::BrIf(jump_index));
        }
        JumpCondition::JEQ => {
            wasm_instructions.push(Instruction::I32Eqz);
            wasm_instructions.push(Instruction::BrIf(jump_index));
        }
        JumpCondition::JLE => {
            wasm_instructions.push(Instruction::I32Const(0));
            wasm_instructions.push(Instruction::I32LeS);
            wasm_instructions.push(Instruction::BrIf(jump_index));
        }
        JumpCondition::JGE => {
            wasm_instructions.push(Instruction::I32Const(0));
            wasm_instructions.push(Instruction::I32GeS);
            wasm_instructions.push(Instruction::BrIf(jump_index));
        }
        JumpCondition::JLT => {
            wasm_instructions.push(Instruction::I32Const(0));
            wasm_instructions.push(Instruction::I32LtS);
            wasm_instructions.push(Instruction::BrIf(jump_index));
        }
        JumpCondition::JGT => {
            wasm_instructions.push(Instruction::I32Const(0));
            wasm_instructions.push(Instruction::I32GtS);
            wasm_instructions.push(Instruction::BrIf(jump_index));
        }
        JumpCondition::NoJump => {}
    };

    wasm_instructions
}

fn hack_dynamic_slow(
    instructions: &[crate::hardware::Instruction],
    loop_id: Id<'static>,
) -> Vec<Vec<Instruction<'static>>> {
    instructions
        .iter()
        .enumerate()
        .map(|(i, instruction)| {
            let mut instrs = hack_instr_to_wasm(instruction, Index::Id(loop_id));
            instrs.push(Instruction::I32Const(i as i32 + 1));
            instrs.push(Instruction::LocalSet(index_jump_target()));
            instrs.push(Instruction::Br(Index::Id(loop_id)));
            instrs
        })
        .collect()
}

fn hack_to_dynamic_cases(
    instructions: &[crate::hardware::Instruction],
    loop_id: Id<'static>,
) -> (Vec<Vec<Instruction<'static>>>, HashMap<usize, i32>) {
    let cases = instructions
        .iter()
        .map(|instruction| hack_instr_to_wasm(instruction, Index::Id(loop_id)))
        .collect();

    (cases, HashMap::new())
}

fn hack_to_static_cases(
    instructions: &[crate::hardware::Instruction],
    loop_id: Id<'static>,
) -> (Vec<Vec<Instruction<'static>>>, HashMap<usize, i32>) {
    let mut targets = HashSet::new();
    targets.insert(0);
    {
        let mut a_value = None;
        for instruction in instructions.iter() {
            if instruction.instruction_type() == crate::hardware::InstructionType::A {
                a_value = Some(instruction.loaded_value() as i32);
            } else {
                if instruction.jump_condition() != crate::hardware::JumpCondition::NoJump
                    && let Some(target) = a_value
                {
                    targets.insert(target as usize);
                }

                if instruction.dst_has_a() {
                    a_value = None;
                }
            }
        }
    }
    targets.insert(instructions.len());

    let mut sorted: Vec<_> = targets.into_iter().collect();
    sorted.sort();
    let index_to_case_index: HashMap<_, _> = sorted
        .into_iter()
        .enumerate()
        .map(|(i, target)| (target, i as i32))
        .collect();

    let mut cases = vec![];
    for (index, instruction) in instructions.iter().enumerate() {
        let mut case = hack_instr_to_wasm(instruction, Index::Id(loop_id));
        if let Some(&case_index) = index_to_case_index.get(&(index + 1)) {
            let offset = case_index as usize + instructions.len() - index;
            case.push(Instruction::Br(Index::Num(
                offset as u32,
                Span::from_offset(0),
            )));
        }
        cases.push(case);
    }

    cases.push(vec![Instruction::Br(Index::Num(
        index_to_case_index.len() as u32 - 1,
        Span::from_offset(0),
    ))]);

    let mut current_case = vec![];
    let mut a_value = None;
    for (index, instruction) in instructions.iter().enumerate() {
        let mut jump_index = Index::Id(loop_id);

        if instruction.instruction_type() == crate::hardware::InstructionType::A {
            a_value = Some(instruction.loaded_value() as i32);
        } else {
            if instruction.jump_condition() != crate::hardware::JumpCondition::NoJump
                && let Some(a_value) = a_value
                && let Some(&case_index) = index_to_case_index.get(&(a_value as usize))
            {
                let offset = case_index - (cases.len() - instructions.len()) as i32;
                if offset >= 0 {
                    jump_index = Index::Num(offset as u32, Span::from_offset(0));
                }
            }

            if instruction.dst_has_a() {
                a_value = None;
            }
        }

        current_case.extend(hack_instr_to_wasm(instruction, jump_index));

        if index_to_case_index.contains_key(&(index + 1)) {
            cases.push(current_case);
            current_case = vec![];
            a_value = None;
        }
    }

    let overrides = index_to_case_index
        .into_iter()
        .map(|(i, j)| (i, j + instructions.len() as i32 + 1))
        .collect();

    (cases, overrides)
}

pub fn hack_to_wasm(
    instructions: &[crate::hardware::Instruction],
    with_limit: bool,
) -> Result<Vec<u8>, String> {
    let loop_id = Id::new("loop", Span::from_offset(0));

    let (cases, overrides) = hack_to_static_cases(instructions, loop_id);
    let case_count = cases.len();

    let memory_id = Id::new("memory", Span::from_offset(0));

    let expression = ExpressionBuilder::default()
        .instr(Instruction::GlobalGet(index_a()))
        .instr(Instruction::LocalSet(index_a()))
        .instr(Instruction::GlobalGet(index_d()))
        .instr(Instruction::LocalSet(index_d()))
        .instr(Instruction::GlobalGet(index_jump_target()))
        .instr(Instruction::LocalSet(index_jump_target()))
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
                    .instr(Instruction::LocalGet(index_a()))
                    .instr(Instruction::GlobalSet(index_a()))
                    .instr(Instruction::LocalGet(index_d()))
                    .instr(Instruction::GlobalSet(index_d()))
                    .instr(Instruction::LocalGet(index_jump_target()))
                    .instr(Instruction::GlobalSet(index_jump_target()))
                    .instr(Instruction::LocalGet(index_ticks()))
                    .instr(Instruction::Return)
                    .instr(Instruction::End(None));
            }
            builder.switch(index_jump_target(), cases, vec![], overrides)
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
        .fields(globals().into_iter().map(ModuleField::Global).collect())
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

    Ok(unoptimized_data)
}
