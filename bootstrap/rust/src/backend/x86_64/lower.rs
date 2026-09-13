use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::frontend::ast::{BinaryOperator, UnaryOperator, Visibility};
use crate::frontend::resolution::SymbolId;
use crate::frontend::types::Type;
use crate::middle::ir::{
    IrConstant, IrFunction, IrInstruction, IrInstructionKind, IrModule, IrTerminator, ValueId,
};

use super::instruction::{Condition, Instruction, MachineFunction, MachineModule};
use super::register::Register;
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
pub fn lower(module: &IrModule) -> Result<MachineModule, BackendError> {
    let main = module
        .functions
        .iter()
        .find(|function| function.name == "main")
        .ok_or(BackendError::MissingMain)?;
    if main.visibility != Visibility::Public
        || main.return_type != Type::Void
        || !main.parameters.is_empty()
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

    let functions = module
        .functions
        .iter()
        .map(lower_function)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(MachineModule {
        startup: startup::generate(main.symbol),
        entry_function: main.symbol,
        functions,
    })
}

fn validate_signature(function: &IrFunction) -> Result<(), BackendError> {
    if !matches!(function.return_type, Type::Int | Type::Bool | Type::Void) {
        return Err(BackendError::UnsupportedType {
            function: function.name.clone(),
            ty: function.return_type.clone(),
        });
    }
    for local in function.parameters.iter().chain(&function.locals) {
        if !matches!(local.ty, Type::Int | Type::Bool) {
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

fn lower_function(function: &IrFunction) -> Result<MachineFunction, BackendError> {
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
            lower_instruction(function, instruction, &slots, &mut instructions)?;
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
            count += 1;
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
        IrInstructionKind::Constant(IrConstant::Bool(value)) => {
            output.push(Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: u64::from(*value),
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
                    output.push(Instruction::SignExtendRaxIntoRdx);
                    Instruction::DivideSigned(Register::Rcx)
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
        lower(&ir_lower::lower(&hir)).unwrap()
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
        assert!(
            instructions
                .iter()
                .any(|item| matches!(item, Instruction::DivideSigned(_)))
        );
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
}
