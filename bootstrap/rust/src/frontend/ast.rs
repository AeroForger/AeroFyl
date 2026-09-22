use super::source::Span;
use super::types::Type;

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub imports: Vec<Import>,
    pub structs: Vec<StructDeclaration>,
    pub enums: Vec<EnumDeclaration>,
    pub functions: Vec<Function>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Import {
    pub name: Name,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructDeclaration {
    pub name: Name,
    pub fields: Vec<FieldDeclaration>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldDeclaration {
    pub ty: TypeNode,
    pub name: Name,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumDeclaration {
    pub name: Name,
    pub variants: Vec<EnumVariantDeclaration>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumVariantDeclaration {
    pub name: Name,
    pub payload: Option<TypeNode>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub visibility: Visibility,
    pub return_type: TypeNode,
    pub name: Name,
    pub parameters: Vec<Parameter>,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Parameter {
    pub ty: TypeNode,
    pub name: Name,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeNode {
    pub kind: Type,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Name {
    pub text: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Statement {
    pub kind: StatementKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StatementKind {
    Variable(VariableDeclaration),
    Assignment(Assignment),
    Return(Option<Expression>),
    Expression(Expression),
    If {
        condition: Expression,
        then_block: Block,
        else_block: Option<Block>,
    },
    While {
        condition: Expression,
        body: Block,
    },
    /// A C-style loop. The initializer runs once, then the condition is checked
    /// before each body execution; the increment runs after the body and on
    /// `continue`.
    For {
        initializer: Box<Statement>,
        condition: Expression,
        increment: Box<Statement>,
        body: Block,
    },
    Break,
    Continue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariableDeclaration {
    pub ty: TypeNode,
    pub name: Name,
    pub initializer: Expression,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Assignment {
    pub target: Expression,
    pub value: Expression,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expression {
    pub kind: ExpressionKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExpressionKind {
    Literal(Literal),
    Identifier(Name),
    Call {
        callee: Name,
        arguments: Vec<Expression>,
    },
    Unary {
        operator: UnaryOperator,
        operand: Box<Expression>,
    },
    Binary {
        operator: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Collection(Vec<Expression>),
    StructLiteral {
        name: Name,
        fields: Vec<StructLiteralField>,
    },
    Member {
        base: Box<Expression>,
        name: Name,
    },
    Index {
        base: Box<Expression>,
        index: Box<Expression>,
    },
    MethodCall {
        receiver: Box<Expression>,
        method: Name,
        arguments: Vec<Expression>,
    },
    /// Reserved for the documented tuple concept. Tuple value syntax is not yet
    /// specified, so the parser does not construct this variant.
    Tuple(Vec<Expression>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructLiteralField {
    pub name: Name,
    pub value: Expression,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOperator {
    Negate,
    Not,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    LogicalAnd,
    LogicalOr,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Literal {
    Integer(String),
    Float(String),
    String(String),
    Char(char),
    Bool(bool),
}
