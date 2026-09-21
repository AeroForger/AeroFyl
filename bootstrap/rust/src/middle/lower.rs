use crate::frontend::ast::{BinaryOperator, Literal};
use crate::frontend::source::Span;
use crate::frontend::types::Type;
use crate::middle::hir::*;
use crate::middle::ir::*;
use std::collections::HashMap;

/// Lowers checked HIR to a simple non-SSA control-flow graph.
pub fn lower(module: &HirModule) -> IrModule {
    IrModule {
        structs: module
            .structs
            .iter()
            .map(|item| IrStruct {
                id: item.id,
                name: item.name.clone(),
                fields: item
                    .fields
                    .iter()
                    .map(|field| IrField {
                        name: field.name.clone(),
                        ty: field.ty.clone(),
                        offset: field.offset,
                    })
                    .collect(),
            })
            .collect(),
        enums: module
            .enums
            .iter()
            .map(|item| IrEnum {
                id: item.id,
                name: item.name.clone(),
                variants: item.variants.clone(),
            })
            .collect(),
        functions: module
            .functions
            .iter()
            .map(FunctionLowerer::lower)
            .collect(),
    }
}

#[derive(Clone, Copy)]
struct LoopTargets {
    break_block: BlockId,
    continue_block: BlockId,
}

struct FunctionLowerer {
    blocks: Vec<IrBlock>,
    current: BlockId,
    next_value: u32,
    locals: Vec<IrLocal>,
    loops: Vec<LoopTargets>,
    local_types: HashMap<crate::frontend::resolution::SymbolId, Type>,
}

impl FunctionLowerer {
    fn lower(function: &HirFunction) -> IrFunction {
        let entry = BlockId(0);
        let mut lowerer = Self {
            blocks: vec![IrBlock {
                id: entry,
                instructions: Vec::new(),
                terminator: None,
            }],
            current: entry,
            next_value: 0,
            locals: Vec::new(),
            loops: Vec::new(),
            local_types: function
                .parameters
                .iter()
                .map(|parameter| (parameter.symbol, parameter.ty.clone()))
                .collect(),
        };
        lowerer.block(&function.body);
        if !lowerer.is_terminated() {
            lowerer.terminate(IrTerminator::Return(None));
        }
        IrFunction {
            symbol: function.symbol,
            name: function.name.clone(),
            visibility: function.visibility,
            parameters: function
                .parameters
                .iter()
                .map(|parameter| IrLocal {
                    symbol: parameter.symbol,
                    ty: parameter.ty.clone(),
                })
                .collect(),
            return_type: function.return_type.clone(),
            locals: lowerer.locals,
            entry,
            blocks: lowerer.blocks,
        }
    }

    fn block(&mut self, block: &HirBlock) {
        for statement in &block.statements {
            if self.is_terminated() {
                break;
            }
            self.statement(statement);
        }
    }

