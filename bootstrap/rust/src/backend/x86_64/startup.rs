use crate::frontend::resolution::SymbolId;

use super::instruction::Instruction;
use super::register::Register;

/// Linux process-entry glue. The kernel supplies a 16-byte-aligned stack; the
/// direct call therefore gives Aerofyl `main` the SysV function-entry alignment.
pub fn generate(entry_function: SymbolId) -> Vec<Instruction> {
    vec![
        Instruction::Call(entry_function),
        Instruction::MoveImmediate64 {
            destination: Register::Rax,
            value: 60,
        },
        Instruction::MoveImmediate64 {
            destination: Register::Rdi,
            value: 0,
        },
        Instruction::Syscall,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_calls_main_and_exits_zero() {
        let main = SymbolId(7);
        assert_eq!(
            generate(main),
            vec![
                Instruction::Call(main),
                Instruction::MoveImmediate64 {
                    destination: Register::Rax,
                    value: 60,
                },
                Instruction::MoveImmediate64 {
                    destination: Register::Rdi,
                    value: 0,
                },
                Instruction::Syscall,
            ]
        );
    }
}
