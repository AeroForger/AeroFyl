use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

use crate::frontend::ast::{BinaryOperator, UnaryOperator};
use crate::frontend::resolution::SymbolId;
use crate::frontend::types::Type;
use crate::frontend::types::TypeId;

use super::ir::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IrVerificationError {
    pub function: Option<String>,
    pub block: Option<BlockId>,
    pub kind: IrVerificationErrorKind,
}

#[derive(Clone, Copy, Debug)]
pub struct VerifiedIrModule<'module> {
    module: &'module IrModule,
}

impl<'module> VerifiedIrModule<'module> {
    pub(crate) const fn as_module(self) -> &'module IrModule {
        self.module
    }
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
    UnknownStruct(TypeId),
    InvalidField {
        struct_id: TypeId,
        field: u32,
    },
    UnknownEnum(TypeId),
    InvalidEnumVariant {
        enum_id: TypeId,
        variant: u32,
    },
    DuplicateType(TypeId),
    InvalidType(Type),
    InvalidFieldOffset {
        struct_id: TypeId,
        field: u32,
    },
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
            Self::UnknownStruct(id) => write!(formatter, "unknown struct type {id:?}"),
            Self::InvalidField { struct_id, field } => {
                write!(formatter, "unknown field {field} for struct {struct_id:?}")
            }
            Self::UnknownEnum(id) => write!(formatter, "unknown enum type {id:?}"),
            Self::InvalidEnumVariant { enum_id, variant } => {
                write!(formatter, "unknown variant {variant} for enum {enum_id:?}")
            }
            Self::DuplicateType(id) => write!(formatter, "duplicate IR type ID {id:?}"),
            Self::InvalidType(ty) => write!(formatter, "unknown or unresolved IR type `{ty}`"),
            Self::InvalidFieldOffset { struct_id, field } => write!(
                formatter,
                "field {field} of struct {struct_id:?} has an invalid bootstrap offset"
            ),
        }
    }
}

#[derive(Clone)]
struct Signature {
    parameters: Vec<Type>,
    return_type: Type,
}

#[derive(Clone, Copy)]
struct InstructionContext<'a> {
    function: &'a IrFunction,
    block: BlockId,
    signatures: &'a HashMap<SymbolId, Signature>,
    structs: &'a HashMap<TypeId, &'a IrStruct>,
    enums: &'a HashMap<TypeId, &'a IrEnum>,
    locals: &'a HashMap<SymbolId, Type>,
    values: &'a HashMap<ValueId, Type>,
}

/// Verifies all functions and collects independent IR invariant violations.
/// Unreachable blocks are hard failures because lowering never intentionally
/// creates them and silently accepting them can hide broken CFG construction.
pub fn verify_module(module: &IrModule) -> Result<VerifiedIrModule<'_>, Vec<IrVerificationError>> {
    let mut errors = Vec::new();
    let mut type_ids = HashSet::new();
    let mut structs = HashMap::new();
    for item in &module.structs {
        if !type_ids.insert(item.id) || structs.insert(item.id, item).is_some() {
            errors.push(module_error(IrVerificationErrorKind::DuplicateType(
                item.id,
            )));
        }
        let mut names = HashSet::new();
        for (field, metadata) in item.fields.iter().enumerate() {
            if !names.insert(&metadata.name) {
                errors.push(module_error(IrVerificationErrorKind::InvalidOperation(
                    "struct metadata contains duplicate field names",
                )));
            }
            if metadata.offset != field as u32 * 8 {
                errors.push(module_error(IrVerificationErrorKind::InvalidFieldOffset {
                    struct_id: item.id,
                    field: field as u32,
                }));
            }
        }
    }
    let mut enums = HashMap::new();
    for item in &module.enums {
        if !type_ids.insert(item.id) || enums.insert(item.id, item).is_some() {
            errors.push(module_error(IrVerificationErrorKind::DuplicateType(
                item.id,
            )));
        }
        let mut names = HashSet::new();
        for variant in &item.variants {
            if !names.insert(&variant.name) {
                errors.push(module_error(IrVerificationErrorKind::InvalidOperation(
                    "enum metadata contains duplicate variant names",
                )));
            }
        }
    }
    for item in &module.enums {
        for variant in &item.variants {
            if let Some(payload) = &variant.payload {
                validate_known_type(payload, &structs, &enums, None, &mut errors);
            }
        }
    }
    for item in &module.structs {
        for field in &item.fields {
            validate_known_type(&field.ty, &structs, &enums, None, &mut errors);
            if !matches!(
                field.ty,
                Type::Int
                    | Type::Byte
                    | Type::Bool
                    | Type::Char
                    | Type::String
                    | Type::Struct(_)
                    | Type::Enum(_)
                    | Type::Array { .. }
                    | Type::List(_)
                    | Type::Optional(_)
                    | Type::Ref(_)
            ) {
                errors.push(module_error(IrVerificationErrorKind::InvalidOperation(
                    "struct field has an unsupported bootstrap layout",
                )));
            }
        }
    }
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
        validate_known_type(
            &function.return_type,
            &structs,
            &enums,
            Some(&function.name),
            &mut errors,
        );
        for local in function.parameters.iter().chain(&function.locals) {
            validate_known_type(
                &local.ty,
                &structs,
                &enums,
                Some(&function.name),
                &mut errors,
            );
        }
        let cli_parameter_is_valid = function.name == "main"
            && matches!(
                function.parameters.as_slice(),
                [IrLocal {
                    ty: Type::CliArgs,
                    ..
                }]
            );
        if function.return_type == Type::CliArgs
            || function
                .locals
                .iter()
                .any(|local| local.ty == Type::CliArgs)
            || (function
                .parameters
                .iter()
                .any(|local| local.ty == Type::CliArgs)
                && !cli_parameter_is_valid)
            || function
                .blocks
                .iter()
                .flat_map(|block| &block.instructions)
                .any(|instruction| instruction.result_type == Some(Type::CliArgs))
        {
            push_error(
                &mut errors,
                function,
                None,
                IrVerificationErrorKind::InvalidOperation(
                    "string[] is valid only as the sole parameter of main",
                ),
            );
        }
        for instruction in function.blocks.iter().flat_map(|block| &block.instructions) {
            if let Some(ty) = &instruction.result_type {
                validate_known_type(ty, &structs, &enums, Some(&function.name), &mut errors);
            }
        }
        verify_function(function, &signatures, &structs, &enums, &mut errors);
    }
    if errors.is_empty() {
        Ok(VerifiedIrModule { module })
    } else {
        Err(errors)
    }
}

