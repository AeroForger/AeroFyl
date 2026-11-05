use std::collections::{HashMap, HashSet};

use super::ast::{
    Expression, ExpressionKind, Literal, Module, Statement, StatementKind, Visibility,
};
use super::diagnostics::Diagnostic;
use super::resolution::{NamespaceIdx, Resolution, SymbolId, SymbolKind, resolve};
use super::types::{Type, TypeId};
use crate::middle::hir::*;

pub fn analyze(
    module: &Module,
    file_names: &HashMap<super::source::FileId, String>,
) -> Result<HirModule, Vec<Diagnostic>> {
    let resolution = resolve(module, file_names)?;
    let mut analyzer = Analyzer {
        resolution: &resolution,
        diagnostics: Vec::new(),
        structs: Vec::new(),
        enums: Vec::new(),
        type_names: HashMap::new(),
        current_ancestors: Vec::new(),
    };
    analyzer.collect_types(module);
    analyzer.analyze_module(module)
}

/// Validates the additional source-level contract required for an executable.
pub fn validate_executable(module: &HirModule) -> Result<SymbolId, Vec<Diagnostic>> {
    let main_functions: Vec<_> = module
        .functions
        .iter()
        .filter(|function| function.name == "main" && function.name_span.file == module.span.file)
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
    if !matches!(
        main.parameters.as_slice(),
        [] | [HirParameter {
            ty: Type::CliArgs,
            ..
        }]
    ) {
        diagnostics.push(Diagnostic::error(
            "executable entry function `main` must take no parameters or one `string[]` parameter",
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
    /// Named types keyed by their canonical dotted namespace path.
    type_names: HashMap<String, Type>,
    /// Enclosing namespace chain of the declaration being checked, most
    /// specific first.
    current_ancestors: Vec<NamespaceIdx>,
}

impl Analyzer<'_> {
    fn collect_types(&mut self, module: &Module) {
        let mut structs: Vec<(&super::ast::StructDeclaration, String, NamespaceIdx)> = Vec::new();
        let mut enums: Vec<(&super::ast::EnumDeclaration, String, NamespaceIdx)> = Vec::new();
        for declaration in &module.structs {
            structs.push((
                declaration,
                self.type_declaration_path_struct(module, declaration),
                self.declaration_namespace(module, declaration.name.span),
            ));
        }
        for declaration in &module.enums {
            enums.push((
                declaration,
                self.type_declaration_path(
                    module,
                    &declaration.name,
                    self.declaration_namespace(module, declaration.name.span),
                ),
                self.declaration_namespace(module, declaration.name.span),
            ));
        }
        for declaration in &module.namespaces {
            let Some(root) = self
                .resolution
                .root_namespace_of_file(declaration.name.span.file)
            else {
                continue;
            };
            self.collect_namespace_types(root, declaration, &mut structs, &mut enums);
        }

        for (_, path, _) in &structs {
            let id = TypeId(self.type_names.len() as u32);
            self.type_names
                .entry(path.clone())
                .or_insert(Type::Struct(id));
        }
        for (_, path, _) in &enums {
            let id = TypeId(self.type_names.len() as u32);
            self.type_names
                .entry(path.clone())
                .or_insert(Type::Enum(id));
        }
        for (declaration, path, namespace) in &structs {
            let Some(Type::Struct(id)) = self.type_names.get(path).cloned() else {
                continue;
            };
            let ancestors = self.resolution.namespace_ancestors_from(*namespace);
            self.current_ancestors = ancestors;
            let fields = declaration
                .fields
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    let ty = self.resolve_type(&field.ty.kind, field.ty.span);
                    if !self.is_supported_value_type(&ty) {
                        self.diagnostics.push(Diagnostic::error(
                            "bootstrap struct fields do not support this type",
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
        for (declaration, path, namespace) in &enums {
            let Some(Type::Enum(id)) = self.type_names.get(path).cloned() else {
                continue;
            };
            self.current_ancestors = self.resolution.namespace_ancestors_from(*namespace);
            let variants = declaration
                .variants
                .iter()
                .map(|variant| HirEnumVariant {
                    name: variant.name.text.clone(),
                    payload: variant
                        .payload
                        .as_ref()
                        .map(|payload| self.resolve_type(&payload.kind, payload.span)),
                })
                .collect();
            self.enums.push(HirEnum {
                id,
                name: declaration.name.text.clone(),
                variants,
            });
        }
        for (declaration, _, namespace) in &enums {
            self.current_ancestors = self.resolution.namespace_ancestors_from(*namespace);
            for variant in &declaration.variants {
                if let Some(payload) = &variant.payload {
                    let ty = self.resolve_type(&payload.kind, payload.span);
                    if !self.is_supported_value_type(&ty) {
                        self.diagnostics.push(Diagnostic::error(
                            "payload enum variants require a supported value type",
                            payload.span,
                        ));
                    }
                }
            }
        }
        for (declaration, path, namespace) in &structs {
            let Some(Type::Struct(owner)) = self.type_names.get(path).cloned() else {
                continue;
            };
            let ancestors = self.resolution.namespace_ancestors_from(*namespace);
            self.current_ancestors = ancestors;
            for field in &declaration.fields {
                let field_type = self.resolve_type(&field.ty.kind, field.ty.span);
                if let Type::Struct(nested) = field_type
                    && self.direct_struct_reaches(nested, owner, &mut HashSet::new())
                {
                    self.diagnostics.push(Diagnostic::error(
                        "recursive struct fields require explicit indirection such as `ref T`, `optional ref T`, or a collection",
                        field.ty.span,
                    ));
                }
            }
        }
    }

    /// Canonical namespace path of one struct declaration. The compiler-built
    /// `Process` struct spans the `std.os` import instead of a real
    /// declaration site, so it is keyed as `std.os.Process`.
    fn type_declaration_path_struct(
        &self,
        module: &Module,
        declaration: &super::ast::StructDeclaration,
    ) -> String {
        if Self::is_injected_process(module, declaration.name.span) {
            return "std.os.Process".to_owned();
        }
        self.type_declaration_path(
            module,
            &declaration.name,
            self.declaration_namespace(module, declaration.name.span),
        )
    }

    fn type_declaration_path(
        &self,
        module: &Module,
        name: &super::ast::Name,
        fallback: NamespaceIdx,
    ) -> String {
        if Self::is_injected_process(module, name.span) {
            return "std.os.Process".to_owned();
        }
        format!("{}.{}", self.resolution.namespace(fallback).path, name.text)
    }

    /// The namespace a top-level declaration belongs to: always the root
    /// namespace of its file.
    fn declaration_namespace(&self, _module: &Module, span: super::source::Span) -> NamespaceIdx {
        self.resolution
            .root_namespace_of_file(span.file)
            .unwrap_or(usize::MAX)
    }

    fn collect_namespace_types<'a>(
        &mut self,
        parent: NamespaceIdx,
        declaration: &'a super::ast::NamespaceDeclaration,
        structs: &mut Vec<(&'a super::ast::StructDeclaration, String, NamespaceIdx)>,
        enums: &mut Vec<(&'a super::ast::EnumDeclaration, String, NamespaceIdx)>,
    ) {
        let Some(namespace) = self.child_namespace(parent, &declaration.name.text) else {
            return;
        };
        for type_declaration in &declaration.structs {
            let path = format!(
                "{}.{}",
                self.resolution.namespace(namespace).path,
                type_declaration.name.text
            );
            structs.push((type_declaration, path, namespace));
        }
        for type_declaration in &declaration.enums {
            let path = format!(
                "{}.{}",
                self.resolution.namespace(namespace).path,
                type_declaration.name.text
            );
            enums.push((type_declaration, path, namespace));
        }
        for child in &declaration.namespaces {
            self.collect_namespace_types(namespace, child, structs, enums);
        }
    }

    fn child_namespace(&self, parent: NamespaceIdx, name: &str) -> Option<NamespaceIdx> {
        let symbol = self
            .resolution
            .namespace(parent)
            .members
            .get(name)
            .copied()?;
        match self.resolution.symbol(symbol)?.kind.clone() {
            SymbolKind::Module { namespace } => Some(namespace),
            _ => None,
        }
    }

    fn is_injected_process(module: &Module, span: super::source::Span) -> bool {
        module
            .imports
            .iter()
            .any(|import| import.name.text == "std.os" && import.name.span == span)
    }

    fn direct_struct_reaches(
        &self,
        current: TypeId,
        target: TypeId,
        visiting: &mut HashSet<TypeId>,
    ) -> bool {
        if current == target {
            return true;
        }
        if !visiting.insert(current) {
            return false;
        }
        let reaches = self
            .structs
            .iter()
            .find(|item| item.id == current)
            .is_some_and(|item| {
                item.fields.iter().any(|field| {
                    matches!(field.ty, Type::Struct(next) if self.direct_struct_reaches(next, target, visiting))
                })
            });
        visiting.remove(&current);
        reaches
    }

    fn analyze_module(mut self, module: &Module) -> Result<HirModule, Vec<Diagnostic>> {
        let mut functions = Vec::new();
        for function in &module.functions {
            if let Some(root) = self
                .resolution
                .root_namespace_of_file(function.name.span.file)
            {
                self.current_ancestors = vec![root];
            }
            functions.push(self.check_function(function));
        }
        for declaration in &module.namespaces {
            if let Some(root) = self
                .resolution
                .root_namespace_of_file(declaration.name.span.file)
            {
                self.collect_namespace_functions(root, declaration, &mut functions);
            }
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

    fn check_function(&mut self, function: &super::ast::Function) -> HirFunction {
        let symbol = self.declaration_id(function.name.span);
        let return_type = self.resolve_type(&function.return_type.kind, function.return_type.span);
        self.validate_collection_type(&return_type, function.return_type.span);
        if return_type == Type::CliArgs {
            self.diagnostics.push(Diagnostic::error(
                "`string[]` cannot be used as a return type",
                function.return_type.span,
            ));
        }
        let parameters = function
            .parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                let ty = self.resolve_type(&parameter.ty.kind, parameter.ty.span);
                self.validate_collection_type(&ty, parameter.ty.span);
                if ty == Type::CliArgs
                    && !(function.name.text == "main"
                        && self.current_ancestors.len() == 1
                        && function.parameters.len() == 1
                        && index == 0)
                {
                    self.diagnostics.push(Diagnostic::error(
                        "`string[]` is reserved for the sole parameter of executable `main`",
                        parameter.ty.span,
                    ));
                }
                HirParameter {
                    symbol: self.declaration_id(parameter.name.span),
                    ty,
                }
            })
            .collect();
        let body = self.analyze_block(&function.body, &return_type, 0);
        if return_type != Type::Void && !block_definitely_returns(&function.body) {
            self.diagnostics.push(Diagnostic::error(
                format!("a `{return_type}` function must return a value"),
                function.name.span,
            ));
        }
        let qualified_name = self
            .symbol(symbol)
            .path
            .clone()
            .unwrap_or_else(|| function.name.text.clone());
        HirFunction {
            symbol,
            name: function.name.text.clone(),
            qualified_name,
            name_span: function.name.span,
            visibility: function.visibility,
            return_type,
            parameters,
            body,
            span: function.span,
        }
    }

    fn collect_namespace_functions(
        &mut self,
        parent: NamespaceIdx,
        declaration: &super::ast::NamespaceDeclaration,
        functions: &mut Vec<HirFunction>,
    ) {
        let Some(namespace) = self.child_namespace(parent, &declaration.name.text) else {
            return;
        };
        self.current_ancestors = self.resolution.namespace_ancestors_from(namespace);
        for function in &declaration.functions {
            functions.push(self.check_function(function));
        }
        for child in &declaration.namespaces {
            self.collect_namespace_functions(namespace, child, functions);
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
                    self.validate_collection_type(&ty, variable.ty.span);
                    if ty == Type::CliArgs {
                        self.diagnostics.push(Diagnostic::error(
                            "`string[]` is reserved for the parameter of executable `main`",
                            variable.ty.span,
                        ));
                    }
                    if matches!(ty, Type::List(_))
                        && !matches!(
                            variable.initializer.kind,
                            ExpressionKind::Collection(_)
                                | ExpressionKind::Call { .. }
                                | ExpressionKind::MethodCall { .. }
                        )
                    {
                        self.diagnostics.push(Diagnostic::error(
                            "whole-list cloning is not supported",
                            variable.initializer.span,
                        ));
                    }
                    let initializer = self.check_expression(&variable.initializer, Some(&ty));
                    let initializer = self.struct_copy_if_symbol(initializer);
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
                        Some(value) => {
                            let value = self.check_expression(value, Some(return_type));
                            Some(self.struct_copy_if_symbol(value))
                        }
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
                StatementKind::For {
                    initializer,
                    condition,
                    increment,
                    body,
                } => {
                    let initializer = self.analyze_for_clause(initializer);
                    let condition = self.check_expression(condition, Some(&Type::Bool));
                    let increment = self.analyze_for_clause(increment);
                    let body = self.analyze_block(body, return_type, loop_depth + 1);
                    HirStatement::For {
                        initializer: Box::new(initializer),
                        condition,
                        increment: Box::new(increment),
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

    fn analyze_for_clause(&mut self, statement: &Statement) -> HirStatement {
        match &statement.kind {
            StatementKind::Variable(variable) => {
                let ty = self.resolve_type(&variable.ty.kind, variable.ty.span);
                self.validate_collection_type(&ty, variable.ty.span);
                let initializer = self.check_expression(&variable.initializer, Some(&ty));
                HirStatement::Variable {
                    symbol: self.declaration_id(variable.name.span),
                    ty,
                    initializer: self.struct_copy_if_symbol(initializer),
                    span: statement.span,
                }
            }
            StatementKind::Assignment(assignment) => {
                self.check_assignment(&assignment.target, &assignment.value, statement.span)
            }
            _ => unreachable!("parser only permits declarations and assignments in for clauses"),
        }
    }

    fn check_expression(
        &mut self,
        expression: &Expression,
        expected: Option<&Type>,
    ) -> HirExpression {
        let (kind, actual) = match &expression.kind {
            ExpressionKind::Interpolated(_) => {
                self.diagnostics.push(Diagnostic::error(
                    "interpolated strings are only supported as a direct output argument",
                    expression.span,
                ));
                (
                    HirExpressionKind::StringLiteral(String::new()),
                    Type::String,
                )
            }
            ExpressionKind::Literal(literal) => {
                let ty = match literal {
                    Literal::Integer(value) => {
                        if expected == Some(&Type::Byte) {
                            self.validate_integer_magnitude(
                                value,
                                u8::MAX as u128,
                                expression.span,
                            );
                            Type::Byte
                        } else {
                            self.validate_integer_magnitude(
                                value,
                                i64::MAX as u128,
                                expression.span,
                            );
                            Type::Int
                        }
                    }
                    Literal::Float(_) => Type::Float,
                    Literal::String(value) => {
                        return self.finish_expression(
                            expression,
                            expected,
                            HirExpressionKind::StringLiteral(value.clone()),
                            Type::String,
                        );
                    }
                    Literal::Char(value) if expected == Some(&Type::Int) => {
                        return self.finish_expression(
                            expression,
                            expected,
                            HirExpressionKind::Literal(Literal::Integer(
                                (*value as u32).to_string(),
                            )),
                            Type::Int,
                        );
                    }
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
                    SymbolKind::Function { .. }
                    | SymbolKind::Builtin
                    | SymbolKind::Module { .. }
                    | SymbolKind::NamedType { .. } => {
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
                    } => self.check_called_function(
                        id,
                        &callee.text,
                        &parameters,
                        &return_type,
                        arguments,
                        expression.span,
                    ),
                    SymbolKind::Builtin => {
                        if callee.text == "reference" {
                            if arguments.len() != 1 {
                                self.diagnostics.push(Diagnostic::error(
                                    format!(
                                        "`reference` expects exactly one argument, found {}",
                                        arguments.len()
                                    ),
                                    expression.span,
                                ));
                            }
                            let value = arguments
                                .first()
                                .map(|argument| self.check_expression(argument, None))
                                .unwrap_or(HirExpression {
                                    kind: HirExpressionKind::Collection(Vec::new()),
                                    ty: Type::Dynamic,
                                    span: expression.span,
                                });
                            for argument in arguments.iter().skip(1) {
                                self.check_expression(argument, None);
                            }
                            let ty = value.ty.clone();
                            (
                                HirExpressionKind::Reference {
                                    value: Box::new(value),
                                },
                                Type::Ref(Box::new(ty)),
                            )
                        } else if callee.text == "some" {
                            let element = match expected {
                                Some(Type::Optional(element)) => (**element).clone(),
                                _ => {
                                    self.diagnostics.push(Diagnostic::error(
                                        "`some` requires an `optional T` type context",
                                        expression.span,
                                    ));
                                    Type::Dynamic
                                }
                            };
                            if arguments.len() != 1 {
                                self.diagnostics.push(Diagnostic::error(
                                    format!(
                                        "`some` expects exactly one argument, found {}",
                                        arguments.len()
                                    ),
                                    expression.span,
                                ));
                            }
                            let value = arguments
                                .first()
                                .map(|argument| self.check_expression(argument, Some(&element)))
                                .unwrap_or(HirExpression {
                                    kind: HirExpressionKind::Collection(Vec::new()),
                                    ty: Type::Dynamic,
                                    span: expression.span,
                                });
                            let value = self.struct_copy_if_symbol(value);
                            for argument in arguments.iter().skip(1) {
                                self.check_expression(argument, None);
                            }
                            (
                                HirExpressionKind::OptionalSome {
                                    value: Box::new(value),
                                },
                                Type::Optional(Box::new(element)),
                            )
                        } else if callee.text == "none" {
                            let element = match expected {
                                Some(Type::Optional(element)) => (**element).clone(),
                                _ => {
                                    self.diagnostics.push(Diagnostic::error(
                                        "`none` requires an `optional T` type context",
                                        expression.span,
                                    ));
                                    Type::Dynamic
                                }
                            };
                            if !arguments.is_empty() {
                                self.diagnostics.push(Diagnostic::error(
                                    format!(
                                        "`none` expects no arguments, found {}",
                                        arguments.len()
                                    ),
                                    expression.span,
                                ));
                                for argument in arguments {
                                    self.check_expression(argument, None);
                                }
                            }
                            (
                                HirExpressionKind::OptionalNone,
                                Type::Optional(Box::new(element)),
                            )
                        } else if matches!(
                            callee.text.as_str(),
                            "print"
                                | "println"
                                | "eprint"
                                | "eprintln"
                                | "input"
                                | "readFile"
                                | "readBytes"
                                | "writeFile"
                                | "writeBytes"
                                | "exists"
                        ) {
                            // Standard-library builtins are only reachable as
                            // dotted namespace calls; see `check_std_call`.
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
                        } else if callee.text == "exit" {
                            if arguments.len() != 1 {
                                self.diagnostics.push(Diagnostic::error(
                                    format!(
                                        "`exit` expects exactly one argument, found {}",
                                        arguments.len()
                                    ),
                                    expression.span,
                                ));
                            }
                            let code = arguments
                                .first()
                                .map(|argument| self.check_expression(argument, Some(&Type::Int)))
                                .unwrap_or(HirExpression {
                                    kind: HirExpressionKind::Literal(Literal::Integer("0".into())),
                                    ty: Type::Dynamic,
                                    span: expression.span,
                                });
                            for argument in arguments.iter().skip(1) {
                                self.check_expression(argument, None);
                            }
                            (
                                HirExpressionKind::Exit {
                                    code: Box::new(code),
                                },
                                Type::Void,
                            )
                        } else if matches!(callee.text.as_str(), "int" | "char" | "byte") {
                            if arguments.len() != 1 {
                                self.diagnostics.push(Diagnostic::error(
                                    format!(
                                        "`{}` conversion expects exactly one argument, found {}",
                                        callee.text,
                                        arguments.len()
                                    ),
                                    expression.span,
                                ));
                            }
                            let value = arguments
                                .first()
                                .map(|argument| self.check_expression(argument, None))
                                .unwrap_or(HirExpression {
                                    kind: HirExpressionKind::Literal(Literal::Integer("0".into())),
                                    ty: Type::Dynamic,
                                    span: expression.span,
                                });
                            for argument in arguments.iter().skip(1) {
                                self.check_expression(argument, None);
                            }
                            let target = match callee.text.as_str() {
                                "int" => Type::Int,
                                "char" => Type::Char,
                                "byte" => Type::Byte,
                                _ => unreachable!(),
                            };
                            let valid = match (&value.ty, &target) {
                                (Type::Int, Type::Int)
                                | (Type::Char, Type::Char)
                                | (Type::Char, Type::Int)
                                | (Type::Byte, Type::Byte)
                                | (Type::Byte, Type::Int)
                                | (Type::Int, Type::Char)
                                | (Type::Int, Type::Byte) => true,
                                (Type::Enum(enum_id), Type::Int) => self
                                    .enums
                                    .iter()
                                    .find(|item| item.id == *enum_id)
                                    .is_some_and(|item| {
                                        item.variants
                                            .iter()
                                            .all(|variant| variant.payload.is_none())
                                    }),
                                _ => false,
                            };
                            if !valid {
                                self.diagnostics.push(Diagnostic::error(
                                    format!(
                                        "cannot explicitly convert `{}` to `{target}`",
                                        value.ty
                                    ),
                                    expression.span,
                                ));
                            }
                            (
                                HirExpressionKind::Convert {
                                    value: Box::new(value),
                                    target: target.clone(),
                                },
                                target,
                            )
                        } else {
                            // TODO(spec): other builtin signatures and behavior are missing.
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
                    super::ast::BinaryOperator::Add => {
                        let left = self.check_expression(left, None);
                        if left.ty == Type::String {
                            let right = self.check_expression(right, Some(&Type::String));
                            return self.finish_expression(
                                expression,
                                expected,
                                HirExpressionKind::StringConcat {
                                    left: Box::new(left),
                                    right: Box::new(right),
                                },
                                Type::String,
                            );
                        }
                        if left.ty != Type::Int {
                            self.diagnostics.push(Diagnostic::error(
                                "addition requires matching `int` operands or matching `string` operands",
                                expression.span,
                            ));
                        }
                        (
                            left,
                            self.check_expression(right, Some(&Type::Int)),
                            Type::Int,
                        )
                    }
                    super::ast::BinaryOperator::Subtract
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
                        if !matches!(left.ty, Type::Int | Type::Byte) {
                            self.diagnostics.push(Diagnostic::error(
                                "ordering comparison expected matching `int` or `byte` operands; enums support only `==` and `!=`",
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
                        let right = self.check_expression(right, Some(&left.ty));
                        if left.ty == Type::String && right.ty == Type::String {
                            return self.finish_expression(
                                expression,
                                expected,
                                HirExpressionKind::StringEqual {
                                    equal: *operator == super::ast::BinaryOperator::Equal,
                                    left: Box::new(left),
                                    right: Box::new(right),
                                },
                                Type::Bool,
                            );
                        }
                        if let Type::Enum(enum_id) = left.ty
                            && self
                                .enums
                                .iter()
                                .find(|item| item.id == enum_id)
                                .is_some_and(|item| {
                                    item.variants
                                        .iter()
                                        .any(|variant| variant.payload.is_some())
                                })
                        {
                            self.diagnostics.push(Diagnostic::error(
                                "payload enums must be discriminated with `.is(Enum.Variant)`",
                                expression.span,
                            ));
                        }
                        if left.ty != right.ty
                            || !matches!(
                                left.ty,
                                Type::Int | Type::Byte | Type::Bool | Type::Char | Type::Enum(_)
                            )
                        {
                            self.diagnostics.push(Diagnostic::error(
                                "equality requires matching `int`, `byte`, `bool`, `char`, or enum operands",
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
                let values: Vec<_> = values
                    .iter()
                    .map(|value| self.check_expression(value, Some(&element_type)))
                    .collect();
                let kind = match actual_type {
                    Type::Array { .. } => HirExpressionKind::ArrayLiteral(values),
                    Type::List(_) => HirExpressionKind::ListLiteral(values),
                    _ => HirExpressionKind::Collection(values),
                };
                (kind, actual_type)
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
            ExpressionKind::Index { base, index } => {
                self.check_index_expression(base, index, expression.span)
            }
            ExpressionKind::MethodCall {
                receiver,
                method,
                arguments,
            } => self.check_method_call(receiver, method, arguments, expression.span),
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

    fn finish_expression(
        &mut self,
        expression: &Expression,
        expected: Option<&Type>,
        kind: HirExpressionKind,
        actual: Type,
    ) -> HirExpression {
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
                if matches!(target_type, Type::List(_)) {
                    self.diagnostics.push(Diagnostic::error(
                        "whole-list assignment/cloning is not supported",
                        target.span,
                    ));
                }
                HirStatement::Assignment {
                    symbol,
                    value: {
                        let value = self.check_expression(value, Some(&target_type));
                        self.struct_copy_if_symbol(value)
                    },
                    span,
                }
            }
            ExpressionKind::Index { base, index } => {
                let index = self.check_expression(index, Some(&Type::Int));
                let collection = self.check_expression(base, None);
                let collection_type = collection.ty.clone();
                if matches!(collection_type, Type::List(_))
                    && matches!(
                        &collection.kind,
                        HirExpressionKind::Symbol(local)
                            if matches!(self.symbol(*local).kind, SymbolKind::Parameter { .. })
                    )
                {
                    self.diagnostics.push(Diagnostic::error(
                        "list parameters are read-only",
                        target.span,
                    ));
                }
                let element_type = match &collection_type {
                    Type::Array { element, .. } | Type::List(element) => (**element).clone(),
                    Type::String => {
                        self.diagnostics.push(Diagnostic::error(
                            "string indexing is not supported; string length is a UTF-8 byte count",
                            target.span,
                        ));
                        Type::Dynamic
                    }
                    Type::CliArgs => {
                        self.diagnostics.push(Diagnostic::error(
                            "command-line arguments are read-only",
                            target.span,
                        ));
                        Type::String
                    }
                    _ => {
                        self.diagnostics.push(Diagnostic::error(
                            "indexed assignment requires an array or list",
                            target.span,
                        ));
                        Type::Dynamic
                    }
                };
                if let ExpressionKind::Binary {
                    operator,
                    left,
                    right: _,
                } = &value.kind
                    && left.as_ref() == target
                {
                    let checked = self.check_expression(value, Some(&element_type));
                    let HirExpressionKind::Binary { right, .. } = checked.kind else {
                        unreachable!("checked compound assignment remains binary")
                    };
                    return HirStatement::CompoundIndexedAssignment {
                        collection,
                        collection_type,
                        index,
                        operator: *operator,
                        value: *right,
                        span,
                    };
                }
                HirStatement::IndexedAssignment {
                    collection,
                    collection_type,
                    index,
                    value: self.check_expression(value, Some(&element_type)),
                    span,
                }
            }
            ExpressionKind::Member { base, name } => {
                let checked_base = self.check_expression(base, None);
                if let Type::Ref(value_type) = checked_base.ty.clone() {
                    if name.text != "value" {
                        self.diagnostics.push(Diagnostic::error(
                            "reference assignment requires the `.value` property",
                            name.span,
                        ));
                    }
                    return HirStatement::ReferenceAssignment {
                        reference: checked_base,
                        value: self.check_expression(value, Some(&value_type)),
                        value_type: *value_type,
                        span,
                    };
                }
                let ExpressionKind::Identifier(base_name) = &base.kind else {
                    self.diagnostics.push(Diagnostic::error(
                        "field assignment currently requires a local struct variable",
                        target.span,
                    ));
                    return HirStatement::Expression(self.check_expression(value, None));
                };
                if self
                    .type_key(&base_name.text, base_name.span.file)
                    .is_some()
                {
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
        self.check_named_type_visibility(&name.text, name.span);
        let Some(Type::Struct(struct_id)) = self
            .type_key(&name.text, name.span.file)
            .and_then(|key| self.type_names.get(&key).cloned())
        else {
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

    /// A dotted type path receiver or base (`pack.Tag`, `en.State`) names a
    /// registered enum when chain resolution recorded module symbols before
    /// the named type. Returns the enum id so variant construction can
    /// proceed.
    fn enum_of_type_chain(&self, expression: &Expression) -> Option<TypeId> {
        let segments = flattened_names(expression)?;
        if segments.len() < 2 {
            return None;
        }
        for (index, segment) in segments.iter().enumerate() {
            let Some(id) = self.resolution.reference(segment.span) else {
                return None;
            };
            if index + 1 == segments.len() {
                let SymbolKind::NamedType { path } = self.symbol(id).kind.clone() else {
                    return None;
                };
                return match self.type_names.get(&path).cloned() {
                    Some(Type::Enum(enum_id)) => Some(enum_id),
                    _ => None,
                };
            }
            if let SymbolKind::Module { .. } = self.symbol(id).kind {
                continue;
            }
            return None;
        }
        None
    }

    fn is_std_os_process_namespace(&self, receiver: &Expression) -> bool {
        let is_reference_kind = |span: super::source::Span, want: fn(&SymbolKind) -> bool| {
            self.resolution
                .reference(span)
                .map_or(false, |id| want(&self.symbol(id).kind))
        };
        let module_path = |span: super::source::Span| -> Option<String> {
            self.resolution
                .reference(span)
                .and_then(|id| self.symbol(id).path.clone())
        };
        match &receiver.kind {
            ExpressionKind::Identifier(name) => {
                name.text == "process"
                    && is_reference_kind(name.span, |kind| matches!(kind, SymbolKind::Builtin))
            }
            ExpressionKind::Member { base, name } => {
                if name.text != "process" {
                    return false;
                }
                match &base.kind {
                    // `os.process` after `use std.os;`: `os` resolves to the
                    // `std.os` module alias, not a builtin, so look at the
                    // aliased module's path.
                    ExpressionKind::Identifier(base_name) => {
                        base_name.text == "os"
                            && module_path(base_name.span).as_deref() == Some("std.os")
                    }
                    ExpressionKind::Member {
                        base: inner_base,
                        name: inner_name,
                    } => {
                        // `std.os.process`: the chain resolver recorded the
                        // `process` builtin itself.
                        inner_name.text == "os"
                            && matches!(
                                &inner_base.kind,
                                ExpressionKind::Identifier(std_name) if std_name.text == "std",
                            )
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// Signature check shared by direct calls and namespace-qualified calls.
    fn check_called_function(
        &mut self,
        id: SymbolId,
        name: &str,
        parameters: &[Type],
        return_type: &Type,
        arguments: &[Expression],
        span: super::source::Span,
    ) -> (HirExpressionKind, Type) {
        let parameters: Vec<_> = parameters
            .iter()
            .map(|ty| self.resolve_type(ty, span))
            .collect();
        let return_type = self.resolve_type(return_type, span);
        if parameters.len() != arguments.len() {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "function `{name}` expects {} argument(s), found {}",
                    parameters.len(),
                    arguments.len()
                ),
                span,
            ));
        }
        let checked = arguments
            .iter()
            .enumerate()
            .map(|(index, argument)| {
                let value = self.check_expression(argument, parameters.get(index));
                if matches!(value.ty, Type::Struct(_)) {
                    self.struct_copy(value)
                } else {
                    value
                }
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

    /// Dispatches one `std.<module>.<name>` builtin call by its canonical
    /// path.
    fn check_std_call(
        &mut self,
        path: &str,
        arguments: &[Expression],
        span: super::source::Span,
    ) -> (HirExpressionKind, Type) {
        let name = path.rsplit_once('.').map_or(path, |(_, leaf)| leaf);
        match name {
            "print" | "println" | "eprint" | "eprintln" => {
                self.check_print_call(name, arguments, span)
            }
            "input" => self.check_input_call(arguments, span),
            "readFile" | "readBytes" => self.check_fs_read_call(name, arguments, span),
            "writeFile" | "writeBytes" => self.check_fs_write_call(name, arguments, span),
            "exists" => self.check_fs_exists_call(arguments, span),
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    format!("unknown standard-library function `{path}`"),
                    span,
                ));
                (HirExpressionKind::Collection(Vec::new()), Type::Dynamic)
            }
        }
    }

    fn check_print_call(
        &mut self,
        name: &str,
        arguments: &[Expression],
        span: super::source::Span,
    ) -> (HirExpressionKind, Type) {
        if arguments.len() != 1 {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "`{name}` expects exactly one argument, found {}",
                    arguments.len()
                ),
                span,
            ));
        }
        if let Some(Expression {
            kind: ExpressionKind::Interpolated(parts),
            ..
        }) = arguments.first()
        {
            let mut values = Vec::new();
            for part in parts {
                match part {
                    super::token::InterpolationPart::Text(text) => values.push(HirExpression {
                        kind: HirExpressionKind::StringLiteral(text.clone()),
                        ty: Type::String,
                        span,
                    }),
                    super::token::InterpolationPart::Variable(name, span) => {
                        let variable = Expression {
                            kind: ExpressionKind::Identifier(super::ast::Name {
                                text: name.clone(),
                                span: *span,
                            }),
                            span: *span,
                        };
                        values.push(self.check_expression(&variable, None));
                    }
                }
            }
            for argument in arguments.iter().skip(1) {
                self.check_expression(argument, None);
            }
            for value in &values {
                if !self.is_printable_type(&value.ty, &mut HashSet::new()) {
                    self.diagnostics.push(Diagnostic::error(
                        format!("`{name}` cannot print a `{}` value", value.ty),
                        value.span,
                    ));
                }
            }
            return (
                HirExpressionKind::PrintSeries {
                    values,
                    stderr: name.starts_with('e'),
                    newline: name.ends_with("println"),
                },
                Type::Void,
            );
        }
        let value = arguments
            .first()
            .map(|argument| self.check_expression(argument, None))
            .unwrap_or(HirExpression {
                kind: HirExpressionKind::StringLiteral(String::new()),
                ty: Type::Dynamic,
                span,
            });
        for argument in arguments.iter().skip(1) {
            self.check_expression(argument, None);
        }
        if !self.is_printable_type(&value.ty, &mut HashSet::new()) {
            self.diagnostics.push(Diagnostic::error(
                format!("`{name}` cannot print a `{}` value", value.ty),
                value.span,
            ));
        }
        (
            HirExpressionKind::Print {
                value: Box::new(value),
                stderr: name.starts_with('e'),
                newline: name.ends_with("println"),
            },
            Type::Void,
        )
    }

    fn check_input_call(
        &mut self,
        arguments: &[Expression],
        span: super::source::Span,
    ) -> (HirExpressionKind, Type) {
        if arguments.len() > 1 {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "`input` expects zero or one argument, found {}",
                    arguments.len()
                ),
                span,
            ));
        }
        let target = arguments.first().and_then(|argument| {
            let ExpressionKind::Identifier(name) = &argument.kind else {
                self.diagnostics.push(Diagnostic::error(
                    "`input` type argument must be a supported type name",
                    argument.span,
                ));
                return None;
            };
            match name.text.as_str() {
                "string" => Some(Type::String),
                "int" => Some(Type::Int),
                "bool" => Some(Type::Bool),
                "char" => Some(Type::Char),
                "float" => {
                    self.diagnostics.push(Diagnostic::error(
                        "`input(float)` is unavailable because floating-point execution is not implemented",
                        argument.span,
                    ));
                    None
                }
                _ => {
                    self.diagnostics.push(Diagnostic::error(
                        "`input` supports string, int, bool, and char",
                        argument.span,
                    ));
                    None
                }
            }
        }).unwrap_or(Type::String);
        (
            HirExpressionKind::Input {
                target: target.clone(),
            },
            target,
        )
    }

    fn check_fs_read_call(
        &mut self,
        name: &str,
        arguments: &[Expression],
        span: super::source::Span,
    ) -> (HirExpressionKind, Type) {
        if arguments.len() != 1 {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "`{name}` expects exactly one argument, found {}",
                    arguments.len()
                ),
                span,
            ));
        }
        let path = arguments
            .first()
            .map(|argument| self.check_expression(argument, Some(&Type::String)))
            .unwrap_or(HirExpression {
                kind: HirExpressionKind::StringLiteral(String::new()),
                ty: Type::Dynamic,
                span,
            });
        for argument in arguments.iter().skip(1) {
            self.check_expression(argument, None);
        }
        if name == "readFile" {
            (
                HirExpressionKind::ReadFile {
                    path: Box::new(path),
                },
                Type::String,
            )
        } else {
            (
                HirExpressionKind::ReadBytes {
                    path: Box::new(path),
                },
                Type::List(Box::new(Type::Byte)),
            )
        }
    }

    fn check_fs_write_call(
        &mut self,
        name: &str,
        arguments: &[Expression],
        span: super::source::Span,
    ) -> (HirExpressionKind, Type) {
        if arguments.len() != 2 {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "`{name}` expects exactly two arguments, found {}",
                    arguments.len()
                ),
                span,
            ));
        }
        let placeholder = || HirExpression {
            kind: HirExpressionKind::StringLiteral(String::new()),
            ty: Type::Dynamic,
            span,
        };
        let path = arguments
            .first()
            .map(|argument| self.check_expression(argument, Some(&Type::String)))
            .unwrap_or_else(&placeholder);
        let data_type = if name == "writeFile" {
            Type::String
        } else {
            Type::List(Box::new(Type::Byte))
        };
        let data = arguments
            .get(1)
            .map(|argument| self.check_expression(argument, Some(&data_type)))
            .unwrap_or_else(placeholder);
        for argument in arguments.iter().skip(2) {
            self.check_expression(argument, None);
        }
        let kind = if name == "writeFile" {
            HirExpressionKind::WriteFile {
                path: Box::new(path),
                data: Box::new(data),
            }
        } else {
            HirExpressionKind::WriteBytes {
                path: Box::new(path),
                data: Box::new(data),
            }
        };
        (kind, Type::Void)
    }

    fn check_fs_exists_call(
        &mut self,
        arguments: &[Expression],
        span: super::source::Span,
    ) -> (HirExpressionKind, Type) {
        if arguments.len() != 1 {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "`exists` expects exactly one argument, found {}",
                    arguments.len()
                ),
                span,
            ));
        }
        let path = arguments
            .first()
            .map(|argument| self.check_expression(argument, Some(&Type::String)))
            .unwrap_or(HirExpression {
                kind: HirExpressionKind::StringLiteral(String::new()),
                ty: Type::Dynamic,
                span,
            });
        for argument in arguments.iter().skip(1) {
            self.check_expression(argument, None);
        }
        (
            HirExpressionKind::Exists {
                path: Box::new(path),
            },
            Type::Bool,
        )
    }

    fn check_process_execute(
        &mut self,
        arguments: &[Expression],
        span: super::source::Span,
    ) -> (HirExpressionKind, Type) {
        if arguments.len() != 1 {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "`execute` expects exactly one argument, found {}",
                    arguments.len()
                ),
                span,
            ));
        }
        let command = arguments
            .first()
            .map(|argument| self.check_expression(argument, Some(&Type::String)))
            .unwrap_or(HirExpression {
                kind: HirExpressionKind::StringLiteral(String::new()),
                ty: Type::Dynamic,
                span,
            });
        for argument in arguments.iter().skip(1) {
            self.check_expression(argument, None);
        }
        let Some(Type::Struct(struct_id)) = self.type_names.get("std.os.Process").cloned() else {
            self.diagnostics.push(Diagnostic::error(
                "`std.os` did not provide the `Process` type",
                span,
            ));
            return (HirExpressionKind::Collection(Vec::new()), Type::Dynamic);
        };
        (
            HirExpressionKind::ProcessExecute {
                command: Box::new(command),
            },
            Type::Struct(struct_id),
        )
    }

    /// Builds one enum variant value from a call or bare member when the
    /// receiver or base is a dotted type path. Shared by local and
    /// namespace-qualified type chains.
    fn construct_enum_variant(
        &mut self,
        definition: &HirEnum,
        variant_name: &str,
        arguments: &[Expression],
        span: super::source::Span,
    ) -> (HirExpressionKind, Type) {
        let enum_id = definition.id;
        let Some((variant, metadata)) = definition
            .variants
            .iter()
            .enumerate()
            .find(|(_, variant)| variant.name == variant_name)
        else {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "unknown variant `{variant_name}` for enum `{}`",
                    definition.name
                ),
                span,
            ));
            return (HirExpressionKind::Collection(Vec::new()), Type::Dynamic);
        };
        let Some(payload_type) = metadata.payload.clone() else {
            if !arguments.is_empty() {
                self.diagnostics.push(Diagnostic::error(
                    format!("enum variant `{variant_name}` does not take a payload"),
                    span,
                ));
            }
            for argument in arguments {
                self.check_expression(argument, None);
            }
            return (
                HirExpressionKind::EnumValue {
                    enum_id,
                    variant: variant as u32,
                    payload: None,
                    boxed: definition
                        .variants
                        .iter()
                        .any(|variant| variant.payload.is_some()),
                },
                Type::Enum(enum_id),
            );
        };
        if arguments.len() != 1 {
            self.diagnostics.push(Diagnostic::error(
                format!("enum variant `{variant_name}` expects one payload"),
                span,
            ));
        }
        let payload = arguments
            .first()
            .map(|argument| Box::new(self.check_expression(argument, Some(&payload_type))));
        for argument in arguments.iter().skip(1) {
            self.check_expression(argument, None);
        }
        (
            HirExpressionKind::EnumValue {
                enum_id,
                variant: variant as u32,
                payload,
                boxed: true,
            },
            Type::Enum(enum_id),
        )
    }

    fn check_member_expression(
        &mut self,
        base: &Expression,
        name: &super::ast::Name,
        span: super::source::Span,
    ) -> (HirExpressionKind, Type) {
        // A value use of a namespace-qualified function or builtin.
        if self.resolution.reference(name.span).is_some_and(|id| {
            matches!(
                self.symbol(id).kind,
                SymbolKind::Function { .. } | SymbolKind::Builtin
            )
        }) {
            self.diagnostics.push(Diagnostic::error(
                format!("`{}` cannot be used as a value", name.text),
                name.span,
            ));
            return (
                HirExpressionKind::FieldLoad {
                    base: Box::new(HirExpression {
                        kind: HirExpressionKind::Collection(Vec::new()),
                        ty: Type::Dynamic,
                        span,
                    }),
                    struct_id: TypeId(u32::MAX),
                    field: u32::MAX,
                },
                Type::Dynamic,
            );
        }
        // Payload-free enum values through a dotted type path, for example
        // `en.State.idle` or `outer.inner.Tag.zero`.
        if let Some(enum_id) = self.enum_of_type_chain(base) {
            let definition = self
                .enums
                .iter()
                .find(|item| item.id == enum_id)
                .cloned()
                .expect("resolved enum has HIR metadata");
            let (kind, ty) = self.construct_enum_variant(&definition, &name.text, &[], span);
            if let HirExpressionKind::EnumValue { .. } = kind {
                return (kind, ty);
            }
            // Not a variant of this enum: fall through to the usual member
            // checks, which report the bad member access.
        }
        if let ExpressionKind::Identifier(type_name) = &base.kind
            && let Some(Type::Enum(enum_id)) = self
                .type_key(&type_name.text, type_name.span.file)
                .and_then(|key| self.type_names.get(&key).cloned())
        {
            self.check_named_type_visibility(&type_name.text, type_name.span);
            let definition = self
                .enums
                .iter()
                .find(|item| item.id == enum_id)
                .expect("resolved enum has HIR metadata");
            if let Some(variant) = definition
                .variants
                .iter()
                .position(|variant| variant.name == name.text)
            {
                if definition.variants[variant].payload.is_some() {
                    self.diagnostics.push(Diagnostic::error(
                        format!(
                            "payload enum variant `{}` must be constructed with a value",
                            name.text
                        ),
                        name.span,
                    ));
                }
                return (
                    HirExpressionKind::EnumValue {
                        enum_id,
                        variant: variant as u32,
                        payload: None,
                        boxed: definition
                            .variants
                            .iter()
                            .any(|variant| variant.payload.is_some()),
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
                    payload: None,
                    boxed: definition
                        .variants
                        .iter()
                        .any(|variant| variant.payload.is_some()),
                },
                Type::Enum(enum_id),
            );
        }

        if let ExpressionKind::Identifier(type_name) = &base.kind
            && self
                .type_key(&type_name.text, type_name.span.file)
                .is_some_and(|key| self.type_names.contains_key(&key))
        {
            self.diagnostics.push(Diagnostic::error(
                format!("`{}` is a type, not a struct value", type_name.text),
                type_name.span,
            ));
            return (
                HirExpressionKind::FieldLoad {
                    base: Box::new(HirExpression {
                        kind: HirExpressionKind::Collection(Vec::new()),
                        ty: Type::Dynamic,
                        span,
                    }),
                    struct_id: TypeId(u32::MAX),
                    field: u32::MAX,
                },
                Type::Dynamic,
            );
        }

        let checked_base = self.check_expression(base, None);
        if let Type::Optional(element) = checked_base.ty.clone() {
            return match name.text.as_str() {
                "hasValue" => (
                    HirExpressionKind::OptionalHasValue {
                        value: Box::new(checked_base),
                    },
                    Type::Bool,
                ),
                "value" => (
                    HirExpressionKind::OptionalValue {
                        value: Box::new(checked_base),
                    },
                    *element,
                ),
                _ => {
                    self.diagnostics.push(Diagnostic::error(
                        format!("unknown optional property `{}`", name.text),
                        name.span,
                    ));
                    (HirExpressionKind::Collection(Vec::new()), Type::Dynamic)
                }
            };
        }
        if let Type::Ref(element) = checked_base.ty.clone() {
            return if name.text == "value" {
                (
                    HirExpressionKind::ReferenceValue {
                        value: Box::new(checked_base),
                    },
                    *element,
                )
            } else {
                self.diagnostics.push(Diagnostic::error(
                    format!("unknown reference property `{}`", name.text),
                    name.span,
                ));
                (HirExpressionKind::Collection(Vec::new()), Type::Dynamic)
            };
        }
        if name.text == "length" {
            match &checked_base.ty {
                Type::Array { length, .. } => {
                    return (
                        HirExpressionKind::ArrayLength { length: *length },
                        Type::Int,
                    );
                }
                Type::List(_) => {
                    return (
                        HirExpressionKind::ListLength {
                            collection: Box::new(checked_base),
                        },
                        Type::Int,
                    );
                }
                Type::String => {
                    return (
                        HirExpressionKind::StringLength {
                            value: Box::new(checked_base),
                        },
                        Type::Int,
                    );
                }
                Type::CliArgs => {
                    if let HirExpressionKind::Symbol(local) = checked_base.kind {
                        return (HirExpressionKind::CliArgsLength { local }, Type::Int);
                    }
                }
                _ => {}
            }
        }
        let Type::Struct(struct_id) = checked_base.ty.clone() else {
            self.diagnostics.push(Diagnostic::error(
                "field access requires a struct value",
                span,
            ));
            return (
                HirExpressionKind::FieldLoad {
                    base: Box::new(checked_base),
                    struct_id: TypeId(u32::MAX),
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
                    base: Box::new(checked_base),
                    struct_id,
                    field: u32::MAX,
                },
                Type::Dynamic,
            );
        };
        (
            HirExpressionKind::FieldLoad {
                base: Box::new(checked_base),
                struct_id,
                field,
            },
            ty,
        )
    }

    fn check_index_expression(
        &mut self,
        base: &Expression,
        index: &Expression,
        span: super::source::Span,
    ) -> (HirExpressionKind, Type) {
        let index = self.check_expression(index, Some(&Type::Int));
        let collection = self.check_expression(base, None);
        let ty = collection.ty.clone();
        match ty {
            Type::Array { element, .. } => (
                HirExpressionKind::ArrayLoad {
                    collection: Box::new(collection),
                    index: Box::new(index),
                },
                *element,
            ),
            Type::List(element) => (
                HirExpressionKind::ListLoad {
                    collection: Box::new(collection),
                    index: Box::new(index),
                },
                *element,
            ),
            Type::CliArgs => {
                let HirExpressionKind::Symbol(local) = collection.kind else {
                    self.diagnostics.push(Diagnostic::error(
                        "command-line argument access requires the main parameter",
                        span,
                    ));
                    return (HirExpressionKind::Collection(Vec::new()), Type::Dynamic);
                };
                (
                    HirExpressionKind::CliArgLoad {
                        local,
                        index: Box::new(index),
                    },
                    Type::String,
                )
            }
            Type::String => {
                self.diagnostics.push(Diagnostic::error(
                    "string indexing is not supported; use `.length` for UTF-8 byte length",
                    span,
                ));
                (HirExpressionKind::Collection(Vec::new()), Type::Dynamic)
            }
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    "indexed access requires an array or list",
                    span,
                ));
                (HirExpressionKind::Collection(Vec::new()), Type::Dynamic)
            }
        }
    }

    fn check_method_call(
        &mut self,
        receiver: &Expression,
        method: &super::ast::Name,
        arguments: &[Expression],
        span: super::source::Span,
    ) -> (HirExpressionKind, Type) {
        if method.text == "execute" && self.is_std_os_process_namespace(receiver) {
            return self.check_process_execute(arguments, span);
        }
        // Payload enum values through a dotted type path, for example
        // `pack.Tag.one(9)` or `en.State.running(2)`.
        if let Some(enum_id) = self.enum_of_type_chain(receiver) {
            let definition = self
                .enums
                .iter()
                .find(|item| item.id == enum_id)
                .cloned()
                .expect("resolved enum has HIR metadata");
            return self.construct_enum_variant(&definition, &method.text, arguments, span);
        }
        // A namespace-qualified call records its leaf reference here.
        if let Some(id) = self.resolution.reference(method.span) {
            let symbol_kind = self.symbol(id).kind.clone();
            match symbol_kind {
                SymbolKind::Function {
                    parameters,
                    return_type,
                } => {
                    return self.check_called_function(
                        id,
                        &method.text,
                        &parameters,
                        &return_type,
                        arguments,
                        span,
                    );
                }
                SymbolKind::Builtin => {
                    if let Some(path) = self.symbol(id).path.clone() {
                        return self.check_std_call(&path, arguments, span);
                    }
                }
                _ => {
                    self.diagnostics.push(Diagnostic::error(
                        format!("`{}` is not a function", method.text),
                        method.span,
                    ));
                    return (HirExpressionKind::Collection(Vec::new()), Type::Dynamic);
                }
            }
        }
        if let ExpressionKind::Identifier(type_name) = &receiver.kind
            && let Some(Type::Enum(enum_id)) = self
                .type_key(&type_name.text, type_name.span.file)
                .and_then(|key| self.type_names.get(&key).cloned())
        {
            self.check_named_type_visibility(&type_name.text, type_name.span);
            let definition = self
                .enums
                .iter()
                .find(|item| item.id == enum_id)
                .cloned()
                .expect("resolved enum has HIR metadata");
            if let Some((variant, metadata)) = definition
                .variants
                .iter()
                .enumerate()
                .find(|(_, variant)| variant.name == method.text)
            {
                let Some(payload_type) = metadata.payload.clone() else {
                    self.diagnostics.push(Diagnostic::error(
                        format!("enum variant `{}` does not take a payload", method.text),
                        span,
                    ));
                    for argument in arguments {
                        self.check_expression(argument, None);
                    }
                    return (
                        HirExpressionKind::EnumValue {
                            enum_id,
                            variant: variant as u32,
                            payload: None,
                            boxed: true,
                        },
                        Type::Enum(enum_id),
                    );
                };
                if arguments.len() != 1 {
                    self.diagnostics.push(Diagnostic::error(
                        format!("enum variant `{}` expects one payload", method.text),
                        span,
                    ));
                }
                let payload = arguments
                    .first()
                    .map(|argument| Box::new(self.check_expression(argument, Some(&payload_type))));
                for argument in arguments.iter().skip(1) {
                    self.check_expression(argument, None);
                }
                return (
                    HirExpressionKind::EnumValue {
                        enum_id,
                        variant: variant as u32,
                        payload,
                        boxed: true,
                    },
                    Type::Enum(enum_id),
                );
            }
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "unknown variant `{}` for enum `{}`",
                    method.text, type_name.text
                ),
                method.span,
            ));
            return (HirExpressionKind::Collection(Vec::new()), Type::Dynamic);
        }

        if matches!(method.text.as_str(), "is" | "payload") {
            let checked_receiver = self.check_expression(receiver, None);
            let Type::Enum(enum_id) = checked_receiver.ty else {
                self.diagnostics.push(Diagnostic::error(
                    "enum inspection requires an enum receiver",
                    receiver.span,
                ));
                return (HirExpressionKind::Collection(Vec::new()), Type::Dynamic);
            };
            let selector = arguments.first().and_then(|argument| {
                let ExpressionKind::Member { base, name } = &argument.kind else {
                    return None;
                };
                if let ExpressionKind::Identifier(type_name) = &base.kind {
                    return (self
                        .type_key(&type_name.text, type_name.span.file)
                        .and_then(|key| self.type_names.get(&key).cloned())
                        == Some(Type::Enum(enum_id)))
                    .then_some(name);
                }
                // A dotted type path selector, for example
                // `value.payload(kinds.pack.Tag.one)`.
                if self.enum_of_type_chain(base) == Some(enum_id) {
                    return Some(name);
                }
                None
            });
            let definition = self.enums.iter().find(|item| item.id == enum_id).cloned();
            let variant = selector.and_then(|name| {
                definition
                    .as_ref()?
                    .variants
                    .iter()
                    .position(|item| item.name == name.text)
            });
            if arguments.len() != 1 || variant.is_none() {
                self.diagnostics.push(Diagnostic::error(
                    "enum inspection expects one matching `Enum.Variant` selector",
                    span,
                ));
            }
            let variant = variant.unwrap_or(usize::MAX);
            if method.text == "is" {
                return (
                    HirExpressionKind::EnumIs {
                        value: Box::new(checked_receiver),
                        enum_id,
                        variant: variant as u32,
                    },
                    Type::Bool,
                );
            }
            let payload_type = definition
                .and_then(|item| item.variants.get(variant).cloned())
                .and_then(|variant| variant.payload);
            let Some(payload_type) = payload_type else {
                self.diagnostics.push(Diagnostic::error(
                    "selected enum variant has no payload",
                    span,
                ));
                return (HirExpressionKind::Collection(Vec::new()), Type::Dynamic);
            };
            return (
                HirExpressionKind::EnumPayload {
                    value: Box::new(checked_receiver),
                    enum_id,
                    variant: variant as u32,
                    payload_type: payload_type.clone(),
                },
                payload_type,
            );
        }
        if method.text == "byte" {
            let value = self.check_expression(receiver, None);
            if value.ty != Type::String {
                self.diagnostics.push(Diagnostic::error(
                    format!(
                        "string `byte` requires a `string` receiver, found `{}`",
                        value.ty
                    ),
                    receiver.span,
                ));
            }
            if arguments.len() != 1 {
                self.diagnostics.push(Diagnostic::error(
                    format!(
                        "string `byte` expects exactly one argument, found {}",
                        arguments.len()
                    ),
                    span,
                ));
            }
            let index = arguments
                .first()
                .map(|argument| self.check_expression(argument, Some(&Type::Int)))
                .unwrap_or(HirExpression {
                    kind: HirExpressionKind::Literal(Literal::Integer("0".into())),
                    ty: Type::Dynamic,
                    span,
                });
            for argument in arguments.iter().skip(1) {
                self.check_expression(argument, None);
            }
            return (
                HirExpressionKind::StringByte {
                    value: Box::new(value),
                    index: Box::new(index),
                },
                Type::Int,
            );
        }
        if method.text == "slice" {
            let value = self.check_expression(receiver, None);
            if value.ty != Type::String {
                self.diagnostics.push(Diagnostic::error(
                    format!(
                        "string `slice` requires a `string` receiver, found `{}`",
                        value.ty
                    ),
                    receiver.span,
                ));
            }
            if arguments.len() != 2 {
                self.diagnostics.push(Diagnostic::error(
                    format!(
                        "string `slice` expects exactly two arguments, found {}",
                        arguments.len()
                    ),
                    span,
                ));
            }
            let placeholder = || HirExpression {
                kind: HirExpressionKind::Literal(Literal::Integer("0".into())),
                ty: Type::Dynamic,
                span,
            };
            let start = arguments
                .first()
                .map(|argument| self.check_expression(argument, Some(&Type::Int)))
                .unwrap_or_else(&placeholder);
            let end = arguments
                .get(1)
                .map(|argument| self.check_expression(argument, Some(&Type::Int)))
                .unwrap_or_else(placeholder);
            for argument in arguments.iter().skip(2) {
                self.check_expression(argument, None);
            }
            return (
                HirExpressionKind::StringSlice {
                    value: Box::new(value),
                    start: Box::new(start),
                    end: Box::new(end),
                },
                Type::String,
            );
        }
        let checked_receiver = self.check_expression(receiver, None);
        let Type::List(element) = checked_receiver.ty.clone() else {
            self.diagnostics.push(Diagnostic::error(
                "method calls are supported only for `list` values",
                span,
            ));
            return (HirExpressionKind::Collection(Vec::new()), Type::Dynamic);
        };
        let local = match &checked_receiver.kind {
            HirExpressionKind::Symbol(local) => Some(*local),
            _ => None,
        };
        if matches!(method.text.as_str(), "push" | "pop")
            && local.is_some_and(|local| {
                matches!(self.symbol(local).kind, SymbolKind::Parameter { .. })
            })
        {
            self.diagnostics.push(Diagnostic::error(
                "list parameters are read-only",
                receiver.span,
            ));
        }
        match method.text.as_str() {
            "push" => {
                if arguments.len() != 1 {
                    self.diagnostics.push(Diagnostic::error(
                        "list `push` expects exactly one argument",
                        span,
                    ));
                }
                let value = arguments
                    .first()
                    .map(|value| self.check_expression(value, Some(&element)))
                    .unwrap_or(HirExpression {
                        kind: HirExpressionKind::Collection(Vec::new()),
                        ty: Type::Dynamic,
                        span,
                    });
                let kind = match &checked_receiver.kind {
                    HirExpressionKind::Symbol(local) => HirExpressionKind::ListPush {
                        local: *local,
                        value: Box::new(value),
                    },
                    HirExpressionKind::FieldLoad {
                        base,
                        struct_id,
                        field,
                    } => {
                        let HirExpressionKind::Symbol(local) = base.kind else {
                            self.diagnostics.push(Diagnostic::error(
                                "list `push` requires an assignable receiver",
                                receiver.span,
                            ));
                            return (HirExpressionKind::Collection(Vec::new()), Type::Dynamic);
                        };
                        HirExpressionKind::ListPushField {
                            local,
                            struct_id: *struct_id,
                            field: *field,
                            element_type: (*element).clone(),
                            value: Box::new(value),
                        }
                    }
                    HirExpressionKind::ListLoad { collection, index } => {
                        HirExpressionKind::ListPushIndexed {
                            collection: collection.clone(),
                            collection_type: collection.ty.clone(),
                            index: index.clone(),
                            element_type: (*element).clone(),
                            value: Box::new(value),
                        }
                    }
                    _ => {
                        self.diagnostics.push(Diagnostic::error(
                            "list `push` requires an assignable receiver",
                            receiver.span,
                        ));
                        HirExpressionKind::Collection(Vec::new())
                    }
                };
                (kind, Type::Void)
            }
            "pop" => {
                if !arguments.is_empty() {
                    self.diagnostics
                        .push(Diagnostic::error("list `pop` takes no arguments", span));
                    for argument in arguments {
                        self.check_expression(argument, None);
                    }
                }
                let kind = if let Some(local) = local {
                    HirExpressionKind::ListPop { local }
                } else {
                    HirExpressionKind::ListPopValue {
                        collection: Box::new(checked_receiver),
                    }
                };
                (kind, *element)
            }
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    format!("unsupported list method `{}`", method.text),
                    method.span,
                ));
                for argument in arguments {
                    self.check_expression(argument, None);
                }
                (HirExpressionKind::Collection(Vec::new()), Type::Dynamic)
            }
        }
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
            Type::Named(name) => {
                let resolved = self
                    .type_key(name, span.file)
                    .and_then(|key| self.type_names.get(&key).cloned());
                let ty = resolved.unwrap_or(Type::Dynamic);
                if ty == Type::Dynamic {
                    self.diagnostics
                        .push(Diagnostic::error(format!("unknown type `{name}`"), span));
                }
                ty
            }
            Type::Array { element, length } => Type::Array {
                element: Box::new(self.resolve_type(element, span)),
                length: *length,
            },
            Type::List(element) => Type::List(Box::new(self.resolve_type(element, span))),
            Type::Optional(element) => Type::Optional(Box::new(self.resolve_type(element, span))),
            Type::Ref(element) => Type::Ref(Box::new(self.resolve_type(element, span))),
            Type::Tuple(elements) => Type::Tuple(
                elements
                    .iter()
                    .map(|element| self.resolve_type(element, span))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    fn check_named_type_visibility(&mut self, name: &str, span: super::source::Span) {
        if self.type_key(name, span.file).is_none() {
            self.diagnostics
                .push(Diagnostic::error(format!("unknown type `{name}`"), span));
        }
    }

    /// Canonical key of one named type as written in a context. Unqualified
    /// leaf names resolve through the enclosing namespace chain, then through
    /// `using` selections of the file, then the `std.os` `Process` legacy
    /// binding. Dotted names are canonical already.
    fn type_key(&self, name: &str, file: super::source::FileId) -> Option<String> {
        if name.contains('.') {
            if self.type_names.contains_key(name) {
                return Some(name.to_owned());
            }
            return self.qualified_type_chain_key(name);
        }
        for namespace in &self.current_ancestors {
            let candidate = format!("{}.{}", self.resolution.namespace(*namespace).path, name);
            if self.type_names.contains_key(&candidate) {
                return Some(candidate);
            }
        }
        for scoped in &self.resolution.imports {
            if !self.current_ancestors.contains(&scoped.namespace) {
                continue;
            }
            if scoped.import.name.text.starts_with("std.") {
                continue;
            }
            let Some(selected) = &scoped.import.selected else {
                // A whole-module import exposes the file's types by simple
                // name in type positions when the type is in its root
                // namespace.
                let Some(namespace) = self.resolution.namespace_for_import(&scoped.import) else {
                    continue;
                };
                let candidate = format!("{}.{}", self.resolution.namespace(namespace).path, name);
                if self.type_names.contains_key(&candidate) {
                    return Some(candidate);
                }
                continue;
            };
            if !selected.iter().any(|item| item.text == name) {
                continue;
            }
            if let Some(namespace) = self.resolution.namespace_for_import(&scoped.import) {
                let candidate = format!("{}.{}", self.resolution.namespace(namespace).path, name);
                if self.type_names.contains_key(&candidate) {
                    return Some(candidate);
                }
            }
        }
        if name == "Process"
            && self.resolution.imports.iter().any(|scoped| {
                scoped.import.name.text == "std.os" && scoped.import.span.file == file
            })
        {
            return Some("std.os.Process".to_owned());
        }
        None
    }

    /// Resolves a dotted type path used in a type position, for example
    /// `pack.Tag`, `outer.inner.Tag`, or `en.State`. The first segment is
    /// either a namespace inside the current chain, an imported module stem,
    /// or `std`; the rest walk the tree segment by segment.
    fn qualified_type_chain_key(&self, name: &str) -> Option<String> {
        let segments: Vec<&str> = name.split('.').collect();
        for namespace in &self.current_ancestors {
            if let Some(key) = self.walk_type_chain(*namespace, &segments) {
                return Some(key);
            }
        }
        for scoped in &self.resolution.imports {
            if !self.current_ancestors.contains(&scoped.namespace) {
                continue;
            }
            if scoped.import.name.text.starts_with("std.") {
                continue;
            }
            let stem = scoped.import.name.text.split('.').next()?;
            if segments.first() == Some(&stem) {
                let namespace = self.resolution.namespace_for_import(&scoped.import)?;
                let key = self.walk_type_chain(namespace, &segments[1..])?;
                return Some(key);
            }
        }
        None
    }

    /// Walks one dotted chain through a namespace. Every segment except the
    /// last must be a nested namespace; the last one must be a type.
    fn walk_type_chain(&self, start: NamespaceIdx, segments: &[&str]) -> Option<String> {
        let mut current = start;
        for (index, segment) in segments.iter().enumerate() {
            if index + 1 == segments.len() {
                let key = format!("{}.{}", self.resolution.namespace(current).path, segment);
                return self.type_names.contains_key(&key).then_some(key);
            }
            let space = self.resolution.namespace(current);
            let symbol = *space.members.get(*segment)?;
            match self.resolution.symbol(symbol)?.kind.clone() {
                crate::frontend::resolution::SymbolKind::Module { namespace } => {
                    current = namespace;
                }
                _ => return None,
            }
        }
        None
    }

    fn is_printable_type(&self, ty: &Type, visited: &mut HashSet<TypeId>) -> bool {
        match ty {
            Type::String | Type::Char | Type::Int | Type::Byte | Type::Bool => true,
            Type::Array { element, .. }
            | Type::List(element)
            | Type::Optional(element)
            | Type::Ref(element) => self.is_printable_type(element, visited),
            Type::Struct(id) => {
                if !visited.insert(*id) {
                    return true;
                }
                self.structs
                    .iter()
                    .find(|item| item.id == *id)
                    .is_some_and(|item| {
                        item.fields
                            .iter()
                            .all(|field| self.is_printable_type(&field.ty, visited))
                    })
            }
            Type::Enum(id) => {
                if !visited.insert(*id) {
                    return true;
                }
                self.enums
                    .iter()
                    .find(|item| item.id == *id)
                    .is_some_and(|item| {
                        item.variants.iter().all(|variant| {
                            variant
                                .payload
                                .as_ref()
                                .is_none_or(|payload| self.is_printable_type(payload, visited))
                        })
                    })
            }
            _ => false,
        }
    }

    fn validate_collection_type(&mut self, ty: &Type, span: super::source::Span) {
        let element = match ty {
            Type::Array { element, .. } | Type::List(element) => element,
            _ => return,
        };
        let supported = self.is_supported_value_type(element)
            && (!matches!(&**element, Type::Byte) || matches!(ty, Type::List(_)));
        if !supported {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "bootstrap collections do not support element type `{}`",
                    element
                ),
                span,
            ));
        }
    }

    fn struct_copy_if_symbol(&self, expression: HirExpression) -> HirExpression {
        let Type::Struct(struct_id) = &expression.ty else {
            return expression;
        };
        let struct_id = *struct_id;
        if !matches!(
            &expression.kind,
            HirExpressionKind::Symbol(_)
                | HirExpressionKind::OptionalValue { .. }
                | HirExpressionKind::ReferenceValue { .. }
                | HirExpressionKind::EnumPayload { .. }
        ) {
            return expression;
        }
        HirExpression {
            span: expression.span,
            ty: Type::Struct(struct_id),
            kind: HirExpressionKind::StructCopy {
                struct_id,
                source: Box::new(expression),
            },
        }
    }

    fn struct_copy(&self, expression: HirExpression) -> HirExpression {
        let Type::Struct(struct_id) = expression.ty else {
            return expression;
        };
        HirExpression {
            span: expression.span,
            ty: Type::Struct(struct_id),
            kind: HirExpressionKind::StructCopy {
                struct_id,
                source: Box::new(expression),
            },
        }
    }

    fn is_supported_value_type(&self, ty: &Type) -> bool {
        self.is_supported_value_type_inner(ty, &mut HashSet::new())
    }

    fn is_supported_value_type_inner(&self, ty: &Type, visiting: &mut HashSet<TypeId>) -> bool {
        match ty {
            Type::Int | Type::Byte | Type::Bool | Type::Char | Type::String | Type::Enum(_) => true,
            Type::Ref(element) => self.is_supported_value_type_inner(element, visiting),
            Type::Struct(id) => {
                if !visiting.insert(*id) {
                    return true;
                }
                let supported =
                    self.structs
                        .iter()
                        .find(|definition| definition.id == *id)
                        .is_none_or(|definition| {
                            definition.fields.iter().all(|field| {
                                self.is_supported_value_type_inner(&field.ty, visiting)
                            })
                        });
                visiting.remove(id);
                supported
            }
            Type::Array { element, .. } | Type::List(element) => {
                self.is_supported_value_type_inner(element, visiting)
            }
            Type::Optional(element) => self.is_supported_value_type_inner(element, visiting),
            _ => false,
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

/// Flattens a member-chain expression outside-in into its name segments
/// (`a.b.c` becomes `[a, b, c]`). Returns `None` for non-chain expressions.
fn flattened_names(expression: &super::ast::Expression) -> Option<Vec<&super::ast::Name>> {
    match &expression.kind {
        ExpressionKind::Identifier(name) => Some(vec![name]),
        ExpressionKind::Member { base, name } => {
            let mut names = flattened_names(base)?;
            names.push(name);
            Some(names)
        }
        _ => None,
    }
}

fn block_definitely_returns(block: &super::ast::Block) -> bool {
    block
        .statements
        .iter()
        .any(|statement| match &statement.kind {
            StatementKind::Return(_) => true,
            StatementKind::Expression(Expression {
                kind: ExpressionKind::Call { callee, .. },
                ..
            }) if callee.text == "exit" => true,
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
        let mut names = HashMap::new();
        names.insert(FileId(0), "test".to_owned());
        analyze(
            &parse(lex(FileId(0), source).unwrap().tokens).unwrap(),
            &names,
        )
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
    fn accepts_list_declaration_from_fresh_call_result() {
        analyze_source(
            "use std.fs; private void f(string path) { list byte copy = std.fs.readBytes(path); return; }",
        )
        .unwrap();
    }

    #[test]
    fn rejects_whole_list_cloning_from_variable() {
        let errors = messages(
            "private void f() { list int values = [1, 2, 3]; list int copy = values; return; }",
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("whole-list cloning is not supported"))
        );
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

        let hir =
            analyze_source("public void main(string[] args) { int count = args.length; }").unwrap();
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
    fn accepts_nested_struct_layout_and_rejects_direct_recursion() {
        analyze_source("struct Inner { int x; } struct Outer { Inner inner; }").unwrap();
        assert!(
            messages("struct Node { Node next; }")
                .iter()
                .any(|message| message.contains("explicit indirection"))
        );
    }

    #[test]
    fn accepts_struct_value_copies_and_struct_lists() {
        analyze_source("enum Kind { word } struct Token { Kind kind; string lexeme; int line; bool valid; char marker; } private void f() { Token a = Token { kind: Kind.word, lexeme: \"x\", line: 1, valid: true, marker: 'x' }; Token b = a; b = a; list Token tokens = []; tokens.push(b); Token first = tokens[0]; tokens[0] = first; Token last = tokens.pop(); }").unwrap();
    }

    #[test]
    fn rejects_cross_struct_copies_and_list_elements() {
        let errors = messages(
            "struct A { int value; } struct B { int value; } private void f() { A a = A { value: 1 }; B b = B { value: 2 }; b = a; A copy = b; list A values = []; values.push(b); values[0] = b; }",
        );
        assert!(
            errors
                .iter()
                .filter(|message| message.contains("type mismatch"))
                .count()
                >= 4
        );
    }

    #[test]
    fn rejects_unsupported_struct_field_layouts() {
        let errors = messages(
            "struct Bad { list int values; int[1] fixed; float number; } private void f() { list Bad values = []; }",
        );
        assert!(
            errors
                .iter()
                .any(|message| message.contains("struct fields do not support"))
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

    #[test]
    fn accepts_array_list_and_string_operations() {
        analyze_source(
            "private void f() { int[2] a = [1, 2]; a[0] = a[1]; int al = a.length; list int xs = [3, 4]; xs.push(a[0]); int x = xs.pop(); xs[0] = x; int ll = xs.length; string s = \"ab\" + \"c\"; int sl = s.length; bool same = s == \"abc\"; }",
        )
        .unwrap();
    }

    #[test]
    fn accepts_stage_one_aggregate_and_optional_interactions() {
        analyze_source(
            "struct Data { list list int rows; int[2] fixed; optional string name; } private Data make() { return Data { rows: [[1], [2, 3]], fixed: [4, 5], name: some(\"x\") }; } private int read(Data data) { data.rows[1][0] += 4; if (data.name.hasValue) { return data.fixed[0] + data.rows[1][0]; } else if (false) { return 1; } return 0; } public void main() { Data data = make(); int value = read(data); optional int absent = none(); }",
        )
        .unwrap();
    }

    #[test]
    fn rejects_optional_context_and_payload_mismatches() {
        let errors = messages(
            "private void f() { optional int a = some(\"x\"); int b = none(); optional int c = none(1); }",
        );
        assert!(
            errors
                .iter()
                .any(|message| message.contains("expected `int`, found `string`"))
        );
        assert!(
            errors
                .iter()
                .any(|message| message.contains("requires an `optional T` type context"))
        );
        assert!(
            errors
                .iter()
                .any(|message| message.contains("expects no arguments"))
        );
    }

    #[test]
    fn rejects_array_length_and_element_mismatches() {
        let errors = messages("private void f() { int[2] a = [1]; int[1] b = [true]; }");
        assert!(
            errors
                .iter()
                .any(|message| message.contains("array length is 2"))
        );
        assert!(
            errors
                .iter()
                .any(|message| message.contains("expected `int`, found `bool`"))
        );
    }

    #[test]
    fn rejects_invalid_collection_indexes_and_values() {
        let errors = messages(
            "private void f() { int[1] a = [1]; int x = a[true]; list int xs = [1]; xs.push(false); string bad = xs[0]; }",
        );
        assert!(
            errors
                .iter()
                .any(|message| message.contains("expected `int`, found `bool`"))
        );
        assert!(
            errors
                .iter()
                .any(|message| message.contains("expected `int`, found `bool`"))
        );
        assert!(
            errors
                .iter()
                .any(|message| message.contains("expected `string`, found `int`"))
        );
    }

    #[test]
    fn rejects_invalid_list_methods() {
        let errors =
            messages("private void f() { list int xs = [1]; xs.push(); xs.pop(1); xs.sort(); }");
        assert!(
            errors
                .iter()
                .any(|message| message.contains("push") && message.contains("one argument"))
        );
        assert!(
            errors
                .iter()
                .any(|message| message.contains("pop") && message.contains("no arguments"))
        );
        assert!(
            errors
                .iter()
                .any(|message| message.contains("unsupported list method `sort`"))
        );
    }

    #[test]
    fn rejects_string_ordering_and_indexing() {
        let errors = messages(
            "private void f() { string s = \"abc\"; bool before = s < \"def\"; char c = s[0]; }",
        );
        assert!(errors.iter().any(|message| message.contains("ordering") || message.contains("integer operands")));
        assert!(
            errors
                .iter()
                .any(|message| message.contains("string indexing is not supported"))
        );
    }

    #[test]
    fn rejects_unsupported_collection_element_types() {
        let errors = messages(
            "struct Item { float value; } private void f() { float[1] fs = [1.0]; list Item items = []; int[1][1] nested = [[1]]; }",
        );
        assert!(
            errors
                .iter()
                .filter(|message| message.contains("collections do not support element type"))
                .count()
                >= 2
        );
        assert!(!errors.iter().any(|message| message.contains("int[1]")));
    }

    #[test]
    fn permits_array_handle_copies_and_rejects_list_copies() {
        let errors = messages(
            "private void f() { int[1] a = [1]; int[1] b = a; a = b; list int xs = [1]; list int ys = xs; xs = ys; }",
        );
        assert!(errors.iter().any(|message| message.contains("whole-list")));
    }

    #[test]
    fn validates_string_byte_intrinsic() {
        analyze_source("private int f() { string value = \"a\"; return value.byte(0); }").unwrap();
        let errors = messages(
            "private void f() { int value = 1; int a = value.byte(0); string text = \"a\"; int b = text.byte(); int c = text.byte(0, 1); int d = text.byte(true); }",
        );
        assert!(
            errors
                .iter()
                .any(|message| message.contains("requires a `string` receiver"))
        );
        assert!(errors.iter().any(|message| message.contains("found 0")));
        assert!(errors.iter().any(|message| message.contains("found 2")));
        assert!(
            errors
                .iter()
                .any(|message| message.contains("expected `int`, found `bool`"))
        );
    }

    #[test]
    fn validates_string_slice_intrinsic() {
        analyze_source(
            "private string f(string source) { return source.slice(0, source.length); }",
        )
        .unwrap();
        let errors = messages(
            "private void f() { string source = \"abc\"; string a = source.slice(0); string b = source.slice(true, 2); int value = 1; string c = value.slice(0, 1); }",
        );
        assert!(
            errors
                .iter()
                .any(|message| message.contains("exactly two arguments"))
        );
        assert!(
            errors
                .iter()
                .any(|message| message.contains("expected `int`, found `bool`"))
        );
        assert!(
            errors
                .iter()
                .any(|message| message.contains("requires a `string` receiver"))
        );
    }

    #[test]
    fn validates_read_file_intrinsic() {
        analyze_source(
            "use std.fs; private string f(string path) { return std.fs.readFile(path); }",
        )
        .unwrap();
        let errors = messages(
            "use std.fs; private void f() { string a = std.fs.readFile(); string b = std.fs.readFile(1); string c = std.fs.readFile(\"a\", \"b\"); }",
        );
        assert!(errors.iter().any(|message| message.contains("found 0")));
        assert!(errors.iter().any(|message| message.contains("found 2")));
        assert!(
            errors
                .iter()
                .any(|message| message.contains("expected `string`, found `int`"))
        );
    }

    #[test]
    fn restricts_string_array_to_main_parameter() {
        let errors = messages(
            "private string[] bad(string[] args) { string[] local = args; return args; } public void main() {}",
        );
        assert!(
            errors
                .iter()
                .any(|message| message.contains("sole parameter"))
        );
        assert!(errors.iter().any(|message| message.contains("return type")));
        assert!(
            errors
                .iter()
                .any(|message| message.contains("reserved for the parameter"))
        );

        for source in [
            "public int main() { return 0; }",
            "public void main(int x) {}",
            "public void main(string x) {}",
        ] {
            let hir = analyze_source(source).unwrap();
            assert!(validate_executable(&hir).is_err());
        }
    }
}
