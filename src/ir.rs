use std::collections::HashMap;
use crate::ast::{Initializer, ASN, AST};
use crate::common::BoxError;
use crate::expression::{Expression, Operator};

fn simplify(ast: AST) -> Result<AST, BoxError> {
    let AST { node, children } = ast;
    let mut simplified_children = Vec::new();
    let mut iter = children.into_iter().peekable();

    while let Some(mut child) = iter.next() {
        if let ASN::If { .. } = child.node {
            child = build_if_tree(child, &mut iter)?;
        }
        simplified_children.push(simplify(child)?);
    }

    match node {
        ASN::For { initializer, condition, increment } => {
            let mut scope_children = Vec::new();

            if let Some(init) = initializer {
                scope_children.push(AST::new(match init {
                    Initializer::Declaration { typ, name, expression } =>
                        ASN::Declaration { typ, name, expression },
                    Initializer::Expression { expression } =>
                        ASN::Expression { expression },
                }));
            }

            let mut while_body = simplified_children;
            if let Some(inc) = increment {
                while_body.push(AST::new(ASN::Expression { expression: inc }));
            }

            let condition = condition.unwrap_or_else(|| Expression::Literal("true".into()));
            let while_node = AST::with_children(ASN::While { condition }, while_body);
            scope_children.push(while_node);

            Ok(AST::with_children(ASN::Scope, scope_children))
        }
        other_node => Ok(AST::with_children(other_node, simplified_children)),
    }
}

fn build_if_tree(mut current_node: AST, iter: &mut std::iter::Peekable<impl Iterator<Item = AST>>) -> Result<AST, BoxError> {
    if let ASN::ElseIf { condition } = current_node.node {
        current_node.node = ASN::If { condition };
    }

    match iter.peek().map(|ast| &ast.node) {
        Some(ASN::ElseIf { .. }) => {
            let next_node = iter.next().unwrap();
            let nested_if = build_if_tree(next_node, iter)?;
            current_node.children.push(AST::with_children(ASN::Else, vec![nested_if]));
        }
        Some(ASN::Else) => {
            let else_node = iter.next().unwrap();
            current_node.children.push(else_node);
        }
        _ => {}
    }

    Ok(current_node)
}

pub type BlockId = usize;
pub type ValueId = usize;

#[derive(Debug, Clone)]
pub enum Operand {
    Value(ValueId),
    Constant(String),
    Void,
}

#[derive(Debug, Clone)]
pub enum IrInstruction {
    LoadConst { dest: ValueId, value: String },
    BinaryOp { dest: ValueId, left: Operand, op: Operator, right: Operand },
    Call { dest: Option<ValueId>, name: String, arguments: Vec<Operand> },
    Phi { dest: ValueId, incoming: Vec<(BlockId, Operand)> },
}

#[derive(Debug, Clone)]
pub enum Terminator {
    Jump(BlockId),
    Branch { condition: Operand, true_block: BlockId, false_block: BlockId },
    Return(Option<Operand>),
}

pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<IrInstruction>,
    pub terminator: Option<Terminator>,
    pub predecessors: Vec<BlockId>,
    pub successors: Vec<BlockId>,
}

impl BasicBlock {
    pub fn new(id: BlockId) -> Self {
        Self { id, instructions: Vec::new(), terminator: None, predecessors: Vec::new(), successors: Vec::new() }
    }
}

pub struct CFG {
    pub blocks: Vec<BasicBlock>,
    pub entry_block: BlockId,
}

pub struct IrContext {
    pub blocks: Vec<BasicBlock>,
    current_block: Option<BlockId>,
    value_counter: usize,
    vars: HashMap<String, ValueId>,
}

impl IrContext {
    pub fn new() -> Self {
        let entry_block = BasicBlock::new(0);
        IrContext { value_counter: 0, blocks: vec![entry_block], current_block: Some(0), vars: HashMap::new() }
    }

    pub fn new_value(&mut self) -> ValueId {
        let id = self.value_counter;
        self.value_counter += 1;
        id
    }

