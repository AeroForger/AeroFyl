use crate::frontend::resolution::SymbolId;
use crate::middle::ir::BlockId;
use std::collections::HashSet;

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
pub const WRITE_FILE: SymbolId = SymbolId(u32::MAX - 14);
pub const INT_TO_CHAR: SymbolId = SymbolId(u32::MAX - 15);
pub const INTEGER_DIVIDE: SymbolId = SymbolId(u32::MAX - 16);
pub const WRITE_ALL: SymbolId = SymbolId(u32::MAX - 17);
pub const PRINT: SymbolId = SymbolId(u32::MAX - 18);
pub const INPUT_LINE: SymbolId = SymbolId(u32::MAX - 19);
pub const INPUT_INT: SymbolId = SymbolId(u32::MAX - 20);
pub const INPUT_BOOL: SymbolId = SymbolId(u32::MAX - 21);
pub const INPUT_CHAR: SymbolId = SymbolId(u32::MAX - 22);
pub const INPUT_ERROR: SymbolId = SymbolId(u32::MAX - 23);
pub const INT_TO_BYTE: SymbolId = SymbolId(u32::MAX - 24);
pub const READ_BYTES: SymbolId = SymbolId(u32::MAX - 25);
pub const WRITE_BYTES: SymbolId = SymbolId(u32::MAX - 26);
pub const EXISTS: SymbolId = SymbolId(u32::MAX - 27);
pub const FS_ERROR: SymbolId = SymbolId(u32::MAX - 28);
pub const OPTIONAL_VALUE: SymbolId = SymbolId(u32::MAX - 29);
pub const ENUM_PAYLOAD: SymbolId = SymbolId(u32::MAX - 30);

pub const PROCESS_EXECUTE: SymbolId = SymbolId(u32::MAX - 31);

pub const OS_ERROR: SymbolId = SymbolId(u32::MAX - 32);

pub fn functions_for(roots: &HashSet<SymbolId>) -> Vec<MachineFunction> {
    let functions = vec![
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
        write_file(),
        int_to_char(),
        integer_divide(),
        write_all(),
        print(),
        input_line(),
        input_int(),
        input_bool(),
        input_char(),
        input_error(),
        int_to_byte(),
        read_bytes(),
        write_bytes(),
        exists(),
        fs_error(),
        optional_value(),
        enum_payload(),
        process_execute(),
        os_error(),
    ];
    let runtime_symbols: HashSet<_> = functions.iter().map(|function| function.symbol).collect();
    let mut reachable = HashSet::new();
    let mut pending: Vec<_> = roots
        .iter()
        .copied()
        .filter(|symbol| runtime_symbols.contains(symbol))
        .collect();
    while let Some(symbol) = pending.pop() {
        if !reachable.insert(symbol) {
            continue;
        }
        if let Some(function) = functions.iter().find(|function| function.symbol == symbol) {
            pending.extend(function.instructions.iter().filter_map(|instruction| {
                let Instruction::Call(target) = instruction else {
                    return None;
                };
                runtime_symbols.contains(target).then_some(*target)
            }));
        }
    }
    functions
        .into_iter()
        .filter(|function| reachable.contains(&function.symbol))
        .collect()
}

fn enum_payload() -> MachineFunction {
    MachineFunction {
        symbol: ENUM_PAYLOAD,
        name: "__aerofyl_enum_payload".into(),
        instructions: vec![
            Instruction::Load64 {
                destination: Register::Rax,
                base: Register::Rdi,
                displacement: 0,
            },
            Instruction::Compare {
                left: Register::Rax,
                right: Register::Rsi,
            },
            Instruction::JumpIf {
                condition: Condition::Equal,
                target: BlockId(1),
            },
            Instruction::ExitFailure,
            Instruction::Label(BlockId(1)),
            Instruction::Load64 {
                destination: Register::Rax,
                base: Register::Rdi,
                displacement: 8,
            },
            Instruction::Return,
        ],
    }
}

fn optional_value() -> MachineFunction {
    MachineFunction {
        symbol: OPTIONAL_VALUE,
        name: "__aerofyl_optional_value".into(),
        instructions: vec![
            Instruction::Load64 {
                destination: Register::Rax,
                base: Register::Rdi,
                displacement: 0,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rcx,
                value: 0,
            },
            Instruction::Compare {
                left: Register::Rax,
                right: Register::Rcx,
            },
            Instruction::JumpIf {
                condition: Condition::NotEqual,
                target: BlockId(1),
            },
            Instruction::ExitFailure,
            Instruction::Label(BlockId(1)),
            Instruction::Load64 {
                destination: Register::Rax,
                base: Register::Rdi,
                displacement: 8,
            },
            Instruction::Return,
        ],
    }
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
            Instruction::Load64 {
                destination: Register::R10,
                base: Register::Rdi,
                displacement: 24,
            },
            Instruction::IndexedLoad64 {
                destination: Register::Rax,
                base: Register::R10,
                index: Register::Rsi,
                displacement: 0,
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
            Instruction::Load64 {
                destination: Register::R10,
                base: Register::Rdi,
                displacement: 24,
            },
            Instruction::IndexedStore64 {
                base: Register::R10,
                index: Register::Rsi,
                displacement: 0,
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
            Instruction::Push(Register::R15),
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
            Instruction::Load64 {
                destination: Register::R15,
                base: Register::Rbx,
                displacement: 24,
            },
            Instruction::Compare {
                left: Register::R12,
                right: Register::R13,
            },
            Instruction::JumpIf {
                condition: Condition::Less,
                target: BlockId(2),
            },
            // mremap(data, capacity*stride, capacity*2*stride, MAYMOVE)
            Instruction::MoveRegister {
                destination: Register::Rdi,
                source: Register::R15,
            },
            Instruction::MoveRegister {
                destination: Register::Rax,
                source: Register::R13,
            },
            Instruction::MultiplySigned {
                destination: Register::Rax,
                source: Register::R14,
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
                destination: Register::R15,
                source: Register::Rax,
            },
            Instruction::Store64 {
                base: Register::Rbx,
                displacement: 8,
                source: Register::R13,
            },
            Instruction::Store64 {
                base: Register::Rbx,
                displacement: 24,
                source: Register::R15,
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
                source: Register::R15,
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
            Instruction::Pop(Register::R15),
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
            Instruction::Load64 {
                destination: Register::R10,
                base: Register::Rdi,
                displacement: 24,
            },
            Instruction::IndexedLoad64 {
                destination: Register::Rax,
                base: Register::R10,
                index: Register::Rcx,
                displacement: 0,
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
            value: 4,
        },
        Instruction::Add {
            destination: Register::Rdi,
            source: Register::Rax,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 8,
        },
        Instruction::MultiplySigned {
            destination: Register::Rdi,
            source: Register::Rax,
        },
        Instruction::Call(ALLOC),
        Instruction::Store64 {
            base: Register::Rsp,
            displacement: 0,
            source: Register::Rax,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdi,
            value: 32,
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
        Instruction::MoveRegister {
            destination: Register::Rax,
            source: Register::R12,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rcx,
            value: 4,
        },
        Instruction::Add {
            destination: Register::Rax,
            source: Register::Rcx,
        },
        Instruction::Store64 {
            base: Register::R13,
            displacement: 8,
            source: Register::Rax,
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
        Instruction::Load64 {
            destination: Register::Rax,
            base: Register::Rsp,
            displacement: 0,
        },
        Instruction::Store64 {
            base: Register::R13,
            displacement: 24,
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
        Instruction::Load64 {
            destination: Register::Rax,
            base: Register::R13,
            displacement: 24,
        },
        Instruction::IndexedStore64 {
            base: Register::Rax,
            index: Register::R14,
            displacement: 0,
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
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: (-4_i64) as u64,
        },
        Instruction::Compare {
            left: Register::Rax,
            right: Register::Rdx,
        },
        Instruction::JumpIf {
            condition: Condition::Equal,
            target: BlockId(1),
        },
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
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: (-4_i64) as u64,
        },
        Instruction::Compare {
            left: Register::Rax,
            right: Register::Rdx,
        },
        Instruction::JumpIf {
            condition: Condition::Equal,
            target: BlockId(3),
        },
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
        Instruction::MoveImmediate64 {
            destination: Register::Rdi,
            value: 0,
        },
        Instruction::Call(FS_ERROR),
        Instruction::ExitFailure,
    ]);
    MachineFunction {
        symbol: READ_FILE,
        name: "__aerofyl_read_file".into(),
        instructions,
    }
}

fn write_file() -> MachineFunction {
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
        Instruction::MoveRegister {
            destination: Register::R13,
            source: Register::Rsi,
        },
        Instruction::Load64 {
            destination: Register::R14,
            base: Register::R12,
            displacement: 0,
        },
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::R14,
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
            destination: Register::R15,
            source: Register::Rax,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rbx,
            value: 0,
        },
        Instruction::Label(BlockId(0)),
        Instruction::Compare {
            left: Register::Rbx,
            right: Register::R14,
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
            base: Register::R15,
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
            base: Register::R15,
            index: Register::R14,
            displacement: 0,
            source: Register::Rax,
        },
        // openat(AT_FDCWD, path, O_WRONLY | O_CREAT | O_TRUNC, 0666)
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
            source: Register::R15,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: 577,
        },
        Instruction::MoveImmediate64 {
            destination: Register::R10,
            value: 0o666,
        },
        Instruction::Syscall,
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: (-4_i64) as u64,
        },
        Instruction::Compare {
            left: Register::Rax,
            right: Register::Rdx,
        },
        Instruction::JumpIf {
            condition: Condition::Equal,
            target: BlockId(1),
        },
        Instruction::Test(Register::Rax),
        Instruction::JumpIf {
            condition: Condition::Less,
            target: BlockId(8),
        },
        Instruction::MoveRegister {
            destination: Register::R12,
            source: Register::Rax,
        },
        Instruction::Load64 {
            destination: Register::R15,
            base: Register::R13,
            displacement: 0,
        },
        Instruction::MoveImmediate64 {
            destination: Register::R14,
            value: 0,
        },
        Instruction::Label(BlockId(2)),
        Instruction::Compare {
            left: Register::R14,
            right: Register::R15,
        },
        Instruction::JumpIf {
            condition: Condition::GreaterEqual,
            target: BlockId(3),
        },
        // write(fd, data + 8 + offset, length - offset)
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 1,
        },
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::R12,
        },
        Instruction::MoveRegister {
            destination: Register::Rsi,
            source: Register::R13,
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
            source: Register::R14,
        },
        Instruction::MoveRegister {
            destination: Register::Rdx,
            source: Register::R15,
        },
        Instruction::Subtract {
            destination: Register::Rdx,
            source: Register::R14,
        },
        Instruction::Syscall,
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: (-4_i64) as u64,
        },
        Instruction::Compare {
            left: Register::Rax,
            right: Register::Rdx,
        },
        Instruction::JumpIf {
            condition: Condition::Equal,
            target: BlockId(2),
        },
        Instruction::Test(Register::Rax),
        Instruction::JumpIf {
            condition: Condition::Less,
            target: BlockId(8),
        },
        Instruction::JumpIfZero(BlockId(8)),
        Instruction::Add {
            destination: Register::R14,
            source: Register::Rax,
        },
        Instruction::Jump(BlockId(2)),
        Instruction::Label(BlockId(3)),
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
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0,
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
        Instruction::MoveImmediate64 {
            destination: Register::Rdi,
            value: 1,
        },
        Instruction::Call(FS_ERROR),
        Instruction::ExitFailure,
    ]);
    MachineFunction {
        symbol: WRITE_FILE,
        name: "__aerofyl_write_file".into(),
        instructions,
    }
}

