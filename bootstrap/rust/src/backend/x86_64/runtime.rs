use crate::frontend::resolution::SymbolId;
use crate::middle::ir::BlockId;

use super::instruction::{Condition, Instruction, MachineFunction};
use super::register::Register;

pub const ALLOC: SymbolId = SymbolId(u32::MAX);
pub const BOUNDS_CHECK: SymbolId = SymbolId(u32::MAX - 1);
pub const LIST_LOAD: SymbolId = SymbolId(u32::MAX - 2);
pub const LIST_STORE: SymbolId = SymbolId(u32::MAX - 3);
pub const LIST_PUSH: SymbolId = SymbolId(u32::MAX - 4);
pub const LIST_POP: SymbolId = SymbolId(u32::MAX - 5);
pub const STRING_CONCAT: SymbolId = SymbolId(u32::MAX - 6);
pub const STRING_EQUAL: SymbolId = SymbolId(u32::MAX - 7);
pub const STRING_BYTE: SymbolId = SymbolId(u32::MAX - 8);
pub const READ_FILE: SymbolId = SymbolId(u32::MAX - 9);
pub const MAIN_ARGS: SymbolId = SymbolId(u32::MAX - 10);
pub const LIST_ELEMENT: SymbolId = SymbolId(u32::MAX - 11);
pub const LIST_POP_ELEMENT: SymbolId = SymbolId(u32::MAX - 12);
pub const STRING_SLICE: SymbolId = SymbolId(u32::MAX - 13);

pub fn functions() -> Vec<MachineFunction> {
    vec![
        allocate(),
        bounds_check(),
        list_load(),
        list_store(),
        list_push(),
        list_pop(),
        string_concat(),
        string_equal(),
        string_byte(),
        read_file(),
        main_args(),
        list_element(),
        list_pop_element(),
        string_slice(),
    ]
}

fn increment(register: Register) -> [Instruction; 2] {
    [
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 1,
        },
        Instruction::Add {
            destination: register,
            source: Register::Rax,
        },
    ]
}

fn allocate() -> MachineFunction {
    MachineFunction {
        symbol: ALLOC,
        name: "__aerofyl_alloc".into(),
        instructions: vec![
            Instruction::MoveRegister {
                destination: Register::Rsi,
                source: Register::Rdi,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 9,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rdi,
                value: 0,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rdx,
                value: 3,
            },
            Instruction::MoveImmediate64 {
                destination: Register::R10,
                value: 0x22,
            },
            Instruction::MoveImmediate64 {
                destination: Register::R8,
                value: u64::MAX,
            },
            Instruction::MoveImmediate64 {
                destination: Register::R9,
                value: 0,
            },
            Instruction::Syscall,
            Instruction::Test(Register::Rax),
            Instruction::JumpIf {
                condition: Condition::Less,
                target: BlockId(1),
            },
            Instruction::Return,
            Instruction::Label(BlockId(1)),
            Instruction::ExitFailure,
        ],
    }
}

fn bounds_check() -> MachineFunction {
    MachineFunction {
        symbol: BOUNDS_CHECK,
        name: "__aerofyl_bounds_check".into(),
        instructions: vec![
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 0,
            },
            Instruction::Compare {
                left: Register::Rdi,
                right: Register::Rax,
            },
            Instruction::JumpIf {
                condition: Condition::Less,
                target: BlockId(1),
            },
            Instruction::Compare {
                left: Register::Rdi,
                right: Register::Rsi,
            },
            Instruction::JumpIf {
                condition: Condition::GreaterEqual,
                target: BlockId(1),
            },
            Instruction::Return,
            Instruction::Label(BlockId(1)),
            Instruction::ExitFailure,
        ],
    }
}