    fn statement(&mut self, statement: &HirStatement) {
        match statement {
            HirStatement::Variable {
                symbol,
                ty,
                initializer,
                span,
            } => {
                self.local_types.insert(*symbol, ty.clone());
                self.locals.push(IrLocal {
                    symbol: *symbol,
                    ty: ty.clone(),
                });
                if let HirExpressionKind::StructLiteral { struct_id, fields } = &initializer.kind {
                    let fields = fields
                        .iter()
                        .map(|(field, value)| (*field, self.expression(value)))
                        .collect();
                    self.emit(
                        None,
                        None,
                        IrInstructionKind::StructInit {
                            local: *symbol,
                            struct_id: *struct_id,
                            fields,
                        },
                        *span,
                    );
                } else {
                    let value = self.expression(initializer);
                    self.emit(
                        None,
                        None,
                        IrInstructionKind::BindLocal {
                            local: *symbol,
                            value,
                        },
                        *span,
                    );
                }
            }
            HirStatement::Assignment {
                symbol,
                value,
                span,
            } => {
                let value = self.expression(value);
                self.emit(
                    None,
                    None,
                    IrInstructionKind::BindLocal {
                        local: *symbol,
                        value,
                    },
                    *span,
                );
            }
            HirStatement::FieldAssignment {
                local,
                struct_id,
                field,
                value,
                span,
            } => {
                let value = self.expression(value);
                self.emit(
                    None,
                    None,
                    IrInstructionKind::FieldStore {
                        local: *local,
                        struct_id: *struct_id,
                        field: *field,
                        value,
                    },
                    *span,
                );
            }
            HirStatement::IndexedAssignment {
                collection,
                collection_type,
                index,
                value,
                span,
            } => {
                let collection = self.expression(collection);
                let index = self.expression(index);
                let value = self.expression(value);
                let kind = match collection_type {
                    Type::Array { element, length } => IrInstructionKind::ArrayStore {
                        collection,
                        element_type: (**element).clone(),
                        length: *length,
                        index,
                        value,
                    },
                    Type::List(element) => IrInstructionKind::ListStore {
                        collection,
                        element_type: (**element).clone(),
                        index,
                        value,
                    },
                    _ => unreachable!("indexed assignment has collection type"),
                };
                self.emit(None, None, kind, *span);
            }
            HirStatement::CompoundIndexedAssignment {
                collection,
                collection_type,
                index,
                operator,
                value,
                span,
            } => {
                let collection_value = self.expression(collection);
                let index_value = self.expression(index);
                let element_type = match collection_type {
                    Type::Array { element, .. } | Type::List(element) => (**element).clone(),
                    _ => unreachable!("compound indexed assignment has collection type"),
                };
                let load = match collection_type {
                    Type::Array { element, length } => IrInstructionKind::ArrayLoad {
                        collection: collection_value,
                        element_type: (**element).clone(),
                        length: *length,
                        index: index_value,
                    },
                    Type::List(element) => IrInstructionKind::ListLoad {
                        collection: collection_value,
                        element_type: (**element).clone(),
                        index: index_value,
                    },
                    _ => unreachable!("compound indexed assignment has collection type"),
                };
                let left = self.emit_value(load, element_type.clone(), *span);
                let right = self.expression(value);
                let result = self.emit_value(
                    IrInstructionKind::Binary {
                        operator: *operator,
                        left,
                        right,
                    },
                    element_type.clone(),
                    *span,
                );
                let store = match collection_type {
                    Type::Array { element, length } => IrInstructionKind::ArrayStore {
                        collection: collection_value,
                        element_type: (**element).clone(),
                        length: *length,
                        index: index_value,
                        value: result,
                    },
                    Type::List(element) => IrInstructionKind::ListStore {
                        collection: collection_value,
                        element_type: (**element).clone(),
                        index: index_value,
                        value: result,
                    },
                    _ => unreachable!("compound indexed assignment has collection type"),
                };
                self.emit(None, None, store, *span);
            }
            HirStatement::Return { value, .. } => {
                let value = value.as_ref().map(|value| self.expression(value));
                self.terminate(IrTerminator::Return(value));
            }
            HirStatement::Expression(expression) => {
                if let HirExpressionKind::Exit { code } = &expression.kind {
                    let code = self.expression(code);
                    self.terminate(IrTerminator::Exit(code));
                } else {
                    self.expression(expression);
                }
            }
            HirStatement::If {
                condition,
                then_block,
                else_block,
                ..
            } => self.if_statement(condition, then_block, else_block.as_ref()),
            HirStatement::While {
                condition, body, ..
            } => self.while_statement(condition, body),
            HirStatement::Break(_) => {
                if let Some(targets) = self.loops.last().copied() {
                    self.terminate(IrTerminator::Jump(targets.break_block));
                }
            }
            HirStatement::Continue(_) => {
                if let Some(targets) = self.loops.last().copied() {
                    self.terminate(IrTerminator::Jump(targets.continue_block));
                }
            }
        }
    }

