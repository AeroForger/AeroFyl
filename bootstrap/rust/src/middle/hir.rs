use crate::frontend::ast::{BinaryOperator, Literal, UnaryOperator, Visibility};
use crate::frontend::resolution::SymbolId;
use crate::frontend::source::Span;
use crate::frontend::types::{Type, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct HirModule {
    pub structs: Vec<HirStruct>,
    pub enums: Vec<HirEnum>,
    pub functions: Vec<HirFunction>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HirStruct {
    pub id: TypeId,
    pub name: String,
    pub fields: Vec<HirField>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HirField {
    pub name: String,
    pub ty: Type,
    pub offset: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HirEnum {
    pub id: TypeId,
    pub name: String,
    pub variants: Vec<HirEnumVariant>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HirEnumVariant {
    pub name: String,
    pub payload: Option<Type>,
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
    FieldAssignment {
        local: SymbolId,
        struct_id: TypeId,
        field: u32,
        value: HirExpression,
        span: Span,
    },
    ReferenceAssignment {
        reference: HirExpression,
        value: HirExpression,
        value_type: Type,
        span: Span,
    },
    IndexedAssignment {
        collection: HirExpression,
        collection_type: Type,
        index: HirExpression,
        value: HirExpression,
        span: Span,
    },
    CompoundIndexedAssignment {
        collection: HirExpression,
        collection_type: Type,
        index: HirExpression,
        operator: BinaryOperator,
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
    For {
        initializer: Box<HirStatement>,
        condition: HirExpression,
        increment: Box<HirStatement>,
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
    ArrayLiteral(Vec<HirExpression>),
    ListLiteral(Vec<HirExpression>),
    StructLiteral {
        struct_id: TypeId,
        fields: Vec<(u32, HirExpression)>,
    },
    StructCopy {
        struct_id: TypeId,
        source: Box<HirExpression>,
    },
    OptionalSome {
        value: Box<HirExpression>,
    },
    OptionalNone,
    OptionalHasValue {
        value: Box<HirExpression>,
    },
    OptionalValue {
        value: Box<HirExpression>,
    },
    Reference {
        value: Box<HirExpression>,
    },
    ReferenceValue {
        value: Box<HirExpression>,
    },
    FieldLoad {
        base: Box<HirExpression>,
        struct_id: TypeId,
        field: u32,
    },
    EnumValue {
        enum_id: TypeId,
        variant: u32,
        payload: Option<Box<HirExpression>>,
        boxed: bool,
    },
    EnumIs {
        value: Box<HirExpression>,
        enum_id: TypeId,
        variant: u32,
    },
    EnumPayload {
        value: Box<HirExpression>,
        enum_id: TypeId,
        variant: u32,
        payload_type: Type,
    },
    ArrayLoad {
        collection: Box<HirExpression>,
        index: Box<HirExpression>,
    },
    ArrayLength {
        length: usize,
    },
    ListLoad {
        collection: Box<HirExpression>,
        index: Box<HirExpression>,
    },
    ListLength {
        collection: Box<HirExpression>,
    },
    CliArgLoad {
        local: SymbolId,
        index: Box<HirExpression>,
    },
    CliArgsLength {
        local: SymbolId,
    },
    ListPush {
        local: SymbolId,
        value: Box<HirExpression>,
    },
    ListPushField {
        local: SymbolId,
        struct_id: TypeId,
        field: u32,
        element_type: Type,
        value: Box<HirExpression>,
    },
    ListPushIndexed {
        collection: Box<HirExpression>,
        collection_type: Type,
        index: Box<HirExpression>,
        element_type: Type,
        value: Box<HirExpression>,
    },
    ListPop {
        local: SymbolId,
    },
    ListPopValue {
        collection: Box<HirExpression>,
    },
    StringLiteral(String),
    StringLength {
        value: Box<HirExpression>,
    },
    StringByte {
        value: Box<HirExpression>,
        index: Box<HirExpression>,
    },
    StringSlice {
        value: Box<HirExpression>,
        start: Box<HirExpression>,
        end: Box<HirExpression>,
    },
    ReadFile {
        path: Box<HirExpression>,
    },
    ReadBytes {
        path: Box<HirExpression>,
    },
    Exit {
        code: Box<HirExpression>,
    },
    WriteFile {
        path: Box<HirExpression>,
        data: Box<HirExpression>,
    },
    WriteBytes {
        path: Box<HirExpression>,
        data: Box<HirExpression>,
    },
    Exists {
        path: Box<HirExpression>,
    },
    Print {
        value: Box<HirExpression>,
        stderr: bool,
        newline: bool,
    },
    Input {
        target: Type,
    },
    Convert {
        value: Box<HirExpression>,
        target: Type,
    },
    StringConcat {
        left: Box<HirExpression>,
        right: Box<HirExpression>,
    },
    StringEqual {
        equal: bool,
        left: Box<HirExpression>,
        right: Box<HirExpression>,
    },
    Tuple(Vec<HirExpression>),
}
