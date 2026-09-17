use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::frontend::ast::{BinaryOperator, UnaryOperator, Visibility};
use crate::frontend::resolution::SymbolId;
use crate::frontend::types::Type;
use crate::middle::ir::{
    IrConstant, IrFunction, IrInstruction, IrInstructionKind, IrStruct, IrTerminator, ValueId,
};
use crate::middle::verify::VerifiedIrModule;

use super::instruction::{Condition, Instruction, MachineFunction, MachineModule};
use super::register::Register;
use super::runtime;
use super::startup;

const ARGUMENT_REGISTERS: [Register; 6] = [
    Register::Rdi,
    Register::Rsi,
    Register::Rdx,
    Register::Rcx,
    Register::R8,
    Register::R9,
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendError {
    MissingMain,
    InvalidMain,
    DuplicateFunctionSymbol(SymbolId),
    UnsupportedType {
        function: String,
        ty: Type,
    },
    ArgumentAreaTooLarge(String),
    UnsupportedIr {
        function: String,
        feature: &'static str,
    },
    InvalidIntegerLiteral(String),
    FrameTooLarge(String),
    MissingLocalSlot(SymbolId),
    MissingValueSlot(ValueId),
    MissingInstructionResult,
    MissingBlockTerminator,
    UnknownFunction(SymbolId),
}

impl fmt::Display for BackendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMain => formatter.write_str("executable has no `main` function"),
            Self::InvalidMain => formatter.write_str("entry function must be `public void main()`"),
            Self::DuplicateFunctionSymbol(symbol) => {
                write!(formatter, "duplicate IR function symbol {symbol:?}")
            }
            Self::UnsupportedType { function, ty } => {
                write!(
                    formatter,
                    "x86-64 bootstrap cannot lower type `{ty}` in function `{function}`"
                )
            }
            Self::ArgumentAreaTooLarge(function) => {
                write!(
                    formatter,
                    "stack argument area for `{function}` is too large"
                )
            }
            Self::UnsupportedIr { function, feature } => {
                write!(
                    formatter,
                    "x86-64 bootstrap cannot lower {feature} in function `{function}`"
                )
            }
            Self::InvalidIntegerLiteral(value) => {
                write!(formatter, "invalid signed 64-bit integer literal `{value}`")
            }
            Self::FrameTooLarge(function) => {
                write!(formatter, "stack frame for `{function}` is too large")
            }
            Self::MissingLocalSlot(symbol) => {
                write!(formatter, "no stack slot for local symbol {symbol:?}")
            }
            Self::MissingValueSlot(value) => {
                write!(formatter, "no stack slot for IR value {value:?}")
            }
            Self::MissingInstructionResult => {
                formatter.write_str("value-producing IR instruction has no result")
            }
            Self::MissingBlockTerminator => formatter.write_str("IR basic block has no terminator"),
            Self::UnknownFunction(symbol) => write!(
                formatter,
                "call targets unsupported or unknown function {symbol:?}"
            ),
        }
    }
}

impl std::error::Error for BackendError {}

/// Lowers the supported IR subset using stack slots for locals and temporaries.
/// `rax` and `rcx` are fixed arithmetic temporaries; this is intentionally easy
/// to replace with allocation later and preserves SysV callee-saved registers.
pub fn lower(module: &VerifiedIrModule<'_>) -> Result<MachineModule, BackendError> {
    let module = module.as_module();
    let main = module
        .functions
        .iter()
        .find(|function| function.name == "main")
        .ok_or(BackendError::MissingMain)?;
    let valid_parameters = matches!(
        main.parameters.as_slice(),
        [] | [crate::middle::ir::IrLocal {
            ty: Type::CliArgs,
            ..
        }]
    );
    if main.visibility != Visibility::Public || main.return_type != Type::Void || !valid_parameters
    {
        return Err(BackendError::InvalidMain);
    }

    let mut symbols = HashSet::new();
    for function in &module.functions {
        if !symbols.insert(function.symbol) {
            return Err(BackendError::DuplicateFunctionSymbol(function.symbol));
        }
        validate_signature(function)?;
    }

    for function in &module.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                if let IrInstructionKind::Call {
                    function: target, ..
                } = instruction.kind
                    && !symbols.contains(&target)
                {
                    return Err(BackendError::UnknownFunction(target));
                }
            }
        }
    }

    let structs: HashMap<_, _> = module.structs.iter().map(|item| (item.id, item)).collect();
    let (string_offsets, read_only_data) = collect_string_data(module);
    let mut functions = module
        .functions
        .iter()
        .map(|function| lower_function(function, &structs, &string_offsets))
        .collect::<Result<Vec<_>, _>>()?;
    let startup = startup::generate(main.symbol, !main.parameters.is_empty());
    let runtime_roots: HashSet<_> = functions
        .iter()
        .flat_map(|function| &function.instructions)
        .chain(&startup)
        .filter_map(|instruction| match instruction {
            Instruction::Call(symbol) => Some(*symbol),
            _ => None,
        })
        .collect();
    functions.extend(runtime::functions_for(&runtime_roots));
    Ok(MachineModule {
        startup,
        entry_function: main.symbol,
        functions,
        read_only_data,
    })
}

fn collect_string_data(module: &crate::middle::ir::IrModule) -> (HashMap<String, usize>, Vec<u8>) {
    let mut offsets = HashMap::new();
    let mut data = Vec::new();
    for value in module
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| match &instruction.kind {
            IrInstructionKind::StringConstant(value) => Some(value),
            _ => None,
        })
    {
        if offsets.contains_key(value) {
            continue;
        }
        while data.len() % 8 != 0 {
            data.push(0);
        }
        offsets.insert(value.clone(), data.len());
        data.extend_from_slice(&(value.len() as u64).to_le_bytes());
        data.extend_from_slice(value.as_bytes());
    }
    (offsets, data)
}

fn validate_signature(function: &IrFunction) -> Result<(), BackendError> {
    if !matches!(
        function.return_type,
        Type::Int
            | Type::Byte
            | Type::Bool
            | Type::Char
            | Type::String
            | Type::Enum(_)
            | Type::List(_)
            | Type::Void
    ) {
        return Err(BackendError::UnsupportedType {
            function: function.name.clone(),
            ty: function.return_type.clone(),
        });
    }
    for parameter in &function.parameters {
        if !matches!(
            parameter.ty,
            Type::Int
                | Type::Byte
                | Type::Bool
                | Type::Char
                | Type::String
                | Type::CliArgs
                | Type::Enum(_)
                | Type::List(_)
        ) {
            return Err(BackendError::UnsupportedType {
                function: function.name.clone(),
                ty: parameter.ty.clone(),
            });
        }
    }
    for local in &function.locals {
        if !matches!(
            local.ty,
            Type::Int
                | Type::Byte
                | Type::Bool
                | Type::Char
                | Type::String
                | Type::Enum(_)
                | Type::Struct(_)
                | Type::Array { .. }
                | Type::List(_)
                | Type::CliArgs
        ) {
            return Err(BackendError::UnsupportedType {
                function: function.name.clone(),
                ty: local.ty.clone(),
            });
        }
    }
    Ok(())
}

