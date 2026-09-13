use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

use crate::frontend::ast::{BinaryOperator, UnaryOperator};
use crate::frontend::resolution::SymbolId;
use crate::frontend::types::Type;

use super::ir::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IrVerificationError {
    pub function: Option<String>,
    pub block: Option<BlockId>,
    pub kind: IrVerificationErrorKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IrVerificationErrorKind {
    DuplicateFunction(SymbolId),
    MissingEntry(BlockId),
    DuplicateBlock(BlockId),
    MissingTerminator,
    InvalidTarget(BlockId),
    UnreachableBlock,
    DuplicateValue(ValueId),
    UnknownValue(ValueId),
    ValueNotDefined(ValueId),
    MissingValueResult,
    MissingValueType(ValueId),
    UnexpectedValueResult,
    DuplicateLocal(SymbolId),
    UnknownLocal(SymbolId),
    TypeMismatch {
        context: &'static str,
        expected: Type,
        found: Type,
    },
    InvalidOperation(&'static str),
    UnknownFunction(SymbolId),
    InvalidCallArity {
        function: SymbolId,
        expected: usize,
        found: usize,
    },
    InvalidReturn {
        expected: Type,
        found: Option<Type>,
    },
    InvalidBranchCondition(Type),
}

impl fmt::Display for IrVerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("internal compiler error: invalid IR")?;
        if let Some(function) = &self.function {
            write!(formatter, " in function `{function}`")?;
        }
        if let Some(block) = self.block {
            write!(formatter, " at {block:?}")?;
        }
        write!(formatter, ": {}", self.kind)
    }
}

impl fmt::Display for IrVerificationErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateFunction(symbol) => write!(formatter, "duplicate function {symbol:?}"),
            Self::MissingEntry(block) => write!(formatter, "entry block {block:?} does not exist"),
            Self::DuplicateBlock(block) => write!(formatter, "duplicate block {block:?}"),
            Self::MissingTerminator => formatter.write_str("basic block has no terminator"),
            Self::InvalidTarget(block) => {
                write!(formatter, "branch target {block:?} does not exist")
            }
            Self::UnreachableBlock => formatter.write_str("basic block is unreachable"),
            Self::DuplicateValue(value) => {
                write!(formatter, "value {value:?} has conflicting definitions")
            }
            Self::UnknownValue(value) => write!(formatter, "value {value:?} has no definition"),
            Self::ValueNotDefined(value) => write!(
                formatter,
                "value {value:?} is not defined on every incoming path"
            ),
            Self::MissingValueResult => {
                formatter.write_str("value-producing instruction has no result")
            }
            Self::MissingValueType(value) => {
                write!(formatter, "value {value:?} has no result type")
            }
            Self::UnexpectedValueResult => {
                formatter.write_str("non-value instruction declares a result")
            }
            Self::DuplicateLocal(symbol) => write!(formatter, "duplicate local {symbol:?}"),
            Self::UnknownLocal(symbol) => write!(formatter, "unknown local {symbol:?}"),
            Self::TypeMismatch {
                context,
                expected,
                found,
            } => {
                write!(
                    formatter,
                    "{context} expected `{expected}`, found `{found}`"
                )
            }
            Self::InvalidOperation(operation) => {
                write!(formatter, "invalid IR operation: {operation}")
            }
            Self::UnknownFunction(symbol) => {
                write!(formatter, "call targets unknown function {symbol:?}")
            }
            Self::InvalidCallArity {
                function,
                expected,
                found,
            } => {
                write!(
                    formatter,
                    "call to {function:?} expects {expected} arguments, found {found}"
                )
            }
            Self::InvalidReturn { expected, found } => match found {
                Some(found) => write!(formatter, "return expected `{expected}`, found `{found}`"),
                None => write!(formatter, "return expected `{expected}`, found no value"),
            },
            Self::InvalidBranchCondition(found) => {
                write!(
                    formatter,
                    "branch condition must be `bool`, found `{found}`"
                )
            }
        }
    }
}

#[derive(Clone)]
struct Signature {
    parameters: Vec<Type>,
    return_type: Type,
}

