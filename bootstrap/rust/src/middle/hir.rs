use crate::frontend::ast::{BinaryOperator, Literal, UnaryOperator, Visibility};
use crate::frontend::resolution::SymbolId;
use crate::frontend::source::Span;
use crate::frontend::types::Type;

#[derive(Clone, Debug, PartialEq)]
pub struct HirModule {
    pub functions: Vec<HirFunction>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HirFunction {
    pub symbol: SymbolId,
    pub name: String,
    pub name_span: Span,
    pub visibility: Visibility,
    pub return_type: Type,
    pub parameters: Vec<HirParameter>,
    pub body: HirBlock,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HirParameter {
    pub symbol: SymbolId,
    pub ty: Type,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HirBlock {
    pub statements: Vec<HirStatement>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HirStatement {
    Variable {
        symbol: SymbolId,
        ty: Type,
        initializer: HirExpression,
        span: Span,
    },
    Assignment {
        symbol: SymbolId,
        value: HirExpression,
        span: Span,
    },
    Return {
        value: Option<HirExpression>,
        span: Span,
    },
    Expression(HirExpression),
    If {
        condition: HirExpression,
        then_block: HirBlock,
        else_block: Option<HirBlock>,
        span: Span,
    },
    While {
        condition: HirExpression,
        body: HirBlock,
        span: Span,
    },
    Break(Span),
    Continue(Span),
}

#[derive(Clone, Debug, PartialEq)]
pub struct HirExpression {
    pub kind: HirExpressionKind,
    pub ty: Type,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HirExpressionKind {
    Literal(Literal),
    Symbol(SymbolId),
    Call {
        callee: SymbolId,
        arguments: Vec<HirExpression>,
    },
    Unary {
        operator: UnaryOperator,
        operand: Box<HirExpression>,
    },
    Binary {
        operator: BinaryOperator,
        left: Box<HirExpression>,
        right: Box<HirExpression>,
    },
    Collection(Vec<HirExpression>),
    Tuple(Vec<HirExpression>),
}