fn list_load() -> MachineFunction {
    MachineFunction {
        symbol: LIST_LOAD,
        name: "__aerofyl_list_load".into(),
        instructions: vec![
            Instruction::Load64 {
                destination: Register::Rcx,
                base: Register::Rdi,
                displacement: 0,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 0,
            },
            Instruction::Compare {
                left: Register::Rsi,
                right: Register::Rax,
            },
            Instruction::JumpIf {
                condition: Condition::Less,
                target: BlockId(1),
            },
            Instruction::Compare {
                left: Register::Rsi,
                right: Register::Rcx,
            },
            Instruction::JumpIf {
                condition: Condition::GreaterEqual,
                target: BlockId(1),
            },
            Instruction::IndexedLoad64 {
                destination: Register::Rax,
                base: Register::Rdi,
                index: Register::Rsi,
                displacement: 24,
            },
            Instruction::Return,
            Instruction::Label(BlockId(1)),
            Instruction::ExitFailure,
        ],
    }
}

fn list_store() -> MachineFunction {
    MachineFunction {
        symbol: LIST_STORE,
        name: "__aerofyl_list_store".into(),
        instructions: vec![
            Instruction::Load64 {
                destination: Register::Rcx,
                base: Register::Rdi,
                displacement: 0,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 0,
            },
            Instruction::Compare {
                left: Register::Rsi,
                right: Register::Rax,
            },
            Instruction::JumpIf {
                condition: Condition::Less,
                target: BlockId(1),
            },
            Instruction::Compare {
                left: Register::Rsi,
                right: Register::Rcx,
            },
            Instruction::JumpIf {
                condition: Condition::GreaterEqual,
                target: BlockId(1),
            },
            Instruction::IndexedStore64 {
                base: Register::Rdi,
                index: Register::Rsi,
                displacement: 24,
                source: Register::Rdx,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 0,
            },
            Instruction::Return,
            Instruction::Label(BlockId(1)),
            Instruction::ExitFailure,
        ],
    }
}

fn list_push() -> MachineFunction {
    MachineFunction {
        symbol: LIST_PUSH,
        name: "__aerofyl_list_push".into(),
        instructions: vec![
            Instruction::Push(Register::Rbx),
            Instruction::Push(Register::R12),
            Instruction::Push(Register::R13),
            Instruction::Push(Register::R14),
            Instruction::MoveRegister {
                destination: Register::Rbx,
                source: Register::Rdi,
            },
            Instruction::Load64 {
                destination: Register::R12,
                base: Register::Rbx,
                displacement: 0,
            },
            Instruction::Load64 {
                destination: Register::R13,
                base: Register::Rbx,
                displacement: 8,
            },
            Instruction::Load64 {
                destination: Register::R14,
                base: Register::Rbx,
                displacement: 16,
            },
            Instruction::Compare {
                left: Register::R12,
                right: Register::R13,
            },
            Instruction::JumpIf {
                condition: Condition::Less,
                target: BlockId(2),
            },
            // mremap(old, 24 + capacity*stride, 24 + capacity*2*stride, MAYMOVE)
            Instruction::MoveRegister {
                destination: Register::Rdi,
                source: Register::Rbx,
            },
            Instruction::MoveRegister {
                destination: Register::Rax,
                source: Register::R13,
            },
            Instruction::MultiplySigned {
                destination: Register::Rax,
                source: Register::R14,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rcx,
                value: 24,
            },
            Instruction::Add {
                destination: Register::Rax,
                source: Register::Rcx,
            },
            Instruction::MoveRegister {
                destination: Register::Rsi,
                source: Register::Rax,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 2,
            },
            Instruction::MultiplySigned {
                destination: Register::R13,
                source: Register::Rax,
            },
            Instruction::MoveRegister {
                destination: Register::Rax,
                source: Register::R13,
            },
            Instruction::MultiplySigned {
                destination: Register::Rax,
                source: Register::R14,
            },
            Instruction::Add {
                destination: Register::Rax,
                source: Register::Rcx,
            },
            Instruction::MoveRegister {
                destination: Register::Rdx,
                source: Register::Rax,
            },
            Instruction::MoveImmediate64 {
                destination: Register::R10,
                value: 1,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 25,
            },
            Instruction::Syscall,
            Instruction::Test(Register::Rax),
            Instruction::JumpIf {
                condition: Condition::Less,
                target: BlockId(1),
            },
            Instruction::MoveRegister {
                destination: Register::Rbx,
                source: Register::Rax,
            },
            Instruction::Store64 {
                base: Register::Rbx,
                displacement: 8,
                source: Register::R13,
            },
            Instruction::Label(BlockId(2)),
            Instruction::MoveRegister {
                destination: Register::Rdx,
                source: Register::R12,
            },
            Instruction::MultiplySigned {
                destination: Register::Rdx,
                source: Register::R14,
            },
            Instruction::Add {
                destination: Register::Rdx,
                source: Register::Rbx,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 24,
            },
            Instruction::Add {
                destination: Register::Rdx,
                source: Register::Rax,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 1,
            },
            Instruction::Add {
                destination: Register::R12,
                source: Register::Rax,
            },
            Instruction::Store64 {
                base: Register::Rbx,
                displacement: 0,
                source: Register::R12,
            },
            Instruction::MoveRegister {
                destination: Register::Rax,
                source: Register::Rbx,
            },
            Instruction::Pop(Register::R14),
            Instruction::Pop(Register::R13),
            Instruction::Pop(Register::R12),
            Instruction::Pop(Register::Rbx),
            Instruction::Return,
            Instruction::Label(BlockId(1)),
            Instruction::ExitFailure,
        ],
    }
}

