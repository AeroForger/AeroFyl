use super::ast::*;
use super::diagnostics::Diagnostic;
use super::source::Span;
use super::token::{Keyword, Token, TokenKind};
use super::types::Type;
use std::collections::HashSet;

pub type ParseError = Diagnostic;
pub type ParseResult = Result<Module, Vec<ParseError>>;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
            diagnostics: Vec::new(),
        }
    }

    pub fn parse(mut self) -> ParseResult {
        let file = self.peek().span.file;
        let start = self.peek().span.start;
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut imports = Vec::new();
        let mut imported_names = HashSet::new();
        while !self.at(&TokenKind::Eof) {
            let result = if self.at(&TokenKind::Keyword(Keyword::Use)) {
                self.parse_import().and_then(|import| {
                    if imported_names.insert(import.name.text.clone()) {
                        imports.push(import);
                        Ok(())
                    } else {
                        Err(Diagnostic::error("duplicate import", import.span))
                    }
                })
            } else if self.at(&TokenKind::Keyword(Keyword::Struct)) {
                self.parse_struct()
                    .map(|declaration| structs.push(declaration))
            } else if self.at(&TokenKind::Keyword(Keyword::Enum)) {
                self.parse_enum().map(|declaration| enums.push(declaration))
            } else {
                self.parse_function()
                    .map(|function| functions.push(function))
            };
            match result {
                Ok(()) => {}
                Err(error) => {
                    self.diagnostics.push(error);
                    self.synchronize_top_level();
                }
            }
        }
        let span = Span::new(file, start, self.peek().span.end);
        if self.diagnostics.is_empty() {
            Ok(Module {
                imports,
                structs,
                enums,
                functions,
                span,
            })
        } else {
            Err(self.diagnostics)
        }
    }

    fn parse_import(&mut self) -> Result<Import, Diagnostic> {
        let start = self.advance().span;
        let mut name = self.expect_identifier("expected module name after `use`")?;
        while self.consume(&TokenKind::Dot) {
            let component = self.expect_identifier("expected module name after `.`")?;
            name.text.push('.');
            name.text.push_str(&component.text);
            name.span = name.span.join(component.span);
        }
        let end = self.expect(TokenKind::Semicolon, "expected `;` after import")?;
        Ok(Import {
            name,
            span: start.join(end.span),
        })
    }

    fn parse_struct(&mut self) -> Result<StructDeclaration, Diagnostic> {
        let start = self.advance().span;
        let name = self.expect_identifier("expected struct name")?;
        self.expect(TokenKind::LeftBrace, "expected `{` after struct name")?;
        let mut fields = Vec::new();
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            let ty = self.parse_type()?;
            let field_name = self.expect_identifier("expected field name")?;
            let end = self.expect(TokenKind::Semicolon, "expected `;` after struct field")?;
            fields.push(FieldDeclaration {
                span: ty.span.join(end.span),
                ty,
                name: field_name,
            });
        }
        let end = self.expect(
            TokenKind::RightBrace,
            "expected `}` after struct declaration",
        )?;
        Ok(StructDeclaration {
            name,
            fields,
            span: start.join(end.span),
        })
    }

    fn parse_enum(&mut self) -> Result<EnumDeclaration, Diagnostic> {
        let start = self.advance().span;
        let name = self.expect_identifier("expected enum name")?;
        self.expect(TokenKind::LeftBrace, "expected `{` after enum name")?;
        let mut variants = Vec::new();
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            variants.push(self.expect_identifier("expected enum variant")?);
            self.consume(&TokenKind::Comma);
        }
        let end = self.expect(TokenKind::RightBrace, "expected `}` after enum declaration")?;
        Ok(EnumDeclaration {
            name,
            variants,
            span: start.join(end.span),
        })
    }

    fn parse_function(&mut self) -> Result<Function, Diagnostic> {
        let visibility_token = self.advance().clone();
        let visibility = match visibility_token.kind {
            TokenKind::Keyword(Keyword::Public) => Visibility::Public,
            TokenKind::Keyword(Keyword::Private) => Visibility::Private,
            TokenKind::Keyword(Keyword::Using) => {
                return Err(Diagnostic::error(
                    "`using` is reserved; use `use module;` for imports",
                    visibility_token.span,
                ));
            }
            _ => {
                return Err(Diagnostic::error(
                    "expected `public` or `private` function visibility",
                    visibility_token.span,
                ));
            }
        };
        let return_type = self.parse_type()?;
        let name = self.expect_identifier("expected function name")?;
        self.expect(TokenKind::LeftParen, "expected `(` after function name")?;
        let mut parameters = Vec::new();
        if !self.at(&TokenKind::RightParen) {
            loop {
                let ty = self.parse_type()?;
                let parameter_name = self.expect_identifier("expected parameter name")?;
                let span = ty.span.join(parameter_name.span);
                parameters.push(Parameter {
                    ty,
                    name: parameter_name,
                    span,
                });
                if !self.consume(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RightParen, "expected `)` after parameters")?;
        let body = self.parse_block()?;
        let span = visibility_token.span.join(body.span);
        Ok(Function {
            visibility,
            return_type,
            name,
            parameters,
            body,
            span,
        })
    }

    fn parse_type(&mut self) -> Result<TypeNode, Diagnostic> {
        let start = self.peek().span;
        if self.consume_keyword(Keyword::List) {
            let element = self.parse_type()?;
            return Ok(TypeNode {
                kind: Type::List(Box::new(element.kind)),
                span: start.join(element.span),
            });
        }
        if self.consume(&TokenKind::LeftParen) {
            let mut elements = Vec::new();
            if self.at(&TokenKind::RightParen) {
                return Err(Diagnostic::error(
                    "tuple return types must contain types",
                    self.peek().span,
                ));
            }
            loop {
                elements.push(self.parse_type()?.kind);
                if !self.consume(&TokenKind::Comma) {
                    break;
                }
            }
            let end = self.expect(
                TokenKind::RightParen,
                "expected `)` after tuple return type",
            )?;
            return Ok(TypeNode {
                kind: Type::Tuple(elements),
                span: start.join(end.span),
            });
        }
        let token = self.advance().clone();
        let mut ty = match token.kind {
            TokenKind::Keyword(Keyword::Int) => Type::Int,
            TokenKind::Keyword(Keyword::Byte) => Type::Byte,
            TokenKind::Keyword(Keyword::Float) => Type::Float,
            TokenKind::Keyword(Keyword::Bool) => Type::Bool,
            TokenKind::Keyword(Keyword::Char) => Type::Char,
            TokenKind::Keyword(Keyword::String) => Type::String,
            TokenKind::Keyword(Keyword::Void) => Type::Void,
            TokenKind::Keyword(Keyword::Dynamic) => Type::Dynamic,
            TokenKind::Identifier(name) => Type::Named(name),
            _ => return Err(Diagnostic::error("expected a known type", token.span)),
        };
        let mut end = token.span;
        while self.consume(&TokenKind::LeftBracket) {
            if self.at(&TokenKind::RightBracket) {
                let close = self.advance().clone();
                if ty != Type::String {
                    return Err(Diagnostic::error(
                        "only the bootstrap command-line argument type `string[]` may omit an array length",
                        start.join(close.span),
                    ));
                }
                ty = Type::CliArgs;
                end = close.span;
                continue;
            }
            let length_token = self.advance().clone();
            let length = match length_token.kind {
                TokenKind::Integer(text) => text.parse::<usize>().map_err(|_| {
                    Diagnostic::error(
                        "array length does not fit in the compiler's size type",
                        length_token.span,
                    )
                })?,
                _ => {
                    return Err(Diagnostic::error(
                        "expected integer array length",
                        length_token.span,
                    ));
                }
            };
            let close = self.expect(TokenKind::RightBracket, "expected `]` after array length")?;
            ty = Type::Array {
                element: Box::new(ty),
                length,
            };
            end = close.span;
        }
        Ok(TypeNode {
            kind: ty,
            span: start.join(end),
        })
    }

    fn parse_block(&mut self) -> Result<Block, Diagnostic> {
        let open = self.expect(TokenKind::LeftBrace, "expected function body")?;
        let mut statements = Vec::new();
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            statements.push(self.parse_statement()?);
        }
        let close = self.expect(TokenKind::RightBrace, "expected `}` after function body")?;
        Ok(Block {
            statements,
            span: open.span.join(close.span),
        })
    }

    fn parse_statement(&mut self) -> Result<Statement, Diagnostic> {
        let start = self.peek().span;
        if self.consume_keyword(Keyword::If) {
            self.expect(TokenKind::LeftParen, "expected `(` after `if`")?;
            let condition = self.parse_expression()?;
            self.expect(TokenKind::RightParen, "expected `)` after `if` condition")?;
            let then_block = self.parse_block()?;
            let else_block = if self.consume_keyword(Keyword::Else) {
                Some(self.parse_block()?)
            } else {
                None
            };
            let end = else_block.as_ref().unwrap_or(&then_block).span;
            return Ok(Statement {
                kind: StatementKind::If {
                    condition,
                    then_block,
                    else_block,
                },
                span: start.join(end),
            });
        }
        if self.consume_keyword(Keyword::While) {
            self.expect(TokenKind::LeftParen, "expected `(` after `while`")?;
            let condition = self.parse_expression()?;
            self.expect(
                TokenKind::RightParen,
                "expected `)` after `while` condition",
            )?;
            let body = self.parse_block()?;
            let span = start.join(body.span);
            return Ok(Statement {
                kind: StatementKind::While { condition, body },
                span,
            });
        }
        if self.consume_keyword(Keyword::Break) {
            let end = self.expect(TokenKind::Semicolon, "expected `;` after `break`")?;
            return Ok(Statement {
                kind: StatementKind::Break,
                span: start.join(end.span),
            });
        }
        if self.consume_keyword(Keyword::Continue) {
            let end = self.expect(TokenKind::Semicolon, "expected `;` after `continue`")?;
            return Ok(Statement {
                kind: StatementKind::Continue,
                span: start.join(end.span),
            });
        }
        if self.consume_keyword(Keyword::Return) {
            let value = if self.at(&TokenKind::Semicolon) {
                None
            } else {
                Some(self.parse_expression()?)
            };
            let end = self.expect(TokenKind::Semicolon, "expected `;` after return statement")?;
            return Ok(Statement {
                kind: StatementKind::Return(value),
                span: start.join(end.span),
            });
        }
        if self.starts_variable_declaration() {
            let ty = self.parse_type()?;
            let name = self.expect_identifier("expected variable name")?;
            self.expect(TokenKind::Equal, "expected `=` in variable declaration")?;
            let initializer = self.parse_expression()?;
            let end = self.expect(
                TokenKind::Semicolon,
                "expected `;` after variable declaration",
            )?;
            return Ok(Statement {
                kind: StatementKind::Variable(VariableDeclaration {
                    ty,
                    name,
                    initializer,
                }),
                span: start.join(end.span),
            });
        }
        let expression = self.parse_expression()?;
        if self.consume(&TokenKind::Equal) {
            let value = self.parse_expression()?;
            let end = self.expect(TokenKind::Semicolon, "expected `;` after assignment")?;
            return Ok(Statement {
                kind: StatementKind::Assignment(Assignment {
                    target: expression,
                    value,
                }),
                span: start.join(end.span),
            });
        }
        let end = self.expect(TokenKind::Semicolon, "expected `;` after expression")?;
        Ok(Statement {
            kind: StatementKind::Expression(expression),
            span: start.join(end.span),
        })
    }

    fn parse_expression(&mut self) -> Result<Expression, Diagnostic> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Result<Expression, Diagnostic> {
        self.parse_left_associative(
            Self::parse_logical_and,
            &[(TokenKind::PipePipe, BinaryOperator::LogicalOr)],
        )
    }

    fn parse_logical_and(&mut self) -> Result<Expression, Diagnostic> {
        self.parse_left_associative(
            Self::parse_equality,
            &[(TokenKind::AmpersandAmpersand, BinaryOperator::LogicalAnd)],
        )
    }

    fn parse_equality(&mut self) -> Result<Expression, Diagnostic> {
        self.parse_left_associative(
            Self::parse_comparison,
            &[
                (TokenKind::EqualEqual, BinaryOperator::Equal),
                (TokenKind::BangEqual, BinaryOperator::NotEqual),
            ],
        )
    }

    fn parse_comparison(&mut self) -> Result<Expression, Diagnostic> {
        self.parse_left_associative(
            Self::parse_additive,
            &[
                (TokenKind::Less, BinaryOperator::Less),
                (TokenKind::LessEqual, BinaryOperator::LessEqual),
                (TokenKind::Greater, BinaryOperator::Greater),
                (TokenKind::GreaterEqual, BinaryOperator::GreaterEqual),
            ],
        )
    }

    fn parse_left_associative(
        &mut self,
        operand_parser: fn(&mut Self) -> Result<Expression, Diagnostic>,
        operators: &[(TokenKind, BinaryOperator)],
    ) -> Result<Expression, Diagnostic> {
        let mut expression = operand_parser(self)?;
        while let Some(operator) = operators
            .iter()
            .find_map(|(token, operator)| self.at(token).then_some(*operator))
        {
            self.advance();
            let right = operand_parser(self)?;
            let span = expression.span.join(right.span);
            expression = Expression {
                kind: ExpressionKind::Binary {
                    operator,
                    left: Box::new(expression),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(expression)
    }

    fn parse_additive(&mut self) -> Result<Expression, Diagnostic> {
        let mut expression = self.parse_multiplicative()?;
        loop {
            let operator = if self.consume(&TokenKind::Plus) {
                BinaryOperator::Add
            } else if self.consume(&TokenKind::Minus) {
                BinaryOperator::Subtract
            } else {
                break;
            };
            let right = self.parse_multiplicative()?;
            let span = expression.span.join(right.span);
            expression = Expression {
                kind: ExpressionKind::Binary {
                    operator,
                    left: Box::new(expression),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(expression)
    }

    fn parse_multiplicative(&mut self) -> Result<Expression, Diagnostic> {
        let mut expression = self.parse_unary()?;
        loop {
            let operator = if self.consume(&TokenKind::Star) {
                BinaryOperator::Multiply
            } else if self.consume(&TokenKind::Slash) {
                BinaryOperator::Divide
            } else {
                break;
            };
            let right = self.parse_unary()?;
            let span = expression.span.join(right.span);
            expression = Expression {
                kind: ExpressionKind::Binary {
                    operator,
                    left: Box::new(expression),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(expression)
    }

    fn parse_unary(&mut self) -> Result<Expression, Diagnostic> {
        if self.at(&TokenKind::Minus) {
            let operator = self.advance().span;
            let operand = self.parse_unary()?;
            return Ok(Expression {
                span: operator.join(operand.span),
                kind: ExpressionKind::Unary {
                    operator: UnaryOperator::Negate,
                    operand: Box::new(operand),
                },
            });
        }
        if self.at(&TokenKind::Bang) {
            let operator = self.advance().span;
            let operand = self.parse_unary()?;
            return Ok(Expression {
                span: operator.join(operand.span),
                kind: ExpressionKind::Unary {
                    operator: UnaryOperator::Not,
                    operand: Box::new(operand),
                },
            });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expression, Diagnostic> {
        let token = self.advance().clone();
        let mut expression = match token.kind {
            TokenKind::Integer(value) => Expression {
                kind: ExpressionKind::Literal(Literal::Integer(value)),
                span: token.span,
            },
            TokenKind::Float(value) => Expression {
                kind: ExpressionKind::Literal(Literal::Float(value)),
                span: token.span,
            },
            TokenKind::String(value) => Expression {
                kind: ExpressionKind::Literal(Literal::String(value)),
                span: token.span,
            },
            TokenKind::Char(value) => Expression {
                kind: ExpressionKind::Literal(Literal::Char(value)),
                span: token.span,
            },
            TokenKind::Keyword(Keyword::True) => Expression {
                kind: ExpressionKind::Literal(Literal::Bool(true)),
                span: token.span,
            },
            TokenKind::Keyword(Keyword::False) => Expression {
                kind: ExpressionKind::Literal(Literal::Bool(false)),
                span: token.span,
            },
            TokenKind::Identifier(text) => Expression {
                kind: ExpressionKind::Identifier(Name {
                    text,
                    span: token.span,
                }),
                span: token.span,
            },
            TokenKind::Keyword(Keyword::Print) => Expression {
                kind: ExpressionKind::Identifier(Name {
                    text: "print".into(),
                    span: token.span,
                }),
                span: token.span,
            },
            TokenKind::Keyword(Keyword::Input) => Expression {
                kind: ExpressionKind::Identifier(Name {
                    text: "input".into(),
                    span: token.span,
                }),
                span: token.span,
            },
            TokenKind::Keyword(Keyword::Int) => Expression {
                kind: ExpressionKind::Identifier(Name {
                    text: "int".into(),
                    span: token.span,
                }),
                span: token.span,
            },
            TokenKind::Keyword(Keyword::Byte) => Expression {
                kind: ExpressionKind::Identifier(Name {
                    text: "byte".into(),
                    span: token.span,
                }),
                span: token.span,
            },
            TokenKind::Keyword(Keyword::Char) => Expression {
                kind: ExpressionKind::Identifier(Name {
                    text: "char".into(),
                    span: token.span,
                }),
                span: token.span,
            },
            TokenKind::Keyword(Keyword::String) => Expression {
                kind: ExpressionKind::Identifier(Name {
                    text: "string".into(),
                    span: token.span,
                }),
                span: token.span,
            },
            TokenKind::Keyword(Keyword::Bool) => Expression {
                kind: ExpressionKind::Identifier(Name {
                    text: "bool".into(),
                    span: token.span,
                }),
                span: token.span,
            },
            TokenKind::Keyword(Keyword::Float) => Expression {
                kind: ExpressionKind::Identifier(Name {
                    text: "float".into(),
                    span: token.span,
                }),
                span: token.span,
            },
            TokenKind::LeftBracket => {
                let mut values = Vec::new();
                if !self.at(&TokenKind::RightBracket) {
                    loop {
                        values.push(self.parse_expression()?);
                        if !self.consume(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                let close = self.expect(
                    TokenKind::RightBracket,
                    "expected `]` after collection literal",
                )?;
                Expression {
                    kind: ExpressionKind::Collection(values),
                    span: token.span.join(close.span),
                }
            }
            TokenKind::LeftParen => {
                let inner = self.parse_expression()?;
                let close = self.expect(TokenKind::RightParen, "expected `)` after expression")?;
                Expression {
                    kind: inner.kind,
                    span: token.span.join(close.span),
                }
            }
            _ => return Err(Diagnostic::error("expected expression", token.span)),
        };

        loop {
            if self.at(&TokenKind::LeftBrace)
                && matches!(expression.kind, ExpressionKind::Identifier(_))
            {
                self.advance();
                let name = match expression.kind {
                    ExpressionKind::Identifier(name) => name,
                    _ => {
                        return Err(Diagnostic::error(
                            "only a named struct can begin a struct literal",
                            expression.span,
                        ));
                    }
                };
                let mut fields = Vec::new();
                while !self.at(&TokenKind::RightBrace) {
                    let field_name = self.expect_identifier("expected struct field name")?;
                    self.expect(TokenKind::Colon, "expected `:` after struct field name")?;
                    let value = self.parse_expression()?;
                    let field_span = field_name.span.join(value.span);
                    fields.push(StructLiteralField {
                        name: field_name,
                        value,
                        span: field_span,
                    });
                    if !self.consume(&TokenKind::Comma) {
                        break;
                    }
                }
                let close =
                    self.expect(TokenKind::RightBrace, "expected `}` after struct literal")?;
                expression = Expression {
                    span: expression.span.join(close.span),
                    kind: ExpressionKind::StructLiteral { name, fields },
                };
            } else if self.consume(&TokenKind::LeftParen) {
                let callee = match expression.kind {
                    ExpressionKind::Identifier(name) => name,
                    _ => {
                        return Err(Diagnostic::error(
                            "only a named function can be called",
                            expression.span,
                        ));
                    }
                };
                let mut arguments = Vec::new();
                if !self.at(&TokenKind::RightParen) {
                    loop {
                        arguments.push(self.parse_expression()?);
                        if !self.consume(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                let close =
                    self.expect(TokenKind::RightParen, "expected `)` after call arguments")?;
                expression = Expression {
                    span: expression.span.join(close.span),
                    kind: ExpressionKind::Call { callee, arguments },
                };
            } else if self.consume(&TokenKind::Dot) {
                let name = self.expect_identifier("expected field or variant after `.`")?;
                if self.consume(&TokenKind::LeftParen) {
                    let mut arguments = Vec::new();
                    if !self.at(&TokenKind::RightParen) {
                        loop {
                            arguments.push(self.parse_expression()?);
                            if !self.consume(&TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    let close =
                        self.expect(TokenKind::RightParen, "expected `)` after method arguments")?;
                    expression = Expression {
                        span: expression.span.join(close.span),
                        kind: ExpressionKind::MethodCall {
                            receiver: Box::new(expression),
                            method: name,
                            arguments,
                        },
                    };
                } else {
                    expression = Expression {
                        span: expression.span.join(name.span),
                        kind: ExpressionKind::Member {
                            base: Box::new(expression),
                            name,
                        },
                    };
                }
            } else if self.consume(&TokenKind::LeftBracket) {
                let index = self.parse_expression()?;
                let close = self.expect(TokenKind::RightBracket, "expected `]` after index")?;
                expression = Expression {
                    span: expression.span.join(close.span),
                    kind: ExpressionKind::Index {
                        base: Box::new(expression),
                        index: Box::new(index),
                    },
                };
            } else {
                break;
            }
        }
        Ok(expression)
    }

    fn starts_variable_declaration(&self) -> bool {
        if matches!(self.peek().kind, TokenKind::Identifier(_)) {
            matches!(self.peek_at(1).kind, TokenKind::Identifier(_))
        } else {
            self.starts_type()
        }
    }

    fn starts_type(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Keyword(
                Keyword::List
                    | Keyword::Int
                    | Keyword::Byte
                    | Keyword::Float
                    | Keyword::Bool
                    | Keyword::Char
                    | Keyword::String
                    | Keyword::Void
                    | Keyword::Dynamic
            ) | TokenKind::LeftParen
                | TokenKind::Identifier(_)
        )
    }

    fn expect_identifier(&mut self, message: &str) -> Result<Name, Diagnostic> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Identifier(text) => Ok(Name {
                text,
                span: token.span,
            }),
            TokenKind::Keyword(Keyword::Byte) => Ok(Name {
                text: "byte".into(),
                span: token.span,
            }),
            _ => Err(Diagnostic::error(message, token.span)),
        }
    }

    fn expect(&mut self, kind: TokenKind, message: &str) -> Result<Token, Diagnostic> {
        if self.at(&kind) {
            Ok(self.advance().clone())
        } else {
            Err(Diagnostic::error(message, self.peek().span))
        }
    }

    fn consume(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume_keyword(&mut self, keyword: Keyword) -> bool {
        self.consume(&TokenKind::Keyword(keyword))
    }
    fn at(&self, kind: &TokenKind) -> bool {
        self.peek().kind == *kind
    }
    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }
    fn peek_at(&self, distance: usize) -> &Token {
        let index = self.current.saturating_add(distance);
        self.tokens.get(index).unwrap_or_else(|| self.peek())
    }
    fn advance(&mut self) -> &Token {
        let index = self.current;
        if !self.at(&TokenKind::Eof) {
            self.current += 1;
        }
        &self.tokens[index]
    }

    fn synchronize_top_level(&mut self) {
        while !self.at(&TokenKind::Eof) {
            if matches!(
                self.peek().kind,
                TokenKind::Keyword(
                    Keyword::Public
                        | Keyword::Private
                        | Keyword::Struct
                        | Keyword::Enum
                        | Keyword::Use
                )
            ) {
                return;
            }
            self.advance();
        }
    }
}

pub fn parse(tokens: Vec<Token>) -> ParseResult {
    Parser::new(tokens).parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::lexer::lex;
    use crate::frontend::source::FileId;

    fn parse_source(source: &str) -> Module {
        parse(lex(FileId(0), source).unwrap()).unwrap()
    }

    #[test]
    fn parses_known_function_and_collection_declarations() {
        let module = parse_source("public int values(int n) { int[2] a = [1, 2]; return n; }");
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].parameters[0].ty.kind, Type::Int);
        assert!(matches!(
            module.functions[0].body.statements[0].kind,
            StatementKind::Variable(_)
        ));
    }

    #[test]
    fn parses_tuple_return_types_as_types_only() {
        let module = parse_source("private (int, int) pair() { return; }");
        assert_eq!(
            module.functions[0].return_type.kind,
            Type::Tuple(vec![Type::Int, Type::Int])
        );
    }

    #[test]
    fn requires_semicolons() {
        let tokens = lex(FileId(0), "public int f() { return 1 }").unwrap();
        assert!(parse(tokens).unwrap_err()[0].message.contains("`;`"));
    }

    #[test]
    fn parses_module_import() {
        let tokens = lex(FileId(0), "use thing;").unwrap();
        let module = parse(tokens).unwrap();
        assert_eq!(module.imports.len(), 1);
        assert_eq!(module.imports[0].name.text, "thing");
    }

    #[test]
    fn parses_integer_precedence_and_unary_minus() {
        let module = parse_source("private int f() { return -1 + 2 * 3; }");
        let StatementKind::Return(Some(expression)) = &module.functions[0].body.statements[0].kind
        else {
            panic!("expected return expression");
        };
        let ExpressionKind::Binary {
            operator: BinaryOperator::Add,
            left,
            right,
        } = &expression.kind
        else {
            panic!("expected top-level addition");
        };
        assert!(matches!(left.kind, ExpressionKind::Unary { .. }));
        assert!(matches!(
            right.kind,
            ExpressionKind::Binary {
                operator: BinaryOperator::Multiply,
                ..
            }
        ));
    }

    #[test]
    fn parses_control_flow_assignment_and_boolean_precedence() {
        let module = parse_source(
            "public void main() { bool flag = !false || true && 1 < 2 == true; int x = 0; while (flag) { if (x >= 2) { break; } else { x = x + 1; continue; } } }",
        );
        let statements = &module.functions[0].body.statements;
        assert!(matches!(statements[0].kind, StatementKind::Variable(_)));
        let StatementKind::While { body, .. } = &statements[2].kind else {
            panic!("expected while statement");
        };
        let StatementKind::If { else_block, .. } = &body.statements[0].kind else {
            panic!("expected if statement");
        };
        assert!(else_block.is_some());
        assert!(matches!(
            else_block.as_ref().unwrap().statements[0].kind,
            StatementKind::Assignment(_)
        ));
        assert!(matches!(
            else_block.as_ref().unwrap().statements[1].kind,
            StatementKind::Continue
        ));
    }

    #[test]
    fn parses_struct_declaration() {
        let module = parse_source("struct Point { int x; int y; }");
        assert_eq!(module.structs[0].name.text, "Point");
        assert_eq!(module.structs[0].fields.len(), 2);
    }

    #[test]
    fn parses_enum_declaration_without_variant_semicolons() {
        let module = parse_source("enum State { idle, running, stopped }");
        assert_eq!(module.enums[0].variants.len(), 3);
        assert_eq!(module.enums[0].variants[1].text, "running");
    }

    #[test]
    fn parses_struct_literal() {
        let module =
            parse_source("struct Point { int x; } private void f() { Point p = Point { x: 1 }; }");
        let StatementKind::Variable(variable) = &module.functions[0].body.statements[0].kind else {
            panic!("expected variable declaration");
        };
        assert!(matches!(
            variable.initializer.kind,
            ExpressionKind::StructLiteral { .. }
        ));
    }

    #[test]
    fn parses_field_access() {
        let module = parse_source("struct Point { int x; } private int f(Point p) { return p.x; }");
        let StatementKind::Return(Some(value)) = &module.functions[0].body.statements[0].kind
        else {
            panic!("expected return");
        };
        assert!(matches!(value.kind, ExpressionKind::Member { .. }));
    }

    #[test]
    fn parses_field_assignment() {
        let module = parse_source(
            "struct Point { int x; } private void f() { Point p = Point { x: 1 }; p.x = 2; }",
        );
        let StatementKind::Assignment(assignment) = &module.functions[0].body.statements[1].kind
        else {
            panic!("expected assignment");
        };
        assert!(matches!(
            assignment.target.kind,
            ExpressionKind::Member { .. }
        ));
    }

    #[test]
    fn parses_qualified_enum_variant() {
        let module = parse_source(
            "enum State { idle, running } private void f() { State s = State.running; }",
        );
        let StatementKind::Variable(variable) = &module.functions[0].body.statements[0].kind else {
            panic!("expected variable");
        };
        assert!(matches!(
            variable.initializer.kind,
            ExpressionKind::Member { .. }
        ));
    }

    #[test]
    fn parses_fixed_array_type_and_literal() {
        let module = parse_source("private void f() { bool[2] flags = [true, false]; }");
        let StatementKind::Variable(variable) = &module.functions[0].body.statements[0].kind else {
            panic!("expected array declaration");
        };
        assert_eq!(
            variable.ty.kind,
            Type::Array {
                element: Box::new(Type::Bool),
                length: 2
            }
        );
        assert!(matches!(
            variable.initializer.kind,
            ExpressionKind::Collection(_)
        ));
    }

    #[test]
    fn parses_index_expression() {
        let module = parse_source("private int f() { int[1] values = [1]; return values[0]; }");
        let StatementKind::Return(Some(value)) = &module.functions[0].body.statements[1].kind
        else {
            panic!("expected return");
        };
        assert!(matches!(value.kind, ExpressionKind::Index { .. }));
    }

    #[test]
    fn parses_indexed_assignment() {
        let module = parse_source("private void f() { int[1] values = [1]; values[0] = 2; }");
        let StatementKind::Assignment(assignment) = &module.functions[0].body.statements[1].kind
        else {
            panic!("expected assignment");
        };
        assert!(matches!(
            assignment.target.kind,
            ExpressionKind::Index { .. }
        ));
    }

    #[test]
    fn parses_list_type_and_literal() {
        let module = parse_source("private void f() { list int values = [1, 2]; }");
        let StatementKind::Variable(variable) = &module.functions[0].body.statements[0].kind else {
            panic!("expected list declaration");
        };
        assert_eq!(variable.ty.kind, Type::List(Box::new(Type::Int)));
        assert!(matches!(
            variable.initializer.kind,
            ExpressionKind::Collection(_)
        ));
    }

    #[test]
    fn parses_struct_list_and_copy_expression() {
        let module = parse_source(
            "struct Token { int line; } private void f() { list Token tokens = []; Token a = Token { line: 1 }; Token b = a; }",
        );
        let StatementKind::Variable(list) = &module.functions[0].body.statements[0].kind else {
            panic!("expected list declaration");
        };
        assert!(
            matches!(list.ty.kind, Type::List(ref element) if matches!(**element, Type::Named(_)))
        );
        let StatementKind::Variable(copy) = &module.functions[0].body.statements[2].kind else {
            panic!("expected copied struct declaration");
        };
        assert!(matches!(
            copy.initializer.kind,
            ExpressionKind::Identifier(_)
        ));
    }

    #[test]
    fn parses_list_push_and_pop() {
        let module = parse_source(
            "private void f() { list int values = []; values.push(1); int x = values.pop(); }",
        );
        assert!(matches!(
            module.functions[0].body.statements[1].kind,
            StatementKind::Expression(Expression {
                kind: ExpressionKind::MethodCall { .. },
                ..
            })
        ));
        let StatementKind::Variable(variable) = &module.functions[0].body.statements[2].kind else {
            panic!("expected variable");
        };
        assert!(matches!(
            variable.initializer.kind,
            ExpressionKind::MethodCall { .. }
        ));
    }

    #[test]
    fn parses_collection_and_string_length_properties() {
        let module = parse_source(
            "private void f() { list int values = []; int a = values.length; string text = \"hi\"; int b = text.length; }",
        );
        for statement in [
            &module.functions[0].body.statements[1],
            &module.functions[0].body.statements[3],
        ] {
            let StatementKind::Variable(variable) = &statement.kind else {
                panic!("expected variable");
            };
            assert!(matches!(
                variable.initializer.kind,
                ExpressionKind::Member { .. }
            ));
        }
    }

    #[test]
    fn parses_string_concatenation() {
        let module = parse_source("private void f() { string text = \"a\" + \"b\"; }");
        let StatementKind::Variable(variable) = &module.functions[0].body.statements[0].kind else {
            panic!("expected variable");
        };
        assert!(matches!(
            variable.initializer.kind,
            ExpressionKind::Binary {
                operator: BinaryOperator::Add,
                ..
            }
        ));
    }

    #[test]
    fn parses_string_byte_and_read_file_intrinsics() {
        let module = parse_source(
            "private int f(string path) { string source = readFile(path); return source.byte(0); }",
        );
        let StatementKind::Variable(variable) = &module.functions[0].body.statements[0].kind else {
            panic!("expected variable");
        };
        assert!(matches!(
            variable.initializer.kind,
            ExpressionKind::Call { .. }
        ));
        let StatementKind::Return(Some(value)) = &module.functions[0].body.statements[1].kind
        else {
            panic!("expected return");
        };
        assert!(matches!(value.kind, ExpressionKind::MethodCall { .. }));
    }

    #[test]
    fn parses_string_slice_method_call() {
        let module = parse_source(
            "private string f(string source, int a, int b) { return source.slice(a, b); }",
        );
        let StatementKind::Return(Some(value)) = &module.functions[0].body.statements[0].kind
        else {
            panic!("expected return");
        };
        assert!(matches!(
            value.kind,
            ExpressionKind::MethodCall { ref arguments, .. } if arguments.len() == 2
        ));
    }

    #[test]
    fn parses_bootstrap_main_argument_type() {
        let module = parse_source("public void main(string[] args) { int count = args.length; }");
        assert_eq!(module.functions[0].parameters[0].ty.kind, Type::CliArgs);
    }
}
