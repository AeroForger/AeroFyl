use crate::frontend::ast::{BinaryOperator, Literal};
use crate::frontend::source::Span;
use crate::frontend::types::Type;
use crate::middle::hir::*;
use crate::middle::ir::*;

/// Lowers checked HIR to a simple non-SSA control-flow graph.
pub fn lower(module: &HirModule) -> IrModule {
    IrModule {
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
                let value = self.expression(initializer);
                self.locals.push(IrLocal {
                    symbol: *symbol,
                    ty: ty.clone(),
                });
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
            HirStatement::Return { value, .. } => {
                let value = value.as_ref().map(|value| self.expression(value));
                self.terminate(IrTerminator::Return(value));
            }
            HirStatement::Expression(expression) => {
                self.expression(expression);
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

    trait BlockInstructions<'a> {
        fn flatten_instructions(&'a self) -> Box<dyn Iterator<Item = &'a IrInstruction> + 'a>;
    }

    impl<'a> BlockInstructions<'a> for [IrBlock] {
        fn flatten_instructions(&'a self) -> Box<dyn Iterator<Item = &'a IrInstruction> + 'a> {
            Box::new(self.iter().flat_map(|block| &block.instructions))
        }
    }
}