fn list_pop() -> MachineFunction {
    MachineFunction {
        symbol: LIST_POP,
        name: "__aerofyl_list_pop".into(),
        instructions: vec![
            Instruction::Load64 {
                destination: Register::Rcx,
                base: Register::Rdi,
                displacement: 0,
            },
            Instruction::Test(Register::Rcx),
            Instruction::JumpIfZero(BlockId(1)),
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 1,
            },
            Instruction::Subtract {
                destination: Register::Rcx,
                source: Register::Rax,
            },
            Instruction::Store64 {
                base: Register::Rdi,
                displacement: 0,
                source: Register::Rcx,
            },
            Instruction::IndexedLoad64 {
                destination: Register::Rax,
                base: Register::Rdi,
                index: Register::Rcx,
                displacement: 24,
            },
            Instruction::Return,
            Instruction::Label(BlockId(1)),
            Instruction::ExitFailure,
        ],
    }
}

fn string_concat() -> MachineFunction {
    MachineFunction {
        symbol: STRING_CONCAT,
        name: "__aerofyl_string_concat".into(),
        instructions: vec![
            Instruction::Push(Register::Rbx),
            Instruction::Push(Register::R12),
            Instruction::Push(Register::R13),
            Instruction::Push(Register::R14),
            Instruction::Push(Register::R15),
            Instruction::MoveRegister {
                destination: Register::R12,
                source: Register::Rdi,
            },
            Instruction::MoveRegister {
                destination: Register::R13,
                source: Register::Rsi,
            },
            Instruction::Load64 {
                destination: Register::R14,
                base: Register::R12,
                displacement: 0,
            },
            Instruction::Load64 {
                destination: Register::R15,
                base: Register::R13,
                displacement: 0,
            },
            Instruction::MoveRegister {
                destination: Register::Rdi,
                source: Register::R14,
            },
            Instruction::Add {
                destination: Register::Rdi,
                source: Register::R15,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 8,
            },
            Instruction::Add {
                destination: Register::Rdi,
                source: Register::Rax,
            },
            Instruction::Call(ALLOC),
            Instruction::MoveRegister {
                destination: Register::Rbx,
                source: Register::Rax,
            },
            Instruction::MoveRegister {
                destination: Register::Rcx,
                source: Register::R14,
            },
            Instruction::Add {
                destination: Register::Rcx,
                source: Register::R15,
            },
            Instruction::Store64 {
                base: Register::Rbx,
                displacement: 0,
                source: Register::Rcx,
            },
            Instruction::MoveRegister {
                destination: Register::Rdi,
                source: Register::R12,
            },
            Instruction::MoveRegister {
                destination: Register::Rsi,
                source: Register::Rbx,
            },
            Instruction::MoveRegister {
                destination: Register::Rdx,
                source: Register::R14,
            },
            Instruction::MoveImmediate64 {
                destination: Register::R8,
                value: 1,
            },
            Instruction::Label(BlockId(1)),
            Instruction::Test(Register::Rdx),
            Instruction::JumpIfZero(BlockId(2)),
            Instruction::Load8 {
                destination: Register::Rax,
                base: Register::Rdi,
                displacement: 8,
            },
            Instruction::Store8 {
                base: Register::Rsi,
                displacement: 8,
                source: Register::Rax,
            },
            Instruction::Add {
                destination: Register::Rdi,
                source: Register::R8,
            },
            Instruction::Add {
                destination: Register::Rsi,
                source: Register::R8,
            },
            Instruction::Subtract {
                destination: Register::Rdx,
                source: Register::R8,
            },
            Instruction::Jump(BlockId(1)),
            Instruction::Label(BlockId(2)),
            Instruction::MoveRegister {
                destination: Register::Rdi,
                source: Register::R13,
            },
            Instruction::MoveRegister {
                destination: Register::Rdx,
                source: Register::R15,
            },
            Instruction::Label(BlockId(3)),
            Instruction::Test(Register::Rdx),
            Instruction::JumpIfZero(BlockId(4)),
            Instruction::Load8 {
                destination: Register::Rax,
                base: Register::Rdi,
                displacement: 8,
            },
            Instruction::Store8 {
                base: Register::Rsi,
                displacement: 8,
                source: Register::Rax,
            },
            Instruction::Add {
                destination: Register::Rdi,
                source: Register::R8,
            },
            Instruction::Add {
                destination: Register::Rsi,
                source: Register::R8,
            },
            Instruction::Subtract {
                destination: Register::Rdx,
                source: Register::R8,
            },
            Instruction::Jump(BlockId(3)),
            Instruction::Label(BlockId(4)),
            Instruction::MoveRegister {
                destination: Register::Rax,
                source: Register::Rbx,
            },
            Instruction::Pop(Register::R15),
            Instruction::Pop(Register::R14),
            Instruction::Pop(Register::R13),
            Instruction::Pop(Register::R12),
            Instruction::Pop(Register::Rbx),
            Instruction::Return,
        ],
    }
}

