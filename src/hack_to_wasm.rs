use std::collections::{HashMap, HashSet};

use wast::{
    core::{
        BlockType, BrTableIndices, BranchHint, Export, ExportKind, Expression, Func, FuncKind,
        FunctionType, InlineExport, Instruction, Limits, Local, MemArg, Memory, MemoryKind,
        MemoryType, Module, ModuleField, ModuleKind, TypeUse, ValType,
    },
    token::{Id, Index, NameAnnotation, Span},
};

pub struct ModuleBuilder<'a> {
    pub span: Span,
    pub id: Option<Id<'a>>,
    pub name: Option<NameAnnotation<'a>>,
    pub fields: Vec<ModuleField<'a>>,
}

impl<'a> Default for ModuleBuilder<'a> {
    fn default() -> Self {
        Self {
            span: Span::from_offset(0),
            id: None,
            name: None,
            fields: vec![],
        }
    }
}

#[allow(dead_code)]
impl<'a> ModuleBuilder<'a> {
    fn build(self) -> Module<'a> {
        Module {
            span: self.span,
            id: self.id,
            name: self.name,
            kind: ModuleKind::Text(self.fields),
        }
    }

    fn span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    fn id(mut self, id: Id<'a>) -> Self {
        self.id = Some(id);
        self
    }

    fn name(mut self, name: NameAnnotation<'a>) -> Self {
        self.name = Some(name);
        self
    }

    fn fields(mut self, fields: Vec<ModuleField<'a>>) -> Self {
        self.fields = fields;
        self
    }

    fn field(mut self, field: ModuleField<'a>) -> Self {
        self.fields.push(field);
        self
    }
}

#[derive(Debug)]
pub struct FuncBuilder<'a> {
    pub span: Span,
    pub id: Option<Id<'a>>,
    pub name: Option<NameAnnotation<'a>>,
    pub exports: InlineExport<'a>,
    pub kind: Option<FuncKind<'a>>,
    pub ty: TypeUse<'a, FunctionType<'a>>,
}

impl<'a> Default for FuncBuilder<'a> {
    fn default() -> Self {
        Self {
            span: Span::from_offset(0),
            id: None,
            name: None,
            exports: InlineExport { names: vec![] },
            kind: None,
            ty: TypeUse {
                index: None,
                inline: None,
            },
        }
    }
}

#[allow(dead_code)]
impl<'a> FuncBuilder<'a> {
    fn build(self) -> Func<'a> {
        Func {
            span: self.span,
            id: self.id,
            name: self.name,
            exports: self.exports,
            kind: self.kind.unwrap(),
            ty: self.ty,
        }
    }

    fn span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    fn id(mut self, id: Id<'a>) -> Self {
        self.id = Some(id);
        self
    }

    fn name(mut self, name: NameAnnotation<'a>) -> Self {
        self.name = Some(name);
        self
    }

    fn exports(mut self, exports: InlineExport<'a>) -> Self {
        self.exports = exports;
        self
    }

    fn export(mut self, export: &'a str) -> Self {
        self.exports.names.push(export);
        self
    }

    fn kind(mut self, kind: FuncKind<'a>) -> Self {
        self.kind = Some(kind);
        self
    }

    fn ty(mut self, ty: TypeUse<'a, FunctionType<'a>>) -> Self {
        self.ty = ty;
        self
    }
}

#[derive(Default)]
pub struct ExpressionBuilder<'a> {
    pub instrs: Vec<Instruction<'a>>,
    pub branch_hints: Vec<BranchHint>,
    pub instr_spans: Option<Box<[Span]>>,
}