/// Verifies all functions and collects independent IR invariant violations.
/// Unreachable blocks are hard failures because lowering never intentionally
/// creates them and silently accepting them can hide broken CFG construction.
pub fn verify_module(module: &IrModule) -> Result<(), Vec<IrVerificationError>> {
    let mut errors = Vec::new();
    let mut signatures = HashMap::new();
    for function in &module.functions {
        let signature = Signature {
            parameters: function
                .parameters
                .iter()
                .map(|local| local.ty.clone())
                .collect(),
            return_type: function.return_type.clone(),
        };
        if signatures.insert(function.symbol, signature).is_some() {
            errors.push(IrVerificationError {
                function: Some(function.name.clone()),
                block: None,
                kind: IrVerificationErrorKind::DuplicateFunction(function.symbol),
            });
        }
    }
    for function in &module.functions {
        verify_function(function, &signatures, &mut errors);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn verify_function(
    function: &IrFunction,
    signatures: &HashMap<SymbolId, Signature>,
    errors: &mut Vec<IrVerificationError>,
) {
    let mut blocks = HashMap::new();
    for block in &function.blocks {
        if blocks.insert(block.id, block).is_some() {
            push_error(
                errors,
                function,
                Some(block.id),
                IrVerificationErrorKind::DuplicateBlock(block.id),
            );
        }
        if block.terminator.is_none() {
            push_error(
                errors,
                function,
                Some(block.id),
                IrVerificationErrorKind::MissingTerminator,
            );
        }
    }
    if !blocks.contains_key(&function.entry) {
        push_error(
            errors,
            function,
            None,
            IrVerificationErrorKind::MissingEntry(function.entry),
        );
    }

    let mut successors: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    for block in &function.blocks {
        let targets = block
            .terminator
            .as_ref()
            .map(terminator_targets)
            .unwrap_or_default();
        for target in &targets {
            if !blocks.contains_key(target) {
                push_error(
                    errors,
                    function,
                    Some(block.id),
                    IrVerificationErrorKind::InvalidTarget(*target),
                );
            }
        }
        successors.insert(block.id, targets);
    }

    let reachable = reachable_blocks(function.entry, &successors, &blocks);
    for block in &function.blocks {
        if !reachable.contains(&block.id) {
            push_error(
                errors,
                function,
                Some(block.id),
                IrVerificationErrorKind::UnreachableBlock,
            );
        }
    }

    let locals = collect_locals(function, errors);
    let (value_types, definitions) = collect_definitions(function, &successors, errors);
    validate_instructions(function, signatures, &locals, &value_types, errors);
    validate_definite_values(function, &reachable, &successors, &definitions, errors);
}

fn collect_locals(
    function: &IrFunction,
    errors: &mut Vec<IrVerificationError>,
) -> HashMap<SymbolId, Type> {
    let mut locals = HashMap::new();
    for local in function.parameters.iter().chain(&function.locals) {
        if locals.insert(local.symbol, local.ty.clone()).is_some() {
            push_error(
                errors,
                function,
                None,
                IrVerificationErrorKind::DuplicateLocal(local.symbol),
            );
        }
    }
    locals
}

fn collect_definitions(
    function: &IrFunction,
    successors: &HashMap<BlockId, Vec<BlockId>>,
    errors: &mut Vec<IrVerificationError>,
) -> (HashMap<ValueId, Type>, HashMap<BlockId, HashSet<ValueId>>) {
    let mut value_types = HashMap::new();
    let mut definition_blocks: HashMap<ValueId, Vec<BlockId>> = HashMap::new();
    let mut definitions: HashMap<BlockId, HashSet<ValueId>> = HashMap::new();
    for block in &function.blocks {
        let block_definitions = definitions.entry(block.id).or_default();
        for instruction in &block.instructions {
            let produces_value = !matches!(instruction.kind, IrInstructionKind::BindLocal { .. });
            match (
                instruction.result,
                instruction.result_type.as_ref(),
                produces_value,
            ) {
                (Some(value), Some(ty), true) => {
                    if let Some(previous) = value_types.insert(value, ty.clone())
                        && previous != *ty
                    {
                        push_error(
                            errors,
                            function,
                            Some(block.id),
                            IrVerificationErrorKind::TypeMismatch {
                                context: "multiple value definitions",
                                expected: previous,
                                found: ty.clone(),
                            },
                        );
                    }
                    for previous_block in definition_blocks.entry(value).or_default().iter() {
                        if *previous_block == block.id
                            || !share_single_successor(*previous_block, block.id, successors)
                        {
                            push_error(
                                errors,
                                function,
                                Some(block.id),
                                IrVerificationErrorKind::DuplicateValue(value),
                            );
                        }
                    }
                    definition_blocks.entry(value).or_default().push(block.id);
                    block_definitions.insert(value);
                }
                (Some(value), None, true) => push_error(
                    errors,
                    function,
                    Some(block.id),
                    IrVerificationErrorKind::MissingValueType(value),
                ),
                (None, _, true) => push_error(
                    errors,
                    function,
                    Some(block.id),
                    IrVerificationErrorKind::MissingValueResult,
                ),
                (Some(_), _, false) | (None, Some(_), false) => push_error(
                    errors,
                    function,
                    Some(block.id),
                    IrVerificationErrorKind::UnexpectedValueResult,
                ),
                (None, None, false) => {}
            }
        }
    }
    (value_types, definitions)
}

fn validate_instructions(
    function: &IrFunction,
    signatures: &HashMap<SymbolId, Signature>,
    locals: &HashMap<SymbolId, Type>,
    values: &HashMap<ValueId, Type>,
    errors: &mut Vec<IrVerificationError>,
) {
    for block in &function.blocks {
        for instruction in &block.instructions {
            validate_instruction(
                function,
                block.id,
                instruction,
                signatures,
                locals,
                values,
                errors,
            );
        }
        if let Some(terminator) = &block.terminator {
            match terminator {
                IrTerminator::Return(value) => {
                    let found = value.and_then(|value| values.get(&value).cloned());
                    if (*value).is_some_and(|value| !values.contains_key(&value)) {
                        push_error(
                            errors,
                            function,
                            Some(block.id),
                            IrVerificationErrorKind::UnknownValue(value.unwrap()),
                        );
                    }
                    let valid = match (&function.return_type, &found) {
                        (Type::Void, None) => true,
                        (Type::Void, Some(_)) => false,
                        (_, Some(found)) => *found == function.return_type,
                        (_, None) => false,
                    };
                    if !valid {
                        push_error(
                            errors,
                            function,
                            Some(block.id),
                            IrVerificationErrorKind::InvalidReturn {
                                expected: function.return_type.clone(),
                                found,
                            },
                        );
                    }
                }
                IrTerminator::Branch { condition, .. } => match values.get(condition) {
                    Some(Type::Bool) => {}
                    Some(found) => push_error(
                        errors,
                        function,
                        Some(block.id),
                        IrVerificationErrorKind::InvalidBranchCondition(found.clone()),
                    ),
                    None => push_error(
                        errors,
                        function,
                        Some(block.id),
                        IrVerificationErrorKind::UnknownValue(*condition),
                    ),
                },
                IrTerminator::Jump(_) => {}
            }
        }
    }
}

fn validate_instruction(
    function: &IrFunction,
    block: BlockId,
    instruction: &IrInstruction,
    signatures: &HashMap<SymbolId, Signature>,
    locals: &HashMap<SymbolId, Type>,
    values: &HashMap<ValueId, Type>,
    errors: &mut Vec<IrVerificationError>,
) {
    let result_type = instruction.result.and_then(|value| values.get(&value));
    match &instruction.kind {
        IrInstructionKind::Constant(constant) => {
            let expected = constant_type(constant);
            check_result_type(
                function,
                block,
                result_type,
                &expected,
                "constant result",
                errors,
            );
        }
        IrInstructionKind::LoadLocal(local) => match locals.get(local) {
            Some(ty) => check_result_type(function, block, result_type, ty, "local load", errors),
            None => push_error(
                errors,
                function,
                Some(block),
                IrVerificationErrorKind::UnknownLocal(*local),
            ),
        },
        IrInstructionKind::BindLocal { local, value } => match locals.get(local) {
            Some(local_type) => check_value_type(
                function,
                block,
                *value,
                local_type,
                "local store",
                values,
                errors,
            ),
            None => push_error(
                errors,
                function,
                Some(block),
                IrVerificationErrorKind::UnknownLocal(*local),
            ),
        },
        IrInstructionKind::Copy(value) => {
            if let Some(ty) = values.get(value) {
                check_result_type(function, block, result_type, ty, "copy result", errors);
            } else {
                push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::UnknownValue(*value),
                );
            }
        }
        IrInstructionKind::Call {
            function: callee,
            arguments,
        } => {
            if let Some(signature) = signatures.get(callee) {
                if arguments.len() != signature.parameters.len() {
                    push_error(
                        errors,
                        function,
                        Some(block),
                        IrVerificationErrorKind::InvalidCallArity {
                            function: *callee,
                            expected: signature.parameters.len(),
                            found: arguments.len(),
                        },
                    );
                }
                for (argument, expected) in arguments.iter().zip(&signature.parameters) {
                    check_value_type(
                        function,
                        block,
                        *argument,
                        expected,
                        "call argument",
                        values,
                        errors,
                    );
                }
                check_result_type(
                    function,
                    block,
                    result_type,
                    &signature.return_type,
                    "call result",
                    errors,
                );
            } else {
                push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::UnknownFunction(*callee),
                );
            }
        }
        IrInstructionKind::Unary { operator, operand } => {
            let required = match operator {
                UnaryOperator::Negate => Type::Int,
                UnaryOperator::Not => Type::Bool,
            };
            check_value_type(
                function,
                block,
                *operand,
                &required,
                "unary operand",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                &required,
                "unary result",
                errors,
            );
        }
        IrInstructionKind::Binary {
            operator,
            left,
            right,
        } => {
            validate_binary(
                function,
                block,
                *operator,
                (*left, *right),
                result_type,
                values,
                errors,
            );
        }
        IrInstructionKind::Aggregate(items) => {
            validate_aggregate(function, block, items, result_type, values, errors)
        }
    }
}