fn int_to_char() -> MachineFunction {
    MachineFunction {
        symbol: INT_TO_CHAR,
        name: "__aerofyl_int_to_char".into(),
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
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 0x10ffff,
            },
            Instruction::Compare {
                left: Register::Rdi,
                right: Register::Rax,
            },
            Instruction::JumpIf {
                condition: Condition::Greater,
                target: BlockId(1),
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 0xd800,
            },
            Instruction::Compare {
                left: Register::Rdi,
                right: Register::Rax,
            },
            Instruction::JumpIf {
                condition: Condition::Less,
                target: BlockId(0),
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 0xdfff,
            },
            Instruction::Compare {
                left: Register::Rdi,
                right: Register::Rax,
            },
            Instruction::JumpIf {
                condition: Condition::LessEqual,
                target: BlockId(1),
            },
            Instruction::Label(BlockId(0)),
            Instruction::MoveRegister {
                destination: Register::Rax,
                source: Register::Rdi,
            },
            Instruction::Return,
            Instruction::Label(BlockId(1)),
            Instruction::ExitFailure,
        ],
    }
}

fn integer_divide() -> MachineFunction {
    MachineFunction {
        symbol: INTEGER_DIVIDE,
        name: "__aerofyl_integer_divide".into(),
        instructions: vec![
            Instruction::Test(Register::Rsi),
            Instruction::JumpIfZero(BlockId(1)),
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: i64::MIN as u64,
            },
            Instruction::Compare {
                left: Register::Rdi,
                right: Register::Rax,
            },
            Instruction::JumpIf {
                condition: Condition::NotEqual,
                target: BlockId(0),
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: (-1_i64) as u64,
            },
            Instruction::Compare {
                left: Register::Rsi,
                right: Register::Rax,
            },
            Instruction::JumpIf {
                condition: Condition::Equal,
                target: BlockId(1),
            },
            Instruction::Label(BlockId(0)),
            Instruction::MoveRegister {
                destination: Register::Rax,
                source: Register::Rdi,
            },
            Instruction::SignExtendRaxIntoRdx,
            Instruction::DivideSigned(Register::Rsi),
            Instruction::Return,
            Instruction::Label(BlockId(1)),
            Instruction::ExitFailure,
        ],
    }
}

fn write_all() -> MachineFunction {
    MachineFunction {
        symbol: WRITE_ALL,
        name: "__aerofyl_write_all".into(),
        instructions: vec![
            Instruction::Push(Register::R12),
            Instruction::Push(Register::R13),
            Instruction::Push(Register::R14),
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
            Instruction::Label(BlockId(0)),
            Instruction::Test(Register::R13),
            Instruction::JumpIfZero(BlockId(1)),
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 1,
            },
            Instruction::MoveRegister {
                destination: Register::Rdi,
                source: Register::R14,
            },
            Instruction::MoveRegister {
                destination: Register::Rsi,
                source: Register::R12,
            },
            Instruction::MoveRegister {
                destination: Register::Rdx,
                source: Register::R13,
            },
            Instruction::Syscall,
            Instruction::MoveImmediate64 {
                destination: Register::Rcx,
                value: (-4_i64) as u64,
            },
            Instruction::Compare {
                left: Register::Rax,
                right: Register::Rcx,
            },
            Instruction::JumpIf {
                condition: Condition::Equal,
                target: BlockId(0),
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rcx,
                value: 0,
            },
            Instruction::Compare {
                left: Register::Rax,
                right: Register::Rcx,
            },
            Instruction::JumpIf {
                condition: Condition::LessEqual,
                target: BlockId(2),
            },
            Instruction::Add {
                destination: Register::R12,
                source: Register::Rax,
            },
            Instruction::Subtract {
                destination: Register::R13,
                source: Register::Rax,
            },
            Instruction::Jump(BlockId(0)),
            Instruction::Label(BlockId(1)),
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 0,
            },
            Instruction::Pop(Register::R14),
            Instruction::Pop(Register::R13),
            Instruction::Pop(Register::R12),
            Instruction::Return,
            Instruction::Label(BlockId(2)),
            Instruction::ExitFailure,
        ],
    }
}

fn print() -> MachineFunction {
    let mut instructions = vec![
        Instruction::Push(Register::Rbx),
        Instruction::Push(Register::R12),
        Instruction::Push(Register::R13),
        Instruction::Push(Register::R14),
        Instruction::Push(Register::R15),
        Instruction::Push(Register::Rbp),
        Instruction::StackAllocate(40),
        Instruction::MoveRegister {
            destination: Register::Rbp,
            source: Register::Rsp,
        },
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
        Instruction::MoveRegister {
            destination: Register::R15,
            source: Register::Rcx,
        },
        Instruction::Test(Register::R13),
        Instruction::JumpIfZero(BlockId(10)),
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 1,
        },
        Instruction::Compare {
            left: Register::R13,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Equal,
            target: BlockId(20),
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 2,
        },
        Instruction::Compare {
            left: Register::R13,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Equal,
            target: BlockId(30),
        },
        Instruction::Jump(BlockId(40)),
        // string
        Instruction::Label(BlockId(10)),
        Instruction::Load64 {
            destination: Register::Rsi,
            base: Register::R12,
            displacement: 0,
        },
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::R12,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 8,
        },
        Instruction::Add {
            destination: Register::Rdi,
            source: Register::Rax,
        },
        Instruction::MoveRegister {
            destination: Register::Rdx,
            source: Register::R14,
        },
        Instruction::Call(WRITE_ALL),
        Instruction::Jump(BlockId(50)),
        // int: form digits backwards in the stack buffer, retaining a negative
        // accumulator so INT64_MIN never needs to be negated.
        Instruction::Label(BlockId(20)),
        Instruction::MoveImmediate64 {
            destination: Register::Rbx,
            value: 39,
        },
        Instruction::MoveImmediate64 {
            destination: Register::R13,
            value: 0,
        },
        Instruction::MoveRegister {
            destination: Register::Rax,
            source: Register::R12,
        },
        Instruction::MoveImmediate64 {
            destination: Register::R8,
            value: 0,
        },
        Instruction::Test(Register::Rax),
        Instruction::JumpIf {
            condition: Condition::Less,
            target: BlockId(21),
        },
        Instruction::Negate(Register::Rax),
        Instruction::Jump(BlockId(22)),
        Instruction::Label(BlockId(21)),
        Instruction::MoveImmediate64 {
            destination: Register::R8,
            value: 1,
        },
        Instruction::Label(BlockId(22)),
        Instruction::MoveImmediate64 {
            destination: Register::R9,
            value: 10,
        },
        Instruction::SignExtendRaxIntoRdx,
        Instruction::DivideSigned(Register::R9),
        Instruction::Negate(Register::Rdx),
        Instruction::MoveImmediate64 {
            destination: Register::Rcx,
            value: b'0' as u64,
        },
        Instruction::Add {
            destination: Register::Rdx,
            source: Register::Rcx,
        },
        Instruction::IndexedStore8 {
            base: Register::Rbp,
            index: Register::Rbx,
            displacement: 0,
            source: Register::Rdx,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rcx,
            value: 1,
        },
        Instruction::Subtract {
            destination: Register::Rbx,
            source: Register::Rcx,
        },
        Instruction::Add {
            destination: Register::R13,
            source: Register::Rcx,
        },
        Instruction::Test(Register::Rax),
        Instruction::JumpIf {
            condition: Condition::NotEqual,
            target: BlockId(22),
        },
        Instruction::Test(Register::R8),
        Instruction::JumpIfZero(BlockId(24)),
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: b'-' as u64,
        },
        Instruction::IndexedStore8 {
            base: Register::Rbp,
            index: Register::Rbx,
            displacement: 0,
            source: Register::Rdx,
        },
        Instruction::Subtract {
            destination: Register::Rbx,
            source: Register::Rcx,
        },
        Instruction::Add {
            destination: Register::R13,
            source: Register::Rcx,
        },
        Instruction::Label(BlockId(24)),
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 1,
        },
        Instruction::Add {
            destination: Register::Rbx,
            source: Register::Rax,
        },
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::Rbp,
        },
        Instruction::Add {
            destination: Register::Rdi,
            source: Register::Rbx,
        },
        Instruction::MoveRegister {
            destination: Register::Rsi,
            source: Register::R13,
        },
        Instruction::MoveRegister {
            destination: Register::Rdx,
            source: Register::R14,
        },
        Instruction::Call(WRITE_ALL),
        Instruction::Jump(BlockId(50)),
        // bool
        Instruction::Label(BlockId(30)),
        Instruction::Test(Register::R12),
        Instruction::JumpIfZero(BlockId(31)),
    ];
    append_stack_literal(&mut instructions, b"true", 0);
    instructions.push(Instruction::MoveImmediate64 {
        destination: Register::Rsi,
        value: 4,
    });
    instructions.push(Instruction::Jump(BlockId(32)));
    instructions.push(Instruction::Label(BlockId(31)));
    append_stack_literal(&mut instructions, b"false", 0);
    instructions.extend([
        Instruction::MoveImmediate64 {
            destination: Register::Rsi,
            value: 5,
        },
        Instruction::Label(BlockId(32)),
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::Rbp,
        },
        Instruction::MoveRegister {
            destination: Register::Rdx,
            source: Register::R14,
        },
        Instruction::Call(WRITE_ALL),
        Instruction::Jump(BlockId(50)),
        // char UTF-8 encoding
        Instruction::Label(BlockId(40)),
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0x7f,
        },
        Instruction::Compare {
            left: Register::R12,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::LessEqual,
            target: BlockId(41),
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0x7ff,
        },
        Instruction::Compare {
            left: Register::R12,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::LessEqual,
            target: BlockId(42),
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0xffff,
        },
        Instruction::Compare {
            left: Register::R12,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::LessEqual,
            target: BlockId(43),
        },
        Instruction::Jump(BlockId(44)),
        Instruction::Label(BlockId(41)),
        Instruction::Store8 {
            base: Register::Rbp,
            displacement: 0,
            source: Register::R12,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rsi,
            value: 1,
        },
        Instruction::Jump(BlockId(45)),
        Instruction::Label(BlockId(42)),
    ]);
    append_utf8_division(&mut instructions, 2, 0xc0);
    instructions.push(Instruction::Jump(BlockId(45)));
    instructions.push(Instruction::Label(BlockId(43)));
    append_utf8_division(&mut instructions, 3, 0xe0);
    instructions.push(Instruction::Jump(BlockId(45)));
    instructions.push(Instruction::Label(BlockId(44)));
    append_utf8_division(&mut instructions, 4, 0xf0);
    instructions.extend([
        Instruction::Label(BlockId(45)),
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::Rbp,
        },
        Instruction::MoveRegister {
            destination: Register::Rdx,
            source: Register::R14,
        },
        Instruction::Call(WRITE_ALL),
        Instruction::Label(BlockId(50)),
        Instruction::Test(Register::R15),
        Instruction::JumpIfZero(BlockId(51)),
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: b'\n' as u64,
        },
        Instruction::Store8 {
            base: Register::Rbp,
            displacement: 0,
            source: Register::Rax,
        },
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::Rbp,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rsi,
            value: 1,
        },
        Instruction::MoveRegister {
            destination: Register::Rdx,
            source: Register::R14,
        },
        Instruction::Call(WRITE_ALL),
        Instruction::Label(BlockId(51)),
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0,
        },
        Instruction::StackDeallocate(40),
        Instruction::Pop(Register::Rbp),
        Instruction::Pop(Register::R15),
        Instruction::Pop(Register::R14),
        Instruction::Pop(Register::R13),
        Instruction::Pop(Register::R12),
        Instruction::Pop(Register::Rbx),
        Instruction::Return,
    ]);
    MachineFunction {
        symbol: PRINT,
        name: "__aerofyl_print".into(),
        instructions,
    }
}

