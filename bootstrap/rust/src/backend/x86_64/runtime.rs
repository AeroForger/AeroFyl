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