    fn if_statement(
        &mut self,
        condition: &HirExpression,
        then_block: &HirBlock,
        else_block: Option<&HirBlock>,
    ) {
        let condition = self.expression(condition);
        let then_target = self.new_block();
        let else_target = self.new_block();
        self.terminate(IrTerminator::Branch {
            condition,
            then_block: then_target,
            else_block: else_target,
        });

        self.switch_to(then_target);
        self.block(then_block);
        let then_end = self.current;
        let then_falls_through = !self.is_terminated();

        self.switch_to(else_target);
        if let Some(else_block) = else_block {
            self.block(else_block);
        }
        let else_end = self.current;
        let else_falls_through = !self.is_terminated();

        if then_falls_through || else_falls_through {
            let merge_target = self.new_block();
            if then_falls_through {
                self.terminate_block(then_end, IrTerminator::Jump(merge_target));
            }
            if else_falls_through {
                self.terminate_block(else_end, IrTerminator::Jump(merge_target));
            }
            self.switch_to(merge_target);
        }
    }

    fn while_statement(&mut self, condition: &HirExpression, body: &HirBlock) {
        let condition_block = self.new_block();
        let body_block = self.new_block();
        let exit_block = self.new_block();
        self.terminate(IrTerminator::Jump(condition_block));

        self.switch_to(condition_block);
        let condition = self.expression(condition);
        self.terminate(IrTerminator::Branch {
            condition,
            then_block: body_block,
            else_block: exit_block,
        });

        self.loops.push(LoopTargets {
            break_block: exit_block,
            continue_block: condition_block,
        });
        self.switch_to(body_block);
        self.block(body);
        self.loops.pop();
        if !self.is_terminated() {
            self.terminate(IrTerminator::Jump(condition_block));
        }
        self.switch_to(exit_block);
    }