fn append_utf8_division(instructions: &mut Vec<Instruction>, length: u64, lead: u64) {
    // Encode from the final continuation byte backwards using quotient/remainder
    // division by 64. The final quotient becomes the leading-byte payload.
    instructions.extend([
        Instruction::MoveRegister {
            destination: Register::Rax,
            source: Register::R12,
        },
        Instruction::MoveImmediate64 {
            destination: Register::R9,
            value: 64,
        },
    ]);
    for index in (1..length).rev() {
        instructions.extend([
            Instruction::SignExtendRaxIntoRdx,
            Instruction::DivideSigned(Register::R9),
            Instruction::MoveImmediate64 {
                destination: Register::Rcx,
                value: 0x80,
            },
            Instruction::Add {
                destination: Register::Rdx,
                source: Register::Rcx,
            },
            Instruction::Store8 {
                base: Register::Rbp,
                displacement: index as i32,
                source: Register::Rdx,
            },
        ]);
    }
    instructions.extend([
        Instruction::MoveImmediate64 {
            destination: Register::Rcx,
            value: lead,
        },
        Instruction::Add {
            destination: Register::Rax,
            source: Register::Rcx,
        },
        Instruction::Store8 {
            base: Register::Rbp,
            displacement: 0,
            source: Register::Rax,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rsi,
            value: length,
        },
    ]);
}

fn append_stack_literal(instructions: &mut Vec<Instruction>, bytes: &[u8], offset: i32) {
    for (index, byte) in bytes.iter().copied().enumerate() {
        instructions.extend([
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: byte as u64,
            },
            Instruction::Store8 {
                base: Register::Rbp,
                displacement: offset + index as i32,
                source: Register::Rax,
            },
        ]);
    }
}

fn input_line() -> MachineFunction {
    let mut instructions = vec![
        Instruction::Push(Register::Rbx),
        Instruction::Push(Register::R12),
        Instruction::Push(Register::R13),
        Instruction::Push(Register::R14),
        Instruction::Push(Register::R15),
        Instruction::Push(Register::Rbp),
        Instruction::MoveImmediate64 {
            destination: Register::R14,
            value: 64,
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
            destination: Register::R12,
            source: Register::Rax,
        },
        Instruction::MoveImmediate64 {
            destination: Register::R13,
            value: 0,
        },
        Instruction::Label(BlockId(0)),
        Instruction::Compare {
            left: Register::R13,
            right: Register::R14,
        },
        Instruction::JumpIf {
            condition: Condition::Less,
            target: BlockId(3),
        },
        // Grow by doubling. Allocations are monotonic in the syscall runtime.
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 2,
        },
        Instruction::MultiplySigned {
            destination: Register::R14,
            source: Register::Rax,
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
            destination: Register::R15,
            source: Register::Rax,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rbx,
            value: 0,
        },
        Instruction::Label(BlockId(1)),
        Instruction::Compare {
            left: Register::Rbx,
            right: Register::R13,
        },
        Instruction::JumpIf {
            condition: Condition::GreaterEqual,
            target: BlockId(2),
        },
        Instruction::IndexedLoad8 {
            destination: Register::Rax,
            base: Register::R12,
            index: Register::Rbx,
            displacement: 8,
        },
        Instruction::IndexedStore8 {
            base: Register::R15,
            index: Register::Rbx,
            displacement: 8,
            source: Register::Rax,
        },
    ];
    instructions.extend(increment(Register::Rbx));
    instructions.extend([
        Instruction::Jump(BlockId(1)),
        Instruction::Label(BlockId(2)),
        Instruction::MoveRegister {
            destination: Register::R12,
            source: Register::R15,
        },
        Instruction::Label(BlockId(3)),
        // read(0, bytes + length, 1)
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdi,
            value: 0,
        },
        Instruction::MoveRegister {
            destination: Register::Rsi,
            source: Register::R12,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: 8,
        },
        Instruction::Add {
            destination: Register::Rsi,
            source: Register::Rdx,
        },
        Instruction::Add {
            destination: Register::Rsi,
            source: Register::R13,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: 1,
        },
        Instruction::Syscall,
        Instruction::Test(Register::Rax),
        Instruction::JumpIf {
            condition: Condition::Less,
            target: BlockId(9),
        },
        Instruction::JumpIfZero(BlockId(7)),
        Instruction::IndexedLoad8 {
            destination: Register::Rax,
            base: Register::R12,
            index: Register::R13,
            displacement: 8,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: b'\n' as u64,
        },
        Instruction::Compare {
            left: Register::Rax,
            right: Register::Rdx,
        },
        Instruction::JumpIf {
            condition: Condition::Equal,
            target: BlockId(5),
        },
    ]);
    instructions.extend(increment(Register::R13));
    instructions.extend([
        Instruction::Jump(BlockId(0)),
        Instruction::Label(BlockId(5)),
        // A carriage return is removed only when it is the first half of CRLF.
        Instruction::Test(Register::R13),
        Instruction::JumpIfZero(BlockId(7)),
        Instruction::MoveRegister {
            destination: Register::Rbx,
            source: Register::R13,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 1,
        },
        Instruction::Subtract {
            destination: Register::Rbx,
            source: Register::Rax,
        },
        Instruction::IndexedLoad8 {
            destination: Register::Rax,
            base: Register::R12,
            index: Register::Rbx,
            displacement: 8,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: b'\r' as u64,
        },
        Instruction::Compare {
            left: Register::Rax,
            right: Register::Rdx,
        },
        Instruction::JumpIf {
            condition: Condition::NotEqual,
            target: BlockId(7),
        },
        Instruction::MoveRegister {
            destination: Register::R13,
            source: Register::Rbx,
        },
        Instruction::Label(BlockId(7)),
        Instruction::Store64 {
            base: Register::R12,
            displacement: 0,
            source: Register::R13,
        },
        Instruction::MoveRegister {
            destination: Register::Rax,
            source: Register::R12,
        },
        Instruction::Pop(Register::Rbp),
        Instruction::Pop(Register::R15),
        Instruction::Pop(Register::R14),
        Instruction::Pop(Register::R13),
        Instruction::Pop(Register::R12),
        Instruction::Pop(Register::Rbx),
        Instruction::Return,
        Instruction::Label(BlockId(9)),
        Instruction::ExitFailure,
    ]);
    MachineFunction {
        symbol: INPUT_LINE,
        name: "__aerofyl_input_line".into(),
        instructions,
    }
}