fn validate_binary(
    function: &IrFunction,
    block: BlockId,
    operator: BinaryOperator,
    operands: (ValueId, ValueId),
    result: Option<&Type>,
    values: &HashMap<ValueId, Type>,
    errors: &mut Vec<IrVerificationError>,
) {
    let (left, right) = operands;
    match operator {
        BinaryOperator::Add
        | BinaryOperator::Subtract
        | BinaryOperator::Multiply
        | BinaryOperator::Divide => {
            check_value_type(
                function,
                block,
                left,
                &Type::Int,
                "arithmetic operand",
                values,
                errors,
            );
            check_value_type(
                function,
                block,
                right,
                &Type::Int,
                "arithmetic operand",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result,
                &Type::Int,
                "arithmetic result",
                errors,
            );
        }
        BinaryOperator::Less
        | BinaryOperator::LessEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual => {
            check_value_type(
                function,
                block,
                left,
                &Type::Int,
                "comparison operand",
                values,
                errors,
            );
            check_value_type(
                function,
                block,
                right,
                &Type::Int,
                "comparison operand",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result,
                &Type::Bool,
                "comparison result",
                errors,
            );
        }
        BinaryOperator::Equal | BinaryOperator::NotEqual => {
            let left_type = values.get(&left);
            let right_type = values.get(&right);
            if let (Some(left_type), Some(right_type)) = (left_type, right_type) {
                if left_type != right_type || !matches!(left_type, Type::Int | Type::Bool) {
                    push_error(
                        errors,
                        function,
                        Some(block),
                        IrVerificationErrorKind::InvalidOperation(
                            "equality operands must be matching int or bool values",
                        ),
                    );
                }
            } else {
                if left_type.is_none() {
                    push_error(
                        errors,
                        function,
                        Some(block),
                        IrVerificationErrorKind::UnknownValue(left),
                    );
                }
                if right_type.is_none() {
                    push_error(
                        errors,
                        function,
                        Some(block),
                        IrVerificationErrorKind::UnknownValue(right),
                    );
                }
            }
            check_result_type(
                function,
                block,
                result,
                &Type::Bool,
                "equality result",
                errors,
            );
        }
        BinaryOperator::LogicalAnd | BinaryOperator::LogicalOr => push_error(
            errors,
            function,
            Some(block),
            IrVerificationErrorKind::InvalidOperation(
                "logical operators must be lowered to short-circuit CFG branches",
            ),
        ),
    }
}

