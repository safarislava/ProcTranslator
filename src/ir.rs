use std::collections::HashMap;
use crate::common::{TypedAST, TypedExpression, ASN, Type};
use crate::expression::BinaryOperator;

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
    Call { dest: Register, block: BlockId, arguments: Vec<Operand> },
    LoadParam { dest: Register, index: usize },

    StackAlloc { slot: StackSlot },
    StackStore { slot: StackSlot, value: Operand },
    StackLoad { dest: Register, slot: StackSlot },

    GetField { dest: Register, object: Operand, offset: usize },
    PutField { object: Operand, offset: usize, value: Operand },
    AllocObject { dest: Register, class_name: String },
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

struct ClassInfo {
    pub fields: HashMap<String, (Option<TypedExpression>, usize)>,
    pub methods: HashMap<String, BlockId>,
}

impl ClassInfo {
    pub fn new() -> Self {
        ClassInfo { fields: HashMap::new(), methods: HashMap::new() }
    }
}

struct IrContext {
    pub blocks: Vec<BasicBlock>,
    current_block: Option<BlockId>,
    reg_counter: usize,
    slot_counter: usize,
    scopes: Vec<HashMap<String, StackSlot>>,
    functions: HashMap<String, BlockId>,
    classes: HashMap<String, ClassInfo>,
    current_class: Option<String>,
    this_register: Option<Register>,
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
            classes: HashMap::new(),
            current_class: None,
            this_register: None,
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

    pub fn resolve_var_addr(&self, name: &str) -> StackSlot {
        for scope in self.scopes.iter().rev() {
            if let Some(&slot) = scope.get(name) {
                return slot;
            }
        }
        unreachable!()
    }