fn input_int() -> MachineFunction {
    let mut instructions = vec![
        Instruction::Push(Register::Rbx),
        Instruction::Push(Register::R12),
        Instruction::Push(Register::R13),
        Instruction::Push(Register::R14),
        Instruction::Push(Register::R15),
        Instruction::Call(INPUT_LINE),
        Instruction::MoveRegister {
            destination: Register::R12,
            source: Register::Rax,
        },
        Instruction::Load64 {
            destination: Register::R13,
            base: Register::R12,
            displacement: 0,
        },
        Instruction::Test(Register::R13),
        Instruction::JumpIfZero(BlockId(9)),
        Instruction::MoveImmediate64 {
            destination: Register::Rbx,
            value: 0,
        },
        Instruction::MoveImmediate64 {
            destination: Register::R15,
            value: 0,
        },
        Instruction::Load8 {
            destination: Register::Rax,
            base: Register::R12,
            displacement: 8,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: b'-' as u64,
        },
        Instruction::Compare {
            left: Register::Rax,
            right: Register::Rdx,
        },
        Instruction::JumpIf {
            condition: Condition::Equal,
            target: BlockId(1),
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: b'+' as u64,
        },
        Instruction::Compare {
            left: Register::Rax,
            right: Register::Rdx,
        },
        Instruction::JumpIf {
            condition: Condition::NotEqual,
            target: BlockId(2),
        },
    ];
    instructions.extend(increment(Register::Rbx));
    instructions.push(Instruction::Jump(BlockId(3)));
    instructions.extend([
        Instruction::Label(BlockId(1)),
        Instruction::MoveImmediate64 {
            destination: Register::R15,
            value: 1,
        },
    ]);
    instructions.extend(increment(Register::Rbx));
    instructions.extend([
        Instruction::Label(BlockId(3)),
        Instruction::Compare {
            left: Register::Rbx,
            right: Register::R13,
        },
        Instruction::JumpIf {
            condition: Condition::GreaterEqual,
            target: BlockId(9),
        },
        Instruction::Label(BlockId(2)),
        Instruction::MoveImmediate64 {
            destination: Register::R14,
            value: 0,
        },
        Instruction::Label(BlockId(4)),
        Instruction::Compare {
            left: Register::Rbx,
            right: Register::R13,
        },
        Instruction::JumpIf {
            condition: Condition::GreaterEqual,
            target: BlockId(7),
        },
        Instruction::IndexedLoad8 {
            destination: Register::Rdx,
            base: Register::R12,
            index: Register::Rbx,
            displacement: 8,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: b'0' as u64,
        },
        Instruction::Compare {
            left: Register::Rdx,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Less,
            target: BlockId(9),
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: b'9' as u64,
        },
        Instruction::Compare {
            left: Register::Rdx,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Greater,
            target: BlockId(9),
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: b'0' as u64,
        },
        Instruction::Subtract {
            destination: Register::Rdx,
            source: Register::Rax,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: (-922337203685477580_i64) as u64,
        },
        Instruction::Compare {
            left: Register::R14,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Less,
            target: BlockId(9),
        },
        Instruction::JumpIf {
            condition: Condition::NotEqual,
            target: BlockId(6),
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 7,
        },
        Instruction::Test(Register::R15),
        Instruction::JumpIfZero(BlockId(5)),
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 8,
        },
        Instruction::Label(BlockId(5)),
        Instruction::Compare {
            left: Register::Rdx,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Greater,
            target: BlockId(9),
        },
        Instruction::Label(BlockId(6)),
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 10,
        },
        Instruction::MultiplySigned {
            destination: Register::R14,
            source: Register::Rax,
        },
        Instruction::Subtract {
            destination: Register::R14,
            source: Register::Rdx,
        },
    ]);
    instructions.extend(increment(Register::Rbx));
    instructions.extend([
        Instruction::Jump(BlockId(4)),
        Instruction::Label(BlockId(7)),
        Instruction::Test(Register::R15),
        Instruction::JumpIf {
            condition: Condition::NotEqual,
            target: BlockId(8),
        },
        Instruction::Negate(Register::R14),
        Instruction::Label(BlockId(8)),
        Instruction::MoveRegister {
            destination: Register::Rax,
            source: Register::R14,
        },
        Instruction::Pop(Register::R15),
        Instruction::Pop(Register::R14),
        Instruction::Pop(Register::R13),
        Instruction::Pop(Register::R12),
        Instruction::Pop(Register::Rbx),
        Instruction::Return,
        Instruction::Label(BlockId(9)),
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::R12,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rsi,
            value: 0,
        },
        Instruction::Call(INPUT_ERROR),
        Instruction::ExitFailure,
    ]);
    MachineFunction {
        symbol: INPUT_INT,
        name: "__aerofyl_input_int".into(),
        instructions,
    }
}

fn input_bool() -> MachineFunction {
    let mut instructions = vec![
        Instruction::Push(Register::R12),
        Instruction::Call(INPUT_LINE),
        Instruction::MoveRegister {
            destination: Register::R12,
            source: Register::Rax,
        },
        Instruction::Load64 {
            destination: Register::Rax,
            base: Register::R12,
            displacement: 0,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: 4,
        },
        Instruction::Compare {
            left: Register::Rax,
            right: Register::Rdx,
        },
        Instruction::JumpIf {
            condition: Condition::Equal,
            target: BlockId(1),
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: 5,
        },
        Instruction::Compare {
            left: Register::Rax,
            right: Register::Rdx,
        },
        Instruction::JumpIf {
            condition: Condition::Equal,
            target: BlockId(2),
        },
        Instruction::Jump(BlockId(9)),
        Instruction::Label(BlockId(1)),
    ];
    append_compare_string(&mut instructions, b"true", BlockId(9));
    instructions.extend([
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 1,
        },
        Instruction::Jump(BlockId(8)),
        Instruction::Label(BlockId(2)),
    ]);
    append_compare_string(&mut instructions, b"false", BlockId(9));
    instructions.extend([
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0,
        },
        Instruction::Label(BlockId(8)),
        Instruction::Pop(Register::R12),
        Instruction::Return,
        Instruction::Label(BlockId(9)),
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::R12,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rsi,
            value: 1,
        },
        Instruction::Call(INPUT_ERROR),
        Instruction::ExitFailure,
    ]);
    MachineFunction {
        symbol: INPUT_BOOL,
        name: "__aerofyl_input_bool".into(),
        instructions,
    }
}

fn append_compare_string(instructions: &mut Vec<Instruction>, expected: &[u8], failure: BlockId) {
    for (index, byte) in expected.iter().copied().enumerate() {
        instructions.extend([
            Instruction::Load8 {
                destination: Register::Rax,
                base: Register::R12,
                displacement: 8 + index as i32,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rdx,
                value: byte as u64,
            },
            Instruction::Compare {
                left: Register::Rax,
                right: Register::Rdx,
            },
            Instruction::JumpIf {
                condition: Condition::NotEqual,
                target: failure,
            },
        ]);
    }
}

fn input_char() -> MachineFunction {
    let mut instructions = vec![
        Instruction::Push(Register::Rbx),
        Instruction::Push(Register::R12),
        Instruction::Push(Register::R13),
        Instruction::Push(Register::R14),
        Instruction::Push(Register::R15),
        Instruction::Call(INPUT_LINE),
        Instruction::MoveRegister {
            destination: Register::R12,
            source: Register::Rax,
        },
        Instruction::Load64 {
            destination: Register::R13,
            base: Register::R12,
            displacement: 0,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 1,
        },
        Instruction::Compare {
            left: Register::R13,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Equal,
            target: BlockId(1),
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 2,
        },
        Instruction::Compare {
            left: Register::R13,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Equal,
            target: BlockId(2),
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 3,
        },
        Instruction::Compare {
            left: Register::R13,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Equal,
            target: BlockId(3),
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 4,
        },
        Instruction::Compare {
            left: Register::R13,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Equal,
            target: BlockId(4),
        },
        Instruction::Jump(BlockId(99)),
        Instruction::Label(BlockId(1)),
        Instruction::Load8 {
            destination: Register::R14,
            base: Register::R12,
            displacement: 8,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0x7f,
        },
        Instruction::Compare {
            left: Register::R14,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Greater,
            target: BlockId(99),
        },
        Instruction::Jump(BlockId(90)),
        Instruction::Label(BlockId(2)),
    ];
    append_utf8_decode(&mut instructions, 2, 0xc2, 0xdf, 0xc0, BlockId(99));
    instructions.push(Instruction::Jump(BlockId(90)));
    instructions.push(Instruction::Label(BlockId(3)));
    append_utf8_decode(&mut instructions, 3, 0xe0, 0xef, 0xe0, BlockId(99));
    instructions.extend([
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0x800,
        },
        Instruction::Compare {
            left: Register::R14,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Less,
            target: BlockId(99),
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0xd800,
        },
        Instruction::Compare {
            left: Register::R14,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Less,
            target: BlockId(90),
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0xdfff,
        },
        Instruction::Compare {
            left: Register::R14,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::LessEqual,
            target: BlockId(99),
        },
        Instruction::Jump(BlockId(90)),
        Instruction::Label(BlockId(4)),
    ]);
    append_utf8_decode(&mut instructions, 4, 0xf0, 0xf4, 0xf0, BlockId(99));
    instructions.extend([
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0x10000,
        },
        Instruction::Compare {
            left: Register::R14,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Less,
            target: BlockId(99),
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0x10ffff,
        },
        Instruction::Compare {
            left: Register::R14,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Greater,
            target: BlockId(99),
        },
        Instruction::Label(BlockId(90)),
        Instruction::MoveRegister {
            destination: Register::Rax,
            source: Register::R14,
        },
        Instruction::Pop(Register::R15),
        Instruction::Pop(Register::R14),
        Instruction::Pop(Register::R13),
        Instruction::Pop(Register::R12),
        Instruction::Pop(Register::Rbx),
        Instruction::Return,
        Instruction::Label(BlockId(99)),
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::R12,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rsi,
            value: 2,
        },
        Instruction::Call(INPUT_ERROR),
        Instruction::ExitFailure,
    ]);
    MachineFunction {
        symbol: INPUT_CHAR,
        name: "__aerofyl_input_char".into(),
        instructions,
    }
}

fn append_utf8_decode(
    instructions: &mut Vec<Instruction>,
    length: usize,
    minimum_lead: u64,
    maximum_lead: u64,
    payload_base: u64,
    failure: BlockId,
) {
    instructions.extend([
        Instruction::Load8 {
            destination: Register::R14,
            base: Register::R12,
            displacement: 8,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: minimum_lead,
        },
        Instruction::Compare {
            left: Register::R14,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Less,
            target: failure,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: maximum_lead,
        },
        Instruction::Compare {
            left: Register::R14,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Greater,
            target: failure,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: payload_base,
        },
        Instruction::Subtract {
            destination: Register::R14,
            source: Register::Rax,
        },
    ]);
    for index in 1..length {
        instructions.extend([
            Instruction::Load8 {
                destination: Register::R15,
                base: Register::R12,
                displacement: 8 + index as i32,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 0x80,
            },
            Instruction::Compare {
                left: Register::R15,
                right: Register::Rax,
            },
            Instruction::JumpIf {
                condition: Condition::Less,
                target: failure,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 0xbf,
            },
            Instruction::Compare {
                left: Register::R15,
                right: Register::Rax,
            },
            Instruction::JumpIf {
                condition: Condition::Greater,
                target: failure,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 64,
            },
            Instruction::MultiplySigned {
                destination: Register::R14,
                source: Register::Rax,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 0x80,
            },
            Instruction::Subtract {
                destination: Register::R15,
                source: Register::Rax,
            },
            Instruction::Add {
                destination: Register::R14,
                source: Register::R15,
            },
        ]);
    }
}