fn validate_aggregate(
    function: &IrFunction,
    block: BlockId,
    items: &[ValueId],
    result: Option<&Type>,
    values: &HashMap<ValueId, Type>,
    errors: &mut Vec<IrVerificationError>,
) {
    match result {
        Some(Type::Array { element, length }) => {
            if items.len() != *length {
                push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::InvalidOperation(
                        "array aggregate length does not match its type",
                    ),
                );
            }
            for item in items {
                check_value_type(
                    function,
                    block,
                    *item,
                    element,
                    "array element",
                    values,
                    errors,
                );
            }
        }
        Some(Type::List(element)) => {
            for item in items {
                check_value_type(
                    function,
                    block,
                    *item,
                    element,
                    "list element",
                    values,
                    errors,
                );
            }
        }
        Some(Type::Tuple(elements)) => {
            if items.len() != elements.len() {
                push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::InvalidOperation(
                        "tuple aggregate length does not match its type",
                    ),
                );
            }
            for (item, expected) in items.iter().zip(elements) {
                check_value_type(
                    function,
                    block,
                    *item,
                    expected,
                    "tuple element",
                    values,
                    errors,
                );
            }
        }
        Some(_) | None => push_error(
            errors,
            function,
            Some(block),
            IrVerificationErrorKind::InvalidOperation(
                "aggregate result must have an aggregate type",
            ),
        ),
    }
}

