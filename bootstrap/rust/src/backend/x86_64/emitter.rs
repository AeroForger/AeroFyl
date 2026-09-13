use std::collections::HashMap;
use std::fmt;

use crate::frontend::resolution::SymbolId;
use crate::middle::ir::BlockId;

use super::instruction::{Condition, Instruction, MachineModule};
use super::register::Register;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmitError {
    UnknownCallTarget(SymbolId),
    BranchOutsideFunction,
    UnknownBlockLabel { function: SymbolId, block: BlockId },
    DuplicateBlockLabel { function: SymbolId, block: BlockId },
    RelativeTargetOutOfRange,
}

impl fmt::Display for EmitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCallTarget(symbol) => write!(formatter, "unknown call target {symbol:?}"),
            Self::BranchOutsideFunction => formatter.write_str("branch emitted outside a function"),
            Self::UnknownBlockLabel { function, block } => {
                write!(
                    formatter,
                    "unknown block {block:?} in function {function:?}"
                )
            }
            Self::DuplicateBlockLabel { function, block } => {
                write!(
                    formatter,
                    "duplicate block {block:?} in function {function:?}"
                )
            }
            Self::RelativeTargetOutOfRange => {
                formatter.write_str("relative target is outside rel32 range")
            }
        }
    }
}

impl std::error::Error for EmitError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmittedCode {
    pub bytes: Vec<u8>,
    pub entry_offset: usize,
    pub read_only_data: Vec<u8>,
    pub data_offset: usize,
}

enum FixupTarget {
    Function(SymbolId),
    Block { function: SymbolId, block: BlockId },
    Data(usize),
}

struct RelativeFixup {
    displacement_offset: usize,
    target: FixupTarget,
}

/// Encodes startup and functions, then resolves rel32 call and branch fixups.
pub fn emit_module(module: &MachineModule) -> Result<EmittedCode, EmitError> {
    let mut bytes = Vec::new();
    let mut fixups = Vec::new();
    let mut function_offsets = HashMap::new();
    let mut block_offsets = HashMap::new();
    let entry_offset = 0;

    encode_instructions(
        &module.startup,
        None,
        &mut bytes,
        &mut fixups,
        &mut block_offsets,
    )?;
    for function in &module.functions {
        function_offsets.insert(function.symbol, bytes.len());
        encode_instructions(
            &function.instructions,
            Some(function.symbol),
            &mut bytes,
            &mut fixups,
            &mut block_offsets,
        )?;
    }

    let data_offset = align_up(bytes.len(), 0x1000);
    for fixup in fixups {
        let target = match fixup.target {
            FixupTarget::Function(symbol) => function_offsets
                .get(&symbol)
                .copied()
                .ok_or(EmitError::UnknownCallTarget(symbol))?,
            FixupTarget::Block { function, block } => block_offsets
                .get(&(function, block))
                .copied()
                .ok_or(EmitError::UnknownBlockLabel { function, block })?,
            FixupTarget::Data(offset) => data_offset
                .checked_add(offset)
                .ok_or(EmitError::RelativeTargetOutOfRange)?,
        };
        let end = fixup.displacement_offset + 4;
        let target = i64::try_from(target).map_err(|_| EmitError::RelativeTargetOutOfRange)?;
        let end_i64 = i64::try_from(end).map_err(|_| EmitError::RelativeTargetOutOfRange)?;
        let displacement =
            i32::try_from(target - end_i64).map_err(|_| EmitError::RelativeTargetOutOfRange)?;
        bytes[fixup.displacement_offset..end].copy_from_slice(&displacement.to_le_bytes());
    }
    Ok(EmittedCode {
        bytes,
        entry_offset,
        read_only_data: module.read_only_data.clone(),
        data_offset,
    })
}