    fn expression(&mut self, expression: &HirExpression) -> ValueId {
        if let HirExpressionKind::Binary {
            operator: BinaryOperator::LogicalAnd | BinaryOperator::LogicalOr,
            left,
            right,
        } = &expression.kind
        {
            return self.short_circuit(expression, left, right);
        }
        let kind = match &expression.kind {
            HirExpressionKind::Literal(literal) => IrInstructionKind::Constant(match literal {
                Literal::Integer(value) if expression.ty == Type::Byte => {
                    IrConstant::Byte(value.clone())
                }
                Literal::Integer(value) => IrConstant::Integer(value.clone()),
                Literal::Float(value) => IrConstant::Float(value.clone()),
                Literal::String(value) => IrConstant::String(value.clone()),
                Literal::Char(value) => IrConstant::Char(*value),
                Literal::Bool(value) => IrConstant::Bool(*value),
            }),
            HirExpressionKind::Symbol(symbol) => IrInstructionKind::LoadLocal(*symbol),
            HirExpressionKind::Call { callee, arguments } => {
                let arguments = arguments
                    .iter()
                    .map(|argument| self.expression(argument))
                    .collect();
                IrInstructionKind::Call {
                    function: *callee,
                    arguments,
                }
            }
            HirExpressionKind::Unary { operator, operand } => IrInstructionKind::Unary {
                operator: *operator,
                operand: self.expression(operand),
            },
            HirExpressionKind::Binary {
                operator,
                left,
                right,
            } => IrInstructionKind::Binary {
                operator: *operator,
                left: self.expression(left),
                right: self.expression(right),
            },
            HirExpressionKind::Collection(values) | HirExpressionKind::Tuple(values) => {
                let values = values.iter().map(|value| self.expression(value)).collect();
                IrInstructionKind::Aggregate(values)
            }
            HirExpressionKind::ArrayLiteral(values) => {
                let Type::Array { element, length } = &expression.ty else {
                    unreachable!("array literal has array type")
                };
                IrInstructionKind::ArrayValue {
                    element_type: (**element).clone(),
                    length: *length,
                    values: values.iter().map(|value| self.expression(value)).collect(),
                }
            }
            HirExpressionKind::ListLiteral(values) => {
                let Type::List(element) = &expression.ty else {
                    unreachable!("list literal has list type")
                };
                IrInstructionKind::ListValue {
                    element_type: (**element).clone(),
                    values: values.iter().map(|value| self.expression(value)).collect(),
                }
            }
            HirExpressionKind::StructLiteral { struct_id, fields } => {
                IrInstructionKind::StructValue {
                    struct_id: *struct_id,
                    fields: fields
                        .iter()
                        .map(|(field, value)| (*field, self.expression(value)))
                        .collect(),
                }
            }
            HirExpressionKind::StructCopy { struct_id, source } => {
                IrInstructionKind::AggregateCopy {
                    struct_id: *struct_id,
                    source: self.expression(source),
                }
            }
            HirExpressionKind::OptionalSome { value } => IrInstructionKind::OptionalSome {
                value: self.expression(value),
                value_type: value.ty.clone(),
            },
            HirExpressionKind::OptionalNone => {
                let Type::Optional(value_type) = &expression.ty else {
                    unreachable!("none expression has optional type")
                };
                IrInstructionKind::OptionalNone {
                    value_type: (**value_type).clone(),
                }
            }
            HirExpressionKind::OptionalHasValue { value } => {
                IrInstructionKind::OptionalHasValue(self.expression(value))
            }
            HirExpressionKind::OptionalValue { value } => {
                let Type::Optional(value_type) = &value.ty else {
                    unreachable!("optional value receiver has optional type")
                };
                IrInstructionKind::OptionalValue {
                    optional: self.expression(value),
                    value_type: (**value_type).clone(),
                }
            }
            HirExpressionKind::FieldLoad {
                base,
                struct_id,
                field,
            } => IrInstructionKind::FieldLoad {
                base: self.expression(base),
                struct_id: *struct_id,
                field: *field,
            },
            HirExpressionKind::EnumValue { enum_id, variant } => IrInstructionKind::EnumConstant {
                enum_id: *enum_id,
                variant: *variant,
            },
            HirExpressionKind::ArrayLoad { collection, index } => {
                let Type::Array { element, length } = &collection.ty else {
                    unreachable!("array load local has array type")
                };
                IrInstructionKind::ArrayLoad {
                    collection: self.expression(collection),
                    element_type: (**element).clone(),
                    length: *length,
                    index: self.expression(index),
                }
            }
            HirExpressionKind::ArrayLength { length } => IrInstructionKind::ArrayLength(*length),
            HirExpressionKind::ListLoad { collection, index } => {
                let Type::List(element) = &collection.ty else {
                    unreachable!("list load local has list type")
                };
                IrInstructionKind::ListLoad {
                    collection: self.expression(collection),
                    element_type: (**element).clone(),
                    index: self.expression(index),
                }
            }
            HirExpressionKind::ListLength { collection } => {
                let Type::List(element) = &collection.ty else {
                    unreachable!("list length local has list type")
                };
                IrInstructionKind::ListLength {
                    collection: self.expression(collection),
                    element_type: (**element).clone(),
                }
            }
            HirExpressionKind::CliArgLoad { local, index } => IrInstructionKind::CliArgLoad {
                local: *local,
                index: self.expression(index),
            },
            HirExpressionKind::CliArgsLength { local } => {
                IrInstructionKind::CliArgsLength { local: *local }
            }
            HirExpressionKind::ListPush { local, value } => {
                let Type::List(element) = self.local_type(*local) else {
                    unreachable!("list push local has list type")
                };
                IrInstructionKind::ListPush {
                    local: *local,
                    element_type: *element,
                    value: self.expression(value),
                }
            }
            HirExpressionKind::ListPushField {
                local,
                struct_id,
                field,
                element_type,
                value,
            } => IrInstructionKind::ListPushField {
                local: *local,
                struct_id: *struct_id,
                field: *field,
                element_type: element_type.clone(),
                value: self.expression(value),
            },
            HirExpressionKind::ListPushIndexed {
                collection,
                collection_type,
                index,
                element_type,
                value,
            } => IrInstructionKind::ListPushIndexed {
                collection: self.expression(collection),
                collection_type: collection_type.clone(),
                index: self.expression(index),
                element_type: element_type.clone(),
                value: self.expression(value),
            },
            HirExpressionKind::ListPop { local } => {
                let Type::List(element) = self.local_type(*local) else {
                    unreachable!("list pop local has list type")
                };
                IrInstructionKind::ListPop {
                    local: *local,
                    element_type: *element,
                }
            }
            HirExpressionKind::ListPopValue { collection } => {
                let Type::List(element) = &collection.ty else {
                    unreachable!("list pop receiver has list type")
                };
                IrInstructionKind::ListPopValue {
                    collection: self.expression(collection),
                    element_type: (**element).clone(),
                }
            }
            HirExpressionKind::StringLiteral(value) => {
                IrInstructionKind::StringConstant(value.clone())
            }
            HirExpressionKind::StringLength { value } => {
                IrInstructionKind::StringLength(self.expression(value))
            }
            HirExpressionKind::StringByte { value, index } => IrInstructionKind::StringByte {
                value: self.expression(value),
                index: self.expression(index),
            },
            HirExpressionKind::StringSlice { value, start, end } => {
                IrInstructionKind::StringSlice {
                    value: self.expression(value),
                    start: self.expression(start),
                    end: self.expression(end),
                }
            }
            HirExpressionKind::ReadFile { path } => {
                IrInstructionKind::ReadFile(self.expression(path))
            }
            HirExpressionKind::ReadBytes { path } => {
                IrInstructionKind::ReadBytes(self.expression(path))
            }
            HirExpressionKind::WriteFile { path, data } => IrInstructionKind::WriteFile {
                path: self.expression(path),
                data: self.expression(data),
            },
            HirExpressionKind::WriteBytes { path, data } => IrInstructionKind::WriteBytes {
                path: self.expression(path),
                data: self.expression(data),
            },
            HirExpressionKind::Exists { path } => IrInstructionKind::Exists(self.expression(path)),
            HirExpressionKind::Print {
                value,
                stderr,
                newline,
            } => IrInstructionKind::Print {
                value: self.expression(value),
                value_type: value.ty.clone(),
                stderr: *stderr,
                newline: *newline,
            },
            HirExpressionKind::Input { target } => IrInstructionKind::Input {
                target: target.clone(),
            },
            HirExpressionKind::Convert { value, target } => IrInstructionKind::Convert {
                value: self.expression(value),
                from: value.ty.clone(),
                to: target.clone(),
            },
            HirExpressionKind::Exit { .. } => {
                unreachable!("exit lowers as a control-flow terminator")
            }
            HirExpressionKind::StringConcat { left, right } => IrInstructionKind::StringConcat {
                left: self.expression(left),
                right: self.expression(right),
            },
            HirExpressionKind::StringEqual { equal, left, right } => {
                IrInstructionKind::StringEqual {
                    equal: *equal,
                    left: self.expression(left),
                    right: self.expression(right),
                }
            }
        };
        self.emit_value(kind, expression.ty.clone(), expression.span)
    }

