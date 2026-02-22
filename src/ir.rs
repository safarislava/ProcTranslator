use std::collections::HashMap;
use crate::ast::{Initializer, ASN, AST};
use crate::common::BoxError;
use crate::expression::{Expression, BinaryOperator};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Register(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StackSlot(pub usize);

#[derive(Debug, Clone)]
pub enum Operand {
    Value(Register),
    Constant(String),
    Void,
}

#[derive(Debug, Clone)]
pub enum IrInstruction {
    LoadConst { dest: Register, value: String },
    BinaryOp { dest: Register, left: Operand, op: BinaryOperator, right: Operand },
    Call { dest: Option<Register>, name: String, arguments: Vec<Operand> },
    StackAlloc { slot: StackSlot },
    Store { slot: StackSlot, value: Operand },
    Load { dest: Register, slot: StackSlot },
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
    reg_counter: usize,
    slot_counter: usize,
    scopes: Vec<HashMap<String, StackSlot>>,
    functions: HashMap<String, BasicBlock>,
}

impl IrContext {
    pub fn new() -> Self {
        let entry_block = BasicBlock::new(0);
        IrContext {
            reg_counter: 0,
            slot_counter: 0,
            blocks: vec![entry_block],
            current_block: Some(0),
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
        }
    }

    pub fn new_reg(&mut self) -> Register {
        let id = self.reg_counter;
        self.reg_counter += 1;
        Register(id)
    }

    pub fn new_slot(&mut self) -> StackSlot {
        let id = self.slot_counter;
        self.slot_counter += 1;
        StackSlot(id)
    }

    pub fn create_block(&mut self) -> BlockId {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock::new(id));
        id
    }

    pub fn set_current_block(&mut self, id: BlockId) {
        self.current_block = Some(id);
    }

    fn is_current_terminated(&self) -> bool {
        if let Some(id) = self.current_block {
            self.blocks[id].terminator.is_some()
        } else {
            true
        }
    }

    pub fn emit(&mut self, instruction: IrInstruction) {
        if let Some(block_id) = self.current_block && self.blocks[block_id].terminator.is_none() {
            self.blocks[block_id].instructions.push(instruction);
        }
    }

    pub fn emit_terminator(&mut self, term: Terminator) {
        if let Some(source_block) = self.current_block {
            if self.blocks[source_block].terminator.is_some() {
                return;
            }

            match &term {
                Terminator::Jump(target) => {
                    self.link_blocks(source_block, *target);
                }
                Terminator::Branch { true_block, false_block, .. } => {
                    self.link_blocks(source_block, *true_block);
                    self.link_blocks(source_block, *false_block);
                }
                Terminator::Return(_) => {}
            }

            self.blocks[source_block].terminator = Some(term);
            self.current_block = None;
        }
    }

    fn link_blocks(&mut self, from: BlockId, to: BlockId) {
        self.blocks[from].successors.push(to);
        self.blocks[to].predecessors.push(from);
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn declare_var(&mut self, name: String) -> StackSlot {
        let slot = self.new_slot();
        self.emit(IrInstruction::StackAlloc { slot });

        let current_scope = self.scopes.last_mut().expect("No scope active");
        current_scope.insert(name, slot);
        slot
    }

    pub fn resolve_var_addr(&self, name: &str) -> Result<StackSlot, BoxError> {
        for scope in self.scopes.iter().rev() {
            if let Some(&slot) = scope.get(name) {
                return Ok(slot);
            }
        }
        Err(format!("Undefined variable: {}", name).into())
    }

    pub fn gen_statement(&mut self, ast: AST) -> Result<(), BoxError> {
        match ast.node {
            ASN::If { condition } => {
                let condition = self.gen_expression(condition)?;
                let true_block = self.create_block();
                let false_block = self.create_block();
                let merge_block = self.create_block();
                self.emit_terminator(Terminator::Branch { condition, true_block, false_block });

                self.set_current_block(true_block);
                self.enter_scope();
                let mut else_node: Option<AST> = None;
                self.set_current_block(true_block);
                for child in ast.children {
                    if let ASN::Else = child.node {
                        else_node = Some(child);
                    } else {
                        self.gen_statement(child)?;
                    }
                }
                self.exit_scope();
                self.emit_terminator(Terminator::Jump(merge_block));

                self.set_current_block(false_block);
                if let Some(node) = else_node {
                    self.enter_scope();
                    for child in node.children {
                        self.gen_statement(child)?;
                    }
                    self.exit_scope();
                }
                self.emit_terminator(Terminator::Jump(merge_block));

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
                self.enter_scope();
                for child in ast.children {
                    self.gen_statement(child)?;
                }
                self.exit_scope();
                self.emit_terminator(Terminator::Jump(condition_block));

                self.set_current_block(false_block);
            }
            ASN::Expression { expression } => {
                self.gen_expression(expression)?;
            }
            ASN::Declaration { name, expression, .. } => {
                let slot = self.declare_var(name);
                let value = if let Some(expr) = expression {
                    self.gen_expression(expr)?
                } else {
                    Operand::Constant("0".into())
                };

                if !matches!(value, Operand::Void) {
                    self.emit(IrInstruction::Store { slot, value });
                }
            }
            ASN::Callable { result_type, name, arguments } => {

            }
            ASN::Class { name } => {}
            ASN::Return { value } => {}
            ASN::Break => {}
            ASN::Continue => {}
            ASN::Scope => {
                self.enter_scope();
                for child in ast.children {
                    self.gen_statement(child)?;
                }
                self.exit_scope();
            }
            ASN::File => {}
            _ => return Err("Doesn't expect here".into())
        }
        Ok(())
    }

    pub fn gen_expression(&mut self, expression: Expression) -> Result<Operand, BoxError> {
        match expression {
            Expression::Literal(val) => {
                let dest = self.new_reg();
                self.emit(IrInstruction::LoadConst { dest, value: val });
                Ok(Operand::Value(dest))
            }
            Expression::Variable { name } => {
                let slot = self.resolve_var_addr(&name)?;
                let dest = self.new_reg();
                self.emit(IrInstruction::Load { dest, slot });
                Ok(Operand::Value(dest))
            }
            Expression::BinaryOp { left, op, right } => {
                let left = self.gen_expression(*left)?;
                let right = self.gen_expression(*right)?;
                let dest = self.new_reg();
                self.emit(IrInstruction::BinaryOp { dest, left, op, right });
                Ok(Operand::Value(dest))
            }
            Expression::FunctionCall { name, arguments } => {
                let mut args = Vec::new();
                for arg in arguments {
                    args.push(self.gen_expression(arg)?);
                }
                let dest = self.new_reg();
                self.emit(IrInstruction::Call { dest: Some(dest), name, arguments: args });
                Ok(Operand::Value(dest))
            }
            Expression::Assign { name, value } => {
                let slot = self.resolve_var_addr(&name)?;
                let value = self.gen_expression(*value)?;

                if matches!(value, Operand::Void) {
                    return Err("Cannot assign void value".into());
                }

                self.emit(IrInstruction::Store { slot, value });
                Ok(Operand::Void)
            }
            Expression::Increment { expression, postfix } => {
                match *expression {
                    Expression::Variable { name } => {
                        let slot = self.resolve_var_addr(&name)?;
                        let old_val_reg = self.new_reg();
                        self.emit(IrInstruction::Load { dest: old_val_reg, slot });

                        let new_val_reg = self.new_reg();
                        let one_const = Operand::Constant("1".to_string());
                        self.emit(IrInstruction::BinaryOp {
                            dest: new_val_reg,
                            left: Operand::Value(old_val_reg),
                            op: BinaryOperator::Plus,
                            right: one_const,
                        });
                        self.emit(IrInstruction::Store { slot, value: Operand::Value(new_val_reg) });

                        if postfix {
                            Ok(Operand::Value(old_val_reg))
                        } else {
                            Ok(Operand::Value(new_val_reg))
                        }
                    }
                    Expression::Field { object, name } => {
                        todo!()
                    }
                    _ => Err("Cannot increment to a non-changeable expression".into())
                }
            }
            Expression::Decrement { expression, postfix } => {
                match *expression {
                    Expression::Variable { name } => {
                        let slot = self.resolve_var_addr(&name)?;
                        let old_val_reg = self.new_reg();
                        self.emit(IrInstruction::Load { dest: old_val_reg, slot });

                        let new_val_reg = self.new_reg();
                        let one_const = Operand::Constant("1".to_string());
                        self.emit(IrInstruction::BinaryOp {
                            dest: new_val_reg,
                            left: Operand::Value(old_val_reg),
                            op: BinaryOperator::Minus,
                            right: one_const,
                        });
                        self.emit(IrInstruction::Store { slot, value: Operand::Value(new_val_reg) });

                        if postfix {
                            Ok(Operand::Value(old_val_reg))
                        } else {
                            Ok(Operand::Value(new_val_reg))
                        }
                    }
                    Expression::Field { object, name } => {
                        todo!()
                    }
                    _ => Err("Cannot decrement to a non-changeable expression".into())
                }
            }
            Expression::Negate { expression } => {
                let operand = self.gen_expression(*expression)?;
                let dest = self.new_reg();
                let zero_const = Operand::Constant("0".to_string());
                self.emit(IrInstruction::BinaryOp {
                    dest, left: zero_const, op: BinaryOperator::Minus, right: operand });
                Ok(Operand::Value(dest))
            }
            Expression::Not { expression } => {
                let operand = self.gen_expression(*expression)?;
                let dest = self.new_reg();
                let false_const = Operand::Constant("false".to_string());
                self.emit(IrInstruction::BinaryOp {
                    dest, left: operand, op: BinaryOperator::Equal, right: false_const});
                Ok(Operand::Value(dest))
            }
            Expression::MethodCall { object, name, arguments } => {
                todo!()
            }
            Expression::AssignField { object, name, value } => {
                todo!()
            }
            Expression::New { class_name, arguments } => {
                todo!()
            }
            Expression::Field { object, name } => {
                todo!();
            }
            Expression::This => {
                todo!()
            }
        }
    }
}

pub fn compile(ast: AST) -> Result<CFG, BoxError> {
    let simple_ast = simplify(ast)?;
    let mut ctx = IrContext::new();
    ctx.gen_statement(simple_ast)?;
    if !ctx.is_current_terminated() {
        ctx.emit_terminator(Terminator::Return(None));
    }
    Ok(CFG { blocks: ctx.blocks, entry_block: 0 })
}