fn verify_function(
    function: &IrFunction,
    signatures: &HashMap<SymbolId, Signature>,
    structs: &HashMap<TypeId, &IrStruct>,
    enums: &HashMap<TypeId, &IrEnum>,
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
    validate_instructions(
        function,
        signatures,
        structs,
        enums,
        &locals,
        &value_types,
        errors,
    );
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
            let produces_value = !matches!(
                instruction.kind,
                IrInstructionKind::BindLocal { .. }
                    | IrInstructionKind::StructInit { .. }
                    | IrInstructionKind::FieldStore { .. }
                    | IrInstructionKind::ArrayInit { .. }
                    | IrInstructionKind::ArrayStore { .. }
                    | IrInstructionKind::ListInit { .. }
                    | IrInstructionKind::ListStore { .. }
                    | IrInstructionKind::ReferenceStore { .. }
            );
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
    structs: &HashMap<TypeId, &IrStruct>,
    enums: &HashMap<TypeId, &IrEnum>,
    locals: &HashMap<SymbolId, Type>,
    values: &HashMap<ValueId, Type>,
    errors: &mut Vec<IrVerificationError>,
) {
    for block in &function.blocks {
        for instruction in &block.instructions {
            validate_instruction(
                InstructionContext {
                    function,
                    block: block.id,
                    signatures,
                    structs,
                    enums,
                    locals,
                    values,
                },
                instruction,
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
                IrTerminator::Exit(code) => match values.get(code) {
                    Some(Type::Int) => {}
                    Some(found) => push_error(
                        errors,
                        function,
                        Some(block.id),
                        IrVerificationErrorKind::TypeMismatch {
                            context: "exit code",
                            expected: Type::Int,
                            found: found.clone(),
                        },
                    ),
                    None => push_error(
                        errors,
                        function,
                        Some(block.id),
                        IrVerificationErrorKind::UnknownValue(*code),
                    ),
                },
                IrTerminator::Jump(_) => {}
            }
        }
    }
}

