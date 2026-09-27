use std::collections::{HashMap, HashSet};

use crate::frontend::resolution::SymbolId;
use crate::frontend::types::{Type, TypeId};
use crate::middle::ir::{BlockId, IrEnum, IrModule, IrStruct};

use super::instruction::{Condition, Instruction, MachineFunction};
use super::register::Register as R;
use super::runtime;

pub(super) fn types(module: &IrModule) -> HashMap<Type, SymbolId> {
    let mut pending: Vec<Type> = module
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| match &instruction.kind {
            crate::middle::ir::IrInstructionKind::Print { value_type, .. } => {
                Some(value_type.clone())
            }
            _ => None,
        })
        .collect();
    let structs: HashMap<_, _> = module.structs.iter().map(|item| (item.id, item)).collect();
    let enums: HashMap<_, _> = module.enums.iter().map(|item| (item.id, item)).collect();
    let mut seen = HashSet::new();
    while let Some(ty) = pending.pop() {
        if !seen.insert(ty.clone()) {
            continue;
        }
        match ty {
            Type::List(element)
            | Type::Optional(element)
            | Type::Ref(element)
            | Type::Array { element, .. } => pending.push(*element),
            Type::Struct(id) => {
                if let Some(definition) = structs.get(&id) {
                    pending.extend(definition.fields.iter().map(|field| field.ty.clone()));
                }
            }
            Type::Enum(id) => {
                if let Some(definition) = enums.get(&id) {
                    pending.extend(
                        definition
                            .variants
                            .iter()
                            .filter_map(|variant| variant.payload.clone()),
                    );
                }
            }
            _ => {}
        }
    }
    let first = module
        .functions
        .iter()
        .map(|item| item.symbol.0)
        .max()
        .unwrap_or(0)
        + 1;
    let mut ordered: Vec<_> = seen.into_iter().collect();
    ordered.sort_by_key(|ty| format!("{ty:?}"));
    ordered
        .into_iter()
        .enumerate()
        .map(|(index, ty)| (ty, SymbolId(first + index as u32)))
        .collect()
}

pub(super) fn text_values(module: &IrModule) -> Vec<String> {
    if !module
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .any(|instruction| {
            matches!(
                instruction.kind,
                crate::middle::ir::IrInstructionKind::Print { .. }
            )
        })
    {
        return Vec::new();
    }
    let mut values = vec![
        "".into(),
        "[".into(),
        "]".into(),
        ", ".into(),
        "{".into(),
        "}".into(),
        ": ".into(),
        "(".into(),
        ")".into(),
        ".".into(),
        "some(".into(),
        "none()".into(),
        "ref(".into(),
        "\"".into(),
        "\\n".into(),
        "\\r".into(),
        "\\t".into(),
        "\\0".into(),
        "\\\"".into(),
        "\\\\".into(),
    ];
    for item in &module.structs {
        values.push(item.name.clone());
        values.extend(item.fields.iter().map(|field| field.name.clone()));
    }
    for item in &module.enums {
        values.push(item.name.clone());
        values.extend(item.variants.iter().map(|variant| variant.name.clone()));
    }
    values
}

pub(super) fn functions(
    module: &IrModule,
    symbols: &HashMap<Type, SymbolId>,
    strings: &HashMap<String, usize>,
) -> Vec<MachineFunction> {
    let structs: HashMap<_, _> = module.structs.iter().map(|item| (item.id, item)).collect();
    let enums: HashMap<_, _> = module.enums.iter().map(|item| (item.id, item)).collect();
    let mut entries: Vec<_> = symbols.iter().collect();
    entries.sort_by_key(|(_, symbol)| symbol.0);
    entries
        .into_iter()
        .map(|(ty, symbol)| {
            let mut builder = Builder {
                instructions: Vec::new(),
                next_label: 1,
                strings,
                symbols,
                structs: &structs,
                enums: &enums,
            };
            builder.generate(ty);
            MachineFunction {
                symbol: *symbol,
                name: format!("__aerofyl_format_{}", symbol.0),
                instructions: builder.instructions,
            }
        })
        .collect()
}

struct Builder<'a> {
    instructions: Vec<Instruction>,
    next_label: u32,
    strings: &'a HashMap<String, usize>,
    symbols: &'a HashMap<Type, SymbolId>,
    structs: &'a HashMap<TypeId, &'a IrStruct>,
    enums: &'a HashMap<TypeId, &'a IrEnum>,
}