struct Slots {
    locals: HashMap<SymbolId, i32>,
    values: HashMap<ValueId, i32>,
    frame_size: u32,
}

fn lower_function(
    function: &IrFunction,
    structs: &HashMap<crate::frontend::types::TypeId, &IrStruct>,
    string_offsets: &HashMap<String, usize>,
) -> Result<MachineFunction, BackendError> {
    let slots = build_slots(function)?;
    let mut instructions = vec![
        Instruction::Push(Register::Rbp),
        Instruction::MoveRegister {
            destination: Register::Rbp,
            source: Register::Rsp,
        },
    ];
    if slots.frame_size != 0 {
        instructions.push(Instruction::StackAllocate(slots.frame_size));
    }
    for (parameter, register) in function.parameters.iter().zip(ARGUMENT_REGISTERS) {
        instructions.push(Instruction::Store64 {
            base: Register::Rbp,
            displacement: local_slot(&slots, parameter.symbol)?,
            source: register,
        });
    }
    for (stack_index, parameter) in function
        .parameters
        .iter()
        .skip(ARGUMENT_REGISTERS.len())
        .enumerate()
    {
        instructions.push(Instruction::Load64 {
            destination: Register::Rax,
            base: Register::Rbp,
            displacement: stack_argument_displacement(stack_index, &function.name)?,
        });
        instructions.push(Instruction::Store64 {
            base: Register::Rbp,
            displacement: local_slot(&slots, parameter.symbol)?,
            source: Register::Rax,
        });
    }
    instructions.push(Instruction::Jump(function.entry));
    for block in &function.blocks {
        instructions.push(Instruction::Label(block.id));
        for instruction in &block.instructions {
            lower_instruction(
                function,
                instruction,
                &slots,
                structs,
                string_offsets,
                &mut instructions,
            )?;
        }
        lower_terminator(
            block
                .terminator
                .as_ref()
                .ok_or(BackendError::MissingBlockTerminator)?,
            &slots,
            &mut instructions,
        )?;
    }
    Ok(MachineFunction {
        symbol: function.symbol,
        name: function.name.clone(),
        instructions,
    })
}

fn build_slots(function: &IrFunction) -> Result<Slots, BackendError> {
    let mut locals = HashMap::new();
    let mut values = HashMap::new();
    let mut count = 0usize;
    for local in function.parameters.iter().chain(&function.locals) {
        if let std::collections::hash_map::Entry::Vacant(entry) = locals.entry(local.symbol) {
            let width = match local.ty {
                Type::Array { length, .. } => length,
                _ => 1,
            };
            count += width;
            entry.insert(slot_displacement(count, &function.name)?);
        }
    }
    for block in &function.blocks {
        for instruction in &block.instructions {
            if let Some(value) = instruction.result
                && let std::collections::hash_map::Entry::Vacant(entry) = values.entry(value)
            {
                count += 1;
                entry.insert(slot_displacement(count, &function.name)?);
            }
        }
    }
    let bytes = count
        .checked_mul(8)
        .ok_or_else(|| BackendError::FrameTooLarge(function.name.clone()))?;
    let aligned = bytes
        .checked_add(15)
        .ok_or_else(|| BackendError::FrameTooLarge(function.name.clone()))?
        & !15;
    let frame_size =
        u32::try_from(aligned).map_err(|_| BackendError::FrameTooLarge(function.name.clone()))?;
    Ok(Slots {
        locals,
        values,
        frame_size,
    })
}

fn slot_displacement(index: usize, function: &str) -> Result<i32, BackendError> {
    let bytes = index
        .checked_mul(8)
        .ok_or_else(|| BackendError::FrameTooLarge(function.to_owned()))?;
    let bytes =
        i32::try_from(bytes).map_err(|_| BackendError::FrameTooLarge(function.to_owned()))?;
    Ok(-bytes)
}

fn stack_argument_displacement(index: usize, function: &str) -> Result<i32, BackendError> {
    let bytes = index
        .checked_mul(8)
        .and_then(|value| value.checked_add(16))
        .ok_or_else(|| BackendError::ArgumentAreaTooLarge(function.to_owned()))?;
    i32::try_from(bytes).map_err(|_| BackendError::ArgumentAreaTooLarge(function.to_owned()))
}