fn validate_instruction(
    context: InstructionContext<'_>,
    instruction: &IrInstruction,
    errors: &mut Vec<IrVerificationError>,
) {
    let InstructionContext {
        function,
        block,
        signatures,
        structs,
        enums,
        locals,
        values,
    } = context;
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
        IrInstructionKind::StructInit {
            local,
            struct_id,
            fields,
        } => validate_struct_operation(&context, Some(*local), *struct_id, fields, None, errors),
        IrInstructionKind::StructValue { struct_id, fields } => {
            validate_struct_operation(&context, None, *struct_id, fields, result_type, errors)
        }
        IrInstructionKind::AggregateCopy { struct_id, source } => {
            if !structs.contains_key(struct_id) {
                push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::UnknownStruct(*struct_id),
                );
            }
            check_value_type(
                function,
                block,
                *source,
                &Type::Struct(*struct_id),
                "aggregate copy source",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                &Type::Struct(*struct_id),
                "aggregate copy result",
                errors,
            );
        }
        IrInstructionKind::OptionalSome { value, value_type } => {
            check_value_type(
                function,
                block,
                *value,
                value_type,
                "optional value",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                &Type::Optional(Box::new(value_type.clone())),
                "some result",
                errors,
            );
        }
        IrInstructionKind::OptionalNone { value_type } => check_result_type(
            function,
            block,
            result_type,
            &Type::Optional(Box::new(value_type.clone())),
            "none result",
            errors,
        ),
        IrInstructionKind::OptionalHasValue(optional) => {
            if !matches!(values.get(optional), Some(Type::Optional(_))) {
                push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::InvalidOperation("hasValue requires an optional"),
                );
            }
            check_result_type(
                function,
                block,
                result_type,
                &Type::Bool,
                "hasValue result",
                errors,
            );
        }
        IrInstructionKind::OptionalValue {
            optional,
            value_type,
        } => {
            check_value_type(
                function,
                block,
                *optional,
                &Type::Optional(Box::new(value_type.clone())),
                "optional value receiver",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                value_type,
                "optional value result",
                errors,
            );
        }
        IrInstructionKind::Reference { value, value_type } => {
            check_value_type(
                function,
                block,
                *value,
                value_type,
                "reference value",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                &Type::Ref(Box::new(value_type.clone())),
                "reference result",
                errors,
            );
        }
        IrInstructionKind::ReferenceValue {
            reference,
            value_type,
        } => {
            check_value_type(
                function,
                block,
                *reference,
                &Type::Ref(Box::new(value_type.clone())),
                "reference receiver",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                value_type,
                "dereference result",
                errors,
            );
        }
        IrInstructionKind::ReferenceStore {
            reference,
            value,
            value_type,
        } => {
            check_value_type(
                function,
                block,
                *reference,
                &Type::Ref(Box::new(value_type.clone())),
                "reference store receiver",
                values,
                errors,
            );
            check_value_type(
                function,
                block,
                *value,
                value_type,
                "reference store value",
                values,
                errors,
            );
            if result_type.is_some() {
                push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::InvalidOperation(
                        "reference store cannot produce a value",
                    ),
                );
            }
        }
        IrInstructionKind::FieldLoad {
            base,
            struct_id,
            field,
        } => {
            check_value_type(
                function,
                block,
                *base,
                &Type::Struct(*struct_id),
                "field load base",
                values,
                errors,
            );
            validate_field_metadata(&context, *struct_id, *field, None, result_type, errors);
        }
        IrInstructionKind::FieldStore {
            local,
            struct_id,
            field,
            value,
        } => validate_field_operation(
            &context,
            *local,
            *struct_id,
            *field,
            Some(*value),
            None,
            errors,
        ),
        IrInstructionKind::EnumConstant { enum_id, variant } => {
            match enums.get(enum_id) {
                Some(definition) if (*variant as usize) < definition.variants.len() => {}
                Some(_) => push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::InvalidEnumVariant {
                        enum_id: *enum_id,
                        variant: *variant,
                    },
                ),
                None => push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::UnknownEnum(*enum_id),
                ),
            }
            check_result_type(
                function,
                block,
                result_type,
                &Type::Enum(*enum_id),
                "enum constant result",
                errors,
            );
        }
        IrInstructionKind::EnumValue {
            enum_id,
            variant,
            payload,
        } => {
            let expected_payload = enums
                .get(enum_id)
                .and_then(|definition| definition.variants.get(*variant as usize))
                .and_then(|variant| variant.payload.as_ref());
            match (expected_payload, payload) {
                (Some(expected), Some(value)) => check_value_type(
                    function,
                    block,
                    *value,
                    expected,
                    "enum payload",
                    values,
                    errors,
                ),
                (None, None)
                    if enums.get(enum_id).is_some_and(|definition| {
                        (*variant as usize) < definition.variants.len()
                    }) => {}
                _ => push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::InvalidOperation(
                        "enum payload does not match variant declaration",
                    ),
                ),
            }
            check_result_type(
                function,
                block,
                result_type,
                &Type::Enum(*enum_id),
                "enum value result",
                errors,
            );
        }
        IrInstructionKind::EnumIs {
            value,
            enum_id,
            variant,
        } => {
            check_value_type(
                function,
                block,
                *value,
                &Type::Enum(*enum_id),
                "enum discrimination value",
                values,
                errors,
            );
            if !enums
                .get(enum_id)
                .is_some_and(|definition| (*variant as usize) < definition.variants.len())
            {
                push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::InvalidEnumVariant {
                        enum_id: *enum_id,
                        variant: *variant,
                    },
                );
            }
            check_result_type(
                function,
                block,
                result_type,
                &Type::Bool,
                "enum discrimination result",
                errors,
            );
        }
        IrInstructionKind::EnumPayload {
            value,
            enum_id,
            variant,
            payload_type,
        } => {
            check_value_type(
                function,
                block,
                *value,
                &Type::Enum(*enum_id),
                "enum payload value",
                values,
                errors,
            );
            let expected = enums
                .get(enum_id)
                .and_then(|definition| definition.variants.get(*variant as usize))
                .and_then(|variant| variant.payload.as_ref());
            if expected != Some(payload_type) {
                push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::InvalidOperation(
                        "enum payload extraction type does not match metadata",
                    ),
                );
            }
            check_result_type(
                function,
                block,
                result_type,
                payload_type,
                "enum payload result",
                errors,
            );
        }
        IrInstructionKind::ArrayInit {
            local,
            element_type,
            length,
            values: items,
        } => {
            check_local_type(
                &context,
                *local,
                &Type::Array {
                    element: Box::new(element_type.clone()),
                    length: *length,
                },
                errors,
            );
            if items.len() != *length {
                push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::InvalidOperation(
                        "array initializer length does not match array type",
                    ),
                );
            }
            for value in items {
                check_value_type(
                    function,
                    block,
                    *value,
                    element_type,
                    "array initializer",
                    values,
                    errors,
                );
            }
        }
        IrInstructionKind::ArrayValue {
            element_type,
            length,
            values: items,
        } => {
            let array_type = Type::Array {
                element: Box::new(element_type.clone()),
                length: *length,
            };
            check_result_type(
                function,
                block,
                result_type,
                &array_type,
                "array value",
                errors,
            );
            if items.len() != *length {
                push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::InvalidOperation(
                        "array value length does not match array type",
                    ),
                );
            }
            for value in items {
                check_value_type(
                    function,
                    block,
                    *value,
                    element_type,
                    "array value",
                    values,
                    errors,
                );
            }
        }
        IrInstructionKind::ArrayLoad {
            collection,
            element_type,
            length,
            index,
        } => {
            check_value_type(
                function,
                block,
                *collection,
                &Type::Array {
                    element: Box::new(element_type.clone()),
                    length: *length,
                },
                "array load collection",
                values,
                errors,
            );
            check_value_type(
                function,
                block,
                *index,
                &Type::Int,
                "array index",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                element_type,
                "array load",
                errors,
            );
        }
        IrInstructionKind::ArrayStore {
            collection,
            element_type,
            length,
            index,
            value,
        } => {
            check_value_type(
                function,
                block,
                *collection,
                &Type::Array {
                    element: Box::new(element_type.clone()),
                    length: *length,
                },
                "array store collection",
                values,
                errors,
            );
            check_value_type(
                function,
                block,
                *index,
                &Type::Int,
                "array index",
                values,
                errors,
            );
            check_value_type(
                function,
                block,
                *value,
                element_type,
                "array store",
                values,
                errors,
            );
        }
        IrInstructionKind::ArrayLength(_) => check_result_type(
            function,
            block,
            result_type,
            &Type::Int,
            "array length",
            errors,
        ),
        IrInstructionKind::ListInit {
            local,
            element_type,
            values: items,
        } => {
            check_collection_local(
                &context,
                *local,
                Type::List(Box::new(element_type.clone())),
                errors,
            );
            for value in items {
                check_value_type(
                    function,
                    block,
                    *value,
                    element_type,
                    "list initializer",
                    values,
                    errors,
                );
            }
        }
        IrInstructionKind::ListValue {
            element_type,
            values: items,
        } => {
            check_result_type(
                function,
                block,
                result_type,
                &Type::List(Box::new(element_type.clone())),
                "list value",
                errors,
            );
            for value in items {
                check_value_type(
                    function,
                    block,
                    *value,
                    element_type,
                    "list value",
                    values,
                    errors,
                );
            }
        }
        IrInstructionKind::ListLoad {
            collection,
            element_type,
            index,
        } => {
            check_value_type(
                function,
                block,
                *collection,
                &Type::List(Box::new(element_type.clone())),
                "list load collection",
                values,
                errors,
            );
            check_value_type(
                function,
                block,
                *index,
                &Type::Int,
                "list index",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                element_type,
                "list load",
                errors,
            );
        }
        IrInstructionKind::ListStore {
            collection,
            element_type,
            index,
            value,
        } => {
            check_value_type(
                function,
                block,
                *collection,
                &Type::List(Box::new(element_type.clone())),
                "list store collection",
                values,
                errors,
            );
            check_value_type(
                function,
                block,
                *index,
                &Type::Int,
                "list index",
                values,
                errors,
            );
            check_value_type(
                function,
                block,
                *value,
                element_type,
                "list store",
                values,
                errors,
            );
        }
        IrInstructionKind::ListPush {
            local,
            element_type,
            value,
        } => {
            check_collection_local(
                &context,
                *local,
                Type::List(Box::new(element_type.clone())),
                errors,
            );
            check_value_type(
                function,
                block,
                *value,
                element_type,
                "list push",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                &Type::Void,
                "list push",
                errors,
            );
        }
        IrInstructionKind::ListPushField {
            local,
            struct_id,
            field,
            element_type,
            value,
        } => {
            check_local_type(&context, *local, &Type::Struct(*struct_id), errors);
            let expected = Type::List(Box::new(element_type.clone()));
            match structs
                .get(struct_id)
                .and_then(|definition| definition.fields.get(*field as usize))
            {
                Some(metadata) if metadata.ty == expected => {}
                Some(metadata) => push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::TypeMismatch {
                        context: "list push field",
                        expected,
                        found: metadata.ty.clone(),
                    },
                ),
                None => push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::InvalidField {
                        struct_id: *struct_id,
                        field: *field,
                    },
                ),
            }
            check_value_type(
                function,
                block,
                *value,
                element_type,
                "list push",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                &Type::Void,
                "list push",
                errors,
            );
        }
        IrInstructionKind::ListPushIndexed {
            collection,
            collection_type,
            index,
            element_type,
            value,
        } => {
            check_value_type(
                function,
                block,
                *collection,
                collection_type,
                "nested list push collection",
                values,
                errors,
            );
            let nested = Type::List(Box::new(element_type.clone()));
            let valid_collection = match collection_type {
                Type::List(element) | Type::Array { element, .. } => **element == nested,
                _ => false,
            };
            if !valid_collection {
                push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::InvalidOperation(
                        "nested list push requires a collection of lists",
                    ),
                );
            }
            check_value_type(
                function,
                block,
                *index,
                &Type::Int,
                "list index",
                values,
                errors,
            );
            check_value_type(
                function,
                block,
                *value,
                element_type,
                "list push",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                &Type::Void,
                "list push",
                errors,
            );
        }
        IrInstructionKind::ListPop {
            local,
            element_type,
        } => {
            check_collection_local(
                &context,
                *local,
                Type::List(Box::new(element_type.clone())),
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                element_type,
                "list pop",
                errors,
            );
        }
        IrInstructionKind::ListPopValue {
            collection,
            element_type,
        } => {
            check_value_type(
                function,
                block,
                *collection,
                &Type::List(Box::new(element_type.clone())),
                "list pop collection",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                element_type,
                "list pop",
                errors,
            );
        }
        IrInstructionKind::ListLength {
            collection,
            element_type,
        } => {
            check_value_type(
                function,
                block,
                *collection,
                &Type::List(Box::new(element_type.clone())),
                "list length collection",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                &Type::Int,
                "list length",
                errors,
            );
        }
        IrInstructionKind::CliArgLoad { local, index } => {
            check_local_type(&context, *local, &Type::CliArgs, errors);
            check_value_type(
                function,
                block,
                *index,
                &Type::Int,
                "command-line argument index",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                &Type::String,
                "command-line argument load",
                errors,
            );
        }
        IrInstructionKind::CliArgsLength { local } => {
            check_local_type(&context, *local, &Type::CliArgs, errors);
            check_result_type(
                function,
                block,
                result_type,
                &Type::Int,
                "command-line argument length",
                errors,
            );
        }
        IrInstructionKind::StringConstant(_) => {
            check_result_type(
                function,
                block,
                result_type,
                &Type::String,
                "string literal",
                errors,
            );
        }
        IrInstructionKind::StringConcat { left, right } => {
            check_value_type(
                function,
                block,
                *left,
                &Type::String,
                "string concat",
                values,
                errors,
            );
            check_value_type(
                function,
                block,
                *right,
                &Type::String,
                "string concat",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                &Type::String,
                "string concat",
                errors,
            );
        }
        IrInstructionKind::StringEqual { left, right, .. } => {
            check_value_type(
                function,
                block,
                *left,
                &Type::String,
                "string equality",
                values,
                errors,
            );
            check_value_type(
                function,
                block,
                *right,
                &Type::String,
                "string equality",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                &Type::Bool,
                "string equality",
                errors,
            );
        }
        IrInstructionKind::StringLength(value) => {
            check_value_type(
                function,
                block,
                *value,
                &Type::String,
                "string length",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                &Type::Int,
                "string length",
                errors,
            );
        }
        IrInstructionKind::StringByte { value, index } => {
            check_value_type(
                function,
                block,
                *value,
                &Type::String,
                "string byte receiver",
                values,
                errors,
            );
            check_value_type(
                function,
                block,
                *index,
                &Type::Int,
                "string byte index",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                &Type::Int,
                "string byte result",
                errors,
            );
        }
        IrInstructionKind::StringSlice { value, start, end } => {
            check_value_type(
                function,
                block,
                *value,
                &Type::String,
                "string slice receiver",
                values,
                errors,
            );
            for index in [*start, *end] {
                check_value_type(
                    function,
                    block,
                    index,
                    &Type::Int,
                    "string slice index",
                    values,
                    errors,
                );
            }
            check_result_type(
                function,
                block,
                result_type,
                &Type::String,
                "string slice result",
                errors,
            );
        }
        IrInstructionKind::ReadFile(path) => {
            check_value_type(
                function,
                block,
                *path,
                &Type::String,
                "readFile path",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                &Type::String,
                "readFile result",
                errors,
            );
        }
        IrInstructionKind::ReadBytes(path) => {
            check_value_type(
                function,
                block,
                *path,
                &Type::String,
                "readBytes path",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                &Type::List(Box::new(Type::Byte)),
                "readBytes result",
                errors,
            );
        }
        IrInstructionKind::WriteFile { path, data } => {
            for (value, context) in [(*path, "writeFile path"), (*data, "writeFile data")] {
                check_value_type(
                    function,
                    block,
                    value,
                    &Type::String,
                    context,
                    values,
                    errors,
                );
            }
            check_result_type(
                function,
                block,
                result_type,
                &Type::Void,
                "writeFile result",
                errors,
            );
        }
        IrInstructionKind::WriteBytes { path, data } => {
            check_value_type(
                function,
                block,
                *path,
                &Type::String,
                "writeBytes path",
                values,
                errors,
            );
            check_value_type(
                function,
                block,
                *data,
                &Type::List(Box::new(Type::Byte)),
                "writeBytes data",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                &Type::Void,
                "writeBytes result",
                errors,
            );
        }
        IrInstructionKind::Exists(path) => {
            check_value_type(
                function,
                block,
                *path,
                &Type::String,
                "exists path",
                values,
                errors,
            );
            check_result_type(
                function,
                block,
                result_type,
                &Type::Bool,
                "exists result",
                errors,
            );
        }
        IrInstructionKind::Print {
            value, value_type, ..
        } => {
            check_value_type(
                function,
                block,
                *value,
                value_type,
                "std.io output value",
                values,
                errors,
            );
            if !matches!(
                value_type,
                Type::String | Type::Char | Type::Int | Type::Bool
            ) {
                push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::InvalidOperation("unsupported std.io output type"),
                );
            }
            check_result_type(
                function,
                block,
                result_type,
                &Type::Void,
                "std.io output result",
                errors,
            );
        }
        IrInstructionKind::Input { target } => {
            if !matches!(target, Type::String | Type::Char | Type::Int | Type::Bool) {
                push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::InvalidOperation("unsupported std.io input type"),
                );
            }
            check_result_type(
                function,
                block,
                result_type,
                target,
                "std.io input result",
                errors,
            );
        }
        IrInstructionKind::Convert { value, from, to } => {
            check_value_type(
                function,
                block,
                *value,
                from,
                "conversion operand",
                values,
                errors,
            );
            let valid = matches!(
                (from, to),
                (Type::Int, Type::Int)
                    | (Type::Char, Type::Char)
                    | (Type::Char, Type::Int)
                    | (Type::Byte, Type::Byte)
                    | (Type::Byte, Type::Int)
                    | (Type::Enum(_), Type::Int)
                    | (Type::Int, Type::Char)
                    | (Type::Int, Type::Byte)
            );
            if !valid {
                push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::InvalidOperation("invalid explicit conversion"),
                );
            }
            check_result_type(
                function,
                block,
                result_type,
                to,
                "conversion result",
                errors,
            );
        }
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