impl Builder<'_> {
    fn push(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }
    fn label(&mut self) -> BlockId {
        let label = BlockId(self.next_label);
        self.next_label += 1;
        label
    }
    fn immediate(&mut self, destination: R, value: u64) {
        self.push(Instruction::MoveImmediate64 { destination, value });
    }
    fn mov(&mut self, destination: R, source: R) {
        self.push(Instruction::MoveRegister {
            destination,
            source,
        });
    }
    fn load(&mut self, destination: R, base: R, displacement: i32) {
        self.push(Instruction::Load64 {
            destination,
            base,
            displacement,
        });
    }
    fn literal(&mut self, value: &str) {
        self.push(Instruction::LoadDataAddress {
            destination: R::Rdi,
            offset: *self
                .strings
                .get(value)
                .expect("format literal in read-only data"),
        });
        self.immediate(R::Rsi, 0);
        self.mov(R::Rdx, R::R13);
        self.immediate(R::Rcx, 0);
        self.push(Instruction::Call(runtime::PRINT));
    }
    fn child(&mut self, ty: &Type) {
        self.mov(R::Rsi, R::R13);
        self.mov(R::Rdx, R::R14);
        self.immediate(R::Rax, 1);
        self.push(Instruction::Add {
            destination: R::Rdx,
            source: R::Rax,
        });
        self.immediate(R::Rcx, 1);
        self.push(Instruction::Call(
            *self.symbols.get(ty).expect("printable child type"),
        ));
    }
    fn scalar(&mut self, code: u64) {
        self.mov(R::Rdi, R::R12);
        self.immediate(R::Rsi, code);
        self.mov(R::Rdx, R::R13);
        self.immediate(R::Rcx, 0);
        self.push(Instruction::Call(runtime::PRINT));
    }
    fn generate(&mut self, ty: &Type) {
        for register in [R::Rbp, R::Rbx, R::R12, R::R13, R::R14, R::R15] {
            self.push(Instruction::Push(register));
        }
        self.push(Instruction::StackAllocate(8));
        self.mov(R::R12, R::Rdi);
        self.mov(R::R13, R::Rsi);
        self.mov(R::R14, R::Rdx);
        self.mov(R::Rbx, R::Rcx);
        self.immediate(R::Rax, 64);
        self.push(Instruction::Compare {
            left: R::R14,
            right: R::Rax,
        });
        let failure = self.label();
        self.push(Instruction::JumpIf {
            condition: Condition::Greater,
            target: failure,
        });
        match ty {
            Type::String => self.string(),
            Type::Int | Type::Byte => self.scalar(1),
            Type::Bool => self.scalar(2),
            Type::Char => self.scalar(3),
            Type::Struct(id) => self.structure(*id),
            Type::Enum(id) => self.enumeration(*id),
            Type::List(element) => self.sequence(element, None),
            Type::Array { element, length } => self.sequence(element, Some(*length)),
            Type::Optional(element) => self.optional(element),
            Type::Ref(element) => {
                self.literal("ref(");
                self.load(R::Rdi, R::R12, 0);
                self.child(element);
                self.literal(")");
            }
            _ => self.push(Instruction::ExitFailure),
        }
        self.push(Instruction::StackDeallocate(8));
        for register in [R::R15, R::R14, R::R13, R::R12, R::Rbx, R::Rbp] {
            self.push(Instruction::Pop(register));
        }
        self.push(Instruction::Return);
        self.push(Instruction::Label(failure));
        self.push(Instruction::ExitFailure);
    }
    fn string(&mut self) {
        let raw = self.label();
        let done = self.label();
        self.push(Instruction::Test(R::Rbx));
        self.push(Instruction::JumpIfZero(raw));
        self.literal("\"");
        self.load(R::Rbp, R::R12, 0);
        self.immediate(R::R15, 0);
        let loop_label = self.label();
        let end_loop = self.label();
        let ordinary = self.label();
        let next = self.label();
        let escapes = [
            (b'\n', "\\n"),
            (b'\r', "\\r"),
            (b'\t', "\\t"),
            (0, "\\0"),
            (b'"', "\\\""),
            (b'\\', "\\\\"),
        ];
        let branches: Vec<_> = escapes.iter().map(|_| self.label()).collect();
        self.push(Instruction::Label(loop_label));
        self.push(Instruction::Compare {
            left: R::R15,
            right: R::Rbp,
        });
        self.push(Instruction::JumpIf {
            condition: Condition::GreaterEqual,
            target: end_loop,
        });
        self.push(Instruction::IndexedLoad8 {
            destination: R::Rax,
            base: R::R12,
            index: R::R15,
            displacement: 8,
        });
        for ((byte, _), branch) in escapes.iter().zip(&branches) {
            self.immediate(R::Rcx, *byte as u64);
            self.push(Instruction::Compare {
                left: R::Rax,
                right: R::Rcx,
            });
            self.push(Instruction::JumpIf {
                condition: Condition::Equal,
                target: *branch,
            });
        }
        self.push(Instruction::Jump(ordinary));
        for ((_, escape), branch) in escapes.iter().zip(branches) {
            self.push(Instruction::Label(branch));
            self.literal(escape);
            self.push(Instruction::Jump(next));
        }
        self.push(Instruction::Label(ordinary));
        self.mov(R::Rdi, R::R12);
        self.push(Instruction::Add {
            destination: R::Rdi,
            source: R::R15,
        });
        self.immediate(R::Rax, 8);
        self.push(Instruction::Add {
            destination: R::Rdi,
            source: R::Rax,
        });
        self.immediate(R::Rsi, 1);
        self.mov(R::Rdx, R::R13);
        self.push(Instruction::Call(runtime::WRITE_ALL));
        self.push(Instruction::Label(next));
        self.immediate(R::Rax, 1);
        self.push(Instruction::Add {
            destination: R::R15,
            source: R::Rax,
        });
        self.push(Instruction::Jump(loop_label));
        self.push(Instruction::Label(end_loop));
        self.literal("\"");
        self.push(Instruction::Jump(done));
        self.push(Instruction::Label(raw));
        self.scalar(0);
        self.push(Instruction::Label(done));
    }
    fn structure(&mut self, id: TypeId) {
        let definition = self.structs.get(&id).expect("known struct");
        self.literal(&definition.name);
        self.literal("{");
        for (index, field) in definition.fields.iter().enumerate() {
            if index != 0 {
                self.literal(", ");
            }
            self.literal(&field.name);
            self.literal(": ");
            self.load(R::Rdi, R::R12, field.offset as i32);
            self.child(&field.ty);
        }
        self.literal("}");
    }
    fn enumeration(&mut self, id: TypeId) {
        let definition = self.enums.get(&id).expect("known enum");
        let boxed = definition
            .variants
            .iter()
            .any(|item| item.payload.is_some());
        if boxed {
            self.load(R::R15, R::R12, 0);
        } else {
            self.mov(R::R15, R::R12);
        }
        let done = self.label();
        let branches: Vec<_> = definition.variants.iter().map(|_| self.label()).collect();
        for (index, branch) in branches.iter().enumerate() {
            self.immediate(R::Rax, index as u64);
            self.push(Instruction::Compare {
                left: R::R15,
                right: R::Rax,
            });
            self.push(Instruction::JumpIf {
                condition: Condition::Equal,
                target: *branch,
            });
        }
        self.push(Instruction::ExitFailure);
        for (variant, branch) in definition.variants.iter().zip(branches) {
            self.push(Instruction::Label(branch));
            self.literal(&definition.name);
            self.literal(".");
            self.literal(&variant.name);
            if let Some(payload) = &variant.payload {
                self.literal("(");
                self.load(R::Rdi, R::R12, 8);
                self.child(payload);
                self.literal(")");
            }
            self.push(Instruction::Jump(done));
        }
        self.push(Instruction::Label(done));
    }
    fn optional(&mut self, element: &Type) {
        let none = self.label();
        let done = self.label();
        self.load(R::Rax, R::R12, 0);
        self.push(Instruction::Test(R::Rax));
        self.push(Instruction::JumpIfZero(none));
        self.literal("some(");
        self.load(R::Rdi, R::R12, 8);
        self.child(element);
        self.literal(")");
        self.push(Instruction::Jump(done));
        self.push(Instruction::Label(none));
        self.literal("none()");
        self.push(Instruction::Label(done));
    }
    fn sequence(&mut self, element: &Type, fixed_length: Option<usize>) {
        self.literal("[");
        if let Some(length) = fixed_length {
            self.immediate(R::Rbx, length as u64);
        } else {
            self.load(R::Rbx, R::R12, 0);
            self.load(R::R12, R::R12, 24);
        }
        self.immediate(R::R15, 0);
        let loop_label = self.label();
        let done = self.label();
        let no_separator = self.label();
        self.push(Instruction::Label(loop_label));
        self.push(Instruction::Compare {
            left: R::R15,
            right: R::Rbx,
        });
        self.push(Instruction::JumpIf {
            condition: Condition::GreaterEqual,
            target: done,
        });
        self.push(Instruction::Test(R::R15));
        self.push(Instruction::JumpIfZero(no_separator));
        self.literal(", ");
        self.push(Instruction::Label(no_separator));
        self.mov(R::Rax, R::R15);
        let stride = if fixed_length.is_none() && *element == Type::Byte {
            1
        } else if fixed_length.is_none() {
            match element {
                Type::Struct(id) => {
                    self.structs
                        .get(id)
                        .expect("known struct")
                        .fields
                        .len()
                        .max(1)
                        * 8
                }
                _ => 8,
            }
        } else {
            8
        };
        self.immediate(R::Rcx, stride as u64);
        self.push(Instruction::MultiplySigned {
            destination: R::Rax,
            source: R::Rcx,
        });
        self.mov(R::Rdi, R::R12);
        self.push(Instruction::Add {
            destination: R::Rdi,
            source: R::Rax,
        });
        if fixed_length.is_none() && *element == Type::Byte {
            self.push(Instruction::Load8 {
                destination: R::Rdi,
                base: R::Rdi,
                displacement: 0,
            });
        } else if fixed_length.is_none() && matches!(element, Type::Struct(_)) {
            // List structs are stored inline, so Rdi already points to the record.
        } else {
            self.load(R::Rdi, R::Rdi, 0);
        }
        self.child(element);
        self.immediate(R::Rax, 1);
        self.push(Instruction::Add {
            destination: R::R15,
            source: R::Rax,
        });
        self.push(Instruction::Jump(loop_label));
        self.push(Instruction::Label(done));
        self.literal("]");
    }
}