fn lower_instruction(
    function: &IrFunction,
    instruction: &IrInstruction,
    slots: &Slots,
    structs: &HashMap<crate::frontend::types::TypeId, &IrStruct>,
    string_offsets: &HashMap<String, usize>,
    output: &mut Vec<Instruction>,
) -> Result<(), BackendError> {
    match &instruction.kind {
        IrInstructionKind::Constant(IrConstant::Integer(value)) => {
            let magnitude = value
                .parse::<u64>()
                .map_err(|_| BackendError::InvalidIntegerLiteral(value.clone()))?;
            output.push(Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: magnitude,
            });
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::Constant(IrConstant::Byte(value)) => {
            let value = value
                .parse::<u8>()
                .map_err(|_| BackendError::InvalidIntegerLiteral(value.clone()))?;
            output.push(Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: u64::from(value),
            });
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::Constant(IrConstant::Bool(value)) => {
            output.push(Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: u64::from(*value),
            });
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::Constant(IrConstant::Char(value)) => {
            output.push(Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: u64::from(*value as u32),
            });
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::Constant(_) => {
            return Err(BackendError::UnsupportedIr {
                function: function.name.clone(),
                feature: "non-integer constants",
            });
        }
        IrInstructionKind::LoadLocal(symbol) => {
            output.push(Instruction::Load64 {
                destination: Register::Rax,
                base: Register::Rbp,
                displacement: local_slot(slots, *symbol)?,
            });
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::BindLocal { local, value } => {
            load_value(output, slots, *value, Register::Rax)?;
            output.push(Instruction::Store64 {
                base: Register::Rbp,
                displacement: local_slot(slots, *local)?,
                source: Register::Rax,
            });
        }
        IrInstructionKind::StructInit {
            local,
            struct_id,
            fields,
        } => {
            emit_struct_allocation(output, structs, *struct_id, function)?;
            output.push(Instruction::MoveRegister {
                destination: Register::R11,
                source: Register::Rax,
            });
            for (field, value) in fields {
                load_value(output, slots, *value, Register::Rax)?;
                output.push(Instruction::Store64 {
                    base: Register::R11,
                    displacement: i32::try_from(field_offset(
                        structs, *struct_id, *field, function,
                    )?)
                    .map_err(|_| BackendError::FrameTooLarge(function.name.clone()))?,
                    source: Register::Rax,
                });
            }
            output.push(Instruction::Store64 {
                base: Register::Rbp,
                displacement: local_slot(slots, *local)?,
                source: Register::R11,
            });
        }
        IrInstructionKind::StructValue { struct_id, fields } => {
            emit_struct_allocation(output, structs, *struct_id, function)?;
            output.push(Instruction::MoveRegister {
                destination: Register::R11,
                source: Register::Rax,
            });
            for (field, value) in fields {
                load_value(output, slots, *value, Register::Rax)?;
                output.push(Instruction::Store64 {
                    base: Register::R11,
                    displacement: i32::try_from(field_offset(
                        structs, *struct_id, *field, function,
                    )?)
                    .map_err(|_| BackendError::FrameTooLarge(function.name.clone()))?,
                    source: Register::Rax,
                });
            }
            output.push(Instruction::MoveRegister {
                destination: Register::Rax,
                source: Register::R11,
            });
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::AggregateCopy { struct_id, source } => {
            load_value(output, slots, *source, Register::Rax)?;
            store_result(instruction, slots, output)?;
            emit_struct_allocation(output, structs, *struct_id, function)?;
            output.push(Instruction::MoveRegister {
                destination: Register::Rdx,
                source: Register::Rax,
            });
            load_value(
                output,
                slots,
                instruction
                    .result
                    .ok_or(BackendError::MissingInstructionResult)?,
                Register::Rcx,
            )?;
            emit_copy_words(
                output,
                Register::Rcx,
                Register::Rdx,
                struct_word_count(structs, *struct_id, function)?,
            )?;
            output.push(Instruction::MoveRegister {
                destination: Register::Rax,
                source: Register::Rdx,
            });
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::FieldLoad {
            local,
            struct_id,
            field,
        } => {
            output.push(Instruction::Load64 {
                destination: Register::Rax,
                base: Register::Rbp,
                displacement: local_slot(slots, *local)?,
            });
            output.push(Instruction::Load64 {
                destination: Register::Rax,
                base: Register::Rax,
                displacement: i32::try_from(field_offset(structs, *struct_id, *field, function)?)
                    .map_err(|_| BackendError::FrameTooLarge(function.name.clone()))?,
            });
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::FieldStore {
            local,
            struct_id,
            field,
            value,
            ..
        } => {
            output.push(Instruction::Load64 {
                destination: Register::Rcx,
                base: Register::Rbp,
                displacement: local_slot(slots, *local)?,
            });
            load_value(output, slots, *value, Register::Rax)?;
            output.push(Instruction::Store64 {
                base: Register::Rcx,
                displacement: i32::try_from(field_offset(structs, *struct_id, *field, function)?)
                    .map_err(|_| BackendError::FrameTooLarge(function.name.clone()))?,
                source: Register::Rax,
            });
        }
        IrInstructionKind::EnumConstant { variant, .. } => {
            output.push(Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: u64::from(*variant),
            });
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::ArrayInit { local, values, .. } => {
            for (index, value) in values.iter().enumerate() {
                load_value(output, slots, *value, Register::Rax)?;
                output.push(Instruction::Store64 {
                    base: Register::Rbp,
                    displacement: local_field_slot(
                        slots,
                        *local,
                        byte_offset(index, &function.name)?,
                    )?,
                    source: Register::Rax,
                });
            }
        }
        IrInstructionKind::ArrayLoad {
            local,
            length,
            index,
            ..
        } => {
            load_value(output, slots, *index, Register::Rdi)?;
            output.push(Instruction::MoveImmediate64 {
                destination: Register::Rsi,
                value: usize_to_u64(*length, &function.name)?,
            });
            output.push(Instruction::Call(runtime::BOUNDS_CHECK));
            output.push(Instruction::IndexedLoad64 {
                destination: Register::Rax,
                base: Register::Rbp,
                index: Register::Rdi,
                displacement: local_slot(slots, *local)?,
            });
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::ArrayStore {
            local,
            length,
            index,
            value,
            ..
        } => {
            load_value(output, slots, *index, Register::Rdi)?;
            output.push(Instruction::MoveImmediate64 {
                destination: Register::Rsi,
                value: usize_to_u64(*length, &function.name)?,
            });
            load_value(output, slots, *value, Register::Rdx)?;
            output.push(Instruction::Call(runtime::BOUNDS_CHECK));
            output.push(Instruction::IndexedStore64 {
                base: Register::Rbp,
                index: Register::Rdi,
                displacement: local_slot(slots, *local)?,
                source: Register::Rdx,
            });
        }
        IrInstructionKind::ArrayLength(length) => {
            output.push(Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: usize_to_u64(*length, &function.name)?,
            });
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::ListInit {
            local,
            element_type,
            values,
        } => {
            let capacity = values.len().max(4);
            let stride = type_stride(element_type, structs, function)?;
            let bytes = capacity
                .checked_mul(stride)
                .and_then(|bytes| bytes.checked_add(24))
                .ok_or_else(|| BackendError::FrameTooLarge(function.name.clone()))?;
            output.push(Instruction::MoveImmediate64 {
                destination: Register::Rdi,
                value: usize_to_u64(bytes, &function.name)?,
            });
            output.push(Instruction::Call(runtime::ALLOC));
            output.push(Instruction::MoveRegister {
                destination: Register::R11,
                source: Register::Rax,
            });
            output.push(Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: usize_to_u64(values.len(), &function.name)?,
            });
            output.push(Instruction::Store64 {
                base: Register::R11,
                displacement: 0,
                source: Register::Rax,
            });
            output.push(Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: usize_to_u64(capacity, &function.name)?,
            });
            output.push(Instruction::Store64 {
                base: Register::R11,
                displacement: 8,
                source: Register::Rax,
            });
            output.push(Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: usize_to_u64(stride, &function.name)?,
            });
            output.push(Instruction::Store64 {
                base: Register::R11,
                displacement: 16,
                source: Register::Rax,
            });
            for (index, value) in values.iter().enumerate() {
                let element_offset = index
                    .checked_mul(stride)
                    .and_then(|offset| offset.checked_add(24))
                    .ok_or_else(|| BackendError::FrameTooLarge(function.name.clone()))?;
                if let Type::Struct(struct_id) = element_type {
                    load_value(output, slots, *value, Register::Rcx)?;
                    for word in 0..struct_word_count(structs, *struct_id, function)? {
                        let source_offset = i32::try_from(word * 8)
                            .map_err(|_| BackendError::FrameTooLarge(function.name.clone()))?;
                        let destination_offset = i32::try_from(element_offset + word * 8)
                            .map_err(|_| BackendError::FrameTooLarge(function.name.clone()))?;
                        output.push(Instruction::Load64 {
                            destination: Register::Rax,
                            base: Register::Rcx,
                            displacement: source_offset,
                        });
                        output.push(Instruction::Store64 {
                            base: Register::R11,
                            displacement: destination_offset,
                            source: Register::Rax,
                        });
                    }
                } else if element_type == &Type::Byte {
                    load_value(output, slots, *value, Register::Rax)?;
                    output.push(Instruction::Store8 {
                        base: Register::R11,
                        displacement: i32::try_from(element_offset)
                            .map_err(|_| BackendError::FrameTooLarge(function.name.clone()))?,
                        source: Register::Rax,
                    });
                } else {
                    load_value(output, slots, *value, Register::Rax)?;
                    output.push(Instruction::Store64 {
                        base: Register::R11,
                        displacement: i32::try_from(element_offset)
                            .map_err(|_| BackendError::FrameTooLarge(function.name.clone()))?,
                        source: Register::Rax,
                    });
                }
            }
            output.push(Instruction::Store64 {
                base: Register::Rbp,
                displacement: local_slot(slots, *local)?,
                source: Register::R11,
            });
        }
        IrInstructionKind::ListLoad {
            local,
            element_type,
            index,
        } => {
            output.push(Instruction::Load64 {
                destination: Register::Rdi,
                base: Register::Rbp,
                displacement: local_slot(slots, *local)?,
            });
            load_value(output, slots, *index, Register::Rsi)?;
            if let Type::Struct(struct_id) = element_type {
                output.push(Instruction::Call(runtime::LIST_ELEMENT));
                store_result(instruction, slots, output)?;
                emit_struct_allocation(output, structs, *struct_id, function)?;
                output.push(Instruction::MoveRegister {
                    destination: Register::Rdx,
                    source: Register::Rax,
                });
                load_value(
                    output,
                    slots,
                    instruction
                        .result
                        .ok_or(BackendError::MissingInstructionResult)?,
                    Register::Rcx,
                )?;
                emit_copy_words(
                    output,
                    Register::Rcx,
                    Register::Rdx,
                    struct_word_count(structs, *struct_id, function)?,
                )?;
                output.push(Instruction::MoveRegister {
                    destination: Register::Rax,
                    source: Register::Rdx,
                });
                store_result(instruction, slots, output)?;
            } else if element_type == &Type::Byte {
                output.push(Instruction::Call(runtime::LIST_ELEMENT));
                output.push(Instruction::Load8 {
                    destination: Register::Rax,
                    base: Register::Rax,
                    displacement: 0,
                });
                store_result(instruction, slots, output)?;
            } else {
                output.push(Instruction::Call(runtime::LIST_LOAD));
                store_result(instruction, slots, output)?;
            }
        }
        IrInstructionKind::ListStore {
            local,
            element_type,
            index,
            value,
        } => {
            output.push(Instruction::Load64 {
                destination: Register::Rdi,
                base: Register::Rbp,
                displacement: local_slot(slots, *local)?,
            });
            load_value(output, slots, *index, Register::Rsi)?;
            if let Type::Struct(struct_id) = element_type {
                output.push(Instruction::Call(runtime::LIST_ELEMENT));
                output.push(Instruction::MoveRegister {
                    destination: Register::Rdx,
                    source: Register::Rax,
                });
                load_value(output, slots, *value, Register::Rcx)?;
                emit_copy_words(
                    output,
                    Register::Rcx,
                    Register::Rdx,
                    struct_word_count(structs, *struct_id, function)?,
                )?;
            } else if element_type == &Type::Byte {
                output.push(Instruction::Call(runtime::LIST_ELEMENT));
                load_value(output, slots, *value, Register::Rcx)?;
                output.push(Instruction::Store8 {
                    base: Register::Rax,
                    displacement: 0,
                    source: Register::Rcx,
                });
            } else {
                load_value(output, slots, *value, Register::Rdx)?;
                output.push(Instruction::Call(runtime::LIST_STORE));
            }
        }
        IrInstructionKind::ListPush {
            local,
            element_type,
            value,
        } => {
            output.push(Instruction::Load64 {
                destination: Register::Rdi,
                base: Register::Rbp,
                displacement: local_slot(slots, *local)?,
            });
            output.push(Instruction::Call(runtime::LIST_PUSH));
            output.push(Instruction::Store64 {
                base: Register::Rbp,
                displacement: local_slot(slots, *local)?,
                source: Register::Rax,
            });
            if let Type::Struct(struct_id) = element_type {
                load_value(output, slots, *value, Register::Rcx)?;
                emit_copy_words(
                    output,
                    Register::Rcx,
                    Register::Rdx,
                    struct_word_count(structs, *struct_id, function)?,
                )?;
            } else if element_type == &Type::Byte {
                load_value(output, slots, *value, Register::Rcx)?;
                output.push(Instruction::Store8 {
                    base: Register::Rdx,
                    displacement: 0,
                    source: Register::Rcx,
                });
            } else {
                load_value(output, slots, *value, Register::Rcx)?;
                output.push(Instruction::Store64 {
                    base: Register::Rdx,
                    displacement: 0,
                    source: Register::Rcx,
                });
            }
            output.push(Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 0,
            });
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::ListPop {
            local,
            element_type,
        } => {
            output.push(Instruction::Load64 {
                destination: Register::Rdi,
                base: Register::Rbp,
                displacement: local_slot(slots, *local)?,
            });
            if let Type::Struct(struct_id) = element_type {
                output.push(Instruction::Call(runtime::LIST_POP_ELEMENT));
                store_result(instruction, slots, output)?;
                emit_struct_allocation(output, structs, *struct_id, function)?;
                output.push(Instruction::MoveRegister {
                    destination: Register::Rdx,
                    source: Register::Rax,
                });
                load_value(
                    output,
                    slots,
                    instruction
                        .result
                        .ok_or(BackendError::MissingInstructionResult)?,
                    Register::Rcx,
                )?;
                emit_copy_words(
                    output,
                    Register::Rcx,
                    Register::Rdx,
                    struct_word_count(structs, *struct_id, function)?,
                )?;
                output.push(Instruction::MoveRegister {
                    destination: Register::Rax,
                    source: Register::Rdx,
                });
                store_result(instruction, slots, output)?;
            } else {
                output.push(Instruction::Call(if element_type == &Type::Byte {
                    runtime::LIST_POP_ELEMENT
                } else {
                    runtime::LIST_POP
                }));
                if element_type == &Type::Byte {
                    output.push(Instruction::Load8 {
                        destination: Register::Rax,
                        base: Register::Rax,
                        displacement: 0,
                    });
                }
                store_result(instruction, slots, output)?;
            }
        }
        IrInstructionKind::ListLength { local, .. } => {
            output.push(Instruction::Load64 {
                destination: Register::Rax,
                base: Register::Rbp,
                displacement: local_slot(slots, *local)?,
            });
            output.push(Instruction::Load64 {
                destination: Register::Rax,
                base: Register::Rax,
                displacement: 0,
            });
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::CliArgLoad { local, index } => {
            output.push(Instruction::Load64 {
                destination: Register::Rdi,
                base: Register::Rbp,
                displacement: local_slot(slots, *local)?,
            });
            load_value(output, slots, *index, Register::Rsi)?;
            output.push(Instruction::Call(runtime::LIST_LOAD));
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::CliArgsLength { local } => {
            output.push(Instruction::Load64 {
                destination: Register::Rax,
                base: Register::Rbp,
                displacement: local_slot(slots, *local)?,
            });
            output.push(Instruction::Load64 {
                destination: Register::Rax,
                base: Register::Rax,
                displacement: 0,
            });
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::StringConstant(value) => {
            output.push(Instruction::LoadDataAddress {
                destination: Register::Rax,
                offset: *string_offsets
                    .get(value)
                    .expect("collected every IR string constant"),
            });
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::StringConcat { left, right } => {
            load_value(output, slots, *left, Register::Rdi)?;
            load_value(output, slots, *right, Register::Rsi)?;
            output.push(Instruction::Call(runtime::STRING_CONCAT));
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::StringEqual { equal, left, right } => {
            load_value(output, slots, *left, Register::Rdi)?;
            load_value(output, slots, *right, Register::Rsi)?;
            output.push(Instruction::Call(runtime::STRING_EQUAL));
            if !equal {
                output.push(Instruction::Test(Register::Rax));
                output.push(Instruction::MaterializeCondition(Condition::Equal));
            }
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::StringLength(value) => {
            load_value(output, slots, *value, Register::Rax)?;
            output.push(Instruction::Load64 {
                destination: Register::Rax,
                base: Register::Rax,
                displacement: 0,
            });
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::StringByte { value, index } => {
            load_value(output, slots, *value, Register::Rdi)?;
            load_value(output, slots, *index, Register::Rsi)?;
            output.push(Instruction::Call(runtime::STRING_BYTE));
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::StringSlice { value, start, end } => {
            load_value(output, slots, *value, Register::Rdi)?;
            load_value(output, slots, *start, Register::Rsi)?;
            load_value(output, slots, *end, Register::Rdx)?;
            output.push(Instruction::Call(runtime::STRING_SLICE));
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::ReadFile(path) => {
            load_value(output, slots, *path, Register::Rdi)?;
            output.push(Instruction::Call(runtime::READ_FILE));
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::ReadBytes(path) => {
            load_value(output, slots, *path, Register::Rdi)?;
            output.push(Instruction::Call(runtime::READ_BYTES));
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::WriteFile { path, data } => {
            load_value(output, slots, *path, Register::Rdi)?;
            load_value(output, slots, *data, Register::Rsi)?;
            output.push(Instruction::Call(runtime::WRITE_FILE));
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::WriteBytes { path, data } => {
            load_value(output, slots, *path, Register::Rdi)?;
            load_value(output, slots, *data, Register::Rsi)?;
            output.push(Instruction::Call(runtime::WRITE_BYTES));
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::Exists(path) => {
            load_value(output, slots, *path, Register::Rdi)?;
            output.push(Instruction::Call(runtime::EXISTS));
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::Print {
            value,
            value_type,
            stderr,
            newline,
        } => {
            load_value(output, slots, *value, Register::Rdi)?;
            let type_code = match value_type {
                Type::String => 0,
                Type::Int => 1,
                Type::Bool => 2,
                Type::Char => 3,
                _ => {
                    return Err(BackendError::UnsupportedIr {
                        function: function.name.clone(),
                        feature: "unsupported std.io output type",
                    });
                }
            };
            output.push(Instruction::MoveImmediate64 {
                destination: Register::Rsi,
                value: type_code,
            });
            output.push(Instruction::MoveImmediate64 {
                destination: Register::Rdx,
                value: if *stderr { 2 } else { 1 },
            });
            output.push(Instruction::MoveImmediate64 {
                destination: Register::Rcx,
                value: u64::from(*newline),
            });
            output.push(Instruction::Call(runtime::PRINT));
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::Input { target } => {
            let function = match target {
                Type::String => runtime::INPUT_LINE,
                Type::Int => runtime::INPUT_INT,
                Type::Bool => runtime::INPUT_BOOL,
                Type::Char => runtime::INPUT_CHAR,
                _ => {
                    return Err(BackendError::UnsupportedIr {
                        function: function.name.clone(),
                        feature: "unsupported std.io input type",
                    });
                }
            };
            output.push(Instruction::Call(function));
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::Convert { value, from, to } => {
            load_value(output, slots, *value, Register::Rax)?;
            if matches!((from, to), (Type::Int, Type::Char)) {
                output.push(Instruction::MoveRegister {
                    destination: Register::Rdi,
                    source: Register::Rax,
                });
                output.push(Instruction::Call(runtime::INT_TO_CHAR));
            } else if matches!((from, to), (Type::Int, Type::Byte)) {
                output.push(Instruction::MoveRegister {
                    destination: Register::Rdi,
                    source: Register::Rax,
                });
                output.push(Instruction::Call(runtime::INT_TO_BYTE));
            }
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::Copy(value) => {
            load_value(output, slots, *value, Register::Rax)?;
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::Unary {
            operator: UnaryOperator::Negate,
            operand,
        } => {
            load_value(output, slots, *operand, Register::Rax)?;
            output.push(Instruction::Negate(Register::Rax));
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::Unary {
            operator: UnaryOperator::Not,
            operand,
        } => {
            load_value(output, slots, *operand, Register::Rax)?;
            output.push(Instruction::Test(Register::Rax));
            output.push(Instruction::MaterializeCondition(Condition::Equal));
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::Binary {
            operator,
            left,
            right,
        } => {
            load_value(output, slots, *left, Register::Rax)?;
            load_value(output, slots, *right, Register::Rcx)?;
            let operation = match operator {
                BinaryOperator::Add => Instruction::Add {
                    destination: Register::Rax,
                    source: Register::Rcx,
                },
                BinaryOperator::Subtract => Instruction::Subtract {
                    destination: Register::Rax,
                    source: Register::Rcx,
                },
                BinaryOperator::Multiply => Instruction::MultiplySigned {
                    destination: Register::Rax,
                    source: Register::Rcx,
                },
                BinaryOperator::Divide => {
                    output.push(Instruction::MoveRegister {
                        destination: Register::Rdi,
                        source: Register::Rax,
                    });
                    output.push(Instruction::MoveRegister {
                        destination: Register::Rsi,
                        source: Register::Rcx,
                    });
                    Instruction::Call(runtime::INTEGER_DIVIDE)
                }
                BinaryOperator::Equal
                | BinaryOperator::NotEqual
                | BinaryOperator::Less
                | BinaryOperator::LessEqual
                | BinaryOperator::Greater
                | BinaryOperator::GreaterEqual => {
                    output.push(Instruction::Compare {
                        left: Register::Rax,
                        right: Register::Rcx,
                    });
                    let condition = comparison_condition(*operator).ok_or_else(|| {
                        BackendError::UnsupportedIr {
                            function: function.name.clone(),
                            feature: "unknown comparison operator",
                        }
                    })?;
                    Instruction::MaterializeCondition(condition)
                }
                BinaryOperator::LogicalAnd | BinaryOperator::LogicalOr => {
                    return Err(BackendError::UnsupportedIr {
                        function: function.name.clone(),
                        feature: "eager logical operation (short-circuit lowering was expected)",
                    });
                }
            };
            output.push(operation);
            store_result(instruction, slots, output)?;
        }
        IrInstructionKind::Call {
            function: target,
            arguments,
        } => {
            for (argument, register) in arguments.iter().take(6).zip(ARGUMENT_REGISTERS) {
                load_value(output, slots, *argument, register)?;
            }
            let stack_arguments = arguments.len().saturating_sub(ARGUMENT_REGISTERS.len());
            let padding = usize::from(stack_arguments % 2 == 1) * 8;
            if padding != 0 {
                output.push(Instruction::StackAllocate(padding as u32));
            }
            for argument in arguments.iter().skip(ARGUMENT_REGISTERS.len()).rev() {
                load_value(output, slots, *argument, Register::Rax)?;
                output.push(Instruction::Push(Register::Rax));
            }
            output.push(Instruction::Call(*target));
            let argument_bytes = stack_arguments
                .checked_mul(8)
                .and_then(|bytes| bytes.checked_add(padding))
                .ok_or_else(|| BackendError::ArgumentAreaTooLarge(function.name.clone()))?;
            if argument_bytes != 0 {
                let argument_bytes = u32::try_from(argument_bytes)
                    .map_err(|_| BackendError::ArgumentAreaTooLarge(function.name.clone()))?;
                output.push(Instruction::StackDeallocate(argument_bytes));
            }
            if instruction.result.is_some() {
                store_result(instruction, slots, output)?;
            }
        }
        IrInstructionKind::Aggregate(_) => {
            return Err(BackendError::UnsupportedIr {
                function: function.name.clone(),
                feature: "aggregate values",
            });
        }
    }
    Ok(())
}

fn comparison_condition(operator: BinaryOperator) -> Option<Condition> {
    Some(match operator {
        BinaryOperator::Equal => Condition::Equal,
        BinaryOperator::NotEqual => Condition::NotEqual,
        BinaryOperator::Less => Condition::Less,
        BinaryOperator::LessEqual => Condition::LessEqual,
        BinaryOperator::Greater => Condition::Greater,
        BinaryOperator::GreaterEqual => Condition::GreaterEqual,
        _ => return None,
    })
}

fn lower_terminator(
    terminator: &IrTerminator,
    slots: &Slots,
    output: &mut Vec<Instruction>,
) -> Result<(), BackendError> {
    match terminator {
        IrTerminator::Return(value) => {
            if let Some(value) = value {
                load_value(output, slots, *value, Register::Rax)?;
            }
            emit_epilogue(output);
        }
        IrTerminator::Exit(code) => {
            load_value(output, slots, *code, Register::Rdi)?;
            output.push(Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 60,
            });
            output.push(Instruction::Syscall);
        }
        IrTerminator::Jump(target) => output.push(Instruction::Jump(*target)),
        IrTerminator::Branch {
            condition,
            then_block,
            else_block,
        } => {
            load_value(output, slots, *condition, Register::Rax)?;
            output.push(Instruction::Test(Register::Rax));
            output.push(Instruction::JumpIfZero(*else_block));
            output.push(Instruction::Jump(*then_block));
        }
    }
    Ok(())
}

fn load_value(
    output: &mut Vec<Instruction>,
    slots: &Slots,
    value: ValueId,
    destination: Register,
) -> Result<(), BackendError> {
    let displacement = slots
        .values
        .get(&value)
        .copied()
        .ok_or(BackendError::MissingValueSlot(value))?;
    output.push(Instruction::Load64 {
        destination,
        base: Register::Rbp,
        displacement,
    });
    Ok(())
}

fn store_result(
    instruction: &IrInstruction,
    slots: &Slots,
    output: &mut Vec<Instruction>,
) -> Result<(), BackendError> {
    let result = instruction
        .result
        .ok_or(BackendError::MissingInstructionResult)?;
    let displacement = slots
        .values
        .get(&result)
        .copied()
        .ok_or(BackendError::MissingValueSlot(result))?;
    output.push(Instruction::Store64 {
        base: Register::Rbp,
        displacement,
        source: Register::Rax,
    });
    Ok(())
}

fn local_slot(slots: &Slots, symbol: SymbolId) -> Result<i32, BackendError> {
    slots
        .locals
        .get(&symbol)
        .copied()
        .ok_or(BackendError::MissingLocalSlot(symbol))
}

fn byte_offset(index: usize, function: &str) -> Result<u32, BackendError> {
    index
        .checked_mul(8)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| BackendError::FrameTooLarge(function.to_owned()))
}

fn usize_to_u64(value: usize, function: &str) -> Result<u64, BackendError> {
    u64::try_from(value).map_err(|_| BackendError::FrameTooLarge(function.to_owned()))
}

fn struct_word_count(
    structs: &HashMap<crate::frontend::types::TypeId, &IrStruct>,
    struct_id: crate::frontend::types::TypeId,
    function: &IrFunction,
) -> Result<usize, BackendError> {
    structs
        .get(&struct_id)
        .map(|definition| definition.fields.len().max(1))
        .ok_or_else(|| BackendError::UnsupportedIr {
            function: function.name.clone(),
            feature: "invalid verified struct layout",
        })
}

fn type_stride(
    ty: &Type,
    structs: &HashMap<crate::frontend::types::TypeId, &IrStruct>,
    function: &IrFunction,
) -> Result<usize, BackendError> {
    match ty {
        Type::Byte => Ok(1),
        Type::Struct(id) => struct_word_count(structs, *id, function)?
            .checked_mul(8)
            .ok_or_else(|| BackendError::FrameTooLarge(function.name.clone())),
        _ => Ok(8),
    }
}

fn emit_struct_allocation(
    output: &mut Vec<Instruction>,
    structs: &HashMap<crate::frontend::types::TypeId, &IrStruct>,
    struct_id: crate::frontend::types::TypeId,
    function: &IrFunction,
) -> Result<(), BackendError> {
    let bytes = struct_word_count(structs, struct_id, function)?
        .checked_mul(8)
        .ok_or_else(|| BackendError::FrameTooLarge(function.name.clone()))?;
    output.push(Instruction::MoveImmediate64 {
        destination: Register::Rdi,
        value: usize_to_u64(bytes, &function.name)?,
    });
    output.push(Instruction::Call(runtime::ALLOC));
    Ok(())
}

fn emit_copy_words(
    output: &mut Vec<Instruction>,
    source: Register,
    destination: Register,
    words: usize,
) -> Result<(), BackendError> {
    for index in 0..words {
        let displacement = i32::try_from(
            index
                .checked_mul(8)
                .ok_or_else(|| BackendError::FrameTooLarge("aggregate copy".into()))?,
        )
        .map_err(|_| BackendError::FrameTooLarge("aggregate copy".into()))?;
        output.push(Instruction::Load64 {
            destination: Register::Rax,
            base: source,
            displacement,
        });
        output.push(Instruction::Store64 {
            base: destination,
            displacement,
            source: Register::Rax,
        });
    }
    Ok(())
}

fn field_offset(
    structs: &HashMap<crate::frontend::types::TypeId, &IrStruct>,
    struct_id: crate::frontend::types::TypeId,
    field: u32,
    function: &IrFunction,
) -> Result<u32, BackendError> {
    structs
        .get(&struct_id)
        .and_then(|item| item.fields.get(field as usize))
        .map(|field| field.offset)
        .ok_or_else(|| BackendError::UnsupportedIr {
            function: function.name.clone(),
            feature: "invalid verified struct layout",
        })
}

fn local_field_slot(
    slots: &Slots,
    symbol: SymbolId,
    byte_offset: u32,
) -> Result<i32, BackendError> {
    let base = local_slot(slots, symbol)?;
    let offset = i32::try_from(byte_offset)
        .ok()
        .ok_or(BackendError::FrameTooLarge(format!("local {symbol:?}")))?;
    base.checked_add(offset)
        .ok_or(BackendError::FrameTooLarge(format!("local {symbol:?}")))
}

fn emit_epilogue(output: &mut Vec<Instruction>) {
    output.extend_from_slice(&[
        Instruction::MoveRegister {
            destination: Register::Rsp,
            source: Register::Rbp,
        },
        Instruction::Pop(Register::Rbp),
        Instruction::Return,
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::{lexer::lex, parser::parse, semantic::analyze, source::FileId};
    use crate::middle::lower as ir_lower;

    fn lower_source(source: &str) -> MachineModule {
        let ast = parse(lex(FileId(0), source).unwrap()).unwrap();
        let hir = analyze(&ast).unwrap();
        let ir = ir_lower::lower(&hir);
        let verified = crate::middle::verify::verify_module(&ir).unwrap();
        lower(&verified).unwrap()
    }

    #[test]
    fn lowers_literals_arithmetic_and_division() {
        let module = lower_source(
            "private int math() { return (8 + 4 - 2) * 3 / -2; } public void main() {}",
        );
        let instructions = &module.functions[0].instructions;
        assert!(
            instructions
                .iter()
                .any(|item| matches!(item, Instruction::Add { .. }))
        );
        assert!(
            instructions
                .iter()
                .any(|item| matches!(item, Instruction::Subtract { .. }))
        );
        assert!(
            instructions
                .iter()
                .any(|item| matches!(item, Instruction::MultiplySigned { .. }))
        );
        assert!(instructions.iter().any(
            |item| matches!(item, Instruction::Call(symbol) if *symbol == runtime::INTEGER_DIVIDE)
        ));
        assert!(
            instructions
                .iter()
                .any(|item| matches!(item, Instruction::Negate(_)))
        );
    }

    #[test]
    fn lowers_locals_arguments_calls_and_return_values() {
        let module = lower_source(
            "public int add(int x, int y) { return x + y; } public void main() { int result = add(2, 3); }",
        );
        let add = &module.functions[0].instructions;
        assert!(add.iter().any(|item| matches!(
            item,
            Instruction::Store64 {
                source: Register::Rdi,
                ..
            }
        )));
        assert!(add.iter().any(|item| matches!(
            item,
            Instruction::Store64 {
                source: Register::Rsi,
                ..
            }
        )));
        let main = &module.functions[1].instructions;
        assert!(main.iter().any(|item| matches!(item, Instruction::Call(_))));
        assert!(
            main.iter()
                .any(|item| matches!(item, Instruction::Store64 { .. }))
        );
        assert!(add.iter().any(|item| matches!(
            item,
            Instruction::Load64 {
                destination: Register::Rax,
                ..
            }
        )));
    }

    #[test]
    fn passes_additional_sysv_arguments_on_stack() {
        let module = lower_source(
            "public int seventh(int a, int b, int c, int d, int e, int f, int g) { return g; } public void main() { int result = seventh(1, 2, 3, 4, 5, 6, 7); }",
        );
        let callee = &module.functions[0].instructions;
        assert!(callee.iter().any(|item| matches!(
            item,
            Instruction::Load64 {
                base: Register::Rbp,
                displacement: 16,
                ..
            }
        )));
        let caller = &module.functions[1].instructions;
        assert!(
            caller
                .iter()
                .any(|item| matches!(item, Instruction::Push(Register::Rax)))
        );
        assert!(
            caller
                .iter()
                .any(|item| matches!(item, Instruction::StackDeallocate(16)))
        );
    }

    #[test]
    fn lowers_comparisons_boole_and_conditional_branches() {
        let module = lower_source(
            "public bool less(int x, int y) { return x < y; } public void main() { bool value = less(1, 2); if (!value) {} }",
        );
        let all = module
            .functions
            .iter()
            .flat_map(|function| &function.instructions)
            .collect::<Vec<_>>();
        assert!(
            all.iter()
                .any(|item| matches!(item, Instruction::Compare { .. }))
        );
        assert!(all.iter().any(|item| matches!(
            item,
            Instruction::MaterializeCondition(Condition::Less | Condition::Equal)
        )));
        assert!(
            all.iter()
                .any(|item| matches!(item, Instruction::JumpIfZero(_)))
        );
    }

    #[test]
    fn allocates_structs_with_reusable_heap_layout() {
        let module = lower_source(
            "struct Pair { int first; int second; } public void main() { Pair pair = Pair { first: 1, second: 2 }; }",
        );
        let stores: Vec<_> = module.functions[0]
            .instructions
            .iter()
            .filter_map(|instruction| match instruction {
                Instruction::Store64 { displacement, .. } => Some(*displacement),
                _ => None,
            })
            .collect();
        assert!(
            module.functions[0]
                .instructions
                .iter()
                .any(|item| matches!(item, Instruction::Call(symbol) if *symbol == runtime::ALLOC))
        );
        assert!(stores.contains(&0));
        assert!(stores.contains(&8));
    }

    #[test]
    fn lowers_struct_field_load_and_store() {
        let module = lower_source(
            "struct Item { int value; } public int test() { Item item = Item { value: 1 }; item.value = 2; return item.value; } public void main() {}",
        );
        let instructions = &module.functions[0].instructions;
        assert!(instructions.iter().any(|item| matches!(
            item,
            Instruction::Store64 {
                displacement: 0,
                ..
            }
        )));
        assert!(instructions.iter().any(|item| matches!(
            item,
            Instruction::Load64 {
                displacement: 0,
                ..
            }
        )));
    }

    #[test]
    fn lowers_enum_constant_to_declaration_order_discriminant() {
        let module = lower_source(
            "enum State { idle, running } public void main() { State state = State.running; }",
        );
        assert!(
            module.functions[0]
                .instructions
                .iter()
                .any(|item| matches!(item, Instruction::MoveImmediate64 { value: 1, .. }))
        );
    }

    #[test]
    fn reuses_integer_comparisons_for_enum_equality_and_inequality() {
        let module = lower_source(
            "enum State { idle, running } public void main() { bool same = State.idle == State.running; bool different = State.idle != State.running; }",
        );
        let instructions = &module.functions[0].instructions;
        assert!(
            instructions
                .iter()
                .any(|item| matches!(item, Instruction::MaterializeCondition(Condition::Equal)))
        );
        assert!(
            instructions
                .iter()
                .any(|item| matches!(item, Instruction::MaterializeCondition(Condition::NotEqual)))
        );
    }

    #[test]
    fn lowers_collections_and_strings_through_runtime_helpers() {
        let module = lower_source(
            "public void main() { int[2] a = [1, 2]; a[0] = a[1]; list int xs = [3]; xs.push(a[0]); int p = xs.pop(); string s = \"a\" + \"b\"; bool same = s == \"ab\"; }",
        );
        let main = &module.functions[0].instructions;
        assert!(
            main.iter()
                .any(|item| matches!(item, Instruction::IndexedLoad64 { .. }))
        );
        assert!(
            main.iter()
                .any(|item| matches!(item, Instruction::IndexedStore64 { .. }))
        );
        assert!(main.iter().any(
            |item| matches!(item, Instruction::Call(symbol) if *symbol == runtime::LIST_PUSH)
        ));
        assert!(
            main.iter().any(
                |item| matches!(item, Instruction::Call(symbol) if *symbol == runtime::LIST_POP)
            )
        );
        assert!(main.iter().any(
            |item| matches!(item, Instruction::Call(symbol) if *symbol == runtime::STRING_CONCAT)
        ));
        assert!(main.iter().any(
            |item| matches!(item, Instruction::Call(symbol) if *symbol == runtime::STRING_EQUAL)
        ));
        assert!(
            module
                .functions
                .iter()
                .any(|function| function.symbol == runtime::ALLOC)
        );
        assert!(
            module
                .functions
                .iter()
                .any(|function| function.symbol == runtime::BOUNDS_CHECK)
        );
        assert!(
            main.iter()
                .any(|item| matches!(item, Instruction::LoadDataAddress { .. }))
        );
        assert!(module.read_only_data.windows(2).any(|bytes| bytes == b"ab"));
    }

    #[test]
    fn lowers_source_ingestion_through_central_runtime_helpers() {
        let module = lower_source(
            "use std.fs; private int scan() { string source = readFile(\"fixture\"); return source.byte(0); } public void main(string[] args) { int count = args.length; string first = args[0]; }",
        );
        let scan = &module.functions[0].instructions;
        assert!(scan.iter().any(
            |item| matches!(item, Instruction::Call(symbol) if *symbol == runtime::READ_FILE)
        ));
        assert!(scan.iter().any(
            |item| matches!(item, Instruction::Call(symbol) if *symbol == runtime::STRING_BYTE)
        ));
        assert!(module.startup.iter().any(
            |item| matches!(item, Instruction::Call(symbol) if *symbol == runtime::MAIN_ARGS)
        ));
        assert!(
            module
                .functions
                .iter()
                .any(|function| function.symbol == runtime::READ_FILE)
        );
        assert!(
            module
                .functions
                .iter()
                .any(|function| function.symbol == runtime::STRING_BYTE)
        );
        assert!(
            module
                .functions
                .iter()
                .any(|function| function.symbol == runtime::MAIN_ARGS)
        );
    }

    #[test]
    fn includes_only_reachable_filesystem_runtime_features() {
        let unused = lower_source("use std.fs; public void main() {}");
        assert_eq!(unused.functions.len(), 1);

        let module =
            lower_source("use std.fs; public void main() { bool present = exists(\"data.txt\"); }");
        let symbols: HashSet<_> = module
            .functions
            .iter()
            .map(|function| function.symbol)
            .collect();
        assert!(symbols.contains(&runtime::EXISTS));
        assert!(symbols.contains(&runtime::ALLOC));
        assert!(symbols.contains(&runtime::FS_ERROR));
        assert!(symbols.contains(&runtime::WRITE_ALL));
        assert!(!symbols.contains(&runtime::READ_FILE));
        assert!(!symbols.contains(&runtime::WRITE_FILE));
        assert!(!symbols.contains(&runtime::READ_BYTES));
        assert!(!symbols.contains(&runtime::WRITE_BYTES));
    }

    #[test]
    fn rejects_struct_function_abi() {
        let source = "struct Item { int value; } private void consume(Item item) {} public void main() { Item item = Item { value: 1 }; consume(item); }";
        let ast = parse(lex(FileId(0), source).unwrap()).unwrap();
        let hir = analyze(&ast).unwrap();
        let ir = ir_lower::lower(&hir);
        let verified = crate::middle::verify::verify_module(&ir).unwrap();
        assert!(matches!(
            lower(&verified),
            Err(BackendError::UnsupportedType {
                ty: Type::Struct(_),
                ..
            })
        ));
    }

    #[test]
    fn lowers_whole_struct_copy() {
        let source = "struct Item { int value; } public void main() { Item first = Item { value: 1 }; Item second = first; }";
        let ast = parse(lex(FileId(0), source).unwrap()).unwrap();
        let hir = analyze(&ast).unwrap();
        let ir = ir_lower::lower(&hir);
        let verified = crate::middle::verify::verify_module(&ir).unwrap();
        let module = lower(&verified).unwrap();
        assert!(
            module.functions[0]
                .instructions
                .iter()
                .any(|item| matches!(item, Instruction::Call(symbol) if *symbol == runtime::ALLOC))
        );
    }
}