fn string_equal() -> MachineFunction {
    MachineFunction {
        symbol: STRING_EQUAL,
        name: "__aerofyl_string_equal".into(),
        instructions: vec![
            Instruction::Load64 {
                destination: Register::Rdx,
                base: Register::Rdi,
                displacement: 0,
            },
            Instruction::Load64 {
                destination: Register::Rcx,
                base: Register::Rsi,
                displacement: 0,
            },
            Instruction::Compare {
                left: Register::Rdx,
                right: Register::Rcx,
            },
            Instruction::JumpIf {
                condition: Condition::NotEqual,
                target: BlockId(2),
            },
            Instruction::MoveImmediate64 {
                destination: Register::R8,
                value: 1,
            },
            Instruction::Label(BlockId(1)),
            Instruction::Test(Register::Rdx),
            Instruction::JumpIfZero(BlockId(3)),
            Instruction::Load8 {
                destination: Register::Rax,
                base: Register::Rdi,
                displacement: 8,
            },
            Instruction::Load8 {
                destination: Register::Rcx,
                base: Register::Rsi,
                displacement: 8,
            },
            Instruction::Compare {
                left: Register::Rax,
                right: Register::Rcx,
            },
            Instruction::JumpIf {
                condition: Condition::NotEqual,
                target: BlockId(2),
            },
            Instruction::Add {
                destination: Register::Rdi,
                source: Register::R8,
            },
            Instruction::Add {
                destination: Register::Rsi,
                source: Register::R8,
            },
            Instruction::Subtract {
                destination: Register::Rdx,
                source: Register::R8,
            },
            Instruction::Jump(BlockId(1)),
            Instruction::Label(BlockId(2)),
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 0,
            },
            Instruction::Return,
            Instruction::Label(BlockId(3)),
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 1,
            },
            Instruction::Return,
        ],
    }
}

fn string_byte() -> MachineFunction {
    MachineFunction {
        symbol: STRING_BYTE,
        name: "__aerofyl_string_byte".into(),
        instructions: vec![
            Instruction::Load64 {
                destination: Register::Rcx,
                base: Register::Rdi,
                displacement: 0,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 0,
            },
            Instruction::Compare {
                left: Register::Rsi,
                right: Register::Rax,
            },
            Instruction::JumpIf {
                condition: Condition::Less,
                target: BlockId(1),
            },
            Instruction::Compare {
                left: Register::Rsi,
                right: Register::Rcx,
            },
            Instruction::JumpIf {
                condition: Condition::GreaterEqual,
                target: BlockId(1),
            },
            Instruction::IndexedLoad8 {
                destination: Register::Rax,
                base: Register::Rdi,
                index: Register::Rsi,
                displacement: 8,
            },
            Instruction::Return,
            Instruction::Label(BlockId(1)),
            Instruction::ExitFailure,
        ],
    }
}