fn validate_struct_operation(
    context: &InstructionContext<'_>,
    local: Option<SymbolId>,
    struct_id: TypeId,
    fields: &[(u32, ValueId)],
    result: Option<&Type>,
    errors: &mut Vec<IrVerificationError>,
) {
    let Some(definition) = context.structs.get(&struct_id) else {
        push_error(
            errors,
            context.function,
            Some(context.block),
            IrVerificationErrorKind::UnknownStruct(struct_id),
        );
        return;
    };
    if let Some(local) = local {
        check_local_type(context, local, &Type::Struct(struct_id), errors);
    }
    if result.is_some() {
        check_result_type(
            context.function,
            context.block,
            result,
            &Type::Struct(struct_id),
            "struct value result",
            errors,
        );
    }
    let mut seen = HashSet::new();
    for (field, value) in fields {
        let Some(metadata) = definition.fields.get(*field as usize) else {
            push_error(
                errors,
                context.function,
                Some(context.block),
                IrVerificationErrorKind::InvalidField {
                    struct_id,
                    field: *field,
                },
            );
            continue;
        };
        if !seen.insert(*field) {
            push_error(
                errors,
                context.function,
                Some(context.block),
                IrVerificationErrorKind::InvalidOperation(
                    "struct construction contains a duplicate field",
                ),
            );
        }
        check_value_type(
            context.function,
            context.block,
            *value,
            &metadata.ty,
            "struct field initializer",
            context.values,
            errors,
        );
    }
    if seen.len() != definition.fields.len() {
        push_error(
            errors,
            context.function,
            Some(context.block),
            IrVerificationErrorKind::InvalidOperation(
                "struct construction does not initialize every field exactly once",
            ),
        );
    }
}

