use crate::translator::common::{AbstractSyntaxNode, Type, TypedAST, TypedExpression};
use crate::translator::expression::BinaryOperator;
use std::collections::HashMap;

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
    LoadConst {
        dest: Register,
        value: String,
    },
    BinaryOp {
        dest: Register,
        left: Operand,
        op: BinaryOperator,
        right: Operand,
    },
    Call {
        dest: Register,
        block: BlockId,
        arguments: Vec<Operand>,
    },
    LoadParam {
        dest: Register,
        index: usize,
    },

    StackAlloc {
        slot: StackSlot,
    },
    StackStore {
        slot: StackSlot,
        value: Operand,
    },
    StackLoad {
        dest: Register,
        slot: StackSlot,
    },

    GetField {
        dest: Register,
        object: Operand,
        offset: usize,
    },
    PutField {
        object: Operand,
        offset: usize,
        value: Operand,
    },
    AllocObject {
        dest: Register,
        class_name: String,
    },
}

#[derive(Debug, Clone)]
pub enum Terminator {
    Jump(BlockId),
    Branch {
        condition: Operand,
        true_block: BlockId,
        false_block: BlockId,
    },
    Return(Option<Operand>),
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<IrInstruction>,
    pub terminator: Option<Terminator>,
    pub predecessors: Vec<BlockId>,
    pub successors: Vec<BlockId>,
}

impl BasicBlock {
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            instructions: Vec::new(),
            terminator: None,
            predecessors: Vec::new(),
            successors: Vec::new(),
        }
    }
}

struct ClassInfo {
    pub fields: HashMap<String, (Option<TypedExpression>, usize)>,
    pub methods: HashMap<String, BlockId>,
}

impl ClassInfo {
    pub fn new() -> Self {
        ClassInfo {
            fields: HashMap::new(),
            methods: HashMap::new(),
        }
    }
}

struct IrContext {
    pub blocks: Vec<BasicBlock>,
    current_block: Option<BlockId>,
    reg_counter: usize,
    slot_counter: usize,
    scopes: Vec<HashMap<String, StackSlot>>,
    loop_stack: Vec<(BlockId, BlockId)>,
    functions: HashMap<String, BlockId>,
    classes: HashMap<String, ClassInfo>,
    current_class: Option<String>,
    this_register: Option<Register>,
    main_block: Option<BlockId>,
}

impl IrContext {
    pub fn new() -> Self {
        IrContext {
            reg_counter: 0,
            slot_counter: 0,
            blocks: vec![],
            current_block: None,
            scopes: vec![HashMap::new()],
            loop_stack: Vec::new(),
            functions: HashMap::new(),
            classes: HashMap::new(),
            current_class: None,
            this_register: None,
            main_block: None,
        }
    }

    fn new_reg(&mut self) -> Register {
        let id = self.reg_counter;
        self.reg_counter += 1;
        Register(id)
    }

    fn new_slot(&mut self) -> StackSlot {
        let id = self.slot_counter;
        self.slot_counter += 1;
        StackSlot(id)
    }