#[allow(dead_code)]
impl<'a> ExpressionBuilder<'a> {
    fn build(self) -> Expression<'a> {
        Expression {
            instrs: self.instrs.into(),
            branch_hints: self.branch_hints.into(),
            instr_spans: self.instr_spans,
        }
    }

    fn instrs(mut self, instrs: Vec<Instruction<'a>>) -> Self {
        self.instrs.extend(instrs);
        self
    }

    fn instr(mut self, instr: Instruction<'a>) -> Self {
        self.instrs.push(instr);
        self
    }

    fn branch_hints(mut self, branch_hints: Vec<BranchHint>) -> Self {
        self.branch_hints = branch_hints;
        self
    }

    fn instr_spans(mut self, instr_spans: Box<[Span]>) -> Self {
        self.instr_spans = Some(instr_spans);
        self
    }

    fn with_loop<F: FnOnce(Self) -> Self>(mut self, id: Id<'a>, f: F) -> Self {
        self = self.instr(Instruction::Loop(Box::new(BlockType {
            label: Some(id),
            label_name: None,
            ty: TypeUse {
                index: None,
                inline: None,
            },
        })));
        self = f(self);

        self.instr(Instruction::End(None))
    }

    fn switch(mut self, cases: Vec<Vec<Instruction<'a>>>, default: Vec<Instruction<'a>>) -> Self {
        for _ in 0..(cases.len() + 1) {
            self = self.instr(Instruction::Block(Box::new(BlockType {
                label: None,
                label_name: None,
                ty: TypeUse {
                    index: None,
                    inline: None,
                },
            })))
        }

        self = self
            .instr(Instruction::LocalGet(index_jump_target()))
            .instr(Instruction::BrTable(BrTableIndices {
                labels: (0..cases.len())
                    .map(|i| Index::Num(i as u32, Span::from_offset(0)))
                    .collect(),
                default: Index::Num(cases.len() as u32, Span::from_offset(0)),
            }));

        for case in cases.into_iter().chain(std::iter::once(default)) {
            self = self.instr(Instruction::End(None));
            for instr in case {
                self = self.instr(instr)
            }
        }

        self
    }
}

fn create_memory(id: Id, size: u64) -> Memory {
    Memory {
        span: Span::from_offset(0),
        id: Some(id),
        name: None,
        exports: InlineExport { names: vec![] },
        kind: MemoryKind::Normal(MemoryType {
            limits: Limits {
                is64: false,
                min: size,
                max: None,
            },
            shared: false,
            page_size_log2: None,
        }),
    }
}

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

fn mem_arg_m() -> MemArg<'static> {
    MemArg {
        align: 4,
        offset: 0,
        memory: Index::Num(0, Span::from_offset(0)),
    }
}