fn main_args() -> MachineFunction {
    let mut instructions = vec![
        Instruction::Push(Register::Rbx),
        Instruction::Push(Register::R12),
        Instruction::Push(Register::R13),
        Instruction::Push(Register::R14),
        Instruction::Push(Register::R15),
        Instruction::Push(Register::Rbp),
        Instruction::StackAllocate(8),
        Instruction::MoveRegister {
            destination: Register::Rbx,
            source: Register::Rdi,
        },
        Instruction::Load64 {
            destination: Register::R12,
            base: Register::Rbx,
            displacement: 0,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0,
        },
        Instruction::Compare {
            left: Register::R12,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::LessEqual,
            target: BlockId(6),
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 1,
        },
        Instruction::Subtract {
            destination: Register::R12,
            source: Register::Rax,
        },
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::R12,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 8,
        },
        Instruction::MultiplySigned {
            destination: Register::Rdi,
            source: Register::Rax,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 24,
        },
        Instruction::Add {
            destination: Register::Rdi,
            source: Register::Rax,
        },
        Instruction::Call(ALLOC),
        Instruction::MoveRegister {
            destination: Register::R13,
            source: Register::Rax,
        },
        Instruction::Store64 {
            base: Register::R13,
            displacement: 0,
            source: Register::R12,
        },
        Instruction::Store64 {
            base: Register::R13,
            displacement: 8,
            source: Register::R12,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 8,
        },
        Instruction::Store64 {
            base: Register::R13,
            displacement: 16,
            source: Register::Rax,
        },
        Instruction::MoveImmediate64 {
            destination: Register::R14,
            value: 0,
        },
        Instruction::Label(BlockId(0)),
        Instruction::Compare {
            left: Register::R14,
            right: Register::R12,
        },
        Instruction::JumpIf {
            condition: Condition::GreaterEqual,
            target: BlockId(4),
        },
        Instruction::IndexedLoad64 {
            destination: Register::Rbp,
            base: Register::Rbx,
            index: Register::R14,
            displacement: 16,
        },
        Instruction::MoveImmediate64 {
            destination: Register::R15,
            value: 0,
        },
        Instruction::Label(BlockId(1)),
        Instruction::IndexedLoad8 {
            destination: Register::Rax,
            base: Register::Rbp,
            index: Register::R15,
            displacement: 0,
        },
        Instruction::Test(Register::Rax),
        Instruction::JumpIfZero(BlockId(2)),
    ];
    instructions.extend(increment(Register::R15));
    instructions.extend([
        Instruction::Jump(BlockId(1)),
        Instruction::Label(BlockId(2)),
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::R15,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 8,
        },
        Instruction::Add {
            destination: Register::Rdi,
            source: Register::Rax,
        },
        Instruction::Call(ALLOC),
        Instruction::MoveRegister {
            destination: Register::R11,
            source: Register::Rax,
        },
        Instruction::Store64 {
            base: Register::R11,
            displacement: 0,
            source: Register::R15,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rcx,
            value: 0,
        },
        Instruction::Label(BlockId(3)),
        Instruction::Compare {
            left: Register::Rcx,
            right: Register::R15,
        },
        Instruction::JumpIf {
            condition: Condition::GreaterEqual,
            target: BlockId(5),
        },
        Instruction::IndexedLoad8 {
            destination: Register::Rdx,
            base: Register::Rbp,
            index: Register::Rcx,
            displacement: 0,
        },
        Instruction::IndexedStore8 {
            base: Register::R11,
            index: Register::Rcx,
            displacement: 8,
            source: Register::Rdx,
        },
    ]);
    instructions.extend(increment(Register::Rcx));
    instructions.extend([
        Instruction::Jump(BlockId(3)),
        Instruction::Label(BlockId(5)),
        Instruction::IndexedStore64 {
            base: Register::R13,
            index: Register::R14,
            displacement: 24,
            source: Register::R11,
        },
    ]);
    instructions.extend(increment(Register::R14));
    instructions.extend([
        Instruction::Jump(BlockId(0)),
        Instruction::Label(BlockId(4)),
        Instruction::MoveRegister {
            destination: Register::Rax,
            source: Register::R13,
        },
        Instruction::StackDeallocate(8),
        Instruction::Pop(Register::Rbp),
        Instruction::Pop(Register::R15),
        Instruction::Pop(Register::R14),
        Instruction::Pop(Register::R13),
        Instruction::Pop(Register::R12),
        Instruction::Pop(Register::Rbx),
        Instruction::Return,
        Instruction::Label(BlockId(6)),
        Instruction::ExitFailure,
    ]);
    MachineFunction {
        symbol: MAIN_ARGS,
        name: "__aerofyl_main_args".into(),
        instructions,
    }
}