fn input_error() -> MachineFunction {
    let mut instructions = vec![
        Instruction::Push(Register::R12),
        Instruction::Push(Register::R13),
        Instruction::Push(Register::Rbp),
        Instruction::StackAllocate(40),
        Instruction::MoveRegister {
            destination: Register::Rbp,
            source: Register::Rsp,
        },
        Instruction::MoveRegister {
            destination: Register::R12,
            source: Register::Rdi,
        },
        Instruction::MoveRegister {
            destination: Register::R13,
            source: Register::Rsi,
        },
    ];
    append_write_literal(&mut instructions, b"InputTypeError: expected ", 0);
    instructions.extend([
        Instruction::Test(Register::R13),
        Instruction::JumpIfZero(BlockId(1)),
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 1,
        },
        Instruction::Compare {
            left: Register::R13,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Equal,
            target: BlockId(2),
        },
    ]);
    append_write_literal(&mut instructions, b"char", 0);
    instructions.push(Instruction::Jump(BlockId(3)));
    instructions.push(Instruction::Label(BlockId(1)));
    append_write_literal(&mut instructions, b"int", 0);
    instructions.push(Instruction::Jump(BlockId(3)));
    instructions.push(Instruction::Label(BlockId(2)));
    append_write_literal(&mut instructions, b"bool", 0);
    instructions.push(Instruction::Label(BlockId(3)));
    append_write_literal(&mut instructions, b", got \"", 0);
    instructions.extend([
        Instruction::Load64 {
            destination: Register::Rsi,
            base: Register::R12,
            displacement: 0,
        },
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::R12,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 8,
        },
        Instruction::Add {
            destination: Register::Rdi,
            source: Register::Rax,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: 2,
        },
        Instruction::Call(WRITE_ALL),
    ]);
    append_write_literal(&mut instructions, b"\"\n", 0);
    instructions.push(Instruction::ExitFailure);
    MachineFunction {
        symbol: INPUT_ERROR,
        name: "__aerofyl_input_error".into(),
        instructions,
    }
}

fn int_to_byte() -> MachineFunction {
    MachineFunction {
        symbol: INT_TO_BYTE,
        name: "__aerofyl_int_to_byte".into(),
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
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 255,
            },
            Instruction::Compare {
                left: Register::Rdi,
                right: Register::Rax,
            },
            Instruction::JumpIf {
                condition: Condition::Greater,
                target: BlockId(1),
            },
            Instruction::MoveRegister {
                destination: Register::Rax,
                source: Register::Rdi,
            },
            Instruction::Return,
            Instruction::Label(BlockId(1)),
            Instruction::ExitFailure,
        ],
    }
}

fn read_bytes() -> MachineFunction {
    let mut instructions = vec![
        Instruction::Push(Register::Rbx),
        Instruction::Push(Register::R12),
        Instruction::Push(Register::R13),
        Instruction::Push(Register::R14),
        Instruction::Push(Register::R15),
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::Rdi,
        },
        Instruction::Call(READ_FILE),
        Instruction::MoveRegister {
            destination: Register::R12,
            source: Register::Rax,
        },
        Instruction::Load64 {
            destination: Register::R13,
            base: Register::R12,
            displacement: 0,
        },
        Instruction::MoveRegister {
            destination: Register::R14,
            source: Register::R13,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 4,
        },
        Instruction::Compare {
            left: Register::R14,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::GreaterEqual,
            target: BlockId(0),
        },
        Instruction::MoveRegister {
            destination: Register::R14,
            source: Register::Rax,
        },
        Instruction::Label(BlockId(0)),
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::R14,
        },
        Instruction::Call(ALLOC),
        Instruction::MoveRegister {
            destination: Register::Rbx,
            source: Register::Rax,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdi,
            value: 32,
        },
        Instruction::Call(ALLOC),
        Instruction::MoveRegister {
            destination: Register::R15,
            source: Register::Rax,
        },
        Instruction::Store64 {
            base: Register::R15,
            displacement: 0,
            source: Register::R13,
        },
        Instruction::Store64 {
            base: Register::R15,
            displacement: 8,
            source: Register::R14,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 1,
        },
        Instruction::Store64 {
            base: Register::R15,
            displacement: 16,
            source: Register::Rax,
        },
        Instruction::Store64 {
            base: Register::R15,
            displacement: 24,
            source: Register::Rbx,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rbx,
            value: 0,
        },
        Instruction::Label(BlockId(1)),
        Instruction::Compare {
            left: Register::Rbx,
            right: Register::R13,
        },
        Instruction::JumpIf {
            condition: Condition::GreaterEqual,
            target: BlockId(2),
        },
        Instruction::IndexedLoad8 {
            destination: Register::Rdx,
            base: Register::R12,
            index: Register::Rbx,
            displacement: 8,
        },
        Instruction::Load64 {
            destination: Register::Rax,
            base: Register::R15,
            displacement: 24,
        },
        Instruction::IndexedStore8 {
            base: Register::Rax,
            index: Register::Rbx,
            displacement: 0,
            source: Register::Rdx,
        },
    ];
    instructions.extend(increment(Register::Rbx));
    instructions.extend([
        Instruction::Jump(BlockId(1)),
        Instruction::Label(BlockId(2)),
        Instruction::MoveRegister {
            destination: Register::Rax,
            source: Register::R15,
        },
        Instruction::Pop(Register::R15),
        Instruction::Pop(Register::R14),
        Instruction::Pop(Register::R13),
        Instruction::Pop(Register::R12),
        Instruction::Pop(Register::Rbx),
        Instruction::Return,
    ]);
    MachineFunction {
        symbol: READ_BYTES,
        name: "__aerofyl_read_bytes".into(),
        instructions,
    }
}

fn write_bytes() -> MachineFunction {
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
        Instruction::Load64 {
            destination: Register::R14,
            base: Register::R13,
            displacement: 0,
        },
        Instruction::Load64 {
            destination: Register::R13,
            base: Register::R13,
            displacement: 24,
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
            destination: Register::R15,
            source: Register::Rax,
        },
        Instruction::Store64 {
            base: Register::R15,
            displacement: 0,
            source: Register::R14,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rbx,
            value: 0,
        },
        Instruction::Label(BlockId(0)),
        Instruction::Compare {
            left: Register::Rbx,
            right: Register::R14,
        },
        Instruction::JumpIf {
            condition: Condition::GreaterEqual,
            target: BlockId(1),
        },
        Instruction::IndexedLoad8 {
            destination: Register::Rax,
            base: Register::R13,
            index: Register::Rbx,
            displacement: 0,
        },
        Instruction::IndexedStore8 {
            base: Register::R15,
            index: Register::Rbx,
            displacement: 8,
            source: Register::Rax,
        },
    ];
    instructions.extend(increment(Register::Rbx));
    instructions.extend([
        Instruction::Jump(BlockId(0)),
        Instruction::Label(BlockId(1)),
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::R12,
        },
        Instruction::MoveRegister {
            destination: Register::Rsi,
            source: Register::R15,
        },
        Instruction::Call(WRITE_FILE),
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0,
        },
        Instruction::Pop(Register::R15),
        Instruction::Pop(Register::R14),
        Instruction::Pop(Register::R13),
        Instruction::Pop(Register::R12),
        Instruction::Pop(Register::Rbx),
        Instruction::Return,
    ]);
    MachineFunction {
        symbol: WRITE_BYTES,
        name: "__aerofyl_write_bytes".into(),
        instructions,
    }
}

fn exists() -> MachineFunction {
    let mut instructions = vec![
        Instruction::Push(Register::Rbx),
        Instruction::Push(Register::R12),
        Instruction::Push(Register::R13),
        Instruction::Push(Register::R14),
        Instruction::Push(Register::R15),
        Instruction::Push(Register::Rbp),
        Instruction::StackAllocate(160),
        Instruction::MoveRegister {
            destination: Register::Rbp,
            source: Register::Rsp,
        },
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
            destination: Register::Rax,
            base: Register::R12,
            index: Register::Rbx,
            displacement: 8,
        },
        Instruction::IndexedStore8 {
            base: Register::R14,
            index: Register::Rbx,
            displacement: 0,
            source: Register::Rax,
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
        // newfstatat(AT_FDCWD, path, statbuf, 0)
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 262,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdi,
            value: (-100_i64) as u64,
        },
        Instruction::MoveRegister {
            destination: Register::Rsi,
            source: Register::R14,
        },
        Instruction::MoveRegister {
            destination: Register::Rdx,
            source: Register::Rbp,
        },
        Instruction::MoveImmediate64 {
            destination: Register::R10,
            value: 0,
        },
        Instruction::Syscall,
        Instruction::Test(Register::Rax),
        Instruction::JumpIf {
            condition: Condition::GreaterEqual,
            target: BlockId(2),
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: (-2_i64) as u64,
        },
        Instruction::Compare {
            left: Register::Rax,
            right: Register::Rdx,
        },
        Instruction::JumpIf {
            condition: Condition::Equal,
            target: BlockId(3),
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdi,
            value: 2,
        },
        Instruction::Call(FS_ERROR),
        Instruction::Label(BlockId(2)),
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 1,
        },
        Instruction::Jump(BlockId(4)),
        Instruction::Label(BlockId(3)),
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 0,
        },
        Instruction::Label(BlockId(4)),
        Instruction::StackDeallocate(160),
        Instruction::Pop(Register::Rbp),
        Instruction::Pop(Register::R15),
        Instruction::Pop(Register::R14),
        Instruction::Pop(Register::R13),
        Instruction::Pop(Register::R12),
        Instruction::Pop(Register::Rbx),
        Instruction::Return,
    ]);
    MachineFunction {
        symbol: EXISTS,
        name: "__aerofyl_exists".into(),
        instructions,
    }
}

fn fs_error() -> MachineFunction {
    let mut instructions = vec![
        Instruction::Push(Register::R12),
        Instruction::Push(Register::Rbp),
        Instruction::StackAllocate(24),
        Instruction::MoveRegister {
            destination: Register::Rbp,
            source: Register::Rsp,
        },
        Instruction::MoveRegister {
            destination: Register::R12,
            source: Register::Rdi,
        },
        Instruction::Test(Register::R12),
        Instruction::JumpIfZero(BlockId(0)),
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 1,
        },
        Instruction::Compare {
            left: Register::R12,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Equal,
            target: BlockId(1),
        },
    ];
    append_write_literal(&mut instructions, b"FileSystemError\n", 0);
    instructions.push(Instruction::ExitFailure);
    instructions.push(Instruction::Label(BlockId(0)));
    append_write_literal(&mut instructions, b"FileReadError\n", 0);
    instructions.push(Instruction::ExitFailure);
    instructions.push(Instruction::Label(BlockId(1)));
    append_write_literal(&mut instructions, b"FileWriteError\n", 0);
    instructions.push(Instruction::ExitFailure);
    MachineFunction {
        symbol: FS_ERROR,
        name: "__aerofyl_fs_error".into(),
        instructions,
    }
}