fn hack_instr_to_wasm(
    hack_instr: crate::hardware::Instruction,
    jump_index: Index,
    jump_target: Option<i32>,
) -> Vec<Instruction> {
    let mut wasm_instructions = vec![
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

    if hack_instr.jump_condition() != crate::hardware::JumpCondition::NoJump {
        if let Some(jump_target) = jump_target {
            wasm_instructions.push(Instruction::I32Const(jump_target));
            wasm_instructions.push(Instruction::LocalSet(index_jump_target()));
        }
    }

    if hack_instr.dst_has_m() || hack_instr.op_name().contains('M') {
        wasm_instructions.extend([
            Instruction::LocalGet(index_a()),
            Instruction::I32Const(2),
            Instruction::I32Shl,
            Instruction::LocalSet(index_ram_address()),
        ]);
    }

    let load_m = [
        Instruction::LocalGet(index_ram_address()),
        Instruction::I32Load(mem_arg_m()),
    ];

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
        }
        "D+1" => {
            wasm_instructions.push(Instruction::LocalGet(index_d()));
            wasm_instructions.push(Instruction::I32Const(1));
            wasm_instructions.push(Instruction::I32Add);
        }
        "D-1" => {
            wasm_instructions.push(Instruction::LocalGet(index_d()));
            wasm_instructions.push(Instruction::I32Const(1));
            wasm_instructions.push(Instruction::I32Sub);
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
        }
        "A+1" => {
            wasm_instructions.push(Instruction::LocalGet(index_a()));
            wasm_instructions.push(Instruction::I32Const(1));
            wasm_instructions.push(Instruction::I32Add);
        }
        "A-1" => {
            wasm_instructions.push(Instruction::LocalGet(index_a()));
            wasm_instructions.push(Instruction::I32Const(1));
            wasm_instructions.push(Instruction::I32Sub);
        }
        "D+A" => {
            wasm_instructions.push(Instruction::LocalGet(index_d()));
            wasm_instructions.push(Instruction::LocalGet(index_a()));
            wasm_instructions.push(Instruction::I32Add);
        }
        "D-A" => {
            wasm_instructions.push(Instruction::LocalGet(index_d()));
            wasm_instructions.push(Instruction::LocalGet(index_a()));
            wasm_instructions.push(Instruction::I32Sub);
        }
        "A-D" => {
            wasm_instructions.push(Instruction::LocalGet(index_a()));
            wasm_instructions.push(Instruction::LocalGet(index_d()));
            wasm_instructions.push(Instruction::I32Sub);
        }
        "D&A" => {
            wasm_instructions.push(Instruction::LocalGet(index_d()));
            wasm_instructions.push(Instruction::LocalGet(index_a()));
            wasm_instructions.push(Instruction::I32And);
        }
        "D|A" => {
            wasm_instructions.push(Instruction::LocalGet(index_d()));
            wasm_instructions.push(Instruction::LocalGet(index_a()));
            wasm_instructions.push(Instruction::I32Or);
        }
        "M" => {
            wasm_instructions.extend(load_m.clone());
        }
        "!M" => {
            wasm_instructions.extend(load_m.clone());
            wasm_instructions.push(Instruction::I32Const(-1));
            wasm_instructions.push(Instruction::I32Xor);
        }
        "-M" => {
            wasm_instructions.push(Instruction::I32Const(0));
            wasm_instructions.extend(load_m.clone());
            wasm_instructions.push(Instruction::I32Sub);
        }
        "M+1" => {
            wasm_instructions.extend(load_m.clone());
            wasm_instructions.push(Instruction::I32Const(1));
            wasm_instructions.push(Instruction::I32Add);
        }
        "M-1" => {
            wasm_instructions.extend(load_m.clone());
            wasm_instructions.push(Instruction::I32Const(1));
            wasm_instructions.push(Instruction::I32Sub);
        }
        "D+M" => {
            wasm_instructions.push(Instruction::LocalGet(index_d()));
            wasm_instructions.extend(load_m.clone());
            wasm_instructions.push(Instruction::I32Add);
        }
        "D-M" => {
            wasm_instructions.push(Instruction::LocalGet(index_d()));
            wasm_instructions.extend(load_m.clone());
            wasm_instructions.push(Instruction::I32Sub);
        }
        "M-D" => {
            wasm_instructions.extend(load_m.clone());
            wasm_instructions.push(Instruction::LocalGet(index_d()));
            wasm_instructions.push(Instruction::I32Sub);
        }
        "D&M" => {
            wasm_instructions.push(Instruction::LocalGet(index_d()));
            wasm_instructions.extend(load_m.clone());
            wasm_instructions.push(Instruction::I32And);
        }
        "D|M" => {
            wasm_instructions.push(Instruction::LocalGet(index_d()));
            wasm_instructions.extend(load_m.clone());
            wasm_instructions.push(Instruction::I32Or);
        }
        _ => panic!("Unknown instruction: {}", hack_instr.op_name()),
    }

    wasm_instructions.push(Instruction::I32Extend16S);
    wasm_instructions.push(Instruction::LocalSet(index_result()));

    if hack_instr.dst_has_a() {
        wasm_instructions.push(Instruction::LocalGet(index_result()));
        wasm_instructions.push(Instruction::LocalSet(index_a()));
    }

    if hack_instr.dst_has_d() {
        wasm_instructions.push(Instruction::LocalGet(index_result()));
        wasm_instructions.push(Instruction::LocalSet(index_d()));
    }

    if hack_instr.dst_has_m() {
        wasm_instructions.push(Instruction::LocalGet(index_ram_address()));
        wasm_instructions.push(Instruction::LocalGet(index_result()));
        wasm_instructions.push(Instruction::I32Store(mem_arg_m()));
    }

    use crate::hardware::JumpCondition;
    if hack_instr.jump_condition() != JumpCondition::NoJump {
        wasm_instructions.push(Instruction::LocalGet(index_result()));
    }
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