    pub fn create_block(&mut self) -> BlockId {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock::new(id));
        id
    }

    pub fn set_current_block(&mut self, id: BlockId) {
        self.current_block = Some(id);
    }

    pub fn current_block(&self) -> BlockId {
        self.current_block.expect("Not writing to any block!")
    }

    pub fn push_instruction(&mut self, instruction: IrInstruction) {
        if let Some(block_id) = self.current_block {
            self.blocks[block_id].instructions.push(instruction);
        }
    }

    pub fn emit(&mut self, instr: IrInstruction) {
        let block_id = self.current_block();
        self.blocks[block_id].instructions.push(instr);
    }

    pub fn emit_terminator(&mut self, term: Terminator) {
        let source_block = self.current_block();

        match &term {
            Terminator::Jump(target) => {
                self.blocks[source_block].successors.push(*target);
                self.blocks[*target].predecessors.push(source_block);
            }
            Terminator::Branch { true_block: true_target, false_block: false_target, .. } => {
                self.blocks[source_block].successors.push(*true_target);
                self.blocks[source_block].successors.push(*false_target);
                self.blocks[*true_target].predecessors.push(source_block);
                self.blocks[*false_target].predecessors.push(source_block);
            }
            Terminator::Return(_) => {}
        }

        self.blocks[source_block].terminator = Some(term);
        self.current_block = None;
    }

    pub fn write_var(&mut self, name: String, val: ValueId) {
        self.vars.insert(name, val);
    }

    pub fn read_var(&mut self, name: &str) -> ValueId {
        *self.vars.get(name).expect("Variable not defined")
    }

    pub fn gen_statement(&mut self, ast: AST) -> Result<(), BoxError> {
        match ast.node {
            ASN::If { condition } => {
                let condition = self.gen_expression(condition)?;
                let true_block = self.create_block();
                let false_block = self.create_block();
                let merge_block = self.create_block();
                self.emit_terminator(Terminator::Branch { condition, true_block, false_block });

                let mut else_node: Option<AST> = None;
                self.set_current_block(true_block);
                for child in ast.children {
                    if let ASN::Else = child.node {
                        else_node = Some(child);
                    } else {
                        self.gen_statement(child)?;
                    }
                }
                self.emit_terminator(Terminator::Jump(merge_block));

                self.set_current_block(false_block);
                if let Some(else_node) = else_node {
                    for child in else_node.children {
                        self.gen_statement(child)?;
                    }
                }
                if self.blocks[false_block].terminator.is_none() {
                    self.emit_terminator(Terminator::Jump(merge_block));
                }

                self.set_current_block(merge_block);
            }
            ASN::While { condition } => {
                let condition_block = self.create_block();
                let true_block = self.create_block();
                let false_block = self.create_block();
                self.emit_terminator(Terminator::Jump(condition_block));

                self.set_current_block(condition_block);
                let condition = self.gen_expression(condition)?;
                self.emit_terminator(Terminator::Branch { condition, true_block, false_block });

                self.set_current_block(true_block);
                for child in ast.children {
                    self.gen_statement(child)?;
                }
                self.emit_terminator(Terminator::Jump(condition_block));

                self.set_current_block(false_block);
            }
            ASN::Expression { expression } => {
                self.gen_expression(expression)?;
            }
            ASN::Declaration { name, expression, .. } => {
                let operand = if let Some(expression) = expression {
                    self.gen_expression(expression)?
                } else {
                    Operand::Constant("0".into())
                };
                let value_id = match operand {
                    Operand::Value(id) => id,
                    Operand::Constant(c) => {
                        let new_id = self.new_value();
                        self.emit(IrInstruction::LoadConst { dest: new_id, value: c });
                        new_id
                    }
                    Operand::Void => return Err("Cannot declare void".into(),),
                };
                self.write_var(name, value_id);
            }
            ASN::Function { result_type, name, arguments } => {

            }
            ASN::Class { .. } => {}
            ASN::Return { .. } => {}
            ASN::Break => {}
            ASN::Continue => {}
            ASN::Scope => {}
            ASN::File => {}
            _ => return Err("Doesn't expect here".into())
        }
        Ok(())
    }

    pub fn gen_expression(&mut self, expression: Expression) -> Result<Operand, BoxError> {
        match expression {
            Expression::Literal(val) => {
                let dest = self.new_value();
                self.emit(IrInstruction::LoadConst { dest, value: val });
                Ok(Operand::Value(dest))
            }
            Expression::Variable { name } => {
                let val_id = self.read_var(&name);
                Ok(Operand::Value(val_id))
            }
            Expression::BinaryOp { left, op, right } => {
                let left = self.gen_expression(*left)?;
                let right = self.gen_expression(*right)?;

                let dest = self.new_value();

                self.emit(IrInstruction::BinaryOp {dest, left, op, right});

                Ok(Operand::Value(dest))
            }
            Expression::FunctionCall { name, arguments } => {
                let dest = self.new_value();
                let mut operands = vec![];
                for argument in arguments {
                    operands.push(self.gen_expression(argument)?);
                }
                self.emit(IrInstruction::Call { dest: Some(dest), name, arguments: operands });
                Ok(Operand::Value(dest))
            }
            Expression::Assign { name, value } => {
                let operand = self.gen_expression(*value)?;
                let dest = match operand {
                    Operand::Value(dest) => dest,
                    Operand::Constant(c) => {
                        let dest = self.new_value();
                        self.emit(IrInstruction::LoadConst { dest, value: c });
                        dest
                    }
                    Operand::Void => return Err("Cannot assign void".into(),),
                };
                self.write_var(name, dest);
                Ok(Operand::Void)
            }
            Expression::Increment { name } => {
                let val_id = self.read_var(&name);
                let dest = self.new_value();
                let one_const = Operand::Constant("1".to_string());
                self.emit(IrInstruction::BinaryOp {
                    dest, left: Operand::Value(val_id), op: Operator::Plus, right: one_const });
                self.write_var(name, dest);
                Ok(Operand::Value(val_id))
            }
            Expression::Decrement { name } => {
                let val_id = self.read_var(&name);
                let dest = self.new_value();
                let one_const = Operand::Constant("1".to_string());
                self.emit(IrInstruction::BinaryOp {
                    dest, left: Operand::Value(val_id), op: Operator::Minus, right: one_const });
                self.write_var(name, dest);
                Ok(Operand::Value(val_id))
            }
            Expression::Negate { expression } => {
                let operand = self.gen_expression(*expression)?;
                let dest = self.new_value();
                let zero_const = Operand::Constant("0".to_string());
                self.emit(IrInstruction::BinaryOp {
                    dest, left: zero_const, op: Operator::Minus, right: operand });
                Ok(Operand::Value(dest))
            }
            Expression::Not { expression } => {
                let operand = self.gen_expression(*expression)?;
                let dest = self.new_value();
                let false_const = Operand::Constant("false".to_string());
                self.emit(IrInstruction::BinaryOp {
                    dest, left: operand, op: Operator::Equal, right: false_const});
                Ok(Operand::Value(dest))
            }
        }
    }
}

pub fn compile(ast: AST) -> Result<CFG, BoxError> {
    let simple_ast = simplify(ast)?;
    let mut ctx = IrContext::new();
    let entry_block = ctx.create_block();
    ctx.set_current_block(entry_block);
    ctx.gen_statement(simple_ast)?;
    Ok(CFG { blocks: ctx.blocks, entry_block })
}
