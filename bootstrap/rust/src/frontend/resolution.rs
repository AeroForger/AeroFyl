use std::collections::HashMap;

use super::ast::{Block, Expression, ExpressionKind, Module, StatementKind};
use super::diagnostics::Diagnostic;
use super::source::Span;
use super::types::Type;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SymbolId(pub u32);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SymbolKind {
    Function {
        parameters: Vec<Type>,
        return_type: Type,
    },
    Parameter {
        ty: Type,
    },
    Variable {
        ty: Type,
    },
    /// Builtin signatures and behavior have not been specified.
    Builtin,
    /// Reserved for names introduced by imports once import grammar is specified.
    Module,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub declaration: Option<Span>,
}

#[derive(Clone, Debug, Default)]
pub struct Resolution {
    pub symbols: Vec<Symbol>,
    pub declarations: HashMap<Span, SymbolId>,
    pub references: HashMap<Span, SymbolId>,
}

impl Resolution {
    pub fn symbol(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(id.0 as usize)
    }
    pub fn declaration(&self, span: Span) -> Option<SymbolId> {
        self.declarations.get(&span).copied()
    }
    pub fn reference(&self, span: Span) -> Option<SymbolId> {
        self.references.get(&span).copied()
    }
}

pub fn resolve(module: &Module) -> Result<Resolution, Vec<Diagnostic>> {
    Resolver::new().resolve(module)
}

struct Resolver {
    resolution: Resolution,
    scopes: Vec<HashMap<String, SymbolId>>,
    diagnostics: Vec<Diagnostic>,
}

impl Resolver {
    fn new() -> Self {
        let mut resolver = Self {
            resolution: Resolution::default(),
            scopes: vec![HashMap::new()],
            diagnostics: Vec::new(),
        };
        resolver.define_builtin("print");
        resolver.define_builtin("input");
        resolver
    }

    fn resolve(mut self, module: &Module) -> Result<Resolution, Vec<Diagnostic>> {
        for function in &module.functions {
            self.define(
                &function.name.text,
                SymbolKind::Function {
                    parameters: function
                        .parameters
                        .iter()
                        .map(|parameter| parameter.ty.kind.clone())
                        .collect(),
                    return_type: function.return_type.kind.clone(),
                },
                function.name.span,
            );
        }

        for function in &module.functions {
            self.scopes.push(HashMap::new());
            for parameter in &function.parameters {
                self.define(
                    &parameter.name.text,
                    SymbolKind::Parameter {
                        ty: parameter.ty.kind.clone(),
                    },
                    parameter.name.span,
                );
            }
            self.resolve_block(&function.body, false);
            self.scopes.pop();
        }

        if self.diagnostics.is_empty() {
            Ok(self.resolution)
        } else {
            Err(self.diagnostics)
        }
    }

    fn resolve_block(&mut self, block: &Block, nested: bool) {
        if nested {
            self.scopes.push(HashMap::new());
        }
        for statement in &block.statements {
            match &statement.kind {
                StatementKind::Variable(variable) => {
                    self.resolve_expression(&variable.initializer);
                    self.define(
                        &variable.name.text,
                        SymbolKind::Variable {
                            ty: variable.ty.kind.clone(),
                        },
                        variable.name.span,
                    );
                }
                StatementKind::Assignment(assignment) => {
                    self.resolve_name(&assignment.target.text, assignment.target.span);
                    self.resolve_expression(&assignment.value);
                }
                StatementKind::Return(Some(value)) | StatementKind::Expression(value) => {
                    self.resolve_expression(value);
                }
                StatementKind::If {
                    condition,
                    then_block,
                    else_block,
                } => {
                    self.resolve_expression(condition);
                    self.resolve_block(then_block, true);
                    if let Some(else_block) = else_block {
                        self.resolve_block(else_block, true);
                    }
                }
                StatementKind::While { condition, body } => {
                    self.resolve_expression(condition);
                    self.resolve_block(body, true);
                }
                StatementKind::Return(None) | StatementKind::Break | StatementKind::Continue => {}
            }
        }
        if nested {
            self.scopes.pop();
        }
    }

    fn resolve_expression(&mut self, expression: &Expression) {
        match &expression.kind {
            ExpressionKind::Identifier(name) => self.resolve_name(&name.text, name.span),
            ExpressionKind::Call { callee, arguments } => {
                self.resolve_name(&callee.text, callee.span);
                for argument in arguments {
                    self.resolve_expression(argument);
                }
            }
            ExpressionKind::Unary { operand, .. } => self.resolve_expression(operand),
            ExpressionKind::Binary { left, right, .. } => {
                self.resolve_expression(left);
                self.resolve_expression(right);
            }
            ExpressionKind::Collection(values) | ExpressionKind::Tuple(values) => {
                for value in values {
                    self.resolve_expression(value);
                }
            }
            ExpressionKind::Literal(_) => {}
        }
    }

    fn resolve_name(&mut self, name: &str, span: Span) {
        let symbol = self
            .scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .copied();
        if let Some(symbol) = symbol {
            self.resolution.references.insert(span, symbol);
        } else {
            self.diagnostics.push(Diagnostic::error(
                format!("unknown identifier `{name}`"),
                span,
            ));
        }
    }

    fn define_builtin(&mut self, name: &str) {
        let id = self.push_symbol(name, SymbolKind::Builtin, None);
        self.scopes[0].insert(name.to_owned(), id);
    }

    fn define(&mut self, name: &str, kind: SymbolKind, span: Span) {
        let scope = self.scopes.last_mut().expect("resolver always has a scope");
        if scope.contains_key(name) {
            self.diagnostics.push(Diagnostic::error(
                format!("duplicate declaration of `{name}` in the same scope"),
                span,
            ));
            return;
        }
        let id = self.push_symbol(name, kind, Some(span));
        self.scopes.last_mut().unwrap().insert(name.to_owned(), id);
        self.resolution.declarations.insert(span, id);
    }

    fn push_symbol(&mut self, name: &str, kind: SymbolKind, declaration: Option<Span>) -> SymbolId {
        let id = SymbolId(self.resolution.symbols.len() as u32);
        self.resolution.symbols.push(Symbol {
            id,
            name: name.to_owned(),
            kind,
            declaration,
        });
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::{lexer::lex, parser::parse, source::FileId};

    fn resolve_source(source: &str) -> Result<Resolution, Vec<Diagnostic>> {
        resolve(&parse(lex(FileId(0), source).unwrap()).unwrap())
    }

    #[test]
    fn resolves_parameters_and_prior_locals() {
        let result = resolve_source("private int f(int x) { int y = x; return y; }").unwrap();
        assert_eq!(result.references.len(), 2);
    }

    #[test]
    fn reports_unknown_names() {
        let errors = resolve_source("private int f() { return missing; }").unwrap_err();
        assert!(errors[0].message.contains("unknown identifier"));
    }

    #[test]
    fn reports_duplicate_names_in_one_scope() {
        let errors = resolve_source("private int f(int x, int x) { return x; }").unwrap_err();
        assert!(errors[0].message.contains("duplicate declaration"));
    }
}