fn hack_to_cases(hack: &str, loop_id: Id<'static>) -> Vec<Vec<Instruction<'static>>> {
    let instructions = crate::hardware_parse::assemble_hack_file(hack).unwrap().1;

    let mut a_values: Vec<Option<i32>> = Vec::new();
    a_values.resize(instructions.len() + 1, None);
    let mut targets = HashSet::new();
    targets.insert(0);
    for (index, instruction) in instructions.iter().enumerate() {
        if instruction.instruction_type() == crate::hardware::InstructionType::A {
            a_values[index + 1] = Some(instruction.loaded_value() as i32);
            continue;
        }

        if instruction.jump_condition() != crate::hardware::JumpCondition::NoJump {
            let target = a_values[index].unwrap() as usize;
            targets.insert(target);
            a_values[target] = None;
            continue;
        }

        if instruction.dst_has_a() {
            a_values[index + 1] = None;
        } else {
            a_values[index + 1] = a_values[index];
        }
    }

    let mut sorted: Vec<_> = targets.into_iter().collect();
    sorted.sort();
    let index_to_case_index: HashMap<_, _> = sorted
        .into_iter()
        .enumerate()
        .map(|(i, target)| (target, i as i32))
        .collect();

    let mut cases = vec![];
    let mut current_case = vec![];
    for (index, instruction) in instructions.into_iter().enumerate() {
        let mut jump_target = None;
        let mut loop_index = Index::Id(loop_id);

        if instruction.instruction_type() == crate::hardware::InstructionType::C
            && instruction.jump_condition() != crate::hardware::JumpCondition::NoJump
        {
            if let Some(a_value) = a_values[index] {
                if let Some(&case_index) = index_to_case_index.get(&(a_value as usize)) {
                    if case_index as usize > cases.len() {
                        loop_index = Index::Num(
                            case_index as u32 - cases.len() as u32 - 1,
                            Span::from_offset(0),
                        );
                    }
                    jump_target = Some(case_index);
                }
            }
        }

        current_case.extend(hack_instr_to_wasm(instruction, loop_index, jump_target));

        if index_to_case_index.contains_key(&(index + 1)) {
            cases.push(current_case);
            current_case = vec![];
        }
    }
    cases.push(current_case);

    cases
}

pub fn hack_to_wasm(hack: &str) -> Result<Vec<u8>, String> {
    let loop_id = Id::new("loop", Span::from_offset(0));

    let cases = hack_to_cases(hack, loop_id);

    let memory_id = Id::new("memory", Span::from_offset(0));

    let expression = ExpressionBuilder::default()
        .instr(Instruction::I32Const(0))
        .instr(Instruction::LocalGet(Index::Num(0, Span::from_offset(0))))
        .instr(Instruction::I32Store(MemArg {
            align: 4,
            offset: 0,
            memory: Index::Num(0, Span::from_offset(0)),
        }))
        .instr(Instruction::I32Const(4))
        .instr(Instruction::LocalGet(Index::Num(1, Span::from_offset(0))))
        .instr(Instruction::I32Store(MemArg {
            align: 4,
            offset: 0,
            memory: Index::Num(0, Span::from_offset(0)),
        }))
        .with_loop(loop_id, |builder| builder.switch(cases, vec![]))
        .instr(Instruction::I32Const(8))
        .instr(Instruction::I32Load(MemArg {
            align: 4,
            offset: 0,
            memory: Index::Num(0, Span::from_offset(0)),
        }))
        .instr(Instruction::LocalGet(index_ticks()))
        .build();

    let mut m = ModuleBuilder::default()
        .field(ModuleField::Memory(create_memory(memory_id, 32768)))
        .field(ModuleField::Export(Export {
            span: Span::from_offset(0),
            name: "memory",
            kind: ExportKind::Memory,
            item: Index::Id(memory_id),
        }))
        .field(ModuleField::Func(
            FuncBuilder::default()
                .export("foo")
                .kind(FuncKind::Inline {
                    locals: locals(),
                    expression,
                })
                .ty(TypeUse {
                    index: None,
                    inline: Some(FunctionType {
                        params: [(None, None, ValType::I32), (None, None, ValType::I32)].into(),
                        results: [ValType::I32, ValType::I32].into(),
                    }),
                })
                .build(),
        ))
        .build();

    let unoptimized_data = m.encode().map_err(|e| e.to_string())?;

    // unsafe {
    //     let binaryen_module = binaryen_sys::BinaryenModuleRead(unoptimized_data.as_ptr() as *mut _, unoptimized_data.len());
    //     binaryen_sys::BinaryenModuleSetFeatures(binaryen_module, binaryen_sys::BinaryenFeatureSignExt());
    //     binaryen_sys::BinaryenSetOptimizeLevel(2);
    //     binaryen_sys::BinaryenSetShrinkLevel(2);
    //     binaryen_sys::BinaryenModuleOptimize(binaryen_module);
    //     binaryen_sys::BinaryenModuleOptimize(binaryen_module);
    //     Ok(write_module(binaryen_module))
    // }

    Ok(unoptimized_data)
}

// pub fn write_module(module: binaryen_sys::BinaryenModuleRef) -> Vec<u8> {
//     unsafe {
//         let write_result =
//             binaryen_sys::BinaryenModuleAllocateAndWrite(module, std::ptr::null());

//         // Create a slice from the resulting array and then copy it in vector.
//         let binary_buf = if write_result.binaryBytes == 0 {
//             vec![]
//         } else {
//             std::slice::from_raw_parts(write_result.binary as *const u8, write_result.binaryBytes)
//                 .to_vec()
//         };

//         // This will free buffers in the write_result.
//         binaryen_sys::BinaryenShimDisposeBinaryenModuleAllocateAndWriteResult(write_result);

//         binary_buf
//     }
// }