fn append_write_literal(instructions: &mut Vec<Instruction>, bytes: &[u8], offset: i32) {
    append_stack_literal(instructions, bytes, offset);
    instructions.extend([
        Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::Rbp,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: offset as u64,
        },
        Instruction::Add {
            destination: Register::Rdi,
            source: Register::Rax,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rsi,
            value: bytes.len() as u64,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdx,
            value: 2,
        },
        Instruction::Call(WRITE_ALL),
    ]);
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
            Instruction::Load64 {
                destination: Register::Rax,
                base: Register::Rdi,
                displacement: 24,
            },
            Instruction::Add {
                destination: Register::Rax,
                source: Register::Rsi,
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
            Instruction::Load64 {
                destination: Register::Rax,
                base: Register::Rdi,
                displacement: 24,
            },
            Instruction::Add {
                destination: Register::Rax,
                source: Register::Rcx,
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

// ---------------------------------------------------------------------------
// std.os process execution
//
// `execute` runs a command without a shell: the command string is parsed into
// an argv array (whitespace-separated tokens, where both quote styles group a
// run of characters so arguments and executable paths may contain spaces), and
// the child image is replaced with `execve`, so no shell interprets the string.
// `stdout` and `stderr` are captured through separate non-blocking pipes that
// are drained in turns, which avoids deadlocks for large or interleaved output.
// ---------------------------------------------------------------------------

fn pe_mov(dst: Register, value: u64) -> Instruction {
    Instruction::MoveImmediate64 {
        destination: dst,
        value,
    }
}

fn pe_reg(dst: Register, src: Register) -> Instruction {
    Instruction::MoveRegister {
        destination: dst,
        source: src,
    }
}

fn pe_load(dst: Register, base: Register, displacement: i32) -> Instruction {
    Instruction::Load64 {
        destination: dst,
        base,
        displacement,
    }
}

fn pe_store(base: Register, displacement: i32, src: Register) -> Instruction {
    Instruction::Store64 {
        base,
        displacement,
        source: src,
    }
}

fn pe_byte(base: Register, displacement: i32, src: Register) -> Instruction {
    Instruction::Store8 {
        base,
        displacement,
        source: src,
    }
}

fn pe_load8(dst: Register, base: Register, index: Register, displacement: i32) -> Instruction {
    Instruction::IndexedLoad8 {
        destination: dst,
        base,
        index,
        displacement,
    }
}

fn pe_store8(base: Register, index: Register, displacement: i32, src: Register) -> Instruction {
    Instruction::IndexedStore8 {
        base,
        index,
        displacement,
        source: src,
    }
}

fn pe_slot(dst: Register, base: Register, index: Register) -> Instruction {
    Instruction::IndexedLoad64 {
        destination: dst,
        base,
        index,
        displacement: 0,
    }
}

fn pe_slot_store(base: Register, index: Register, src: Register) -> Instruction {
    Instruction::IndexedStore64 {
        base,
        index,
        displacement: 0,
        source: src,
    }
}

fn pe_add(dst: Register, src: Register) -> Instruction {
    Instruction::Add {
        destination: dst,
        source: src,
    }
}

fn pe_sub(dst: Register, src: Register) -> Instruction {
    Instruction::Subtract {
        destination: dst,
        source: src,
    }
}

fn pe_cmp(left: Register, right: Register) -> Instruction {
    Instruction::Compare { left, right }
}

fn pe_jcc(condition: Condition, target: BlockId) -> Instruction {
    Instruction::JumpIf { condition, target }
}

fn pe_jump(target: BlockId) -> Instruction {
    Instruction::Jump(target)
}

fn pe_label(target: BlockId) -> Instruction {
    Instruction::Label(target)
}

const PE_HEAD: BlockId = BlockId(0);
const PE_REG: BlockId = BlockId(1);
const PE_REGSTART: BlockId = BlockId(2);
const PE_REGCOPY: BlockId = BlockId(3);
const PE_SEP: BlockId = BlockId(4);
const PE_QUOTESTART: BlockId = BlockId(5);
const PE_QUOTESET: BlockId = BlockId(6);
const PE_INQUOTE: BlockId = BlockId(7);
const PE_ADV: BlockId = BlockId(8);
const PE_TOKEND: BlockId = BlockId(9);
const PE_SETNB: BlockId = BlockId(10);
const PE_MAINLOOP: BlockId = BlockId(11);
const PE_READOUT: BlockId = BlockId(12);
const PE_OUTEOF: BlockId = BlockId(13);
const PE_OUTGROWCHK: BlockId = BlockId(14);
const PE_OUTALLOC: BlockId = BlockId(15);
const PE_OUTCOPY: BlockId = BlockId(16);
const PE_OUTDONE: BlockId = BlockId(17);
const PE_PREERR: BlockId = BlockId(18);
const PE_READERR: BlockId = BlockId(19);
const PE_ERREOF: BlockId = BlockId(20);
const PE_ERRGROWCHK: BlockId = BlockId(21);
const PE_ERRALLOC: BlockId = BlockId(22);
const PE_ERRCOPY: BlockId = BlockId(23);
const PE_ERRDONE: BlockId = BlockId(24);
const PE_ENDCHECK: BlockId = BlockId(25);
const PE_OUTSTR: BlockId = BlockId(26);
const PE_ERRSTR: BlockId = BlockId(27);
const PE_WAIT: BlockId = BlockId(28);
const PE_CODEZERO: BlockId = BlockId(29);
const PE_BUILD: BlockId = BlockId(30);
const PE_EPILOGUE: BlockId = BlockId(31);
const PE_QCLOSE: BlockId = BlockId(32);
const PE_OUTAGAIN: BlockId = BlockId(33);
const PE_ERRAGAIN: BlockId = BlockId(34);
const PE_CHILD: BlockId = BlockId(35);
const PE_ERR_EMPTY: BlockId = BlockId(36);
const PE_ERR_QUOTE: BlockId = BlockId(37);
const PE_ERR_SIG: BlockId = BlockId(38);
const PE_SRCCOPY: BlockId = BlockId(39);
const PE_SRCDONE: BlockId = BlockId(40);

/// Frame offsets for the process-execution helper (relative to `Rbp`):
///   +0 child pid, +8 wait status, +16/+24 stdout read/write fds,
///   +32/+40 stderr read/write fds, +48 token count, +56 stdout string,
///   +64 stderr string, +72/+88 EOF flags, +96 stderr buffer.
const PE_FRAME: u32 = 112;

fn process_execute() -> MachineFunction {
    let mut c: Vec<Instruction> = Vec::new();
    build_process_execute(&mut c);
    MachineFunction {
        symbol: PROCESS_EXECUTE,
        name: "__aerofyl_process_exec".into(),
        instructions: c,
    }
}

fn build_process_execute(c: &mut Vec<Instruction>) {
    // ---- prologue ----------------------------------------------------------
    c.extend([
        Instruction::Push(Register::Rbx),
        Instruction::Push(Register::R12),
        Instruction::Push(Register::R13),
        Instruction::Push(Register::R14),
        Instruction::Push(Register::R15),
        Instruction::Push(Register::Rbp),
        Instruction::StackAllocate(PE_FRAME),
        pe_reg(Register::Rbp, Register::Rsp),
        pe_reg(Register::R12, Register::Rdi),
        pe_load(Register::Rbx, Register::R12, 0),
        pe_mov(Register::Rdi, 0),
        pe_cmp(Register::Rbx, Register::Rdi),
        pe_jcc(Condition::Equal, PE_ERR_EMPTY),
    ]);

    // ---- working copy of the command string + argv array --------------------
    c.extend([
        pe_reg(Register::Rdi, Register::Rbx),
        pe_mov(Register::Rax, 1),
        pe_add(Register::Rdi, Register::Rax),
        Instruction::Call(ALLOC),
        pe_reg(Register::R13, Register::Rax),
        pe_mov(Register::R14, 0),
        pe_label(PE_SRCCOPY),
        pe_cmp(Register::R14, Register::Rbx),
        pe_jcc(Condition::GreaterEqual, PE_SRCDONE),
        pe_load8(Register::Rax, Register::R12, Register::R14, 8),
        pe_store8(Register::R13, Register::R14, 0, Register::Rax),
        pe_mov(Register::Rax, 1),
        pe_add(Register::R14, Register::Rax),
        pe_jump(PE_SRCCOPY),
        pe_label(PE_SRCDONE),
        pe_mov(Register::Rcx, 0),
        pe_store8(Register::R13, Register::R14, 0, Register::Rcx),
        pe_reg(Register::R12, Register::R13),
        // argv array: (len + 2) pointer slots; the terminator is stored later.
        pe_reg(Register::Rdi, Register::Rbx),
        pe_mov(Register::Rax, 2),
        pe_add(Register::Rdi, Register::Rax),
        pe_mov(Register::Rdx, 8),
        Instruction::MultiplySigned {
            destination: Register::Rdi,
            source: Register::Rdx,
        },
        Instruction::Call(ALLOC),
        pe_reg(Register::R13, Register::Rax),
        // tokenizer state: i = R14, j = R15, open quote = R9, in-token = Rcx,
        // token count at [rbp + 48]; Rdi stays zero inside the tokenizer.
        pe_mov(Register::Rdx, 0),
        pe_store(Register::Rbp, 48, Register::Rdx),
        pe_mov(Register::R14, 0),
        pe_mov(Register::R15, 0),
        pe_mov(Register::R9, 0),
        pe_mov(Register::Rcx, 0),
    ]);

    // ---- tokenizer ----------------------------------------------------------
    // The separator/quote constants are reloaded on every pass because several
    // are on scratch registers reused inside the loop.
    c.extend([
        pe_label(PE_HEAD),
        pe_mov(Register::R8, 32),
        pe_mov(Register::R10, 9),
        pe_mov(Register::R11, 10),
        pe_mov(Register::Rsi, 39),
        pe_mov(Register::Rdx, 34),
        pe_cmp(Register::R14, Register::Rbx),
        pe_jcc(Condition::GreaterEqual, PE_TOKEND),
        pe_load8(Register::Rax, Register::R12, Register::R14, 0),
        pe_cmp(Register::R9, Register::Rdi),
        pe_jcc(Condition::NotEqual, PE_INQUOTE),
        pe_cmp(Register::Rax, Register::R8),
        pe_jcc(Condition::Equal, PE_SEP),
        pe_cmp(Register::Rax, Register::R10),
        pe_jcc(Condition::Equal, PE_SEP),
        pe_cmp(Register::Rax, Register::R11),
        pe_jcc(Condition::Equal, PE_SEP),
        pe_cmp(Register::Rax, Register::Rsi),
        pe_jcc(Condition::Equal, PE_QUOTESTART),
        pe_cmp(Register::Rax, Register::Rdx),
        pe_jcc(Condition::Equal, PE_QUOTESTART),
        // plain character: open a token if needed, then append the byte
        pe_label(PE_REG),
        pe_cmp(Register::Rcx, Register::Rdi),
        pe_jcc(Condition::NotEqual, PE_REGSTART),
        pe_load(Register::R8, Register::Rbp, 48),
        pe_reg(Register::R11, Register::R12),
        pe_add(Register::R11, Register::R15),
        pe_slot_store(Register::R13, Register::R8, Register::R11),
        pe_mov(Register::Rdx, 1),
        pe_add(Register::R8, Register::Rdx),
        pe_store(Register::Rbp, 48, Register::R8),
        pe_mov(Register::Rcx, 1),
        pe_label(PE_REGSTART),
        pe_store8(Register::R12, Register::R15, 0, Register::Rax),
        pe_mov(Register::Rdx, 1),
        pe_add(Register::R15, Register::Rdx),
        pe_jump(PE_ADV),
        pe_label(PE_SEP),
        pe_cmp(Register::Rcx, Register::Rdi),
        pe_jcc(Condition::Equal, PE_ADV),
        pe_store8(Register::R12, Register::R15, 0, Register::Rdi),
        pe_mov(Register::Rdx, 1),
        pe_add(Register::R15, Register::Rdx),
        pe_mov(Register::Rcx, 0),
        pe_jump(PE_ADV),
        pe_label(PE_QUOTESTART),
        pe_cmp(Register::Rcx, Register::Rdi),
        pe_jcc(Condition::NotEqual, PE_QUOTESET),
        pe_load(Register::R8, Register::Rbp, 48),
        pe_reg(Register::R11, Register::R12),
        pe_add(Register::R11, Register::R15),
        pe_slot_store(Register::R13, Register::R8, Register::R11),
        pe_mov(Register::Rdx, 1),
        pe_add(Register::R8, Register::Rdx),
        pe_store(Register::Rbp, 48, Register::R8),
        pe_mov(Register::Rcx, 1),
        pe_label(PE_QUOTESET),
        pe_reg(Register::R9, Register::Rax),
        pe_jump(PE_ADV),
        pe_label(PE_INQUOTE),
        pe_cmp(Register::Rax, Register::R9),
        pe_jcc(Condition::Equal, PE_QCLOSE),
        pe_store8(Register::R12, Register::R15, 0, Register::Rax),
        pe_mov(Register::Rdx, 1),
        pe_add(Register::R15, Register::Rdx),
        pe_jump(PE_ADV),
        pe_label(PE_QCLOSE),
        pe_reg(Register::R9, Register::Rdi),
        pe_label(PE_ADV),
        pe_mov(Register::Rdx, 1),
        pe_add(Register::R14, Register::Rdx),
        pe_jump(PE_HEAD),
    ]);

    // ---- terminate the token stream ------------------------------------------
    // A while an unterminated quote is a parse failure; then NUL-terminate the
    // last token and NULL-terminate the argv array.
    c.extend([
        pe_label(PE_TOKEND),
        pe_cmp(Register::R9, Register::Rdi),
        pe_jcc(Condition::NotEqual, PE_ERR_QUOTE),
        pe_store8(Register::R12, Register::R15, 0, Register::Rdi),
        pe_mov(Register::Rdx, 1),
        pe_add(Register::R15, Register::Rdx),
        pe_load(Register::R8, Register::Rbp, 48),
        pe_slot_store(Register::R13, Register::R8, Register::Rdi),
    ]);

    // ---- pipes ----------------------------------------------------------------
    c.extend([
        pe_mov(Register::Rax, 22),
        pe_reg(Register::Rdi, Register::Rbp),
        pe_mov(Register::Rcx, 16),
        pe_add(Register::Rdi, Register::Rcx),
        Instruction::Syscall,
        Instruction::Test(Register::Rax),
        pe_jcc(Condition::Less, PE_ERR_EMPTY),
        // pipe writes two 32-bit fds at +16/+20; split them into the 8-byte
        // slots at +16 (read) and +24 (write) with a signed division.
        pe_load(Register::Rax, Register::Rbp, 16),
        Instruction::SignExtendRaxIntoRdx,
        pe_mov(Register::Rcx, 0x1_0000_0000),
        Instruction::DivideSigned(Register::Rcx),
        pe_store(Register::Rbp, 24, Register::Rax),
        pe_store(Register::Rbp, 16, Register::Rdx),
        pe_mov(Register::Rax, 22),
        pe_reg(Register::Rdi, Register::Rbp),
        pe_mov(Register::Rcx, 32),
        pe_add(Register::Rdi, Register::Rcx),
        Instruction::Syscall,
        Instruction::Test(Register::Rax),
        pe_jcc(Condition::Less, PE_ERR_EMPTY),
        pe_load(Register::Rax, Register::Rbp, 32),
        Instruction::SignExtendRaxIntoRdx,
        pe_mov(Register::Rcx, 0x1_0000_0000),
        Instruction::DivideSigned(Register::Rcx),
        pe_store(Register::Rbp, 40, Register::Rax),
        pe_store(Register::Rbp, 32, Register::Rdx),
    ]);

    // ---- fork; the child takes the dup2/execve path ---------------------------
    c.extend([
        pe_mov(Register::Rax, 57),
        Instruction::Syscall,
        pe_store(Register::Rbp, 0, Register::Rax),
        pe_mov(Register::Rsi, 0),
        pe_cmp(Register::Rax, Register::Rsi),
        pe_jcc(Condition::Equal, PE_CHILD),
        Instruction::Test(Register::Rax),
        pe_jcc(Condition::Less, PE_ERR_EMPTY),
    ]);

    // ---- parent: set read ends non-blocking, close both write ends ------------
    c.extend([
        pe_label(PE_SETNB),
        pe_mov(Register::Rax, 72),
        pe_load(Register::Rdi, Register::Rbp, 16),
        pe_mov(Register::Rsi, 4),
        pe_mov(Register::Rdx, 0x800),
        Instruction::Syscall,
        pe_mov(Register::Rax, 72),
        pe_load(Register::Rdi, Register::Rbp, 32),
        pe_mov(Register::Rsi, 4),
        pe_mov(Register::Rdx, 0x800),
        Instruction::Syscall,
        pe_mov(Register::Rax, 3),
        pe_load(Register::Rdi, Register::Rbp, 24),
        Instruction::Syscall,
        pe_mov(Register::Rax, 3),
        pe_load(Register::Rdi, Register::Rbp, 40),
        Instruction::Syscall,
        // capture state: out: cap = Rbx, buffer = R12, len = R13;
        // err: cap = R14, len = R15, buffer at [rbp + 96]
        pe_mov(Register::Rdi, 72),
        Instruction::Call(ALLOC),
        pe_reg(Register::R12, Register::Rax),
        pe_mov(Register::Rbx, 72),
        pe_mov(Register::R13, 0),
        pe_mov(Register::Rdi, 72),
        Instruction::Call(ALLOC),
        pe_store(Register::Rbp, 96, Register::Rax),
        pe_mov(Register::R14, 72),
        pe_mov(Register::R15, 0),
        pe_mov(Register::Rdi, 0),
        pe_store(Register::Rbp, 72, Register::Rdi),
        pe_store(Register::Rbp, 88, Register::Rdi),
    ]);

    // ---- alternating non-blocking drain ---------------------------------------
    c.extend([
        pe_label(PE_MAINLOOP),
        pe_mov(Register::Rdi, 0),
        pe_load(Register::Rax, Register::Rbp, 72),
        pe_cmp(Register::Rax, Register::Rdi),
        pe_jcc(Condition::NotEqual, PE_PREERR),
        // stdout drain attempt
        pe_label(PE_READOUT),
        pe_mov(Register::Rax, 0),
        pe_load(Register::Rdi, Register::Rbp, 16),
        pe_reg(Register::Rsi, Register::R12),
        pe_mov(Register::Rcx, 8),
        pe_add(Register::Rsi, Register::Rcx),
        pe_add(Register::Rsi, Register::R13),
        pe_reg(Register::Rdx, Register::Rbx),
        pe_mov(Register::Rcx, 8),
        pe_sub(Register::Rdx, Register::Rcx),
        pe_sub(Register::Rdx, Register::R13),
        Instruction::Syscall,
        pe_mov(Register::Rcx, (-4_i64) as u64),
        pe_cmp(Register::Rax, Register::Rcx),
        pe_jcc(Condition::Equal, PE_READOUT),
        pe_mov(Register::Rcx, (-11_i64) as u64),
        pe_cmp(Register::Rax, Register::Rcx),
        pe_jcc(Condition::Equal, PE_OUTAGAIN),
        Instruction::Test(Register::Rax),
        pe_jcc(Condition::Less, PE_ERR_EMPTY),
        Instruction::JumpIfZero(PE_OUTEOF),
        pe_add(Register::R13, Register::Rax),
        pe_label(PE_OUTGROWCHK),
        pe_mov(Register::Rcx, 8),
        pe_reg(Register::R10, Register::Rbx),
        pe_sub(Register::R10, Register::Rcx),
        pe_cmp(Register::R13, Register::R10),
        pe_jcc(Condition::Equal, PE_OUTALLOC),
        pe_jump(PE_PREERR),
        pe_label(PE_OUTEOF),
        pe_mov(Register::Rsi, 1),
        pe_store(Register::Rbp, 72, Register::Rsi),
        pe_jump(PE_PREERR),
        pe_label(PE_OUTAGAIN),
        pe_mov(Register::Rax, 24),
        Instruction::Syscall,
        pe_jump(PE_PREERR),
        // stdout buffer growth: double the capacity and copy
        pe_label(PE_OUTALLOC),
        pe_reg(Register::Rdx, Register::Rbx),
        pe_add(Register::Rdx, Register::Rbx),
        pe_reg(Register::Rdi, Register::Rdx),
        Instruction::Call(ALLOC),
        pe_reg(Register::Rsi, Register::Rax),
        pe_mov(Register::R10, 0),
        pe_label(PE_OUTCOPY),
        pe_cmp(Register::R10, Register::R13),
        pe_jcc(Condition::GreaterEqual, PE_OUTDONE),
        pe_load8(Register::Rax, Register::R12, Register::R10, 8),
        pe_store8(Register::Rsi, Register::R10, 8, Register::Rax),
        pe_mov(Register::Rcx, 1),
        pe_add(Register::R10, Register::Rcx),
        pe_jump(PE_OUTCOPY),
        pe_label(PE_OUTDONE),
        pe_reg(Register::R12, Register::Rsi),
        pe_reg(Register::Rdx, Register::Rbx),
        pe_add(Register::Rdx, Register::Rbx),
        pe_reg(Register::Rbx, Register::Rdx),
        pe_jump(PE_READOUT),
        // stderr sequence mirrors stdout
        pe_label(PE_PREERR),
        pe_mov(Register::Rdi, 0),
        pe_load(Register::Rax, Register::Rbp, 88),
        pe_cmp(Register::Rax, Register::Rdi),
        pe_jcc(Condition::NotEqual, PE_ENDCHECK),
        pe_label(PE_READERR),
        pe_mov(Register::Rax, 0),
        pe_load(Register::Rdi, Register::Rbp, 32),
        pe_load(Register::Rsi, Register::Rbp, 96),
        pe_mov(Register::Rcx, 8),
        pe_add(Register::Rsi, Register::Rcx),
        pe_add(Register::Rsi, Register::R15),
        pe_reg(Register::Rdx, Register::R14),
        pe_mov(Register::Rcx, 8),
        pe_sub(Register::Rdx, Register::Rcx),
        pe_sub(Register::Rdx, Register::R15),
        Instruction::Syscall,
        pe_mov(Register::Rcx, (-4_i64) as u64),
        pe_cmp(Register::Rax, Register::Rcx),
        pe_jcc(Condition::Equal, PE_READERR),
        pe_mov(Register::Rcx, (-11_i64) as u64),
        pe_cmp(Register::Rax, Register::Rcx),
        pe_jcc(Condition::Equal, PE_ERRAGAIN),
        Instruction::Test(Register::Rax),
        pe_jcc(Condition::Less, PE_ERR_EMPTY),
        Instruction::JumpIfZero(PE_ERREOF),
        pe_add(Register::R15, Register::Rax),
        pe_label(PE_ERRGROWCHK),
        pe_mov(Register::Rcx, 8),
        pe_reg(Register::R10, Register::R14),
        pe_sub(Register::R10, Register::Rcx),
        pe_cmp(Register::R15, Register::R10),
        pe_jcc(Condition::Equal, PE_ERRALLOC),
        pe_jump(PE_ENDCHECK),
        pe_label(PE_ERREOF),
        pe_mov(Register::Rsi, 1),
        pe_store(Register::Rbp, 88, Register::Rsi),
        pe_jump(PE_ENDCHECK),
        pe_label(PE_ERRAGAIN),
        pe_mov(Register::Rax, 24),
        Instruction::Syscall,
        pe_jump(PE_ENDCHECK),
        // stderr buffer growth: double the capacity and copy
        pe_label(PE_ERRALLOC),
        pe_reg(Register::Rdx, Register::R14),
        pe_add(Register::Rdx, Register::R14),
        pe_reg(Register::Rdi, Register::Rdx),
        Instruction::Call(ALLOC),
        pe_reg(Register::Rcx, Register::Rax),
        pe_load(Register::Rsi, Register::Rbp, 96),
        pe_mov(Register::R10, 0),
        pe_label(PE_ERRCOPY),
        pe_cmp(Register::R10, Register::R15),
        pe_jcc(Condition::GreaterEqual, PE_ERRDONE),
        pe_load8(Register::Rax, Register::Rsi, Register::R10, 8),
        pe_store8(Register::Rcx, Register::R10, 8, Register::Rax),
        pe_mov(Register::R11, 1),
        pe_add(Register::R10, Register::R11),
        pe_jump(PE_ERRCOPY),
        pe_label(PE_ERRDONE),
        pe_store(Register::Rbp, 96, Register::Rcx),
        pe_reg(Register::Rdx, Register::R14),
        pe_add(Register::Rdx, Register::R14),
        pe_reg(Register::R14, Register::Rdx),
        pe_jump(PE_READERR),
        pe_label(PE_ENDCHECK),
        pe_mov(Register::Rdi, 0),
        pe_load(Register::Rax, Register::Rbp, 72),
        pe_cmp(Register::Rax, Register::Rdi),
        pe_jcc(Condition::Equal, PE_MAINLOOP),
        pe_load(Register::Rax, Register::Rbp, 88),
        pe_cmp(Register::Rax, Register::Rdi),
        pe_jcc(Condition::Equal, PE_MAINLOOP),
    ]);

    // ---- finalize the strings, then wait for the child ------------------------
    c.extend([
        pe_label(PE_OUTSTR),
        pe_store(Register::R12, 0, Register::R13),
        pe_reg(Register::Rdi, Register::R12),
        pe_mov(Register::Rcx, 8),
        pe_add(Register::Rdi, Register::Rcx),
        pe_add(Register::Rdi, Register::R13),
        pe_mov(Register::Rcx, 0),
        pe_byte(Register::Rdi, 0, Register::Rcx),
        pe_store(Register::Rbp, 56, Register::R12),
        pe_label(PE_ERRSTR),
        pe_load(Register::Rbx, Register::Rbp, 96),
        pe_store(Register::Rbx, 0, Register::R15),
        pe_reg(Register::Rdi, Register::Rbx),
        pe_mov(Register::Rcx, 8),
        pe_add(Register::Rdi, Register::Rcx),
        pe_add(Register::Rdi, Register::R15),
        pe_mov(Register::Rcx, 0),
        pe_byte(Register::Rdi, 0, Register::Rcx),
        pe_store(Register::Rbp, 64, Register::Rbx),
        pe_label(PE_WAIT),
        pe_mov(Register::Rax, 61),
        pe_load(Register::Rdi, Register::Rbp, 0),
        pe_reg(Register::Rsi, Register::Rbp),
        pe_mov(Register::Rcx, 8),
        pe_add(Register::Rsi, Register::Rcx),
        pe_mov(Register::Rdx, 0),
        pe_mov(Register::R10, 0),
        Instruction::Syscall,
        Instruction::Test(Register::Rax),
        pe_jcc(Condition::Less, PE_ERR_EMPTY),
        pe_load(Register::Rax, Register::Rbp, 8),
        Instruction::Test(Register::Rax),
        Instruction::JumpIfZero(PE_CODEZERO),
        Instruction::SignExtendRaxIntoRdx,
        pe_mov(Register::Rcx, 256),
        Instruction::DivideSigned(Register::Rcx),
        pe_mov(Register::Rdi, 0),
        pe_cmp(Register::Rax, Register::Rdi),
        pe_jcc(Condition::Equal, PE_ERR_SIG),
        // exit code in Rax; skip the "status was zero" shortcut
        pe_jump(PE_BUILD),
    ]);

    // ---- assemble the Process struct ------------------------------------------
    c.extend([
        pe_label(PE_CODEZERO),
        pe_mov(Register::Rax, 0),
        pe_label(PE_BUILD),
        pe_reg(Register::R12, Register::Rax),
        pe_mov(Register::Rdi, 24),
        Instruction::Call(ALLOC),
        pe_reg(Register::R10, Register::Rax),
        pe_store(Register::R10, 0, Register::R12),
        pe_load(Register::Rax, Register::Rbp, 56),
        pe_store(Register::R10, 8, Register::Rax),
        pe_load(Register::Rax, Register::Rbp, 64),
        pe_store(Register::R10, 16, Register::Rax),
        pe_reg(Register::Rax, Register::R10),
        pe_label(PE_EPILOGUE),
        Instruction::StackDeallocate(PE_FRAME),
        Instruction::Pop(Register::Rbp),
        Instruction::Pop(Register::R15),
        Instruction::Pop(Register::R14),
        Instruction::Pop(Register::R13),
        Instruction::Pop(Register::R12),
        Instruction::Pop(Register::Rbx),
        Instruction::Return,
    ]);

    // ---- child path -------------------------------------------------------------
    c.extend([
        pe_label(PE_CHILD),
        pe_mov(Register::Rsi, 1),
        pe_mov(Register::Rax, 33),
        pe_load(Register::Rdi, Register::Rbp, 24),
        Instruction::Syscall,
        pe_mov(Register::Rsi, 2),
        pe_mov(Register::Rax, 33),
        pe_load(Register::Rdi, Register::Rbp, 40),
        Instruction::Syscall,
        pe_mov(Register::Rax, 3),
        pe_load(Register::Rdi, Register::Rbp, 16),
        Instruction::Syscall,
        pe_mov(Register::Rax, 3),
        pe_load(Register::Rdi, Register::Rbp, 32),
        Instruction::Syscall,
        pe_mov(Register::Rcx, 0),
        pe_slot(Register::Rdi, Register::R13, Register::Rcx),
        pe_reg(Register::Rsi, Register::R13),
        pe_mov(Register::Rdx, 0),
        pe_mov(Register::Rax, 59),
        Instruction::Syscall,
        // execve failed: report it as exit code 127
        pe_mov(Register::Rdi, 127),
        pe_mov(Register::Rax, 60),
        Instruction::Syscall,
        // execve never returns; keep a hard stop here just in case
        Instruction::ExitFailure,
    ]);

    // ---- shared failure exits -----------------------------------------------------
    c.extend([
        pe_label(PE_ERR_EMPTY),
        pe_mov(Register::Rdi, 0),
        Instruction::Call(OS_ERROR),
        Instruction::ExitFailure,
        pe_label(PE_ERR_QUOTE),
        pe_mov(Register::Rdi, 0),
        Instruction::Call(OS_ERROR),
        Instruction::ExitFailure,
        pe_label(PE_ERR_SIG),
        pe_mov(Register::Rdi, 1),
        Instruction::Call(OS_ERROR),
        Instruction::ExitFailure,
    ]);
}

/// Reports `std.os` runtime failures the same way `FS_ERROR` reports file
/// failures: write a literal line, then exit with the runtime failure status.
/// `Rdi`: 0 = generic process failure, 1 = child killed by a signal.
fn os_error() -> MachineFunction {
    let mut instructions = vec![
        Instruction::Push(Register::R12),
        Instruction::Push(Register::Rbp),
        Instruction::StackAllocate(24),
        Instruction::MoveRegister {
            destination: Register::Rbp,
            source: Register::Rsp,
        },
        Instruction::MoveRegister {
            destination: Register::R12,
            source: Register::Rdi,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 1,
        },
        Instruction::Compare {
            left: Register::R12,
            right: Register::Rax,
        },
        Instruction::JumpIf {
            condition: Condition::Equal,
            target: BlockId(0),
        },
    ];
    append_write_literal(&mut instructions, b"ProcessError\n", 0);
    instructions.push(Instruction::ExitFailure);
    instructions.push(Instruction::Label(BlockId(0)));
    append_write_literal(&mut instructions, b"ProcessSignalError\n", 0);
    instructions.push(Instruction::ExitFailure);
    MachineFunction {
        symbol: OS_ERROR,
        name: "__aerofyl_os_error".into(),
        instructions,
    }
}
