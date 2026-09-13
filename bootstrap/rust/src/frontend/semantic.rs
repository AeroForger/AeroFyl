use super::ast::{Expression, ExpressionKind, Literal, Module, StatementKind, Visibility};
use super::diagnostics::Diagnostic;
use super::resolution::{Resolution, SymbolId, SymbolKind, resolve};
use super::types::Type;
use crate::middle::hir::*;

pub fn analyze(module: &Module) -> Result<HirModule, Vec<Diagnostic>> {
    let resolution = resolve(module)?;
    Analyzer {
        resolution: &resolution,
        diagnostics: Vec::new(),
    }
    .analyze_module(module)
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
}

impl Analyzer<'_> {
    fn analyze_module(mut self, module: &Module) -> Result<HirModule, Vec<Diagnostic>> {
        let mut functions = Vec::new();
        for function in &module.functions {
            let symbol = self.declaration_id(function.name.span);
            let parameters = function
                .parameters
                .iter()
                .map(|parameter| HirParameter {
                    symbol: self.declaration_id(parameter.name.span),
                    ty: parameter.ty.kind.clone(),
                })
                .collect();
            let body = self.analyze_block(&function.body, &function.return_type.kind, 0);
            if function.return_type.kind == Type::Int && !block_definitely_returns(&function.body) {
                self.diagnostics.push(Diagnostic::error(
                    "an `int` function must return an `int` value",
                    function.name.span,
                ));
            }
            functions.push(HirFunction {
                symbol,
                name: function.name.text.clone(),
                name_span: function.name.span,
                visibility: function.visibility,
                return_type: function.return_type.kind.clone(),
                parameters,
                body,
                span: function.span,
            });
        }
        if self.diagnostics.is_empty() {
            Ok(HirModule {
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
                    let initializer =
                        self.check_expression(&variable.initializer, Some(&variable.ty.kind));
                    HirStatement::Variable {
                        symbol: self.declaration_id(variable.name.span),
                        ty: variable.ty.kind.clone(),
                        initializer,
                        span: statement.span,
                    }
                }
                StatementKind::Assignment(assignment) => {
                    let symbol = self.reference_id(assignment.target.span);
                    let target_type = match &self.symbol(symbol).kind {
                        SymbolKind::Variable { ty } | SymbolKind::Parameter { ty } => ty.clone(),
                        _ => {
                            self.diagnostics.push(Diagnostic::error(
                                format!(
                                    "`{}` is not an assignable variable",
                                    assignment.target.text
                                ),
                                assignment.target.span,
                            ));
                            Type::Dynamic
                        }
                    };
                    let value = self.check_expression(&assignment.value, Some(&target_type));
                    HirStatement::Assignment {
                        symbol,
                        value,
                        span: statement.span,
                    }
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
                let ty = match &self.symbol(id).kind {
                    SymbolKind::Variable { ty } | SymbolKind::Parameter { ty } => ty.clone(),
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
                    | super::ast::BinaryOperator::GreaterEqual => (
                        self.check_expression(left, Some(&Type::Int)),
                        self.check_expression(right, Some(&Type::Int)),
                        Type::Bool,
                    ),
                    super::ast::BinaryOperator::LogicalAnd
                    | super::ast::BinaryOperator::LogicalOr => (
                        self.check_expression(left, Some(&Type::Bool)),
                        self.check_expression(right, Some(&Type::Bool)),
                        Type::Bool,
                    ),
                    super::ast::BinaryOperator::Equal | super::ast::BinaryOperator::NotEqual => {
                        let left = self.check_expression(left, None);
                        let right = self.check_expression(right, None);
                        if left.ty != right.ty || !matches!(left.ty, Type::Int | Type::Bool) {
                            self.diagnostics.push(Diagnostic::error(
                                "equality requires matching `int` or matching `bool` operands",
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
}