fn encode_instructions(
    instructions: &[Instruction],
    function: Option<SymbolId>,
    bytes: &mut Vec<u8>,
    fixups: &mut Vec<RelativeFixup>,
    block_offsets: &mut HashMap<(SymbolId, BlockId), usize>,
) -> Result<(), EmitError> {
    for instruction in instructions {
        match *instruction {
            Instruction::Nop => bytes.push(0x90),
            Instruction::Label(block) => {
                let function = function.ok_or(EmitError::BranchOutsideFunction)?;
                if block_offsets
                    .insert((function, block), bytes.len())
                    .is_some()
                {
                    return Err(EmitError::DuplicateBlockLabel { function, block });
                }
            }
            Instruction::Push(register) => encode_push_pop(bytes, register, 0x50),
            Instruction::Pop(register) => encode_push_pop(bytes, register, 0x58),
            Instruction::MoveRegister {
                destination,
                source,
            } => encode_register_binary(bytes, 0x89, destination, source),
            Instruction::MoveImmediate64 { destination, value } => {
                let register = destination.encoding();
                bytes.push(rex(true, false, register >= 8));
                bytes.push(0xb8 + (register & 7));
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            Instruction::LoadDataAddress {
                destination,
                offset,
            } => {
                let register = destination.encoding();
                bytes.push(rex(true, register >= 8, false));
                bytes.extend_from_slice(&[0x8d, 0x05 | ((register & 7) << 3)]);
                record_fixup(bytes, fixups, FixupTarget::Data(offset));
            }
            Instruction::Load64 {
                destination,
                base,
                displacement,
            } => encode_memory(bytes, 0x8b, destination, base, displacement),
            Instruction::Load8 {
                destination,
                base,
                displacement,
            } => encode_load8(bytes, destination, base, displacement),
            Instruction::Store64 {
                base,
                displacement,
                source,
            } => encode_memory(bytes, 0x89, source, base, displacement),
            Instruction::Store8 {
                base,
                displacement,
                source,
            } => encode_store8(bytes, base, displacement, source),
            Instruction::IndexedLoad64 {
                destination,
                base,
                index,
                displacement,
            } => encode_indexed_memory(bytes, 0x8b, destination, base, index, displacement),
            Instruction::IndexedLoad8 {
                destination,
                base,
                index,
                displacement,
            } => encode_indexed_load8(bytes, destination, base, index, displacement),
            Instruction::IndexedStore64 {
                base,
                index,
                displacement,
                source,
            } => encode_indexed_memory(bytes, 0x89, source, base, index, displacement),
            Instruction::IndexedStore8 {
                base,
                index,
                displacement,
                source,
            } => encode_indexed_store8(bytes, base, index, displacement, source),
            Instruction::Add {
                destination,
                source,
            } => encode_register_binary(bytes, 0x01, destination, source),
            Instruction::Subtract {
                destination,
                source,
            } => encode_register_binary(bytes, 0x29, destination, source),
            Instruction::MultiplySigned {
                destination,
                source,
            } => {
                let destination = destination.encoding();
                let source = source.encoding();
                bytes.push(rex(true, destination >= 8, source >= 8));
                bytes.extend_from_slice(&[0x0f, 0xaf]);
                bytes.push(0xc0 | ((destination & 7) << 3) | (source & 7));
            }
            Instruction::Negate(register) => encode_group_register(bytes, register, 3),
            Instruction::SignExtendRaxIntoRdx => bytes.extend_from_slice(&[0x48, 0x99]),
            Instruction::DivideSigned(register) => encode_group_register(bytes, register, 7),
            Instruction::Compare { left, right } => {
                encode_register_binary(bytes, 0x39, left, right)
            }
            Instruction::Test(register) => encode_register_binary(bytes, 0x85, register, register),
            Instruction::MaterializeCondition(condition) => {
                bytes.extend_from_slice(&[0x0f, condition_opcode(condition), 0xc0]);
                bytes.extend_from_slice(&[0x48, 0x0f, 0xb6, 0xc0]);
            }
            Instruction::StackAllocate(amount) => encode_stack_adjust(bytes, 0xec, amount),
            Instruction::StackDeallocate(amount) => encode_stack_adjust(bytes, 0xc4, amount),
            Instruction::Call(target) => {
                bytes.push(0xe8);
                record_fixup(bytes, fixups, FixupTarget::Function(target));
            }
            Instruction::Jump(target) => {
                bytes.push(0xe9);
                record_block_fixup(function, target, bytes, fixups)?;
            }
            Instruction::JumpIfZero(target) => {
                bytes.extend_from_slice(&[0x0f, 0x84]);
                record_block_fixup(function, target, bytes, fixups)?;
            }
            Instruction::JumpIf { condition, target } => {
                bytes.extend_from_slice(&[0x0f, branch_condition_opcode(condition)]);
                record_block_fixup(function, target, bytes, fixups)?;
            }
            Instruction::Return => bytes.push(0xc3),
            Instruction::Syscall => bytes.extend_from_slice(&[0x0f, 0x05]),
            Instruction::ExitFailure => {
                bytes.extend_from_slice(&[0x48, 0xb8]);
                bytes.extend_from_slice(&60_u64.to_le_bytes());
                bytes.extend_from_slice(&[0x48, 0xbf]);
                bytes.extend_from_slice(&70_u64.to_le_bytes());
                bytes.extend_from_slice(&[0x0f, 0x05]);
            }
        }
    }
    Ok(())
}

fn align_up(value: usize, alignment: usize) -> usize {
    value.div_ceil(alignment) * alignment
}

fn record_block_fixup(
    function: Option<SymbolId>,
    block: BlockId,
    bytes: &mut Vec<u8>,
    fixups: &mut Vec<RelativeFixup>,
) -> Result<(), EmitError> {
    let function = function.ok_or(EmitError::BranchOutsideFunction)?;
    record_fixup(bytes, fixups, FixupTarget::Block { function, block });
    Ok(())
}

fn record_fixup(bytes: &mut Vec<u8>, fixups: &mut Vec<RelativeFixup>, target: FixupTarget) {
    let displacement_offset = bytes.len();
    bytes.extend_from_slice(&[0; 4]);
    fixups.push(RelativeFixup {
        displacement_offset,
        target,
    });
}

fn encode_push_pop(bytes: &mut Vec<u8>, register: Register, opcode: u8) {
    let code = register.encoding();
    if code >= 8 {
        bytes.push(0x41);
    }
    bytes.push(opcode + (code & 7));
}

fn encode_register_binary(
    bytes: &mut Vec<u8>,
    opcode: u8,
    destination: Register,
    source: Register,
) {
    let destination = destination.encoding();
    let source = source.encoding();
    bytes.push(rex(true, source >= 8, destination >= 8));
    bytes.push(opcode);
    bytes.push(0xc0 | ((source & 7) << 3) | (destination & 7));
}

fn encode_memory(
    bytes: &mut Vec<u8>,
    opcode: u8,
    register: Register,
    base: Register,
    displacement: i32,
) {
    let register = register.encoding();
    let base = base.encoding();
    bytes.push(rex(true, register >= 8, base >= 8));
    bytes.push(opcode);
    bytes.push(0x80 | ((register & 7) << 3) | (base & 7));
    if base & 7 == 4 {
        bytes.push(0x24);
    }
    bytes.extend_from_slice(&displacement.to_le_bytes());
}

fn encode_load8(bytes: &mut Vec<u8>, destination: Register, base: Register, displacement: i32) {
    let destination = destination.encoding();
    let base = base.encoding();
    bytes.push(rex(true, destination >= 8, base >= 8));
    bytes.extend_from_slice(&[0x0f, 0xb6]);
    bytes.push(0x80 | ((destination & 7) << 3) | (base & 7));
    if base & 7 == 4 {
        bytes.push(0x24);
    }
    bytes.extend_from_slice(&displacement.to_le_bytes());
}

fn encode_store8(bytes: &mut Vec<u8>, base: Register, displacement: i32, source: Register) {
    let source = source.encoding();
    let base = base.encoding();
    bytes.push(rex(false, source >= 8, base >= 8));
    bytes.push(0x88);
    bytes.push(0x80 | ((source & 7) << 3) | (base & 7));
    if base & 7 == 4 {
        bytes.push(0x24);
    }
    bytes.extend_from_slice(&displacement.to_le_bytes());
}

fn encode_indexed_memory(
    bytes: &mut Vec<u8>,
    opcode: u8,
    register: Register,
    base: Register,
    index: Register,
    displacement: i32,
) {
    let register = register.encoding();
    let base = base.encoding();
    let index = index.encoding();
    bytes.push(0x48 | ((register >= 8) as u8) << 2 | ((index >= 8) as u8) << 1 | (base >= 8) as u8);
    bytes.push(opcode);
    bytes.push(0x84 | ((register & 7) << 3));
    bytes.push(0xc0 | ((index & 7) << 3) | (base & 7));
    bytes.extend_from_slice(&displacement.to_le_bytes());
}

fn encode_indexed_load8(
    bytes: &mut Vec<u8>,
    destination: Register,
    base: Register,
    index: Register,
    displacement: i32,
) {
    let destination = destination.encoding();
    let base = base.encoding();
    let index = index.encoding();
    bytes.push(
        0x48 | ((destination >= 8) as u8) << 2 | ((index >= 8) as u8) << 1 | (base >= 8) as u8,
    );
    bytes.extend_from_slice(&[0x0f, 0xb6]);
    bytes.push(0x84 | ((destination & 7) << 3));
    bytes.push(((index & 7) << 3) | (base & 7));
    bytes.extend_from_slice(&displacement.to_le_bytes());
}

fn encode_indexed_store8(
    bytes: &mut Vec<u8>,
    base: Register,
    index: Register,
    displacement: i32,
    source: Register,
) {
    let source = source.encoding();
    let base = base.encoding();
    let index = index.encoding();
    bytes.push(0x40 | ((source >= 8) as u8) << 2 | ((index >= 8) as u8) << 1 | (base >= 8) as u8);
    bytes.push(0x88);
    bytes.push(0x84 | ((source & 7) << 3));
    bytes.push(((index & 7) << 3) | (base & 7));
    bytes.extend_from_slice(&displacement.to_le_bytes());
}

fn encode_group_register(bytes: &mut Vec<u8>, register: Register, group: u8) {
    let register = register.encoding();
    bytes.push(rex(true, false, register >= 8));
    bytes.push(0xf7);
    bytes.push(0xc0 | (group << 3) | (register & 7));
}

fn encode_stack_adjust(bytes: &mut Vec<u8>, operation: u8, amount: u32) {
    bytes.extend_from_slice(&[0x48, 0x81, operation]);
    bytes.extend_from_slice(&amount.to_le_bytes());
}

const fn rex(w: bool, r: bool, b: bool) -> u8 {
    0x40 | ((w as u8) << 3) | ((r as u8) << 2) | b as u8
}

const fn condition_opcode(condition: Condition) -> u8 {
    match condition {
        Condition::Equal => 0x94,
        Condition::NotEqual => 0x95,
        Condition::Less => 0x9c,
        Condition::LessEqual => 0x9e,
        Condition::Greater => 0x9f,
        Condition::GreaterEqual => 0x9d,
    }
}

const fn branch_condition_opcode(condition: Condition) -> u8 {
    condition_opcode(condition) - 0x10
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::x86_64::instruction::MachineFunction;

    fn module(instructions: Vec<Instruction>) -> MachineModule {
        let symbol = SymbolId(1);
        MachineModule {
            startup: vec![Instruction::Call(symbol)],
            entry_function: symbol,
            functions: vec![MachineFunction {
                symbol,
                name: "main".into(),
                instructions,
            }],
            read_only_data: Vec::new(),
        }
    }

    #[test]
    fn resolves_forward_and_backward_jump_fixups() {
        let emitted = emit_module(&module(vec![
            Instruction::Label(BlockId(0)),
            Instruction::Jump(BlockId(1)),
            Instruction::Label(BlockId(1)),
            Instruction::JumpIfZero(BlockId(0)),
            Instruction::Return,
        ]))
        .unwrap();
        assert!(emitted.bytes.contains(&0xe9));
        assert!(emitted.bytes.windows(2).any(|bytes| bytes == [0x0f, 0x84]));
        let conditional = emitted
            .bytes
            .windows(6)
            .find(|bytes| bytes[..2] == [0x0f, 0x84])
            .unwrap();
        assert!(i32::from_le_bytes(conditional[2..6].try_into().unwrap()) < 0);
    }

    #[test]
    fn emits_comparison_materialization() {
        let emitted = emit_module(&module(vec![
            Instruction::Compare {
                left: Register::Rax,
                right: Register::Rcx,
            },
            Instruction::MaterializeCondition(Condition::Less),
            Instruction::Return,
        ]))
        .unwrap();
        assert!(
            emitted
                .bytes
                .windows(3)
                .any(|bytes| bytes == [0x48, 0x39, 0xc8])
        );
        assert!(
            emitted
                .bytes
                .windows(3)
                .any(|bytes| bytes == [0x0f, 0x9c, 0xc0])
        );
    }

    #[test]
    fn resolves_startup_call_to_function() {
        let emitted = emit_module(&module(vec![Instruction::Return])).unwrap();
        assert_eq!(emitted.entry_offset, 0);
        assert_eq!(
            i32::from_le_bytes(emitted.bytes[1..5].try_into().unwrap()),
            0
        );
    }

    #[test]
    fn resolves_rip_relative_read_only_data_address() {
        let mut module = module(vec![
            Instruction::LoadDataAddress {
                destination: Register::Rax,
                offset: 0,
            },
            Instruction::Return,
        ]);
        module.read_only_data = b"literal".to_vec();
        let emitted = emit_module(&module).unwrap();
        assert_eq!(emitted.read_only_data, b"literal");
        assert_eq!(emitted.data_offset, 0x1000);
        assert!(
            emitted
                .bytes
                .windows(3)
                .any(|bytes| bytes == [0x48, 0x8d, 0x05])
        );
    }
}