fn read_file() -> MachineFunction {
    let mut instructions = vec![
        Instruction::Push(Register::Rbx),
        Instruction::Push(Register::R12),
        Instruction::Push(Register::R13),
        Instruction::Push(Register::R14),
        Instruction::Push(Register::R15),
        Instruction::Push(Register::Rbp),
        Instruction::StackAllocate(8),
        Instruction::MoveRegister {
            destination: Register::R12,
            source: Register::Rdi,
        },
        Instruction::Load64 {
            destination: Register::R13,
            base: Register::R12,
            displacement: 0,
        },
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::R13,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 1,
        },
        Instruction::Add {
            destination: Register::Rdi,
            source: Register::Rax,
        },
        Instruction::Call(ALLOC),
        Instruction::MoveRegister {
            destination: Register::R14,
            source: Register::Rax,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rbx,
            value: 0,
        },
        Instruction::Label(BlockId(0)),
        Instruction::Compare {
            left: Register::Rbx,
            right: Register::R13,
        },
        Instruction::JumpIf {
            condition: Condition::GreaterEqual,
            target: BlockId(1),
        },
        Instruction::IndexedLoad8 {
            destination: Register::Rdx,
            base: Register::R12,
            index: Register::Rbx,
            displacement: 8,
        },
        Instruction::IndexedStore8 {
            base: Register::R14,
            index: Register::Rbx,
            displacement: 0,
            source: Register::Rdx,
        },
    ];
    instructions.extend(increment(Register::Rbx));
    instructions.extend([
        Instruction::Jump(BlockId(0)),
        Instruction::Label(BlockId(1)),
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0,
        },
        Instruction::IndexedStore8 {
            base: Register::R14,
            index: Register::R13,
            displacement: 0,
            source: Register::Rax,
        },
        // openat(AT_FDCWD, path, O_RDONLY, 0)
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 257,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdi,
            value: (-100_i64) as u64,
        },
        Instruction::MoveRegister {
            destination: Register::Rsi,
            source: Register::R14,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: 0,
        },
        Instruction::MoveImmediate64 {
            destination: Register::R10,
            value: 0,
        },
        Instruction::Syscall,
        Instruction::Test(Register::Rax),
        Instruction::JumpIf {
            condition: Condition::Less,
            target: BlockId(8),
        },
        Instruction::MoveRegister {
            destination: Register::R12,
            source: Register::Rax,
        },
        // lseek(fd, 0, SEEK_END)
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 8,
        },
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::R12,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rsi,
            value: 0,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: 2,
        },
        Instruction::Syscall,
        Instruction::Test(Register::Rax),
        Instruction::JumpIf {
            condition: Condition::Less,
            target: BlockId(8),
        },
        Instruction::MoveRegister {
            destination: Register::R13,
            source: Register::Rax,
        },
        // lseek(fd, 0, SEEK_SET)
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 8,
        },
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::R12,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rsi,
            value: 0,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: 0,
        },
        Instruction::Syscall,
        Instruction::Test(Register::Rax),
        Instruction::JumpIf {
            condition: Condition::Less,
            target: BlockId(8),
        },
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::R13,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 8,
        },
        Instruction::Add {
            destination: Register::Rdi,
            source: Register::Rax,
        },
        Instruction::Call(ALLOC),
        Instruction::MoveRegister {
            destination: Register::R14,
            source: Register::Rax,
        },
        Instruction::MoveImmediate64 {
            destination: Register::R15,
            value: 0,
        },
        Instruction::Label(BlockId(3)),
        Instruction::Compare {
            left: Register::R15,
            right: Register::R13,
        },
        Instruction::JumpIf {
            condition: Condition::GreaterEqual,
            target: BlockId(5),
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0,
        },
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::R12,
        },
        Instruction::MoveRegister {
            destination: Register::Rsi,
            source: Register::R14,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rbp,
            value: 8,
        },
        Instruction::Add {
            destination: Register::Rsi,
            source: Register::Rbp,
        },
        Instruction::Add {
            destination: Register::Rsi,
            source: Register::R15,
        },
        Instruction::MoveRegister {
            destination: Register::Rdx,
            source: Register::R13,
        },
        Instruction::Subtract {
            destination: Register::Rdx,
            source: Register::R15,
        },
        Instruction::Syscall,
        Instruction::Test(Register::Rax),
        Instruction::JumpIf {
            condition: Condition::Less,
            target: BlockId(8),
        },
        Instruction::JumpIfZero(BlockId(5)),
        Instruction::Add {
            destination: Register::R15,
            source: Register::Rax,
        },
        Instruction::Jump(BlockId(3)),
        Instruction::Label(BlockId(5)),
        Instruction::Store64 {
            base: Register::R14,
            displacement: 0,
            source: Register::R15,
        },
        // close(fd)
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 3,
        },
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::R12,
        },
        Instruction::Syscall,
        Instruction::Test(Register::Rax),
        Instruction::JumpIf {
            condition: Condition::Less,
            target: BlockId(8),
        },
        Instruction::MoveRegister {
            destination: Register::Rax,
            source: Register::R14,
        },
        Instruction::StackDeallocate(8),
        Instruction::Pop(Register::Rbp),
        Instruction::Pop(Register::R15),
        Instruction::Pop(Register::R14),
        Instruction::Pop(Register::R13),
        Instruction::Pop(Register::R12),
        Instruction::Pop(Register::Rbx),
        Instruction::Return,
        Instruction::Label(BlockId(8)),
        Instruction::ExitFailure,
    ]);
    MachineFunction {
        symbol: READ_FILE,
        name: "__aerofyl_read_file".into(),
        instructions,
    }
}