fn validate_field_operation(
    context: &InstructionContext<'_>,
    local: SymbolId,
    struct_id: TypeId,
    field: u32,
    stored_value: Option<ValueId>,
    result: Option<&Type>,
    errors: &mut Vec<IrVerificationError>,
) {
    check_local_type(context, local, &Type::Struct(struct_id), errors);
    validate_field_metadata(context, struct_id, field, stored_value, result, errors);
}

fn validate_field_metadata(
    context: &InstructionContext<'_>,
    struct_id: TypeId,
    field: u32,
    stored_value: Option<ValueId>,
    result: Option<&Type>,
    errors: &mut Vec<IrVerificationError>,
) {
    let Some(definition) = context.structs.get(&struct_id) else {
        push_error(
            errors,
            context.function,
            Some(context.block),
            IrVerificationErrorKind::UnknownStruct(struct_id),
        );
        return;
    };
    let Some(metadata) = definition.fields.get(field as usize) else {
        push_error(
            errors,
            context.function,
            Some(context.block),
            IrVerificationErrorKind::InvalidField { struct_id, field },
        );
        return;
    };
    if let Some(value) = stored_value {
        check_value_type(
            context.function,
            context.block,
            value,
            &metadata.ty,
            "field store",
            context.values,
            errors,
        );
    }
    if result.is_some() {
        check_result_type(
            context.function,
            context.block,
            result,
            &metadata.ty,
            "field load",
            errors,
        );
    }
}

fn check_local_type(
    context: &InstructionContext<'_>,
    local: SymbolId,
    expected: &Type,
    errors: &mut Vec<IrVerificationError>,
) {
    match context.locals.get(&local) {
        Some(found) if found != expected => push_error(
            errors,
            context.function,
            Some(context.block),
            IrVerificationErrorKind::TypeMismatch {
                context: "struct local",
                expected: expected.clone(),
                found: found.clone(),
            },
        ),
        Some(_) => {}
        None => push_error(
            errors,
            context.function,
            Some(context.block),
            IrVerificationErrorKind::UnknownLocal(local),
        ),
    }
}

