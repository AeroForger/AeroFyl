use std::collections::{HashMap, HashSet};

use super::ast::{Expression, ExpressionKind, Literal, Module, StatementKind, Visibility};
use super::diagnostics::Diagnostic;
use super::resolution::{Resolution, SymbolId, SymbolKind, resolve};
use super::types::{Type, TypeId};
use crate::middle::hir::*;

pub fn analyze(module: &Module) -> Result<HirModule, Vec<Diagnostic>> {
    let resolution = resolve(module)?;
    let mut analyzer = Analyzer {
        resolution: &resolution,
        diagnostics: Vec::new(),
        structs: Vec::new(),
        enums: Vec::new(),
        type_names: HashMap::new(),
    };
    analyzer.collect_types(module);
    analyzer.analyze_module(module)
}

/// Validates the additional source-level contract required for an executable.
pub fn validate_executable(module: &HirModule) -> Result<SymbolId, Vec<Diagnostic>> {
    let main_functions: Vec<_> = module
        .functions
        .iter()
        .filter(|function| function.name == "main")
        .collect();
    if main_functions.is_empty() {
        return Err(vec![Diagnostic::error(
            "executable requires exactly one `public void main()` function",
            module.span,
        )]);
    }
    if main_functions.len() > 1 {
        return Err(main_functions
            .iter()
            .skip(1)
            .map(|function| Diagnostic::error("duplicate `main` function", function.name_span))
            .collect());
    }
    let main = main_functions[0];
    let mut diagnostics = Vec::new();
    if main.visibility != Visibility::Public {
        diagnostics.push(Diagnostic::error(
            "executable entry function `main` must be public",
            main.name_span,
        ));
    }
    if main.return_type != Type::Void {
        diagnostics.push(Diagnostic::error(
            "executable entry function `main` must return `void`",
            main.name_span,
        ));
    }
    if !main.parameters.is_empty() {
        diagnostics.push(Diagnostic::error(
            "executable entry function `main` must take no parameters",
            main.name_span,
        ));
    }
    if diagnostics.is_empty() {
        Ok(main.symbol)
    } else {
        Err(diagnostics)
    }
}

struct Analyzer<'resolution> {
    resolution: &'resolution Resolution,
    diagnostics: Vec<Diagnostic>,
    structs: Vec<HirStruct>,
    enums: Vec<HirEnum>,
    type_names: HashMap<String, Type>,
}