    pub fn gen_statement(&mut self, ast: TypedAST) {
        match ast.node {
            ASN::If { condition } => {
                let condition = self.gen_expression(condition);
                let true_block = self.create_block();
                let false_block = self.create_block();
                let merge_block = self.create_block();
                self.emit_terminator(Terminator::Branch { condition, true_block, false_block });

                self.set_current_block(true_block);
                self.enter_scope();
                let mut else_node: Option<TypedAST> = None;
                self.set_current_block(true_block);
                for child in ast.children {
                    if let ASN::Else = child.node {
                        else_node = Some(child);
                    } else {
                        self.gen_statement(child);
                    }
                }
                self.exit_scope();
                self.emit_terminator(Terminator::Jump(merge_block));

                self.set_current_block(false_block);
                if let Some(node) = else_node {
                    self.enter_scope();
                    for child in node.children {
                        self.gen_statement(child);
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
                let condition = self.gen_expression(condition);
                self.emit_terminator(Terminator::Branch { condition, true_block, false_block });

                self.set_current_block(true_block);
                self.enter_scope();
                for child in ast.children {
                    self.gen_statement(child);
                }
                self.exit_scope();
                self.emit_terminator(Terminator::Jump(condition_block));

                self.set_current_block(false_block);
            }
            ASN::Expression { expression } => {
                self.gen_expression(expression);
            }
            ASN::Declaration { name, expression, .. } => {
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
            ASN::Callable { name, arguments, .. } => {
                let block_id = match &self.current_class {
                    Some(current_class) => self.classes[current_class].methods[&name],
                    None => self.functions[&name],
                };

                self.set_current_block(block_id);
                self.enter_scope();

                let mut param_offset = 0;
                if self.current_class.is_some() {
                    let reg = self.new_reg();
                    self.emit(IrInstruction::LoadParam { dest: reg, index: 0 });
                    self.this_register = Some(reg);
                    param_offset = 1;
                }

                for (i, arg) in arguments.into_iter().enumerate() {
                    let slot = self.declare_var(arg.name);
                    let reg = self.new_reg();
                    self.emit(IrInstruction::LoadParam { dest: reg, index: i + param_offset });
                    self.emit(IrInstruction::StackStore { slot, value: Operand::Value(reg) });
                }

                for child in ast.children {
                    self.gen_statement(child);
                }

                self.exit_scope();
                self.this_register = None;
            }
            ASN::Class { name } => {

            }
            ASN::Return { value } => {
                todo!()
            }
            ASN::Break => {
                todo!()
            }
            ASN::Continue => {
                todo!()
            }
            ASN::Scope => {
                self.enter_scope();
                for child in ast.children {
                    self.gen_statement(child);
                }
                self.exit_scope();
            }
            ASN::File => {}
            _ => unreachable!()
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
            TypedExpression::BinaryOp { left, op, right, .. } => {
                let left = self.gen_expression(*left);
                let right = self.gen_expression(*right);
                let dest = self.new_reg();
                self.emit(IrInstruction::BinaryOp { dest, left, op, right });
                Operand::Value(dest)
            }
            TypedExpression::FunctionCall { name, arguments, .. } => {
                let mut args = Vec::new();
                for arg in arguments {
                    args.push(self.gen_expression(arg));
                }
                let dest = self.new_reg();
                let block = self.functions[&name];
                self.emit(IrInstruction::Call { dest, block, arguments: args });
                Operand::Value(dest)
            }
            TypedExpression::Assign { name, value, .. } => {
                let slot = self.resolve_var_addr(&name);
                let value = self.gen_expression(*value);

                self.emit(IrInstruction::StackStore { slot, value });
                Operand::Void
            }
            TypedExpression::Increment { expression, postfix, .. } => {
                self.update_value(*expression, postfix, BinaryOperator::Plus)
            }
            TypedExpression::Decrement { expression, postfix, .. } => {
                self.update_value(*expression, postfix, BinaryOperator::Minus)
            }
            TypedExpression::Negate { expression, .. } => {
                let operand = self.gen_expression(*expression);
                let dest = self.new_reg();
                let zero_const = Operand::Constant("0".to_string());
                self.emit(IrInstruction::BinaryOp {
                    dest, left: zero_const, op: BinaryOperator::Minus, right: operand });
                Operand::Value(dest)
            }
            TypedExpression::Not { expression, .. } => {
                let operand = self.gen_expression(*expression);
                let dest = self.new_reg();
                let false_const = Operand::Constant("false".to_string());
                self.emit(IrInstruction::BinaryOp {
                    dest, left: operand, op: BinaryOperator::Equal, right: false_const});
                Operand::Value(dest)
            }
            TypedExpression::MethodCall { object, name, arguments, typ } => {
                let object = self.gen_expression(*object);
                let Type::Class(class_name) = typ else { unreachable!() };

                let mut args = vec![object];
                for arg in arguments {
                    args.push(self.gen_expression(arg));
                }

                let block = self.classes[&class_name].methods[&name];
                let dest = self.new_reg();
                self.emit(IrInstruction::Call { dest, block, arguments: args });

                Operand::Value(dest)
            }
            TypedExpression::AssignField { object, name, value, typ } => {
                let object = self.gen_expression(*object);
                let Type::Class(class_name) = typ else { unreachable!() };
                let offset = self.classes[&class_name].fields[&name].1;
                let value = self.gen_expression(*value);

                self.emit(IrInstruction::PutField {object, offset, value});
                Operand::Void
            }
            TypedExpression::New { class_name, ..} => {
                let dest = self.new_reg();
                self.emit(IrInstruction::AllocObject { dest, class_name: class_name.clone() });
                let object = Operand::Value(dest);

                let fields: Vec<_> = self.classes[&class_name].fields.values()
                    .filter_map(|(e, o)| Some((e.clone()?, *o))).collect();

                for (expression, offset) in fields {
                    let value = self.gen_expression(expression);
                    self.emit(IrInstruction::PutField { object: object.clone(), offset, value });
                }
                object
            }
            TypedExpression::Field { object, name, typ } => {
                let object = self.gen_expression(*object);
                let Type::Class(class_name) = typ else { unreachable!() };
                let offset = self.classes[&class_name].fields[&name].1;

                let dest = self.new_reg();
                self.emit(IrInstruction::GetField { dest, object, offset });

                Operand::Value(dest)
            }
            TypedExpression::This { .. } => {
                if let Some(reg) = self.this_register {
                    Operand::Value(reg)
                } else {
                    unreachable!()
                }
            }
        }
    }

    fn update_value(&mut self, expression: TypedExpression, postfix: bool, op: BinaryOperator) -> Operand {
        match expression {
            TypedExpression::Variable { name, .. } => {
                let slot = self.resolve_var_addr(&name);
                let old_val_reg = self.new_reg();
                self.emit(IrInstruction::StackLoad { dest: old_val_reg, slot });

                let new_val_reg = self.new_reg();
                let one_const = Operand::Constant("1".to_string());
                self.emit(IrInstruction::BinaryOp {
                    dest: new_val_reg, left: Operand::Value(old_val_reg), op, right: one_const, });
                self.emit(IrInstruction::StackStore { slot, value: Operand::Value(new_val_reg) });

                if postfix {
                    Operand::Value(old_val_reg)
                } else {
                    Operand::Value(new_val_reg)
                }
            }
            TypedExpression::Field { object, name, typ } => {
                let object = self.gen_expression(*object);
                let Type::Class(class_name) = typ else { unreachable!() };
                let offset = self.classes[&class_name].fields[&name].1;

                let old_val_reg = self.new_reg();
                self.emit(IrInstruction::GetField { dest: old_val_reg, object: object.clone(), offset });

                let new_val_reg = self.new_reg();
                let one_const = Operand::Constant("1".to_string());
                self.emit(IrInstruction::BinaryOp {
                    dest: new_val_reg, left: Operand::Value(old_val_reg), op, right: one_const, });
                self.emit(IrInstruction::PutField {object,  offset, value: Operand::Value(new_val_reg)});

                if postfix {
                    Operand::Value(old_val_reg)
                } else {
                    Operand::Value(new_val_reg)
                }
            }
            _ => unreachable!()
        }
    }
}

impl IrContext {
    pub fn scan_signatures(&mut self, ast: &TypedAST) {
        match &ast.node {
            ASN::File => {
                for child in &ast.children {
                    self.scan_signatures(child);
                }
            }
            ASN::Class { name } => {
                let mut class_info = ClassInfo::new();
                let mut field_counter = 0;

                for child in &ast.children {
                    match &child.node {
                        ASN::Callable { name, .. } => {
                            class_info.methods.insert(name.clone(), self.create_block());
                        }
                        ASN::Declaration { name, expression, .. } => {
                            class_info.fields.insert(name.clone(), (expression.clone(), field_counter));
                            field_counter += 1;
                        }
                        _ => {}
                    }
                }
                self.classes.insert(name.clone(), class_info);
            }
            ASN::Callable { name, .. } => {
                let block_id = self.create_block();
                self.functions.insert(name.clone(), block_id);
            }
            _ => {}
        }
    }
}

pub fn compile(ast: TypedAST) -> CFG {
    let mut ctx = IrContext::new();
    ctx.scan_signatures(&ast);
    ctx.gen_statement(ast);
    if !ctx.is_current_terminated() {
        ctx.emit_terminator(Terminator::Return(None));
    }
    CFG { blocks: ctx.blocks, entry_block: 0 }
}
