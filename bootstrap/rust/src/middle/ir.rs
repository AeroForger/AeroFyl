use crate::frontend::ast::{BinaryOperator, UnaryOperator, Visibility};
use crate::frontend::resolution::SymbolId;
use crate::frontend::source::Span;
use crate::frontend::types::Type;

#[derive(Clone, Debug, PartialEq)]
pub struct IrModule {
    pub functions: Vec<IrFunction>,
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
    Float(String),
    String(String),
    Char(char),
    Bool(bool),
}
