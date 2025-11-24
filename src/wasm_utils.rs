use std::collections::{HashMap, HashSet};

use wast::{
    core::{
        BlockType, BrTableIndices, BranchHint, Export, ExportKind, Expression, Func, FuncKind,
        FunctionType, Global, GlobalKind, GlobalType, InlineExport, Instruction, Limits, Local,
        MemArg, Memory, MemoryKind, MemoryType, Module, ModuleField, ModuleKind, TypeUse, ValType,
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
    pub fn build(self) -> Module<'a> {
        Module {
            span: self.span,
            id: self.id,
            name: self.name,
            kind: ModuleKind::Text(self.fields),
        }
    }

    pub fn span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    pub fn id(mut self, id: Id<'a>) -> Self {
        self.id = Some(id);
        self
    }

    pub fn name(mut self, name: NameAnnotation<'a>) -> Self {
        self.name = Some(name);
        self
    }

    pub fn fields(mut self, fields: Vec<ModuleField<'a>>) -> Self {
        self.fields = fields;
        self
    }

    pub fn field(mut self, field: ModuleField<'a>) -> Self {
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
    pub fn build(self) -> Func<'a> {
        Func {
            span: self.span,
            id: self.id,
            name: self.name,
            exports: self.exports,
            kind: self.kind.unwrap(),
            ty: self.ty,
        }
    }

    pub fn span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    pub fn id(mut self, id: Id<'a>) -> Self {
        self.id = Some(id);
        self
    }

    pub fn name(mut self, name: NameAnnotation<'a>) -> Self {
        self.name = Some(name);
        self
    }

    pub fn exports(mut self, exports: InlineExport<'a>) -> Self {
        self.exports = exports;
        self
    }

    pub fn export(mut self, export: &'a str) -> Self {
        self.exports.names.push(export);
        self
    }

    pub fn kind(mut self, kind: FuncKind<'a>) -> Self {
        self.kind = Some(kind);
        self
    }

    pub fn ty(mut self, ty: TypeUse<'a, FunctionType<'a>>) -> Self {
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
    pub fn build(self) -> Expression<'a> {
        Expression {
            instrs: self.instrs.into(),
            branch_hints: self.branch_hints.into(),
            instr_spans: self.instr_spans,
        }
    }

    pub fn instrs(mut self, instrs: Vec<Instruction<'a>>) -> Self {
        self.instrs.extend(instrs);
        self
    }

    pub fn instr(mut self, instr: Instruction<'a>) -> Self {
        self.instrs.push(instr);
        self
    }

    pub fn branch_hints(mut self, branch_hints: Vec<BranchHint>) -> Self {
        self.branch_hints = branch_hints;
        self
    }

    pub fn instr_spans(mut self, instr_spans: Box<[Span]>) -> Self {
        self.instr_spans = Some(instr_spans);
        self
    }

    pub fn with_loop<F: FnOnce(Self) -> Self>(mut self, id: Id<'a>, f: F) -> Self {
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

    pub fn switch(
        mut self,
        input_index: Index<'a>,
        cases: Vec<Vec<Instruction<'a>>>,
        default: Vec<Instruction<'a>>,
        overrides: HashMap<usize, i32>,
    ) -> Self {
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
            .instr(Instruction::LocalGet(input_index))
            .instr(Instruction::BrTable(BrTableIndices {
                labels: (0..cases.len())
                    .map(|i| {
                        let target = *overrides.get(&i).unwrap_or(&(i as i32));
                        Index::Num(target as u32, Span::from_offset(0))
                    })
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

pub fn create_memory(id: Id, size: u64) -> Memory {
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