fn list_element() -> MachineFunction {
    MachineFunction {
        symbol: LIST_ELEMENT,
        name: "__aerofyl_list_element".into(),
        instructions: vec![
            Instruction::Load64 {
                destination: Register::Rcx,
                base: Register::Rdi,
                displacement: 0,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 0,
            },
            Instruction::Compare {
                left: Register::Rsi,
                right: Register::Rax,
            },
            Instruction::JumpIf {
                condition: Condition::Less,
                target: BlockId(1),
            },
            Instruction::Compare {
                left: Register::Rsi,
                right: Register::Rcx,
            },
            Instruction::JumpIf {
                condition: Condition::GreaterEqual,
                target: BlockId(1),
            },
            Instruction::Load64 {
                destination: Register::Rdx,
                base: Register::Rdi,
                displacement: 16,
            },
            Instruction::MultiplySigned {
                destination: Register::Rsi,
                source: Register::Rdx,
            },
            Instruction::MoveRegister {
                destination: Register::Rax,
                source: Register::Rdi,
            },
            Instruction::Add {
                destination: Register::Rax,
                source: Register::Rsi,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rdx,
                value: 24,
            },
            Instruction::Add {
                destination: Register::Rax,
                source: Register::Rdx,
            },
            Instruction::Return,
            Instruction::Label(BlockId(1)),
            Instruction::ExitFailure,
        ],
    }
}