    fn create_block(&mut self) -> BlockId {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock::new(id));
        id
    }

    fn set_current_block(&mut self, id: BlockId) {
        self.current_block = Some(id);
    }

    fn is_current_terminated(&self) -> bool {
        if let Some(id) = self.current_block {
            self.blocks[id].terminator.is_some()
        } else {
            true
        }
    }

    fn emit(&mut self, instruction: IrInstruction) {
        if let Some(block_id) = self.current_block
            && self.blocks[block_id].terminator.is_none()
        {
            self.blocks[block_id].instructions.push(instruction);
        }
    }

    fn emit_terminator(&mut self, term: Terminator) {
        if let Some(source_block) = self.current_block {
            if self.blocks[source_block].terminator.is_some() {
                return;
            }

            match &term {
                Terminator::Jump(target) => {
                    self.link_blocks(source_block, *target);
                }
                Terminator::Branch {
                    true_block,
                    false_block,
                    ..
                } => {
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

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_var(&mut self, name: String) -> StackSlot {
        let slot = self.new_slot();
        self.emit(IrInstruction::StackAlloc { slot });

        let current_scope = self.scopes.last_mut().expect("No scope active");
        current_scope.insert(name, slot);
        slot
    }

    fn resolve_var_addr(&self, name: &str) -> StackSlot {
        for scope in self.scopes.iter().rev() {
            if let Some(&slot) = scope.get(name) {
                return slot;
            }
        }
        unreachable!()
    }

    fn resolve_field_offset(&self, object_type: &Type, field_name: &str) -> usize {
        if let Type::Class(class_name) = object_type {
            self.classes
                .get(class_name)
                .and_then(|c| c.fields.get(field_name))
                .map(|(_, offset)| *offset)
                .expect("Field not found")
        } else {
            unreachable!("Type is not a class")
        }
    }

    pub fn gen_statement(&mut self, ast: TypedAST) {
        match ast.node {
            AbstractSyntaxNode::If { condition } => {
                let condition = self.gen_expression(condition);
                let true_block = self.create_block();
                let false_block = self.create_block();
                self.emit_terminator(Terminator::Branch {
                    condition,
                    true_block,
                    false_block,
                });

                self.set_current_block(true_block);
                self.enter_scope();

                let mut has_else = false;
                let mut else_if_node: Option<TypedAST> = None;

                for child in &ast.children {
                    if let AbstractSyntaxNode::Else = child.node {
                        has_else = true;
                        if child.children.len() == 1
                            && let AbstractSyntaxNode::If { .. } = child.children[0].node
                        {
                            else_if_node = Some(child.children[0].clone());
                        }
                    } else {
                        self.gen_statement(child.clone());
                    }
                }
                self.exit_scope();

                let true_terminated = self.is_current_terminated();
                if !true_terminated {
                    if else_if_node.is_some() || has_else {
                        self.emit_terminator(Terminator::Jump(false_block));
                    } else {
                        let merge_block = self.create_block();
                        self.emit_terminator(Terminator::Jump(merge_block));
                        self.set_current_block(merge_block);
                        return;
                    }
                }

                self.set_current_block(false_block);
                if let Some(else_if) = else_if_node {
                    self.gen_statement(else_if);
                } else if has_else {
                    self.enter_scope();
                    for child in &ast.children {
                        if let AbstractSyntaxNode::Else = child.node {
                            for grandchild in &child.children {
                                self.gen_statement(grandchild.clone());
                            }
                        }
                    }
                    self.exit_scope();
                } else if !true_terminated {
                    let merge_block = self.create_block();
                    self.emit_terminator(Terminator::Jump(merge_block));
                    self.set_current_block(merge_block);
                    return;
                }

                let false_terminated = self.is_current_terminated();
                if !false_terminated {
                    let merge_block = self.create_block();
                    self.emit_terminator(Terminator::Jump(merge_block));
                    self.set_current_block(merge_block);
                }
            }
            AbstractSyntaxNode::While { condition } => {
                let condition_block = self.create_block();
                let true_block = self.create_block();
                let false_block = self.create_block();
                self.emit_terminator(Terminator::Jump(condition_block));

                self.set_current_block(condition_block);
                let condition = self.gen_expression(condition);
                self.emit_terminator(Terminator::Branch {
                    condition,
                    true_block,
                    false_block,
                });

                self.set_current_block(true_block);
                self.enter_scope();
                self.loop_stack.push((condition_block, false_block));
                for child in ast.children {
                    self.gen_statement(child);
                }
                self.loop_stack.pop();
                self.exit_scope();
                self.emit_terminator(Terminator::Jump(condition_block));

                self.set_current_block(false_block);
            }
            AbstractSyntaxNode::Expression { expression } => {
                self.gen_expression(expression);
            }
            AbstractSyntaxNode::Declaration {
                name, expression, ..
            } => {
                let slot = self.declare_var(name);
                let value = if let Some(expr) = expression {
                    self.gen_expression(expr)
                } else {
                    Operand::Constant("0".into())
                };

                if !matches!(value, Operand::Void) {
                    self.emit(IrInstruction::StackStore { slot, value });
                }
            }
            AbstractSyntaxNode::Callable {
                name, arguments, ..
            } => {
                let block_id = match &self.current_class {
                    Some(current_class) => self.classes[current_class].methods[&name],
                    None => self.functions[&name],
                };

                self.set_current_block(block_id);
                self.enter_scope();

                let mut param_offset = 0;
                if self.current_class.is_some() {
                    let reg = self.new_reg();
                    self.emit(IrInstruction::LoadParam {
                        dest: reg,
                        index: 0,
                    });
                    self.this_register = Some(reg);
                    param_offset = 1;
                }

                for (i, arg) in arguments.into_iter().enumerate() {
                    let slot = self.declare_var(arg.name);
                    let reg = self.new_reg();
                    self.emit(IrInstruction::LoadParam {
                        dest: reg,
                        index: i + param_offset,
                    });
                    self.emit(IrInstruction::StackStore {
                        slot,
                        value: Operand::Value(reg),
                    });
                }

                for child in ast.children {
                    self.gen_statement(child);
                }

                self.exit_scope();
                self.this_register = None;
            }
            AbstractSyntaxNode::Class { name } => {
                self.current_class = Some(name);
                for child in ast.children {
                    self.gen_statement(child);
                }
            }
            AbstractSyntaxNode::Return { value } => {
                let operand = value.map(|value| self.gen_expression(value));
                self.emit_terminator(Terminator::Return(operand))
            }
            AbstractSyntaxNode::Break => {
                if let Some((_, break_target)) = self.loop_stack.last() {
                    self.emit_terminator(Terminator::Jump(*break_target));
                } else {
                    unreachable!();
                }
            }
            AbstractSyntaxNode::Continue => {
                if let Some((continue_target, _)) = self.loop_stack.last() {
                    self.emit_terminator(Terminator::Jump(*continue_target));
                } else {
                    unreachable!();
                }
            }
            AbstractSyntaxNode::Scope => {
                self.enter_scope();
                for child in ast.children {
                    self.gen_statement(child);
                }
                self.exit_scope();
            }
            AbstractSyntaxNode::File => {
                for child in ast.children {
                    self.gen_statement(child);
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn gen_expression(&mut self, expression: TypedExpression) -> Operand {
        match expression {
            TypedExpression::Literal { value, .. } => {
                let dest = self.new_reg();
                self.emit(IrInstruction::LoadConst { dest, value });
                Operand::Value(dest)
            }
            TypedExpression::Variable { name, .. } => {
                let slot = self.resolve_var_addr(&name);
                let dest = self.new_reg();
                self.emit(IrInstruction::StackLoad { dest, slot });
                Operand::Value(dest)
            }
            TypedExpression::BinaryOp {
                left, op, right, ..
            } => {
                let left_op = self.gen_expression(*left);
                let right_op = self.gen_expression(*right);
                let dest = self.new_reg();
                self.emit(IrInstruction::BinaryOp {
                    dest,
                    left: left_op,
                    op,
                    right: right_op,
                });
                Operand::Value(dest)
            }
            TypedExpression::FunctionCall {
                name, arguments, ..
            } => {
                let args: Vec<_> = arguments
                    .into_iter()
                    .map(|arg| self.gen_expression(arg))
                    .collect();
                let dest = self.new_reg();
                let block = self.functions[&name];
                self.emit(IrInstruction::Call {
                    dest,
                    block,
                    arguments: args,
                });
                Operand::Value(dest)
            }
            TypedExpression::Assign { name, value, .. } => {
                let slot = self.resolve_var_addr(&name);
                let value_op = self.gen_expression(*value);

                self.emit(IrInstruction::StackStore {
                    slot,
                    value: value_op,
                });
                Operand::Void
            }
            TypedExpression::Increment {
                expression,
                postfix,
                ..
            } => self.update_value(*expression, postfix, BinaryOperator::Plus),
            TypedExpression::Decrement {
                expression,
                postfix,
                ..
            } => self.update_value(*expression, postfix, BinaryOperator::Minus),
            TypedExpression::Negate { expression, .. } => {
                let operand = self.gen_expression(*expression);
                let dest = self.new_reg();
                let zero_const = Operand::Constant("0".to_string());
                self.emit(IrInstruction::BinaryOp {
                    dest,
                    left: zero_const,
                    op: BinaryOperator::Minus,
                    right: operand,
                });
                Operand::Value(dest)
            }
            TypedExpression::Not { expression, .. } => {
                let operand = self.gen_expression(*expression);
                let dest = self.new_reg();
                let false_const = Operand::Constant("false".to_string());
                self.emit(IrInstruction::BinaryOp {
                    dest,
                    left: operand,
                    op: BinaryOperator::Equal,
                    right: false_const,
                });
                Operand::Value(dest)
            }
            TypedExpression::MethodCall {
                object,
                name,
                arguments,
                ..
            } => {
                let Type::Class(class_name) = object.get_type() else {
                    unreachable!()
                };
                let object_op = self.gen_expression(*object);

                let mut args = vec![object_op];
                args.extend(arguments.into_iter().map(|arg| self.gen_expression(arg)));

                let block = self.classes[&class_name].methods[&name];
                let dest = self.new_reg();
                self.emit(IrInstruction::Call {
                    dest,
                    block,
                    arguments: args,
                });

                Operand::Value(dest)
            }
            TypedExpression::AssignField {
                object,
                name,
                value,
                ..
            } => {
                let offset = self.resolve_field_offset(&object.get_type(), &name);
                let object_op = self.gen_expression(*object);
                let value_op = self.gen_expression(*value);

                self.emit(IrInstruction::PutField {
                    object: object_op,
                    offset,
                    value: value_op,
                });
                Operand::Void
            }
            TypedExpression::New { class_name, .. } => {
                let dest = self.new_reg();
                self.emit(IrInstruction::AllocObject {
                    dest,
                    class_name: class_name.clone(),
                });
                let object_op = Operand::Value(dest);

                let class_info = &self.classes[&class_name];
                let mut fields: Vec<_> = class_info
                    .fields
                    .values()
                    .filter_map(|(expr, off)| expr.as_ref().map(|e| (e.clone(), *off)))
                    .collect();

                fields.sort_by_key(|(_, off)| *off);

                for (expr, offset) in fields {
                    let val = self.gen_expression(expr);
                    self.emit(IrInstruction::PutField {
                        object: object_op.clone(),
                        offset,
                        value: val,
                    });
                }
                object_op
            }
            TypedExpression::Field { object, name, .. } => {
                let offset = self.resolve_field_offset(&object.get_type(), &name);
                let object_op = self.gen_expression(*object);
                let dest = self.new_reg();
                self.emit(IrInstruction::GetField {
                    dest,
                    object: object_op,
                    offset,
                });
                Operand::Value(dest)
            }
            TypedExpression::This { .. } => Operand::Value(
                self.this_register
                    .expect("Usage of 'this' outside of method"),
            ),
        }
    }

    fn update_value(
        &mut self,
        expression: TypedExpression,
        postfix: bool,
        op: BinaryOperator,
    ) -> Operand {
        match expression {
            TypedExpression::Variable { name, .. } => {
                let slot = self.resolve_var_addr(&name);
                let old_val_reg = self.new_reg();
                self.emit(IrInstruction::StackLoad {
                    dest: old_val_reg,
                    slot,
                });

                let new_val_reg = self.new_reg();
                let one_const = Operand::Constant("1".to_string());
                self.emit(IrInstruction::BinaryOp {
                    dest: new_val_reg,
                    left: Operand::Value(old_val_reg),
                    op,
                    right: one_const,
                });
                self.emit(IrInstruction::StackStore {
                    slot,
                    value: Operand::Value(new_val_reg),
                });

                if postfix {
                    Operand::Value(old_val_reg)
                } else {
                    Operand::Value(new_val_reg)
                }
            }
            TypedExpression::Field { object, name, .. } => {
                let typ = object.get_type();
                let object = self.gen_expression(*object);
                let Type::Class(class_name) = typ else {
                    unreachable!()
                };
                let offset = self.classes[&class_name].fields[&name].1;

                let old_val_reg = self.new_reg();
                self.emit(IrInstruction::GetField {
                    dest: old_val_reg,
                    object: object.clone(),
                    offset,
                });

                let new_val_reg = self.new_reg();
                let one_const = Operand::Constant("1".to_string());
                self.emit(IrInstruction::BinaryOp {
                    dest: new_val_reg,
                    left: Operand::Value(old_val_reg),
                    op,
                    right: one_const,
                });
                self.emit(IrInstruction::PutField {
                    object,
                    offset,
                    value: Operand::Value(new_val_reg),
                });

                if postfix {
                    Operand::Value(old_val_reg)
                } else {
                    Operand::Value(new_val_reg)
                }
            }
            _ => unreachable!(),
        }
    }
}

impl IrContext {
    pub fn scan_signatures(&mut self, ast: &TypedAST) {
        match &ast.node {
            AbstractSyntaxNode::File => {
                for child in &ast.children {
                    self.scan_signatures(child);
                }
            }
            AbstractSyntaxNode::Class { name } => {
                let mut class_info = ClassInfo::new();
                let mut field_counter = 0;

                for child in &ast.children {
                    match &child.node {
                        AbstractSyntaxNode::Callable { name, .. } => {
                            let block_id = self.create_block();
                            class_info.methods.insert(name.clone(), block_id);

                            if name == "Main" {
                                self.main_block = Some(block_id);
                            }
                        }
                        AbstractSyntaxNode::Declaration {
                            name, expression, ..
                        } => {
                            class_info
                                .fields
                                .insert(name.clone(), (expression.clone(), field_counter));
                            field_counter += 1;
                        }
                        _ => {}
                    }
                }
                self.classes.insert(name.clone(), class_info);
            }
            AbstractSyntaxNode::Callable { name, .. } => {
                let block_id = self.create_block();
                self.functions.insert(name.clone(), block_id);
                if name == "Main" {
                    self.main_block = Some(block_id);
                }
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
pub struct ControlFlowGraph {
    pub blocks: Vec<BasicBlock>,
    pub entry_block: BlockId,
}

pub fn compile(ast: TypedAST) -> ControlFlowGraph {
    let mut ctx = IrContext::new();
    ctx.scan_signatures(&ast);
    let entry_block = ctx.main_block.unwrap();
    ctx.gen_statement(ast);
    if !ctx.is_current_terminated() {
        ctx.emit_terminator(Terminator::Return(None));
    }
    ControlFlowGraph {
        blocks: ctx.blocks,
        entry_block,
    }
}
