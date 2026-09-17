use crate::frontend::ast::{BinaryOperator, UnaryOperator, Visibility};
use crate::frontend::resolution::SymbolId;
use crate::frontend::source::Span;
use crate::frontend::types::{Type, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct IrModule {
    pub structs: Vec<IrStruct>,
    pub enums: Vec<IrEnum>,
    pub functions: Vec<IrFunction>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrStruct {
    pub id: TypeId,
    pub name: String,
    pub fields: Vec<IrField>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrField {
    pub name: String,
    pub ty: Type,
    /// Eight-byte slot offset from the start of the bootstrap struct layout.
    pub offset: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrEnum {
    pub id: TypeId,
    pub name: String,
    pub variants: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrFunction {
    pub symbol: SymbolId,
    pub name: String,
    pub visibility: Visibility,
    pub parameters: Vec<IrLocal>,
    pub return_type: Type,
    pub locals: Vec<IrLocal>,
    pub entry: BlockId,
    pub blocks: Vec<IrBlock>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IrLocal {
    pub symbol: SymbolId,
    pub ty: Type,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ValueId(pub u32);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BlockId(pub u32);

#[derive(Clone, Debug, PartialEq)]
pub struct IrBlock {
    pub id: BlockId,
    pub instructions: Vec<IrInstruction>,
    pub terminator: Option<IrTerminator>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IrTerminator {
    Return(Option<ValueId>),
    Exit(ValueId),
    Jump(BlockId),
    Branch {
        condition: ValueId,
        then_block: BlockId,
        else_block: BlockId,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrInstruction {
    /// A logical value normally has one definition. Short-circuit lowering may
    /// define one value in path-disjoint predecessor blocks that share a merge.
    pub result: Option<ValueId>,
    pub result_type: Option<Type>,
    pub kind: IrInstructionKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IrInstructionKind {
    Constant(IrConstant),
    LoadLocal(SymbolId),
    BindLocal {
        local: SymbolId,
        value: ValueId,
    },
    StructInit {
        local: SymbolId,
        struct_id: TypeId,
        fields: Vec<(u32, ValueId)>,
    },
    StructValue {
        struct_id: TypeId,
        fields: Vec<(u32, ValueId)>,
    },
    AggregateCopy {
        struct_id: TypeId,
        source: ValueId,
    },
    FieldLoad {
        local: SymbolId,
        struct_id: TypeId,
        field: u32,
    },
    FieldStore {
        local: SymbolId,
        struct_id: TypeId,
        field: u32,
        value: ValueId,
    },
    EnumConstant {
        enum_id: TypeId,
        variant: u32,
    },
    ArrayInit {
        local: SymbolId,
        element_type: Type,
        length: usize,
        values: Vec<ValueId>,
    },
    ArrayLoad {
        local: SymbolId,
        element_type: Type,
        length: usize,
        index: ValueId,
    },
    ArrayStore {
        local: SymbolId,
        element_type: Type,
        length: usize,
        index: ValueId,
        value: ValueId,
    },
    ArrayLength(usize),
    ListInit {
        local: SymbolId,
        element_type: Type,
        values: Vec<ValueId>,
    },
    ListLoad {
        local: SymbolId,
        element_type: Type,
        index: ValueId,
    },
    ListStore {
        local: SymbolId,
        element_type: Type,
        index: ValueId,
        value: ValueId,
    },
    ListPush {
        local: SymbolId,
        element_type: Type,
        value: ValueId,
    },
    ListPop {
        local: SymbolId,
        element_type: Type,
    },
    ListLength {
        local: SymbolId,
        element_type: Type,
    },
    CliArgLoad {
        local: SymbolId,
        index: ValueId,
    },
    CliArgsLength {
        local: SymbolId,
    },
    StringConstant(String),
    StringConcat {
        left: ValueId,
        right: ValueId,
    },
    StringEqual {
        equal: bool,
        left: ValueId,
        right: ValueId,
    },
    StringLength(ValueId),
    StringByte {
        value: ValueId,
        index: ValueId,
    },
    StringSlice {
        value: ValueId,
        start: ValueId,
        end: ValueId,
    },
    ReadFile(ValueId),
    ReadBytes(ValueId),
    WriteFile {
        path: ValueId,
        data: ValueId,
    },
    WriteBytes {
        path: ValueId,
        data: ValueId,
    },
    Exists(ValueId),
    Print {
        value: ValueId,
        value_type: Type,
        stderr: bool,
        newline: bool,
    },
    Input {
        target: Type,
    },
    Convert {
        value: ValueId,
        from: Type,
        to: Type,
    },
    Copy(ValueId),
    Call {
        function: SymbolId,
        arguments: Vec<ValueId>,
    },
    Unary {
        operator: UnaryOperator,
        operand: ValueId,
    },
    Binary {
        operator: BinaryOperator,
        left: ValueId,
        right: ValueId,
    },
    Aggregate(Vec<ValueId>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum IrConstant {
    Integer(String),
    Byte(String),
    Float(String),
    String(String),
    Char(char),
    Bool(bool),
}