fn validate_definite_values(
    function: &IrFunction,
    reachable: &HashSet<BlockId>,
    successors: &HashMap<BlockId, Vec<BlockId>>,
    definitions: &HashMap<BlockId, HashSet<ValueId>>,
    errors: &mut Vec<IrVerificationError>,
) {
    let universe: HashSet<_> = definitions.values().flatten().copied().collect();
    let mut predecessors: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    for (from, targets) in successors {
        for target in targets {
            if reachable.contains(from) && reachable.contains(target) {
                predecessors.entry(*target).or_default().push(*from);
            }
        }
    }
    let mut incoming: HashMap<BlockId, HashSet<ValueId>> = reachable
        .iter()
        .map(|block| {
            (
                *block,
                if *block == function.entry {
                    HashSet::new()
                } else {
                    universe.clone()
                },
            )
        })
        .collect();
    let mut outgoing = incoming.clone();
    loop {
        let mut changed = false;
        for block in reachable {
            let new_in = if *block == function.entry {
                HashSet::new()
            } else if let Some(preds) = predecessors.get(block) {
                intersection(preds.iter().filter_map(|pred| outgoing.get(pred)))
            } else {
                HashSet::new()
            };
            let mut new_out = new_in.clone();
            new_out.extend(definitions.get(block).into_iter().flatten().copied());
            if incoming.get(block) != Some(&new_in) || outgoing.get(block) != Some(&new_out) {
                incoming.insert(*block, new_in);
                outgoing.insert(*block, new_out);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    for block in &function.blocks {
        if !reachable.contains(&block.id) {
            continue;
        }
        let mut available = incoming.get(&block.id).cloned().unwrap_or_default();
        for instruction in &block.instructions {
            for value in instruction_uses(&instruction.kind) {
                if !available.contains(&value) {
                    push_error(
                        errors,
                        function,
                        Some(block.id),
                        IrVerificationErrorKind::ValueNotDefined(value),
                    );
                }
            }
            if let Some(result) = instruction.result {
                available.insert(result);
            }
        }
        if let Some(terminator) = &block.terminator {
            for value in terminator_uses(terminator) {
                if !available.contains(&value) {
                    push_error(
                        errors,
                        function,
                        Some(block.id),
                        IrVerificationErrorKind::ValueNotDefined(value),
                    );
                }
            }
        }
    }
}

fn check_value_type(
    function: &IrFunction,
    block: BlockId,
    value: ValueId,
    expected: &Type,
    context: &'static str,
    values: &HashMap<ValueId, Type>,
    errors: &mut Vec<IrVerificationError>,
) {
    match values.get(&value) {
        Some(found) if found != expected => push_error(
            errors,
            function,
            Some(block),
            IrVerificationErrorKind::TypeMismatch {
                context,
                expected: expected.clone(),
                found: found.clone(),
            },
        ),
        Some(_) => {}
        None => push_error(
            errors,
            function,
            Some(block),
            IrVerificationErrorKind::UnknownValue(value),
        ),
    }
}

fn check_result_type(
    function: &IrFunction,
    block: BlockId,
    found: Option<&Type>,
    expected: &Type,
    context: &'static str,
    errors: &mut Vec<IrVerificationError>,
) {
    if let Some(found) = found
        && found != expected
    {
        push_error(
            errors,
            function,
            Some(block),
            IrVerificationErrorKind::TypeMismatch {
                context,
                expected: expected.clone(),
                found: found.clone(),
            },
        );
    }
}

fn constant_type(constant: &IrConstant) -> Type {
    match constant {
        IrConstant::Integer(_) => Type::Int,
        IrConstant::Float(_) => Type::Float,
        IrConstant::String(_) => Type::String,
        IrConstant::Char(_) => Type::Char,
        IrConstant::Bool(_) => Type::Bool,
    }
}

fn instruction_uses(kind: &IrInstructionKind) -> Vec<ValueId> {
    match kind {
        IrInstructionKind::Constant(_) | IrInstructionKind::LoadLocal(_) => Vec::new(),
        IrInstructionKind::BindLocal { value, .. } | IrInstructionKind::Copy(value) => vec![*value],
        IrInstructionKind::Call { arguments, .. } | IrInstructionKind::Aggregate(arguments) => {
            arguments.clone()
        }
        IrInstructionKind::Unary { operand, .. } => vec![*operand],
        IrInstructionKind::Binary { left, right, .. } => vec![*left, *right],
    }
}

fn terminator_uses(terminator: &IrTerminator) -> Vec<ValueId> {
    match terminator {
        IrTerminator::Return(Some(value)) => vec![*value],
        IrTerminator::Branch { condition, .. } => vec![*condition],
        IrTerminator::Return(None) | IrTerminator::Jump(_) => Vec::new(),
    }
}

fn terminator_targets(terminator: &IrTerminator) -> Vec<BlockId> {
    match terminator {
        IrTerminator::Jump(target) => vec![*target],
        IrTerminator::Branch {
            then_block,
            else_block,
            ..
        } => vec![*then_block, *else_block],
        IrTerminator::Return(_) => Vec::new(),
    }
}

fn reachable_blocks(
    entry: BlockId,
    successors: &HashMap<BlockId, Vec<BlockId>>,
    blocks: &HashMap<BlockId, &IrBlock>,
) -> HashSet<BlockId> {
    let mut reachable = HashSet::new();
    let mut queue = VecDeque::from([entry]);
    while let Some(block) = queue.pop_front() {
        if !blocks.contains_key(&block) || !reachable.insert(block) {
            continue;
        }
        queue.extend(successors.get(&block).into_iter().flatten().copied());
    }
    reachable
}

fn share_single_successor(
    left: BlockId,
    right: BlockId,
    successors: &HashMap<BlockId, Vec<BlockId>>,
) -> bool {
    matches!(
        (successors.get(&left), successors.get(&right)),
        (Some(left_targets), Some(right_targets))
            if left_targets.len() == 1 && left_targets == right_targets
    )
}

fn intersection<'a>(mut sets: impl Iterator<Item = &'a HashSet<ValueId>>) -> HashSet<ValueId> {
    let Some(first) = sets.next() else {
        return HashSet::new();
    };
    let mut result = first.clone();
    for set in sets {
        result.retain(|value| set.contains(value));
    }
    result
}

fn push_error(
    errors: &mut Vec<IrVerificationError>,
    function: &IrFunction,
    block: Option<BlockId>,
    kind: IrVerificationErrorKind,
) {
    errors.push(IrVerificationError {
        function: Some(function.name.clone()),
        block,
        kind,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::Visibility;
    use crate::frontend::source::{FileId, Span};
    use crate::frontend::{lexer::lex, parser::parse, semantic::analyze};

    fn span() -> Span {
        Span::empty(FileId(0), 0)
    }

    fn instruction(result: u32, ty: Type, kind: IrInstructionKind) -> IrInstruction {
        IrInstruction {
            result: Some(ValueId(result)),
            result_type: Some(ty),
            kind,
            span: span(),
        }
    }

    fn int_constant(result: u32, value: &str) -> IrInstruction {
        instruction(
            result,
            Type::Int,
            IrInstructionKind::Constant(IrConstant::Integer(value.to_owned())),
        )
    }

    fn bool_constant(result: u32, value: bool) -> IrInstruction {
        instruction(
            result,
            Type::Bool,
            IrInstructionKind::Constant(IrConstant::Bool(value)),
        )
    }

    fn function(return_type: Type, blocks: Vec<IrBlock>) -> IrFunction {
        IrFunction {
            symbol: SymbolId(0),
            name: "f".to_owned(),
            visibility: Visibility::Private,
            parameters: Vec::new(),
            return_type,
            locals: Vec::new(),
            entry: BlockId(0),
            blocks,
        }
    }

    fn int_module() -> IrModule {
        IrModule {
            functions: vec![function(
                Type::Int,
                vec![IrBlock {
                    id: BlockId(0),
                    instructions: vec![int_constant(0, "1")],
                    terminator: Some(IrTerminator::Return(Some(ValueId(0)))),
                }],
            )],
        }
    }

    fn assert_has_error(module: &IrModule, predicate: impl Fn(&IrVerificationErrorKind) -> bool) {
        let errors = verify_module(module).expect_err("malformed IR passed verification");
        assert!(
            errors.iter().any(|error| predicate(&error.kind)),
            "unexpected verifier errors: {errors:#?}"
        );
    }

    fn lower_source(source: &str) -> IrModule {
        let ast = parse(lex(FileId(0), source).unwrap()).unwrap();
        crate::middle::lower::lower(&analyze(&ast).unwrap())
    }

    #[test]
    fn accepts_valid_straight_line_ir() {
        assert_eq!(verify_module(&int_module()), Ok(()));
    }

    #[test]
    fn rejects_missing_entry_block() {
        let mut module = int_module();
        module.functions[0].entry = BlockId(9);
        assert_has_error(&module, |kind| {
            matches!(kind, IrVerificationErrorKind::MissingEntry(BlockId(9)))
        });
    }

    #[test]
    fn rejects_duplicate_block_id() {
        let mut module = int_module();
        module.functions[0].blocks.push(IrBlock {
            id: BlockId(0),
            instructions: Vec::new(),
            terminator: Some(IrTerminator::Return(None)),
        });
        assert_has_error(&module, |kind| {
            matches!(kind, IrVerificationErrorKind::DuplicateBlock(BlockId(0)))
        });
    }

    #[test]
    fn rejects_jump_to_missing_block() {
        let mut module = int_module();
        module.functions[0].blocks[0].terminator = Some(IrTerminator::Jump(BlockId(7)));
        assert_has_error(&module, |kind| {
            matches!(kind, IrVerificationErrorKind::InvalidTarget(BlockId(7)))
        });
    }

    #[test]
    fn rejects_branch_with_missing_then_target() {
        let mut module = branch_module();
        let Some(IrTerminator::Branch { then_block, .. }) =
            &mut module.functions[0].blocks[0].terminator
        else {
            unreachable!()
        };
        *then_block = BlockId(7);
        assert_has_error(&module, |kind| {
            matches!(kind, IrVerificationErrorKind::InvalidTarget(BlockId(7)))
        });
    }

    #[test]
    fn rejects_branch_with_missing_else_target() {
        let mut module = branch_module();
        let Some(IrTerminator::Branch { else_block, .. }) =
            &mut module.functions[0].blocks[0].terminator
        else {
            unreachable!()
        };
        *else_block = BlockId(8);
        assert_has_error(&module, |kind| {
            matches!(kind, IrVerificationErrorKind::InvalidTarget(BlockId(8)))
        });
    }

    #[test]
    fn rejects_block_without_terminator() {
        let mut module = int_module();
        module.functions[0].blocks[0].terminator = None;
        assert_has_error(&module, |kind| {
            matches!(kind, IrVerificationErrorKind::MissingTerminator)
        });
    }

    #[test]
    fn rejects_unknown_value_use() {
        let mut module = int_module();
        module.functions[0].blocks[0].instructions = vec![instruction(
            0,
            Type::Int,
            IrInstructionKind::Copy(ValueId(99)),
        )];
        assert_has_error(&module, |kind| {
            matches!(kind, IrVerificationErrorKind::UnknownValue(ValueId(99)))
        });
    }

    #[test]
    fn rejects_duplicate_value_definition_on_one_path() {
        let mut module = int_module();
        module.functions[0].blocks[0]
            .instructions
            .push(int_constant(0, "2"));
        assert_has_error(&module, |kind| {
            matches!(kind, IrVerificationErrorKind::DuplicateValue(ValueId(0)))
        });
    }

    #[test]
    fn rejects_unknown_local() {
        let mut module = int_module();
        module.functions[0].blocks[0].instructions = vec![instruction(
            0,
            Type::Int,
            IrInstructionKind::LoadLocal(SymbolId(44)),
        )];
        assert_has_error(&module, |kind| {
            matches!(kind, IrVerificationErrorKind::UnknownLocal(SymbolId(44)))
        });
    }

    #[test]
    fn rejects_non_bool_branch_condition() {
        let mut module = branch_module();
        module.functions[0].blocks[0].instructions[0] = int_constant(0, "1");
        assert_has_error(&module, |kind| {
            matches!(
                kind,
                IrVerificationErrorKind::InvalidBranchCondition(Type::Int)
            )
        });
    }

    #[test]
    fn rejects_call_to_unknown_function() {
        let mut module = int_module();
        module.functions[0].blocks[0].instructions = vec![instruction(
            0,
            Type::Int,
            IrInstructionKind::Call {
                function: SymbolId(88),
                arguments: Vec::new(),
            },
        )];
        assert_has_error(&module, |kind| {
            matches!(kind, IrVerificationErrorKind::UnknownFunction(SymbolId(88)))
        });
    }

    #[test]
    fn rejects_call_with_wrong_arity() {
        let mut module = int_module();
        module.functions[0].blocks[0].instructions = vec![instruction(
            0,
            Type::Int,
            IrInstructionKind::Call {
                function: SymbolId(1),
                arguments: Vec::new(),
            },
        )];
        let mut callee = function(
            Type::Int,
            vec![IrBlock {
                id: BlockId(0),
                instructions: vec![instruction(
                    1,
                    Type::Int,
                    IrInstructionKind::LoadLocal(SymbolId(2)),
                )],
                terminator: Some(IrTerminator::Return(Some(ValueId(1)))),
            }],
        );
        callee.symbol = SymbolId(1);
        callee.name = "callee".to_owned();
        callee.parameters.push(IrLocal {
            symbol: SymbolId(2),
            ty: Type::Int,
        });
        module.functions.push(callee);
        assert_has_error(&module, |kind| {
            matches!(
                kind,
                IrVerificationErrorKind::InvalidCallArity {
                    function: SymbolId(1),
                    expected: 1,
                    found: 0
                }
            )
        });
    }

    #[test]
    fn rejects_wrong_return_type() {
        let mut module = int_module();
        module.functions[0].return_type = Type::Bool;
        assert_has_error(&module, |kind| {
            matches!(
                kind,
                IrVerificationErrorKind::InvalidReturn {
                    expected: Type::Bool,
                    found: Some(Type::Int)
                }
            )
        });
    }

    #[test]
    fn rejects_value_returned_from_void_function() {
        let mut module = int_module();
        module.functions[0].return_type = Type::Void;
        assert_has_error(&module, |kind| {
            matches!(
                kind,
                IrVerificationErrorKind::InvalidReturn {
                    expected: Type::Void,
                    found: Some(Type::Int)
                }
            )
        });
    }

    #[test]
    fn rejects_missing_non_void_return_value() {
        let mut module = int_module();
        module.functions[0].blocks[0].terminator = Some(IrTerminator::Return(None));
        assert_has_error(&module, |kind| {
            matches!(
                kind,
                IrVerificationErrorKind::InvalidReturn {
                    expected: Type::Int,
                    found: None
                }
            )
        });
    }

    #[test]
    fn rejects_unreachable_block() {
        let mut module = int_module();
        module.functions[0].blocks.push(IrBlock {
            id: BlockId(1),
            instructions: vec![int_constant(1, "2")],
            terminator: Some(IrTerminator::Return(Some(ValueId(1)))),
        });
        assert_has_error(&module, |kind| {
            matches!(kind, IrVerificationErrorKind::UnreachableBlock)
        });
    }

    #[test]
    fn collects_multiple_structured_errors() {
        let mut module = int_module();
        module.functions[0].entry = BlockId(9);
        module.functions[0].blocks[0].terminator = None;
        let errors = verify_module(&module).unwrap_err();
        assert!(errors.len() >= 2);
        assert!(
            errors
                .iter()
                .all(|error| error.function.as_deref() == Some("f"))
        );
    }

    #[test]
    fn accepts_valid_loop_cfg() {
        let module = lower_source("private void f(bool run) { while (run) { run = false; } }");
        assert_eq!(verify_module(&module), Ok(()));
    }

    #[test]
    fn accepts_valid_if_else_cfg() {
        let module = lower_source(
            "private int f(bool choose) { if (choose) { return 1; } else { return 2; } }",
        );
        assert_eq!(verify_module(&module), Ok(()));
    }

    #[test]
    fn accepts_valid_short_circuit_cfg() {
        let module = lower_source("private bool f(bool a, bool b) { return a && b || a; }");
        assert_eq!(verify_module(&module), Ok(()));
    }

    #[test]
    fn accepts_short_circuit_cfg_inside_loop() {
        let module = lower_source(
            "private void f(bool run, bool a, bool b) { while (run) { bool value = a && b; run = false; } }",
        );
        assert_eq!(verify_module(&module), Ok(()));
    }

    fn branch_module() -> IrModule {
        IrModule {
            functions: vec![function(
                Type::Int,
                vec![
                    IrBlock {
                        id: BlockId(0),
                        instructions: vec![bool_constant(0, true)],
                        terminator: Some(IrTerminator::Branch {
                            condition: ValueId(0),
                            then_block: BlockId(1),
                            else_block: BlockId(2),
                        }),
                    },
                    IrBlock {
                        id: BlockId(1),
                        instructions: vec![int_constant(1, "1")],
                        terminator: Some(IrTerminator::Return(Some(ValueId(1)))),
                    },
                    IrBlock {
                        id: BlockId(2),
                        instructions: vec![int_constant(2, "2")],
                        terminator: Some(IrTerminator::Return(Some(ValueId(2)))),
                    },
                ],
            )],
        }
    }
}