    fn short_circuit(
        &mut self,
        expression: &HirExpression,
        left: &HirExpression,
        right: &HirExpression,
    ) -> ValueId {
        let left_value = self.expression(left);
        let right_block = self.new_block();
        let short_block = self.new_block();
        let merge_block = self.new_block();
        let result = self.new_value();
        let is_and = matches!(
            expression.kind,
            HirExpressionKind::Binary {
                operator: BinaryOperator::LogicalAnd,
                ..
            }
        );
        self.terminate(if is_and {
            IrTerminator::Branch {
                condition: left_value,
                then_block: right_block,
                else_block: short_block,
            }
        } else {
            IrTerminator::Branch {
                condition: left_value,
                then_block: short_block,
                else_block: right_block,
            }
        });

        self.switch_to(short_block);
        self.emit(
            Some(result),
            Some(Type::Bool),
            IrInstructionKind::Constant(IrConstant::Bool(!is_and)),
            expression.span,
        );
        self.terminate(IrTerminator::Jump(merge_block));

        self.switch_to(right_block);
        let right_value = self.expression(right);
        self.emit(
            Some(result),
            Some(Type::Bool),
            IrInstructionKind::Copy(right_value),
            expression.span,
        );
        self.terminate(IrTerminator::Jump(merge_block));

        self.switch_to(merge_block);
        result
    }

    fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.blocks.len() as u32);
        self.blocks.push(IrBlock {
            id,
            instructions: Vec::new(),
            terminator: None,
        });
        id
    }

    fn switch_to(&mut self, block: BlockId) {
        self.current = block;
    }

    fn is_terminated(&self) -> bool {
        self.blocks[self.current.0 as usize].terminator.is_some()
    }

    fn terminate(&mut self, terminator: IrTerminator) {
        self.terminate_block(self.current, terminator);
    }

    fn terminate_block(&mut self, block: BlockId, terminator: IrTerminator) {
        self.blocks[block.0 as usize].terminator = Some(terminator);
    }

    fn new_value(&mut self) -> ValueId {
        let value = ValueId(self.next_value);
        self.next_value += 1;
        value
    }

    fn emit_value(&mut self, kind: IrInstructionKind, ty: Type, span: Span) -> ValueId {
        let value = self.new_value();
        self.emit(Some(value), Some(ty), kind, span);
        value
    }

    fn local_type(&self, local: crate::frontend::resolution::SymbolId) -> Type {
        self.local_types
            .get(&local)
            .cloned()
            .expect("semantic analysis typed every local")
    }

    fn emit(
        &mut self,
        result: Option<ValueId>,
        result_type: Option<Type>,
        kind: IrInstructionKind,
        span: Span,
    ) {
        self.blocks[self.current.0 as usize]
            .instructions
            .push(IrInstruction {
                result,
                result_type,
                kind,
                span,
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::{lexer::lex, parser::parse, semantic::analyze, source::FileId};

    fn lower_source(source: &str) -> IrModule {
        let ast = parse(lex(FileId(0), source).unwrap()).unwrap();
        lower(&analyze(&ast).unwrap())
    }

    #[test]
    fn if_else_creates_branching_cfg() {
        let ir =
            lower_source("private int f(int x) { if (x > 0) { return 1; } else { return 2; } }");
        assert!(matches!(
            ir.functions[0].blocks[0].terminator,
            Some(IrTerminator::Branch { .. })
        ));
        assert_eq!(ir.functions[0].blocks.len(), 3);
    }

    #[test]
    fn if_without_else_branches_to_merge_block() {
        let ir = lower_source("private void f(bool value) { if (value) { value = false; } }");
        let Some(IrTerminator::Branch {
            then_block,
            else_block,
            ..
        }) = ir.functions[0].blocks[0].terminator
        else {
            panic!("expected conditional branch");
        };
        assert_ne!(then_block, else_block);
        assert!(matches!(
            ir.functions[0].blocks[else_block.0 as usize].terminator,
            Some(IrTerminator::Jump(_))
        ));
    }

    #[test]
    fn while_break_and_continue_target_cfg_blocks() {
        let ir =
            lower_source("private void f() { while (true) { if (false) { break; } continue; } }");
        let function = &ir.functions[0];
        assert!(
            function
                .blocks
                .iter()
                .any(|block| matches!(block.terminator, Some(IrTerminator::Jump(BlockId(1)))))
        );
        assert!(
            function
                .blocks
                .iter()
                .any(|block| matches!(block.terminator, Some(IrTerminator::Jump(BlockId(3)))))
        );
    }

    #[test]
    fn logical_operators_create_short_circuit_branches() {
        let ir = lower_source("private bool f(bool a, bool b) { return a && b || a; }");
        let branches = ir.functions[0]
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Some(IrTerminator::Branch { .. })))
            .count();
        assert_eq!(branches, 2);
        assert!(
            ir.functions[0]
                .blocks
                .flatten_instructions()
                .any(|instruction| { matches!(instruction.kind, IrInstructionKind::Copy(_)) })
        );
    }

    #[test]
    fn lowers_array_list_and_string_operations_to_explicit_ir() {
        let ir = lower_source(
            "private void f() { int[2] a = [1, 2]; a[0] = a[1]; int al = a.length; list int xs = [3]; xs.push(4); int p = xs.pop(); xs[0] = p; int ll = xs.length; string s = \"a\" + \"b\"; bool same = s == \"ab\"; int sl = s.length; }",
        );
        let instructions: Vec<_> = ir.functions[0]
            .blocks
            .flatten_instructions()
            .map(|instruction| &instruction.kind)
            .collect();
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::ArrayValue { .. }))
        );
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::ArrayLoad { .. }))
        );
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::ArrayStore { .. }))
        );
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::ArrayLength(2)))
        );
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::ListValue { .. }))
        );
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::ListPush { .. }))
        );
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::ListPop { .. }))
        );
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::ListStore { .. }))
        );
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::ListLength { .. }))
        );
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::StringConstant(_)))
        );
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::StringConcat { .. }))
        );
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::StringEqual { .. }))
        );
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::StringLength(_)))
        );
    }

    #[test]
    fn lowers_source_ingestion_to_explicit_ir() {
        let ir = lower_source(
            "use std.fs; private int scan(string path) { string source = readFile(path); return source.byte(0); } public void main(string[] args) { int count = args.length; string first = args[0]; }",
        );
        let instructions: Vec<_> = ir
            .functions
            .iter()
            .flat_map(|function| function.blocks.flatten_instructions())
            .map(|instruction| &instruction.kind)
            .collect();
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::ReadFile(_)))
        );
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::StringByte { .. }))
        );
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::CliArgsLength { .. }))
        );
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::CliArgLoad { .. }))
        );
        assert_eq!(ir.functions[1].parameters[0].ty, Type::CliArgs);
    }

    #[test]
    fn lowers_struct_copies_struct_lists_and_string_slices_explicitly() {
        let ir = lower_source(
            "struct Token { string text; int line; } private void f() { Token a = Token { text: \"hello\", line: 1 }; Token b = a; list Token values = []; values.push(b); values[0] = b; Token first = values[0]; Token last = values.pop(); string piece = first.text.slice(0, 1); }",
        );
        let instructions: Vec<_> = ir.functions[0]
            .blocks
            .flatten_instructions()
            .map(|instruction| &instruction.kind)
            .collect();
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::AggregateCopy { .. }))
        );
        assert!(instructions.iter().any(|kind| matches!(
            kind,
            IrInstructionKind::ListPush {
                element_type: Type::Struct(_),
                ..
            }
        )));
        assert!(instructions.iter().any(|kind| matches!(
            kind,
            IrInstructionKind::ListLoad {
                element_type: Type::Struct(_),
                ..
            }
        )));
        assert!(instructions.iter().any(|kind| matches!(
            kind,
            IrInstructionKind::ListStore {
                element_type: Type::Struct(_),
                ..
            }
        )));
        assert!(instructions.iter().any(|kind| matches!(
            kind,
            IrInstructionKind::ListPop {
                element_type: Type::Struct(_),
                ..
            }
        )));
        assert!(
            instructions
                .iter()
                .any(|kind| matches!(kind, IrInstructionKind::StringSlice { .. }))
        );
    }

    trait BlockInstructions<'a> {
        fn flatten_instructions(&'a self) -> Box<dyn Iterator<Item = &'a IrInstruction> + 'a>;
    }

    impl<'a> BlockInstructions<'a> for [IrBlock] {
        fn flatten_instructions(&'a self) -> Box<dyn Iterator<Item = &'a IrInstruction> + 'a> {
            Box::new(self.iter().flat_map(|block| &block.instructions))
        }
    }
}
