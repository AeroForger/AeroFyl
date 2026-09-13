use crate::frontend::resolution::SymbolId;

use super::instruction::Instruction;
use super::register::Register;

/// Linux process-entry glue. The kernel supplies a 16-byte-aligned stack; the
/// direct call therefore gives Aerofyl `main` the SysV function-entry alignment.
pub fn generate(entry_function: SymbolId, has_arguments: bool) -> Vec<Instruction> {
    let mut instructions = Vec::new();
    if has_arguments {
        instructions.push(Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::Rsp,
        });
        instructions.push(Instruction::Call(super::runtime::MAIN_ARGS));
        instructions.push(Instruction::MoveRegister {
            destination: Register::Rdi,
            source: Register::Rax,
        });
    }
    instructions.extend([
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
    ]);
    instructions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_calls_main_and_exits_zero() {
        let main = SymbolId(7);
        assert_eq!(
            generate(main, false),
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

    #[test]
    fn startup_materializes_user_arguments_for_argument_main() {
        let instructions = generate(SymbolId(7), true);
        assert!(matches!(
            instructions.as_slice(),
            [
                Instruction::MoveRegister {
                    destination: Register::Rdi,
                    source: Register::Rsp
                },
                Instruction::Call(symbol),
                Instruction::MoveRegister {
                    destination: Register::Rdi,
                    source: Register::Rax
                },
                ..
            ] if *symbol == crate::backend::x86_64::runtime::MAIN_ARGS
        ));
    }
}
