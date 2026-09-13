use crate::frontend::resolution::SymbolId;
use crate::middle::ir::BlockId;

use super::register::Register;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Instruction {
    Nop,
    Label(BlockId),
    Push(Register),
    Pop(Register),
    MoveRegister {
        destination: Register,
        source: Register,
    },
    MoveImmediate64 {
        destination: Register,
        value: u64,
    },
    Load64 {
        destination: Register,
        base: Register,
        displacement: i32,
    },
    Store64 {
        base: Register,
        displacement: i32,
        source: Register,
    },
    Add {
        destination: Register,
        source: Register,
    },
    Subtract {
        destination: Register,
        source: Register,
    },
    MultiplySigned {
        destination: Register,
        source: Register,
    },
    Negate(Register),
    SignExtendRaxIntoRdx,
    DivideSigned(Register),
    Compare {
        left: Register,
        right: Register,
    },
    Test(Register),
    MaterializeCondition(Condition),
    StackAllocate(u32),
    StackDeallocate(u32),
    Call(SymbolId),
    Jump(BlockId),
    JumpIfZero(BlockId),
    Return,
    Syscall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Condition {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineFunction {
    pub symbol: SymbolId,
    pub name: String,
    pub instructions: Vec<Instruction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineModule {
    pub startup: Vec<Instruction>,
    pub entry_function: SymbolId,
    pub functions: Vec<MachineFunction>,
}