fn check_collection_local(
    context: &InstructionContext<'_>,
    local: SymbolId,
    expected: Type,
    errors: &mut Vec<IrVerificationError>,
) {
    check_local_type(context, local, &expected, errors);
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
            let operand_type = values.get(&left);
            match operand_type {
                Some(Type::Int | Type::Byte) => {
                    check_value_type(
                        function,
                        block,
                        right,
                        operand_type.expect("known comparison operand"),
                        "comparison operand",
                        values,
                        errors,
                    );
                }
                Some(_) => push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::InvalidOperation(
                        "ordering operands must be matching int or byte values",
                    ),
                ),
                None => push_error(
                    errors,
                    function,
                    Some(block),
                    IrVerificationErrorKind::UnknownValue(left),
                ),
            }
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
                if left_type != right_type
                    || !matches!(
                        left_type,
                        Type::Int | Type::Byte | Type::Bool | Type::Char | Type::Enum(_)
                    )
                {
                    push_error(
                        errors,
                        function,
                        Some(block),
                        IrVerificationErrorKind::InvalidOperation(
                            "equality operands must be matching int, byte, bool, char, or enum values",
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
        IrConstant::Byte(_) => Type::Byte,
        IrConstant::Float(_) => Type::Float,
        IrConstant::String(_) => Type::String,
        IrConstant::Char(_) => Type::Char,
        IrConstant::Bool(_) => Type::Bool,
    }
}

fn instruction_uses(kind: &IrInstructionKind) -> Vec<ValueId> {
    match kind {
        IrInstructionKind::Constant(_)
        | IrInstructionKind::LoadLocal(_)
        | IrInstructionKind::EnumConstant { .. }
        | IrInstructionKind::ArrayLength(_)
        | IrInstructionKind::ListPop { .. }
        | IrInstructionKind::CliArgsLength { .. }
        | IrInstructionKind::OptionalNone { .. }
        | IrInstructionKind::StringConstant(_)
        | IrInstructionKind::Input { .. } => Vec::new(),
        IrInstructionKind::BindLocal { value, .. } | IrInstructionKind::Copy(value) => vec![*value],
        IrInstructionKind::Call { arguments, .. } | IrInstructionKind::Aggregate(arguments) => {
            arguments.clone()
        }
        IrInstructionKind::StructInit { fields, .. }
        | IrInstructionKind::StructValue { fields, .. } => {
            fields.iter().map(|(_, value)| *value).collect()
        }
        IrInstructionKind::FieldLoad { base, .. } => vec![*base],
        IrInstructionKind::FieldStore { value, .. } => vec![*value],
        IrInstructionKind::AggregateCopy { source, .. } => vec![*source],
        IrInstructionKind::OptionalSome { value, .. } => vec![*value],
        IrInstructionKind::OptionalHasValue(value) => vec![*value],
        IrInstructionKind::OptionalValue { optional, .. } => vec![*optional],
        IrInstructionKind::Reference { value, .. } => vec![*value],
        IrInstructionKind::ReferenceValue { reference, .. } => vec![*reference],
        IrInstructionKind::ReferenceStore {
            reference, value, ..
        } => vec![*reference, *value],
        IrInstructionKind::EnumValue { payload, .. } => payload.iter().copied().collect(),
        IrInstructionKind::EnumIs { value, .. } | IrInstructionKind::EnumPayload { value, .. } => {
            vec![*value]
        }
        IrInstructionKind::ArrayInit { values, .. }
        | IrInstructionKind::ListInit { values, .. }
        | IrInstructionKind::ArrayValue { values, .. }
        | IrInstructionKind::ListValue { values, .. } => values.clone(),
        IrInstructionKind::ArrayLoad {
            collection, index, ..
        }
        | IrInstructionKind::ListLoad {
            collection, index, ..
        } => vec![*collection, *index],
        IrInstructionKind::CliArgLoad { index, .. } => vec![*index],
        IrInstructionKind::ArrayStore {
            collection,
            index,
            value,
            ..
        }
        | IrInstructionKind::ListStore {
            collection,
            index,
            value,
            ..
        } => {
            vec![*collection, *index, *value]
        }
        IrInstructionKind::ListPush { value, .. }
        | IrInstructionKind::ListPushField { value, .. } => vec![*value],
        IrInstructionKind::ListPushIndexed {
            collection,
            index,
            value,
            ..
        } => vec![*collection, *index, *value],
        IrInstructionKind::ListPopValue { collection, .. } => vec![*collection],
        IrInstructionKind::ListLength { collection, .. } => vec![*collection],
        IrInstructionKind::StringConcat { left, right }
        | IrInstructionKind::StringEqual { left, right, .. }
        | IrInstructionKind::WriteFile {
            path: left,
            data: right,
        }
        | IrInstructionKind::WriteBytes {
            path: left,
            data: right,
        } => vec![*left, *right],
        IrInstructionKind::StringLength(value)
        | IrInstructionKind::ReadFile(value)
        | IrInstructionKind::ReadBytes(value)
        | IrInstructionKind::Exists(value)
        | IrInstructionKind::Print { value, .. } => vec![*value],
        IrInstructionKind::StringByte { value, index } => vec![*value, *index],
        IrInstructionKind::StringSlice { value, start, end } => vec![*value, *start, *end],
        IrInstructionKind::Convert { value, .. } => vec![*value],
        IrInstructionKind::Unary { operand, .. } => vec![*operand],
        IrInstructionKind::Binary { left, right, .. } => vec![*left, *right],
    }
}

fn terminator_uses(terminator: &IrTerminator) -> Vec<ValueId> {
    match terminator {
        IrTerminator::Return(Some(value)) => vec![*value],
        IrTerminator::Exit(value) => vec![*value],
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
        IrTerminator::Return(_) | IrTerminator::Exit(_) => Vec::new(),
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

fn module_error(kind: IrVerificationErrorKind) -> IrVerificationError {
    IrVerificationError {
        function: None,
        block: None,
        kind,
    }
}

fn validate_known_type(
    ty: &Type,
    structs: &HashMap<TypeId, &IrStruct>,
    enums: &HashMap<TypeId, &IrEnum>,
    function: Option<&str>,
    errors: &mut Vec<IrVerificationError>,
) {
    let valid = match ty {
        Type::Struct(id) => structs.contains_key(id),
        Type::Enum(id) => enums.contains_key(id),
        Type::Named(_) => false,
        Type::Array { element, .. }
        | Type::List(element)
        | Type::Optional(element)
        | Type::Ref(element) => {
            validate_known_type(element, structs, enums, function, errors);
            true
        }
        Type::Tuple(elements) => {
            for element in elements {
                validate_known_type(element, structs, enums, function, errors);
            }
            true
        }
        _ => true,
    };
    if !valid {
        errors.push(IrVerificationError {
            function: function.map(str::to_owned),
            block: None,
            kind: IrVerificationErrorKind::InvalidType(ty.clone()),
        });
    }
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
            structs: Vec::new(),
            enums: Vec::new(),
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
        assert!(verify_module(&int_module()).is_ok());
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
        assert!(verify_module(&module).is_ok());
    }

    #[test]
    fn accepts_valid_if_else_cfg() {
        let module = lower_source(
            "private int f(bool choose) { if (choose) { return 1; } else { return 2; } }",
        );
        assert!(verify_module(&module).is_ok());
    }

    #[test]
    fn accepts_valid_short_circuit_cfg() {
        let module = lower_source("private bool f(bool a, bool b) { return a && b || a; }");
        assert!(verify_module(&module).is_ok());
    }

    #[test]
    fn accepts_short_circuit_cfg_inside_loop() {
        let module = lower_source(
            "private void f(bool run, bool a, bool b) { while (run) { bool value = a && b; run = false; } }",
        );
        assert!(verify_module(&module).is_ok());
    }

    #[test]
    fn rejects_invalid_struct_field_id() {
        let mut module = lower_source(
            "struct Item { int value; } private int f() { Item item = Item { value: 1 }; return item.value; }",
        );
        let instruction = module.functions[0]
            .blocks
            .iter_mut()
            .flat_map(|block| &mut block.instructions)
            .find(|instruction| matches!(instruction.kind, IrInstructionKind::FieldLoad { .. }))
            .unwrap();
        let IrInstructionKind::FieldLoad { field, .. } = &mut instruction.kind else {
            unreachable!()
        };
        *field = 99;
        assert_has_error(&module, |kind| {
            matches!(
                kind,
                IrVerificationErrorKind::InvalidField { field: 99, .. }
            )
        });
    }

    #[test]
    fn rejects_invalid_enum_variant() {
        let mut module =
            lower_source("enum State { idle } private void f() { State state = State.idle; }");
        let instruction = &mut module.functions[0].blocks[0].instructions[0];
        let IrInstructionKind::EnumConstant { variant, .. } = &mut instruction.kind else {
            unreachable!()
        };
        *variant = 9;
        assert_has_error(&module, |kind| {
            matches!(
                kind,
                IrVerificationErrorKind::InvalidEnumVariant { variant: 9, .. }
            )
        });
    }

    #[test]
    fn rejects_struct_field_initializer_type_mismatch() {
        let mut module = lower_source(
            "struct Item { int value; } private void f() { Item item = Item { value: 1 }; }",
        );
        let value = &mut module.functions[0].blocks[0].instructions[0];
        value.result_type = Some(Type::Bool);
        value.kind = IrInstructionKind::Constant(IrConstant::Bool(true));
        assert_has_error(&module, |kind| {
            matches!(
                kind,
                IrVerificationErrorKind::TypeMismatch {
                    context: "struct field initializer",
                    expected: Type::Int,
                    found: Type::Bool
                }
            )
        });
    }

    #[test]
    fn rejects_enum_comparison_type_mismatch() {
        let mut module =
            lower_source("enum A { one } private void f() { bool same = A.one == A.one; }");
        let mut second_enum = module.enums[0].clone();
        second_enum.id = TypeId(1);
        second_enum.name = "B".to_owned();
        module.enums.push(second_enum);
        let mut constants: Vec<_> = module.functions[0].blocks[0]
            .instructions
            .iter_mut()
            .filter(|instruction| {
                matches!(instruction.kind, IrInstructionKind::EnumConstant { .. })
            })
            .collect();
        constants[1].result_type = Some(Type::Enum(TypeId(1)));
        let IrInstructionKind::EnumConstant { enum_id, .. } = &mut constants[1].kind else {
            unreachable!()
        };
        *enum_id = TypeId(1);
        assert_has_error(
            &module,
            |kind| matches!(kind, IrVerificationErrorKind::InvalidOperation(message) if message.contains("equality operands")),
        );
    }

    #[test]
    fn accepts_valid_struct_and_enum_ir() {
        let module = lower_source(
            "enum Kind { first } struct Item { Kind kind; int value; } private int f() { Item item = Item { kind: Kind.first, value: 1 }; item.value = 2; return item.value; }",
        );
        assert!(verify_module(&module).is_ok());
    }

    #[test]
    fn verifies_aggregate_copy_list_and_slice_ir() {
        let module = lower_source(
            "struct Token { string text; int line; } private void f() { Token a = Token { text: \"abc\", line: 1 }; Token b = a; list Token values = []; values.push(b); values[0] = b; Token first = values[0]; Token last = values.pop(); string piece = first.text.slice(0, 1); }",
        );
        assert!(verify_module(&module).is_ok());
    }

    #[test]
    fn rejects_invalid_aggregate_copy_and_list_result_types() {
        let source = "struct Token { int line; } private void f() { Token a = Token { line: 1 }; Token b = a; list Token values = []; values.push(b); Token first = values[0]; }";
        let mut copy = lower_source(source);
        let operation = copy.functions[0]
            .blocks
            .iter_mut()
            .flat_map(|block| &mut block.instructions)
            .find(|instruction| matches!(instruction.kind, IrInstructionKind::AggregateCopy { .. }))
            .unwrap();
        operation.result_type = Some(Type::Int);
        assert_has_error(&copy, |kind| {
            matches!(
                kind,
                IrVerificationErrorKind::TypeMismatch {
                    context: "aggregate copy result",
                    ..
                }
            )
        });

        let mut list = lower_source(source);
        let operation = list.functions[0]
            .blocks
            .iter_mut()
            .flat_map(|block| &mut block.instructions)
            .find(|instruction| matches!(instruction.kind, IrInstructionKind::ListLoad { .. }))
            .unwrap();
        operation.result_type = Some(Type::Int);
        assert_has_error(&list, |kind| {
            matches!(
                kind,
                IrVerificationErrorKind::TypeMismatch {
                    context: "list load",
                    ..
                }
            )
        });
    }

    #[test]
    fn rejects_invalid_string_slice_ir_types() {
        let mut indexes = lower_source(
            "private string f() { string source = \"abc\"; return source.slice(0, 1); }",
        );
        let start_value = indexes.functions[0].blocks[0]
            .instructions
            .iter()
            .find_map(|instruction| match instruction.kind {
                IrInstructionKind::StringSlice { start, .. } => Some(start),
                _ => None,
            })
            .unwrap();
        let start = indexes.functions[0].blocks[0]
            .instructions
            .iter_mut()
            .find(|instruction| instruction.result == Some(start_value))
            .unwrap();
        start.result_type = Some(Type::Bool);
        start.kind = IrInstructionKind::Constant(IrConstant::Bool(false));
        assert_has_error(&indexes, |kind| {
            matches!(
                kind,
                IrVerificationErrorKind::TypeMismatch {
                    context: "string slice index",
                    ..
                }
            )
        });

        let mut result = lower_source(
            "private string f() { string source = \"abc\"; return source.slice(0, 1); }",
        );
        let slice = result.functions[0]
            .blocks
            .iter_mut()
            .flat_map(|block| &mut block.instructions)
            .find(|instruction| matches!(instruction.kind, IrInstructionKind::StringSlice { .. }))
            .unwrap();
        slice.result_type = Some(Type::Int);
        assert_has_error(&result, |kind| {
            matches!(
                kind,
                IrVerificationErrorKind::TypeMismatch {
                    context: "string slice result",
                    ..
                }
            )
        });
    }

    #[test]
    fn accepts_collection_struct_field_layout_metadata_and_rejects_float() {
        let mut module = lower_source(
            "struct Token { list int values; } private void f() { Token value = Token { values: [1] }; }",
        );
        assert!(verify_module(&module).is_ok());
        module.structs[0].fields[0].ty = Type::Float;
        assert_has_error(
            &module,
            |kind| matches!(kind, IrVerificationErrorKind::InvalidOperation(message) if message.contains("unsupported bootstrap layout")),
        );
    }

    #[test]
    fn rejects_invalid_collection_and_string_ir_types() {
        let mut array = lower_source("private int f() { int[1] values = [1]; return values[0]; }");
        let load = array.functions[0]
            .blocks
            .iter_mut()
            .flat_map(|block| &mut block.instructions)
            .find(|instruction| matches!(instruction.kind, IrInstructionKind::ArrayLoad { .. }))
            .unwrap();
        load.result_type = Some(Type::Bool);
        assert_has_error(&array, |kind| {
            matches!(
                kind,
                IrVerificationErrorKind::TypeMismatch {
                    context: "array load",
                    ..
                }
            )
        });

        let mut list = lower_source("private void f() { list int values = [1]; }");
        let init = list.functions[0]
            .blocks
            .iter_mut()
            .flat_map(|block| &mut block.instructions)
            .find(|instruction| matches!(instruction.kind, IrInstructionKind::ListValue { .. }))
            .unwrap();
        let IrInstructionKind::ListValue { element_type, .. } = &mut init.kind else {
            unreachable!()
        };
        *element_type = Type::Bool;
        assert_has_error(&list, |kind| {
            matches!(kind, IrVerificationErrorKind::TypeMismatch { .. })
        });

        let mut string = lower_source("private string f() { return \"value\"; }");
        string.functions[0].blocks[0].instructions[0].result_type = Some(Type::Int);
        assert_has_error(&string, |kind| {
            matches!(
                kind,
                IrVerificationErrorKind::TypeMismatch {
                    context: "string literal",
                    ..
                }
            )
        });
    }

    #[test]
    fn verifies_source_ingestion_ir_types_and_cli_handoff() {
        let valid = lower_source(
            "use std.fs; private int scan(string path) { string source = readFile(path); return source.byte(0); } public void main(string[] args) { int count = args.length; string first = args[0]; }",
        );
        assert!(verify_module(&valid).is_ok());
        let mut invalid_cli = valid.clone();
        invalid_cli.functions[1].name = "not_main".into();
        assert_has_error(
            &invalid_cli,
            |kind| matches!(kind, IrVerificationErrorKind::InvalidOperation(message) if message.contains("sole parameter of main")),
        );

        let mut byte_index =
            lower_source("private int f() { string source = \"a\"; return source.byte(0); }");
        let index_value = byte_index.functions[0].blocks[0]
            .instructions
            .iter()
            .find_map(|instruction| match instruction.kind {
                IrInstructionKind::StringByte { index, .. } => Some(index),
                _ => None,
            })
            .unwrap();
        let index = byte_index.functions[0].blocks[0]
            .instructions
            .iter_mut()
            .find(|instruction| instruction.result == Some(index_value))
            .unwrap();
        index.result_type = Some(Type::Bool);
        index.kind = IrInstructionKind::Constant(IrConstant::Bool(false));
        assert_has_error(&byte_index, |kind| {
            matches!(
                kind,
                IrVerificationErrorKind::TypeMismatch {
                    context: "string byte index",
                    ..
                }
            )
        });
        assert!(byte_index.functions[0].blocks[0]
            .instructions
            .iter()
            .any(|instruction| matches!(instruction.kind, IrInstructionKind::StringByte { index, .. } if index == index_value)));

        let mut byte_result =
            lower_source("private int f() { string source = \"a\"; return source.byte(0); }");
        let byte = byte_result.functions[0]
            .blocks
            .iter_mut()
            .flat_map(|block| &mut block.instructions)
            .find(|instruction| matches!(instruction.kind, IrInstructionKind::StringByte { .. }))
            .unwrap();
        byte.result_type = Some(Type::String);
        assert_has_error(&byte_result, |kind| {
            matches!(
                kind,
                IrVerificationErrorKind::TypeMismatch {
                    context: "string byte result",
                    ..
                }
            )
        });

        let mut read =
            lower_source("use std.fs; private string f() { return readFile(\"file\"); }");
        let path_value = read.functions[0].blocks[0]
            .instructions
            .iter()
            .find_map(|instruction| match instruction.kind {
                IrInstructionKind::ReadFile(path) => Some(path),
                _ => None,
            })
            .unwrap();
        let path = read.functions[0].blocks[0]
            .instructions
            .iter_mut()
            .find(|instruction| instruction.result == Some(path_value))
            .unwrap();
        path.result_type = Some(Type::Int);
        path.kind = IrInstructionKind::Constant(IrConstant::Integer("1".into()));
        let operation = read.functions[0]
            .blocks
            .iter_mut()
            .flat_map(|block| &mut block.instructions)
            .find(|instruction| matches!(instruction.kind, IrInstructionKind::ReadFile(_)))
            .unwrap();
        operation.result_type = Some(Type::Int);
        assert_has_error(&read, |kind| {
            matches!(
                kind,
                IrVerificationErrorKind::TypeMismatch {
                    context: "readFile path",
                    ..
                }
            )
        });
        assert_has_error(&read, |kind| {
            matches!(
                kind,
                IrVerificationErrorKind::TypeMismatch {
                    context: "readFile result",
                    ..
                }
            )
        });
    }

    fn branch_module() -> IrModule {
        IrModule {
            structs: Vec::new(),
            enums: Vec::new(),
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