impl Analyzer<'_> {
    fn collect_types(&mut self, module: &Module) {
        for declaration in &module.structs {
            let id = TypeId(self.type_names.len() as u32);
            self.type_names
                .entry(declaration.name.text.clone())
                .or_insert(Type::Struct(id));
        }
        for declaration in &module.enums {
            let id = TypeId(self.type_names.len() as u32);
            self.type_names
                .entry(declaration.name.text.clone())
                .or_insert(Type::Enum(id));
        }
        for declaration in &module.structs {
            let Some(Type::Struct(id)) = self.type_names.get(&declaration.name.text).cloned()
            else {
                continue;
            };
            let fields = declaration
                .fields
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    let ty = self.resolve_type(&field.ty.kind, field.ty.span);
                    if matches!(ty, Type::Struct(_)) {
                        self.diagnostics.push(Diagnostic::error(
                            "nested struct fields are not supported by the bootstrap layout",
                            field.ty.span,
                        ));
                    }
                    HirField {
                        name: field.name.text.clone(),
                        ty,
                        offset: index as u32 * 8,
                    }
                })
                .collect();
            self.structs.push(HirStruct {
                id,
                name: declaration.name.text.clone(),
                fields,
            });
        }
        for declaration in &module.enums {
            let Some(Type::Enum(id)) = self.type_names.get(&declaration.name.text).cloned() else {
                continue;
            };
            self.enums.push(HirEnum {
                id,
                name: declaration.name.text.clone(),
                variants: declaration
                    .variants
                    .iter()
                    .map(|variant| variant.text.clone())
                    .collect(),
            });
        }
    }

    fn analyze_module(mut self, module: &Module) -> Result<HirModule, Vec<Diagnostic>> {
        let mut functions = Vec::new();
        for function in &module.functions {
            let symbol = self.declaration_id(function.name.span);
            let return_type =
                self.resolve_type(&function.return_type.kind, function.return_type.span);
            let parameters = function
                .parameters
                .iter()
                .map(|parameter| HirParameter {
                    symbol: self.declaration_id(parameter.name.span),
                    ty: self.resolve_type(&parameter.ty.kind, parameter.ty.span),
                })
                .collect();
            let body = self.analyze_block(&function.body, &return_type, 0);
            if return_type != Type::Void && !block_definitely_returns(&function.body) {
                self.diagnostics.push(Diagnostic::error(
                    format!("a `{return_type}` function must return a value"),
                    function.name.span,
                ));
            }
            functions.push(HirFunction {
                symbol,
                name: function.name.text.clone(),
                name_span: function.name.span,
                visibility: function.visibility,
                return_type,
                parameters,
                body,
                span: function.span,
            });
        }
        if self.diagnostics.is_empty() {
            Ok(HirModule {
                structs: self.structs,
                enums: self.enums,
                functions,
                span: module.span,
            })
        } else {
            Err(self.diagnostics)
        }
    }

    fn analyze_block(
        &mut self,
        block: &super::ast::Block,
        return_type: &Type,
        loop_depth: usize,
    ) -> HirBlock {
        let mut statements = Vec::new();
        for statement in &block.statements {
            let hir = match &statement.kind {
                StatementKind::Variable(variable) => {
                    let ty = self.resolve_type(&variable.ty.kind, variable.ty.span);
                    let initializer = self.check_expression(&variable.initializer, Some(&ty));
                    HirStatement::Variable {
                        symbol: self.declaration_id(variable.name.span),
                        ty,
                        initializer,
                        span: statement.span,
                    }
                }
                StatementKind::Assignment(assignment) => {
                    self.check_assignment(&assignment.target, &assignment.value, statement.span)
                }
                StatementKind::Return(value) => {
                    let value = match value {
                        Some(value) => Some(self.check_expression(value, Some(return_type))),
                        None => {
                            if *return_type != Type::Void {
                                self.diagnostics.push(Diagnostic::error(
                                    format!("return value required for `{return_type}`"),
                                    statement.span,
                                ));
                            }
                            None
                        }
                    };
                    HirStatement::Return {
                        value,
                        span: statement.span,
                    }
                }
                StatementKind::Expression(expression) => {
                    HirStatement::Expression(self.check_expression(expression, None))
                }
                StatementKind::If {
                    condition,
                    then_block,
                    else_block,
                } => {
                    let condition = self.check_expression(condition, Some(&Type::Bool));
                    let then_block = self.analyze_block(then_block, return_type, loop_depth);
                    let else_block = else_block
                        .as_ref()
                        .map(|block| self.analyze_block(block, return_type, loop_depth));
                    HirStatement::If {
                        condition,
                        then_block,
                        else_block,
                        span: statement.span,
                    }
                }
                StatementKind::While { condition, body } => {
                    let condition = self.check_expression(condition, Some(&Type::Bool));
                    let body = self.analyze_block(body, return_type, loop_depth + 1);
                    HirStatement::While {
                        condition,
                        body,
                        span: statement.span,
                    }
                }
                StatementKind::Break => {
                    if loop_depth == 0 {
                        self.diagnostics.push(Diagnostic::error(
                            "`break` may only appear inside a `while` loop",
                            statement.span,
                        ));
                    }
                    HirStatement::Break(statement.span)
                }
                StatementKind::Continue => {
                    if loop_depth == 0 {
                        self.diagnostics.push(Diagnostic::error(
                            "`continue` may only appear inside a `while` loop",
                            statement.span,
                        ));
                    }
                    HirStatement::Continue(statement.span)
                }
            };
            statements.push(hir);
        }
        HirBlock {
            statements,
            span: block.span,
        }
    }

    fn check_expression(
        &mut self,
        expression: &Expression,
        expected: Option<&Type>,
    ) -> HirExpression {
        let (kind, actual) = match &expression.kind {
            ExpressionKind::Literal(literal) => {
                let ty = match literal {
                    Literal::Integer(value) => {
                        self.validate_integer_magnitude(value, i64::MAX as u128, expression.span);
                        Type::Int
                    }
                    Literal::Float(_) => Type::Float,
                    Literal::String(_) => Type::String,
                    Literal::Char(_) => Type::Char,
                    Literal::Bool(_) => Type::Bool,
                };
                (HirExpressionKind::Literal(literal.clone()), ty)
            }
            ExpressionKind::Identifier(name) => {
                let id = self.reference_id(name.span);
                let symbol_kind = self.symbol(id).kind.clone();
                let ty = match symbol_kind {
                    SymbolKind::Variable { ty } | SymbolKind::Parameter { ty } => {
                        self.resolve_type(&ty, name.span)
                    }
                    SymbolKind::Function { .. } | SymbolKind::Builtin | SymbolKind::Module => {
                        self.diagnostics.push(Diagnostic::error(
                            format!("`{}` cannot be used as a value", name.text),
                            name.span,
                        ));
                        Type::Dynamic
                    }
                };
                (HirExpressionKind::Symbol(id), ty)
            }
            ExpressionKind::Call { callee, arguments } => {
                let id = self.reference_id(callee.span);
                let symbol_kind = self.symbol(id).kind.clone();
                match symbol_kind {
                    SymbolKind::Function {
                        parameters,
                        return_type,
                    } => {
                        let parameters: Vec<_> = parameters
                            .iter()
                            .map(|ty| self.resolve_type(ty, callee.span))
                            .collect();
                        let return_type = self.resolve_type(&return_type, callee.span);
                        if parameters.len() != arguments.len() {
                            self.diagnostics.push(Diagnostic::error(
                                format!(
                                    "function `{}` expects {} argument(s), found {}",
                                    callee.text,
                                    parameters.len(),
                                    arguments.len()
                                ),
                                expression.span,
                            ));
                        }
                        let checked = arguments
                            .iter()
                            .enumerate()
                            .map(|(index, argument)| {
                                self.check_expression(argument, parameters.get(index))
                            })
                            .collect();
                        (
                            HirExpressionKind::Call {
                                callee: id,
                                arguments: checked,
                            },
                            return_type,
                        )
                    }
                    SymbolKind::Builtin => {
                        // TODO(spec): builtin signatures, result types, and behavior are missing.
                        let checked = arguments
                            .iter()
                            .map(|argument| self.check_expression(argument, None))
                            .collect();
                        (
                            HirExpressionKind::Call {
                                callee: id,
                                arguments: checked,
                            },
                            Type::Dynamic,
                        )
                    }
                    _ => {
                        self.diagnostics.push(Diagnostic::error(
                            format!("`{}` is not a function", callee.text),
                            callee.span,
                        ));
                        (
                            HirExpressionKind::Call {
                                callee: id,
                                arguments: Vec::new(),
                            },
                            Type::Dynamic,
                        )
                    }
                }
            }
            ExpressionKind::Unary { operator, operand } => {
                let (checked_operand, result_type) = match operator {
                    super::ast::UnaryOperator::Negate => {
                        let operand = if let ExpressionKind::Literal(Literal::Integer(value)) =
                            &operand.kind
                        {
                            self.validate_integer_magnitude(
                                value,
                                (i64::MAX as u128) + 1,
                                operand.span,
                            );
                            HirExpression {
                                kind: HirExpressionKind::Literal(Literal::Integer(value.clone())),
                                ty: Type::Int,
                                span: operand.span,
                            }
                        } else {
                            self.check_expression(operand, Some(&Type::Int))
                        };
                        (operand, Type::Int)
                    }
                    super::ast::UnaryOperator::Not => (
                        self.check_expression(operand, Some(&Type::Bool)),
                        Type::Bool,
                    ),
                };
                (
                    HirExpressionKind::Unary {
                        operator: *operator,
                        operand: Box::new(checked_operand),
                    },
                    result_type,
                )
            }
            ExpressionKind::Binary {
                operator,
                left,
                right,
            } => {
                let (left, right, result_type) = match operator {
                    super::ast::BinaryOperator::Add
                    | super::ast::BinaryOperator::Subtract
                    | super::ast::BinaryOperator::Multiply
                    | super::ast::BinaryOperator::Divide => (
                        self.check_expression(left, Some(&Type::Int)),
                        self.check_expression(right, Some(&Type::Int)),
                        Type::Int,
                    ),
                    super::ast::BinaryOperator::Less
                    | super::ast::BinaryOperator::LessEqual
                    | super::ast::BinaryOperator::Greater
                    | super::ast::BinaryOperator::GreaterEqual => {
                        let left = self.check_expression(left, None);
                        let right = self.check_expression(right, Some(&left.ty));
                        if left.ty != Type::Int {
                            self.diagnostics.push(Diagnostic::error(
                                "ordering comparison expected `int` operands; enums support only `==` and `!=`",
                                expression.span,
                            ));
                        }
                        (left, right, Type::Bool)
                    }
                    super::ast::BinaryOperator::LogicalAnd
                    | super::ast::BinaryOperator::LogicalOr => (
                        self.check_expression(left, Some(&Type::Bool)),
                        self.check_expression(right, Some(&Type::Bool)),
                        Type::Bool,
                    ),
                    super::ast::BinaryOperator::Equal | super::ast::BinaryOperator::NotEqual => {
                        let left = self.check_expression(left, None);
                        let right = self.check_expression(right, None);
                        if left.ty != right.ty
                            || !matches!(left.ty, Type::Int | Type::Bool | Type::Enum(_))
                        {
                            self.diagnostics.push(Diagnostic::error(
                                "equality requires matching `int`, `bool`, or enum operands",
                                expression.span,
                            ));
                        }
                        (left, right, Type::Bool)
                    }
                };
                (
                    HirExpressionKind::Binary {
                        operator: *operator,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    result_type,
                )
            }
            ExpressionKind::Collection(values) => {
                let (element_type, actual_type, expected_length) = match expected {
                    Some(Type::Array { element, length }) => (
                        (**element).clone(),
                        Type::Array {
                            element: element.clone(),
                            length: *length,
                        },
                        Some(*length),
                    ),
                    Some(Type::List(element)) => {
                        ((**element).clone(), Type::List(element.clone()), None)
                    }
                    _ => {
                        self.diagnostics.push(Diagnostic::error(
                            "collection literal needs an array or list type context; standalone collection inference is not specified",
                            expression.span,
                        ));
                        (Type::Dynamic, Type::Dynamic, None)
                    }
                };
                if let Some(length) = expected_length
                    && values.len() != length
                {
                    self.diagnostics.push(Diagnostic::error(
                        format!(
                            "array length is {length}, but literal contains {} element(s)",
                            values.len()
                        ),
                        expression.span,
                    ));
                }
                let values = values
                    .iter()
                    .map(|value| self.check_expression(value, Some(&element_type)))
                    .collect();
                (HirExpressionKind::Collection(values), actual_type)
            }
            ExpressionKind::Tuple(values) => {
                // This is reachable only for programmatically constructed ASTs; parser syntax is pending.
                let expected_elements = match expected {
                    Some(Type::Tuple(items)) => Some(items),
                    _ => None,
                };
                let checked = values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        self.check_expression(
                            value,
                            expected_elements.and_then(|items| items.get(index)),
                        )
                    })
                    .collect();
                (
                    HirExpressionKind::Tuple(checked),
                    expected.cloned().unwrap_or(Type::Dynamic),
                )
            }
            ExpressionKind::StructLiteral { name, fields } => {
                self.check_struct_literal(name, fields, expression.span)
            }
            ExpressionKind::Member { base, name } => {
                self.check_member_expression(base, name, expression.span)
            }
        };

        if let Some(expected) = expected
            && !expected.is_compatible_with(&actual)
        {
            self.diagnostics.push(Diagnostic::error(
                format!("type mismatch: expected `{expected}`, found `{actual}`"),
                expression.span,
            ));
        }
        HirExpression {
            kind,
            ty: actual,
            span: expression.span,
        }
    }

    fn check_assignment(
        &mut self,
        target: &Expression,
        value: &Expression,
        span: super::source::Span,
    ) -> HirStatement {
        match &target.kind {
            ExpressionKind::Identifier(name) => {
                let symbol = self.reference_id(name.span);
                let symbol_kind = self.symbol(symbol).kind.clone();
                let target_type = match symbol_kind {
                    SymbolKind::Variable { ty } | SymbolKind::Parameter { ty } => {
                        self.resolve_type(&ty, name.span)
                    }
                    _ => {
                        self.diagnostics.push(Diagnostic::error(
                            format!("`{}` is not an assignable variable", name.text),
                            name.span,
                        ));
                        Type::Dynamic
                    }
                };
                HirStatement::Assignment {
                    symbol,
                    value: self.check_expression(value, Some(&target_type)),
                    span,
                }
            }
            ExpressionKind::Member { base, name } => {
                let ExpressionKind::Identifier(base_name) = &base.kind else {
                    self.diagnostics.push(Diagnostic::error(
                        "field assignment currently requires a local struct variable",
                        target.span,
                    ));
                    return HirStatement::Expression(self.check_expression(value, None));
                };
                if self.type_names.contains_key(&base_name.text) {
                    self.diagnostics.push(Diagnostic::error(
                        "field assignment requires a local struct variable, not a type name",
                        base_name.span,
                    ));
                    return HirStatement::Expression(self.check_expression(value, None));
                }
                let local = self.reference_id(base_name.span);
                let symbol_kind = self.symbol(local).kind.clone();
                let base_type = match symbol_kind {
                    SymbolKind::Variable { ty } | SymbolKind::Parameter { ty } => {
                        self.resolve_type(&ty, base_name.span)
                    }
                    _ => Type::Dynamic,
                };
                let Type::Struct(struct_id) = base_type else {
                    self.diagnostics.push(Diagnostic::error(
                        "field assignment requires a struct value",
                        target.span,
                    ));
                    return HirStatement::Expression(self.check_expression(value, None));
                };
                let Some((field, field_type)) = self.struct_field(struct_id, &name.text) else {
                    self.diagnostics.push(Diagnostic::error(
                        format!("unknown struct field `{}`", name.text),
                        name.span,
                    ));
                    return HirStatement::Expression(self.check_expression(value, None));
                };
                HirStatement::FieldAssignment {
                    local,
                    struct_id,
                    field,
                    value: self.check_expression(value, Some(&field_type)),
                    span,
                }
            }
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    "assignment target must be a variable or struct field",
                    target.span,
                ));
                HirStatement::Expression(self.check_expression(value, None))
            }
        }
    }

    fn check_struct_literal(
        &mut self,
        name: &super::ast::Name,
        fields: &[super::ast::StructLiteralField],
        span: super::source::Span,
    ) -> (HirExpressionKind, Type) {
        let Some(Type::Struct(struct_id)) = self.type_names.get(&name.text).cloned() else {
            self.diagnostics.push(Diagnostic::error(
                format!("unknown struct `{}`", name.text),
                name.span,
            ));
            return (
                HirExpressionKind::StructLiteral {
                    struct_id: TypeId(u32::MAX),
                    fields: Vec::new(),
                },
                Type::Dynamic,
            );
        };
        let definition = self
            .structs
            .iter()
            .find(|item| item.id == struct_id)
            .cloned()
            .expect("resolved struct has HIR metadata");
        let mut seen = HashSet::new();
        let mut checked = Vec::new();
        for initializer in fields {
            let Some((index, field)) = definition
                .fields
                .iter()
                .enumerate()
                .find(|(_, field)| field.name == initializer.name.text)
            else {
                self.diagnostics.push(Diagnostic::error(
                    format!(
                        "unknown field `{}` in struct `{}`",
                        initializer.name.text, name.text
                    ),
                    initializer.name.span,
                ));
                self.check_expression(&initializer.value, None);
                continue;
            };
            if !seen.insert(index as u32) {
                self.diagnostics.push(Diagnostic::error(
                    format!("duplicate struct literal field `{}`", initializer.name.text),
                    initializer.name.span,
                ));
            }
            checked.push((
                index as u32,
                self.check_expression(&initializer.value, Some(&field.ty)),
            ));
        }
        for (index, field) in definition.fields.iter().enumerate() {
            if !seen.contains(&(index as u32)) {
                self.diagnostics.push(Diagnostic::error(
                    format!("missing field `{}` in struct `{}`", field.name, name.text),
                    span,
                ));
            }
        }
        (
            HirExpressionKind::StructLiteral {
                struct_id,
                fields: checked,
            },
            Type::Struct(struct_id),
        )
    }

    fn check_member_expression(
        &mut self,
        base: &Expression,
        name: &super::ast::Name,
        span: super::source::Span,
    ) -> (HirExpressionKind, Type) {
        if let ExpressionKind::Identifier(type_name) = &base.kind
            && let Some(Type::Enum(enum_id)) = self.type_names.get(&type_name.text).cloned()
        {
            let definition = self
                .enums
                .iter()
                .find(|item| item.id == enum_id)
                .expect("resolved enum has HIR metadata");
            if let Some(variant) = definition
                .variants
                .iter()
                .position(|variant| variant == &name.text)
            {
                return (
                    HirExpressionKind::EnumValue {
                        enum_id,
                        variant: variant as u32,
                    },
                    Type::Enum(enum_id),
                );
            }
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "unknown variant `{}` for enum `{}`",
                    name.text, type_name.text
                ),
                name.span,
            ));
            return (
                HirExpressionKind::EnumValue {
                    enum_id,
                    variant: u32::MAX,
                },
                Type::Enum(enum_id),
            );
        }

        if let ExpressionKind::Identifier(type_name) = &base.kind
            && self.type_names.contains_key(&type_name.text)
        {
            self.diagnostics.push(Diagnostic::error(
                format!("`{}` is a type, not a struct value", type_name.text),
                type_name.span,
            ));
            return (
                HirExpressionKind::FieldLoad {
                    local: SymbolId(u32::MAX),
                    struct_id: TypeId(u32::MAX),
                    field: u32::MAX,
                },
                Type::Dynamic,
            );
        }

        let checked_base = self.check_expression(base, None);
        let Type::Struct(struct_id) = checked_base.ty else {
            self.diagnostics.push(Diagnostic::error(
                "field access requires a struct value",
                span,
            ));
            return (
                HirExpressionKind::FieldLoad {
                    local: SymbolId(u32::MAX),
                    struct_id: TypeId(u32::MAX),
                    field: u32::MAX,
                },
                Type::Dynamic,
            );
        };
        let HirExpressionKind::Symbol(local) = checked_base.kind else {
            self.diagnostics.push(Diagnostic::error(
                "field access currently requires a local struct variable",
                span,
            ));
            return (
                HirExpressionKind::FieldLoad {
                    local: SymbolId(u32::MAX),
                    struct_id,
                    field: u32::MAX,
                },
                Type::Dynamic,
            );
        };
        let Some((field, ty)) = self.struct_field(struct_id, &name.text) else {
            self.diagnostics.push(Diagnostic::error(
                format!("unknown struct field `{}`", name.text),
                name.span,
            ));
            return (
                HirExpressionKind::FieldLoad {
                    local,
                    struct_id,
                    field: u32::MAX,
                },
                Type::Dynamic,
            );
        };
        (
            HirExpressionKind::FieldLoad {
                local,
                struct_id,
                field,
            },
            ty,
        )
    }

    fn struct_field(&self, id: TypeId, name: &str) -> Option<(u32, Type)> {
        self.structs
            .iter()
            .find(|item| item.id == id)?
            .fields
            .iter()
            .enumerate()
            .find(|(_, field)| field.name == name)
            .map(|(index, field)| (index as u32, field.ty.clone()))
    }

    fn resolve_type(&mut self, ty: &Type, span: super::source::Span) -> Type {
        match ty {
            Type::Named(name) => self.type_names.get(name).cloned().unwrap_or_else(|| {
                self.diagnostics
                    .push(Diagnostic::error(format!("unknown type `{name}`"), span));
                Type::Dynamic
            }),
            Type::Array { element, length } => Type::Array {
                element: Box::new(self.resolve_type(element, span)),
                length: *length,
            },
            Type::List(element) => Type::List(Box::new(self.resolve_type(element, span))),
            Type::Tuple(elements) => Type::Tuple(
                elements
                    .iter()
                    .map(|element| self.resolve_type(element, span))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    fn declaration_id(&self, span: super::source::Span) -> SymbolId {
        self.resolution
            .declaration(span)
            .expect("resolver recorded every valid declaration")
    }
    fn reference_id(&self, span: super::source::Span) -> SymbolId {
        self.resolution
            .reference(span)
            .expect("resolver recorded every valid reference")
    }
    fn symbol(&self, id: SymbolId) -> &super::resolution::Symbol {
        self.resolution
            .symbol(id)
            .expect("symbol ID came from this resolution")
    }

    fn validate_integer_magnitude(
        &mut self,
        value: &str,
        maximum: u128,
        span: super::source::Span,
    ) {
        if value
            .parse::<u128>()
            .map_or(true, |magnitude| magnitude > maximum)
        {
            self.diagnostics.push(Diagnostic::error(
                "integer literal is outside the signed 64-bit range",
                span,
            ));
        }
    }
}

fn block_definitely_returns(block: &super::ast::Block) -> bool {
    block
        .statements
        .iter()
        .any(|statement| match &statement.kind {
            StatementKind::Return(_) => true,
            StatementKind::If {
                then_block,
                else_block: Some(else_block),
                ..
            } => block_definitely_returns(then_block) && block_definitely_returns(else_block),
            _ => false,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::{lexer::lex, parser::parse, source::FileId};

    fn analyze_source(source: &str) -> Result<HirModule, Vec<Diagnostic>> {
        analyze(&parse(lex(FileId(0), source).unwrap()).unwrap())
    }

    #[test]
    fn accepts_exact_types_and_known_call_arity() {
        analyze_source("private int id(int x) { return x; } public int run() { return id(1); }")
            .unwrap();
    }

    #[test]
    fn rejects_obvious_type_mismatch() {
        let errors =
            analyze_source("private int f() { int value = 1.5; return value; }").unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("expected `int`, found `float`"))
        );
    }

    #[test]
    fn checks_fixed_array_length() {
        let errors =
            analyze_source("private void f() { int[2] value = [1]; return; }").unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("array length is 2"))
        );
    }

    #[test]
    fn checks_documented_list_declaration_form() {
        analyze_source("private void f() { list int values = [1, 2, 3]; return; }").unwrap();
    }

    #[test]
    fn checks_call_arity() {
        let errors =
            analyze_source("private int id(int x) { return x; } private int f() { return id(); }")
                .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("expects 1 argument"))
        );
    }

    #[test]
    fn validates_executable_main_contract() {
        let hir = analyze_source("public void main() {}").unwrap();
        assert!(validate_executable(&hir).is_ok());

        let hir = analyze_source("public int main() { return 0; }").unwrap();
        assert!(
            validate_executable(&hir).unwrap_err()[0]
                .message
                .contains("must return `void`")
        );

        let hir = analyze_source("public void main(int value) {}").unwrap();
        assert!(
            validate_executable(&hir).unwrap_err()[0]
                .message
                .contains("no parameters")
        );
    }

    #[test]
    fn rejects_duplicate_main() {
        let errors = analyze_source("public void main() {} public void main() {}").unwrap_err();
        assert!(errors[0].message.contains("duplicate declaration"));
    }

    #[test]
    fn accepts_signed_64_bit_integer_bounds() {
        analyze_source(
            "private int low() { return -9223372036854775808; } private int high() { return 9223372036854775807; }",
        )
        .unwrap();
    }

    #[test]
    fn validates_bool_conditions_and_comparison_results() {
        let hir = analyze_source(
            "private void f() { bool result = 1 < 2; if (result) {} while (false) { break; } }",
        )
        .unwrap();
        let HirStatement::Variable { initializer, .. } = &hir.functions[0].body.statements[0]
        else {
            panic!("expected variable declaration");
        };
        assert_eq!(initializer.ty, Type::Bool);
    }

    #[test]
    fn rejects_non_bool_conditions_and_invalid_boolean_operators() {
        let errors = analyze_source(
            "private void f() { if (1) {} while (2) {} bool a = !1; bool b = true && 1; bool c = false || 2; }",
        )
        .unwrap_err();
        assert!(
            errors
                .iter()
                .filter(|error| error.message.contains("expected `bool`"))
                .count()
                >= 5
        );
    }

    #[test]
    fn rejects_incompatible_comparisons() {
        let errors =
            analyze_source("private void f() { bool a = true < false; bool b = 1 == true; }")
                .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("expected `int`"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("equality requires matching"))
        );
    }

    #[test]
    fn validates_assignment_targets_and_types() {
        analyze_source("private void f() { int x = 1; x = 2; }").unwrap();
        let mismatch = analyze_source("private void f() { int x = 1; x = false; }").unwrap_err();
        assert!(
            mismatch
                .iter()
                .any(|error| error.message.contains("expected `int`"))
        );
        let unknown = analyze_source("private void f() { missing = 1; }").unwrap_err();
        assert!(
            unknown
                .iter()
                .any(|error| error.message.contains("unknown identifier"))
        );
    }

    #[test]
    fn rejects_break_and_continue_outside_loops() {
        let errors = analyze_source("private void f() { break; continue; }").unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("`break`")));
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("`continue`"))
        );
    }

    fn messages(source: &str) -> Vec<String> {
        analyze_source(source)
            .unwrap_err()
            .into_iter()
            .map(|error| error.message)
            .collect()
    }

    #[test]
    fn rejects_duplicate_struct_names() {
        assert!(
            messages("struct A {} struct A {}")
                .iter()
                .any(|message| message.contains("duplicate type"))
        );
    }

    #[test]
    fn rejects_duplicate_struct_fields() {
        assert!(
            messages("struct A { int x; int x; }")
                .iter()
                .any(|message| message.contains("duplicate struct field"))
        );
    }

    #[test]
    fn rejects_unknown_struct() {
        assert!(
            messages("private void f() { Missing x = Missing {}; }")
                .iter()
                .any(|message| message.contains("unknown struct")
                    || message.contains("unknown type"))
        );
    }

    #[test]
    fn rejects_missing_struct_field() {
        assert!(
            messages("struct A { int x; int y; } private void f() { A a = A { x: 1 }; }")
                .iter()
                .any(|message| message.contains("missing field `y`"))
        );
    }

    #[test]
    fn rejects_duplicate_struct_literal_field() {
        assert!(
            messages("struct A { int x; } private void f() { A a = A { x: 1, x: 2 }; }")
                .iter()
                .any(|message| message.contains("duplicate struct literal field"))
        );
    }

    #[test]
    fn rejects_unknown_struct_literal_field() {
        assert!(
            messages("struct A { int x; } private void f() { A a = A { x: 1, y: 2 }; }")
                .iter()
                .any(|message| message.contains("unknown field `y`"))
        );
    }

    #[test]
    fn rejects_wrong_struct_field_type() {
        assert!(
            messages("struct A { int x; } private void f() { A a = A { x: true }; }")
                .iter()
                .any(|message| message.contains("expected `int`, found `bool`"))
        );
    }

    #[test]
    fn rejects_field_access_on_non_struct() {
        assert!(
            messages("private void f() { int x = 1; int y = x.nope; }")
                .iter()
                .any(|message| message.contains("field access requires a struct"))
        );
    }

    #[test]
    fn rejects_invalid_field_assignment() {
        assert!(
            messages("struct A { int x; } private void f() { A a = A { x: 1 }; a.x = false; }")
                .iter()
                .any(|message| message.contains("expected `int`, found `bool`"))
        );
    }

    #[test]
    fn rejects_unknown_field_access() {
        assert!(
            messages("struct A { int x; } private void f() { A a = A { x: 1 }; int y = a.nope; }")
                .iter()
                .any(|message| message.contains("unknown struct field `nope`"))
        );
    }

    #[test]
    fn rejects_nested_struct_layout() {
        assert!(
            messages("struct Inner { int x; } struct Outer { Inner inner; }")
                .iter()
                .any(|message| message.contains("nested struct fields are not supported"))
        );
    }

    #[test]
    fn rejects_type_name_used_as_field_value_without_panicking() {
        assert!(
            messages("struct A { int x; } private void f() { int y = A.x; }")
                .iter()
                .any(|message| message.contains("is a type, not a struct value"))
        );
    }

    #[test]
    fn rejects_duplicate_enum_names() {
        assert!(
            messages("enum A { one } enum A { two }")
                .iter()
                .any(|message| message.contains("duplicate type"))
        );
    }

    #[test]
    fn rejects_duplicate_enum_variants() {
        assert!(
            messages("enum A { one, one }")
                .iter()
                .any(|message| message.contains("duplicate enum variant"))
        );
    }

    #[test]
    fn rejects_unknown_enum_variant() {
        assert!(
            messages("enum A { one } private void f() { A a = A.two; }")
                .iter()
                .any(|message| message.contains("unknown variant `two`"))
        );
    }

    #[test]
    fn rejects_wrong_enum_assignment() {
        assert!(
            messages("enum A { one } enum B { one } private void f() { A a = B.one; }")
                .iter()
                .any(|message| message.contains("type mismatch"))
        );
    }

    #[test]
    fn rejects_comparison_between_different_enums() {
        assert!(
            messages(
                "enum A { one } enum B { one } private void f() { bool same = A.one == B.one; }"
            )
            .iter()
            .any(|message| message.contains("equality requires matching"))
        );
    }

    #[test]
    fn rejects_enum_ordering_comparison() {
        assert!(
            messages("enum A { one, two } private void f() { bool less = A.one < A.two; }")
                .iter()
                .any(|message| message.contains("enums support only"))
        );
    }

    #[test]
    fn accepts_struct_containing_enum() {
        analyze_source("enum Kind { first, second } struct Item { Kind kind; int value; } private int f() { Item item = Item { kind: Kind.first, value: 1 }; item.value = 2; if (item.kind == Kind.first) { return item.value; } return 0; }").unwrap();
    }
}