fn list_pop_element() -> MachineFunction {
    MachineFunction {
        symbol: LIST_POP_ELEMENT,
        name: "__aerofyl_list_pop_element".into(),
        instructions: vec![
            Instruction::Load64 {
                destination: Register::Rcx,
                base: Register::Rdi,
                displacement: 0,
            },
            Instruction::Test(Register::Rcx),
            Instruction::JumpIfZero(BlockId(1)),
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 1,
            },
            Instruction::Subtract {
                destination: Register::Rcx,
                source: Register::Rax,
            },
            Instruction::Store64 {
                base: Register::Rdi,
                displacement: 0,
                source: Register::Rcx,
            },
            Instruction::Load64 {
                destination: Register::Rdx,
                base: Register::Rdi,
                displacement: 16,
            },
            Instruction::MultiplySigned {
                destination: Register::Rcx,
                source: Register::Rdx,
            },
            Instruction::MoveRegister {
                destination: Register::Rax,
                source: Register::Rdi,
            },
            Instruction::Add {
                destination: Register::Rax,
                source: Register::Rcx,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rdx,
                value: 24,
            },
            Instruction::Add {
                destination: Register::Rax,
                source: Register::Rdx,
            },
            Instruction::Return,
            Instruction::Label(BlockId(1)),
            Instruction::ExitFailure,
        ],
    }
}

fn string_slice() -> MachineFunction {
    let mut instructions = vec![
        Instruction::Push(Register::Rbx),
        Instruction::Push(Register::R12),
        Instruction::Push(Register::R13),
        Instruction::Push(Register::R14),
        Instruction::Push(Register::R15),
        Instruction::MoveRegister {
            destination: Register::R12,
            source: Register::Rdi,
        },
        Instruction::MoveRegister {
            destination: Register::R13,
            source: Register::Rsi,
        },
        Instruction::MoveRegister {
            destination: Register::R14,
            source: Register::Rdx,
        },
        Instruction::Load64 {
            destination: Register::R15,
            base: Register::R12,
            displacement: 0,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0,
        },
        Instruction::Compare {
            left: Register::R13,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Less,
            target: BlockId(2),
        },
        Instruction::Compare {
            left: Register::R14,
            right: Register::R13,
        },
        Instruction::JumpIf {
            condition: Condition::Less,
            target: BlockId(2),
        },
        Instruction::Compare {
            left: Register::R14,
            right: Register::R15,
        },
        Instruction::JumpIf {
            condition: Condition::Greater,
            target: BlockId(2),
        },
        Instruction::Subtract {
            destination: Register::R14,
            source: Register::R13,
        },
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::R14,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 8,
        },
        Instruction::Add {
            destination: Register::Rdi,
            source: Register::Rax,
        },
        Instruction::Call(ALLOC),
        Instruction::MoveRegister {
            destination: Register::Rbx,
            source: Register::Rax,
        },
        Instruction::Store64 {
            base: Register::Rbx,
            displacement: 0,
            source: Register::R14,
        },
        Instruction::MoveImmediate64 {
            destination: Register::R15,
            value: 0,
        },
        Instruction::Label(BlockId(0)),
        Instruction::Compare {
            left: Register::R15,
            right: Register::R14,
        },
        Instruction::JumpIf {
            condition: Condition::GreaterEqual,
            target: BlockId(1),
        },
        Instruction::IndexedLoad8 {
            destination: Register::Rdx,
            base: Register::R12,
            index: Register::R13,
            displacement: 8,
        },
        Instruction::IndexedStore8 {
            base: Register::Rbx,
            index: Register::R15,
            displacement: 8,
            source: Register::Rdx,
        },
    ];
    instructions.extend(increment(Register::R13));
    instructions.extend(increment(Register::R15));
    instructions.extend([
        Instruction::Jump(BlockId(0)),
        Instruction::Label(BlockId(1)),
        Instruction::MoveRegister {
            destination: Register::Rax,
            source: Register::Rbx,
        },
        Instruction::Pop(Register::R15),
        Instruction::Pop(Register::R14),
        Instruction::Pop(Register::R13),
        Instruction::Pop(Register::R12),
        Instruction::Pop(Register::Rbx),
        Instruction::Return,
        Instruction::Label(BlockId(2)),
        Instruction::ExitFailure,
    ]);
    MachineFunction {
        symbol: STRING_SLICE,
        name: "__aerofyl_string_slice".into(),
        instructions,
    }
}
