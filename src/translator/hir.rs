use crate::translator::common::{
    AbstractSyntaxNode, Type, TypedAbstractSyntaxTree, TypedExpression,
};
use crate::translator::expression::ExpressionBinaryOperator;
use std::collections::HashMap;
use std::iter;

pub type GlobalId = usize;

pub type BlockId = usize;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirRegister(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StackSlot(pub u64);

#[derive(Debug, Clone)]
pub enum HirOperand {
    Value(HirRegister),
    Link(HirRegister),
    Constant(String),
    Void,
}

#[derive(Debug, Clone)]
pub enum HirInstruction {
    BinaryOperator {
        destination: HirOperand,
        left: HirOperand,
        operator: ExpressionBinaryOperator,
        right: HirOperand,
    },
    Call {
        destination: HirOperand,
        block: BlockId,
        arguments: Vec<HirOperand>,
    },
    CallPrologue,

    LoadParameter {
        destination: HirOperand,
        index: usize,
    },

    LoadGlobal {
        destination: HirOperand,
        id: GlobalId,
    },
    StoreGlobal {
        id: GlobalId,
        value: HirOperand,
    },

    AllocateStack {
        slot: StackSlot,
    },
    LoadStack {
        destination: HirOperand,
        slot: StackSlot,
    },
    StoreStack {
        slot: StackSlot,
        value: HirOperand,
    },

    LoadField {
        destination: HirOperand,
        object: HirOperand,
        offset: usize,
    },
    StoreField {
        object: HirOperand,
        offset: usize,
        value: HirOperand,
    },
    AllocateObject {
        destination: HirOperand,
        class_name: String,
    },
}

#[derive(Debug, Clone)]
pub enum HirTerminator {
    Jump(BlockId),
    Branch {
        condition: HirOperand,
        true_block: BlockId,
        false_block: BlockId,
    },
    Return(Option<HirOperand>),
}

#[derive(Debug, Clone)]
pub struct HirBlock {
    pub id: BlockId,
    pub instructions: Vec<HirInstruction>,
    pub terminator: Option<HirTerminator>,
}

impl HirBlock {
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            instructions: Vec::new(),
            terminator: None,
        }
    }
}

#[derive(Debug)]
pub struct ClassInfo {
    pub fields: HashMap<String, (Option<TypedExpression>, usize)>,
    pub methods: HashMap<String, BlockId>,
}

impl ClassInfo {
    pub fn new(
        fields: HashMap<String, (Option<TypedExpression>, usize)>,
        methods: HashMap<String, BlockId>,
    ) -> Self {
        ClassInfo { fields, methods }
    }
}

impl Default for ClassInfo {
    fn default() -> Self {
        Self::new(HashMap::new(), HashMap::new())
    }
}

struct HirContext {
    pub blocks: Vec<HirBlock>,
    current_block: Option<BlockId>,
    register_counter: u64,
    slot_counter: u64,
    global_counter: usize,
    scopes: Vec<HashMap<String, StackSlot>>,
    loop_stack: Vec<(BlockId, BlockId)>,
    functions: HashMap<String, BlockId>,
    classes: HashMap<String, ClassInfo>,
    current_class: Option<String>,
    this_register: Option<HirOperand>,
    main_block: Option<BlockId>,
    interrupt_block: [BlockId; 8],
    globals: HashMap<String, GlobalId>,
}

impl HirContext {
    fn new() -> Self {
        HirContext {
            register_counter: 0,
            slot_counter: 0,
            global_counter: 0,
            blocks: vec![],
            current_block: None,
            scopes: vec![HashMap::new()],
            loop_stack: Vec::new(),
            functions: HashMap::new(),
            classes: HashMap::new(),
            current_class: None,
            this_register: None,
            main_block: None,
            interrupt_block: [0; 8],
            globals: HashMap::new(),
        }
    }

    fn new_global_id(&mut self) -> GlobalId {
        let id = self.global_counter;
        self.global_counter += 1;
        id
    }

    fn new_register(&mut self) -> HirRegister {
        let id = self.register_counter;
        self.register_counter += 1;
        HirRegister(id)
    }

    fn new_slot(&mut self) -> StackSlot {
        let id = self.slot_counter;
        self.slot_counter += 1;
        StackSlot(id)
    }

    fn create_block(&mut self) -> BlockId {
        let id = self.blocks.len();
        self.blocks.push(HirBlock::new(id));
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

    fn emit(&mut self, instruction: HirInstruction) {
        if let Some(block_id) = self.current_block
            && self.blocks[block_id].terminator.is_none()
        {
            self.blocks[block_id].instructions.push(instruction);
        }
    }

    fn emit_terminator(&mut self, term: HirTerminator) {
        if let Some(source_block) = self.current_block {
            if self.blocks[source_block].terminator.is_some() {
                return;
            }
            self.blocks[source_block].terminator = Some(term);
            self.current_block = None;
        }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_variable(&mut self, name: String) -> StackSlot {
        let slot = self.new_slot();
        self.emit(HirInstruction::AllocateStack { slot });

        let current_scope = self.scopes.last_mut().expect("No scope active");
        current_scope.insert(name, slot);
        slot
    }

    fn resolve_variable_address(&self, name: &str) -> StackSlot {
        for scope in self.scopes.iter().rev() {
            if let Some(&slot) = scope.get(name) {
                return slot;
            }
        }
        panic!("Variable {} not found", name);
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
}

impl HirContext {
    fn generate_statement(&mut self, ast: TypedAbstractSyntaxTree) {
        match ast.node {
            AbstractSyntaxNode::If { condition } => {
                let condition = self.generate_expression(condition);
                let true_block = self.create_block();
                let false_block = self.create_block();
                self.emit_terminator(HirTerminator::Branch {
                    condition,
                    true_block,
                    false_block,
                });

                self.set_current_block(true_block);
                self.enter_scope();

                let mut has_else = false;
                let mut else_if_node: Option<TypedAbstractSyntaxTree> = None;

                for child in &ast.children {
                    if let AbstractSyntaxNode::Else = child.node {
                        has_else = true;
                        if child.children.len() == 1
                            && let AbstractSyntaxNode::If { .. } = child.children[0].node
                        {
                            else_if_node = Some(child.children[0].clone());
                        }
                    } else {
                        self.generate_statement(child.clone());
                    }
                }
                self.exit_scope();

                let true_terminated = self.is_current_terminated();
                if !true_terminated {
                    if else_if_node.is_some() || has_else {
                        self.emit_terminator(HirTerminator::Jump(false_block));
                    } else {
                        let merge_block = self.create_block();
                        self.emit_terminator(HirTerminator::Jump(merge_block));
                        self.set_current_block(merge_block);
                        return;
                    }
                }

                self.set_current_block(false_block);
                if let Some(else_if) = else_if_node {
                    self.generate_statement(else_if);
                } else if has_else {
                    self.enter_scope();
                    for child in &ast.children {
                        if let AbstractSyntaxNode::Else = child.node {
                            for grandchild in &child.children {
                                self.generate_statement(grandchild.clone());
                            }
                        }
                    }
                    self.exit_scope();
                } else if !true_terminated {
                    let merge_block = self.create_block();
                    self.emit_terminator(HirTerminator::Jump(merge_block));
                    self.set_current_block(merge_block);
                    return;
                }

                let false_terminated = self.is_current_terminated();
                if !false_terminated {
                    let merge_block = self.create_block();
                    self.emit_terminator(HirTerminator::Jump(merge_block));
                    self.set_current_block(merge_block);
                }
            }
            AbstractSyntaxNode::While { condition } => {
                let condition_block = self.create_block();
                let true_block = self.create_block();
                let false_block = self.create_block();
                self.emit_terminator(HirTerminator::Jump(condition_block));

                self.set_current_block(condition_block);
                let condition = self.generate_expression(condition);
                self.emit_terminator(HirTerminator::Branch {
                    condition,
                    true_block,
                    false_block,
                });

                self.set_current_block(true_block);
                self.enter_scope();
                self.loop_stack.push((condition_block, false_block));
                for child in ast.children {
                    self.generate_statement(child);
                }
                self.loop_stack.pop();
                self.exit_scope();
                self.emit_terminator(HirTerminator::Jump(condition_block));

                self.set_current_block(false_block);
            }
            AbstractSyntaxNode::Expression { expression } => {
                self.generate_expression(expression);
            }
            AbstractSyntaxNode::Declaration {
                name, expression, ..
            } => {
                if self.scopes.len() > 1 {
                    let slot = self.declare_variable(name);
                    let value = if let Some(expression) = expression {
                        self.generate_expression(expression)
                    } else {
                        HirOperand::Void
                    };

                    if !matches!(value, HirOperand::Void) {
                        self.emit(HirInstruction::StoreStack { slot, value });
                    }
                } else {
                    let global_id = self.new_global_id();
                    self.globals.insert(name.clone(), global_id);

                    self.set_current_block(self.main_block.unwrap());
                    if let Some(expression) = expression {
                        let value = self.generate_expression(expression);
                        self.emit(HirInstruction::StoreGlobal {
                            id: global_id,
                            value,
                        });
                    } else {
                        self.emit(HirInstruction::StoreGlobal {
                            id: global_id,
                            value: HirOperand::Constant("0".to_string()),
                        });
                    }
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
                self.emit(HirInstruction::CallPrologue);

                let mut parameter_offset = 0;
                if self.current_class.is_some() {
                    let register = HirOperand::Link(self.new_register());
                    self.emit(HirInstruction::LoadParameter {
                        destination: register.clone(),
                        index: 0,
                    });
                    self.this_register = Some(register);
                    parameter_offset = 1;
                }

                for (i, argument) in arguments.into_iter().enumerate() {
                    let slot = self.declare_variable(argument.name);
                    let register = self.new_register();
                    let destination = match argument.typ {
                        Type::Class(_) => HirOperand::Link(register),
                        _ => HirOperand::Value(register),
                    };
                    self.emit(HirInstruction::LoadParameter {
                        destination,
                        index: i + parameter_offset,
                    });
                    self.emit(HirInstruction::StoreStack {
                        slot,
                        value: HirOperand::Value(register),
                    });
                }

                for child in ast.children {
                    self.generate_statement(child);
                }

                self.exit_scope();
                self.this_register = None;
            }
            AbstractSyntaxNode::Class { name } => {
                self.current_class = Some(name);
                for child in ast.children {
                    self.generate_statement(child);
                }
                self.current_class = None;
            }
            AbstractSyntaxNode::Return { value } => {
                let operand = value.map(|value| self.generate_expression(value));
                self.emit_terminator(HirTerminator::Return(operand))
            }
            AbstractSyntaxNode::Break => {
                if let Some((_, break_target)) = self.loop_stack.last() {
                    self.emit_terminator(HirTerminator::Jump(*break_target));
                } else {
                    unreachable!();
                }
            }
            AbstractSyntaxNode::Continue => {
                if let Some((continue_target, _)) = self.loop_stack.last() {
                    self.emit_terminator(HirTerminator::Jump(*continue_target));
                } else {
                    unreachable!();
                }
            }
            AbstractSyntaxNode::Scope => {
                self.enter_scope();
                for child in ast.children {
                    self.generate_statement(child);
                }
                self.exit_scope();
            }
            AbstractSyntaxNode::File => {
                for child in ast.children {
                    self.generate_statement(child);
                }
            }
            AbstractSyntaxNode::ElseIf { .. }
            | AbstractSyntaxNode::Else
            | AbstractSyntaxNode::For { .. } => unreachable!(),
        }
    }

    fn generate_expression(&mut self, expression: TypedExpression) -> HirOperand {
        let expression_type = expression.get_type();
        match expression {
            TypedExpression::Literal { value, .. } => HirOperand::Constant(value),
            TypedExpression::Variable { name, .. } => {
                let destination = self.new_register();
                let destination = match expression_type {
                    Type::Class(_) => HirOperand::Link(destination),
                    _ => HirOperand::Value(destination),
                };

                let is_local = self.scopes.iter().any(|scope| scope.contains_key(&name));
                if is_local {
                    let slot = self.resolve_variable_address(&name);
                    self.emit(HirInstruction::LoadStack {
                        destination: destination.clone(),
                        slot,
                    });
                    destination
                } else if let Some(&id) = self.globals.get(&name) {
                    self.emit(HirInstruction::LoadGlobal {
                        destination: destination.clone(),
                        id,
                    });
                    destination
                } else {
                    panic!("Variable {} not found", name);
                }
            }
            TypedExpression::BinaryOperator {
                left,
                operator,
                right,
                ..
            } => {
                let left = self.generate_expression(*left);
                let right = self.generate_expression(*right);
                let destination = self.new_register();
                let destination = match expression_type {
                    Type::Class(_) => HirOperand::Link(destination),
                    _ => HirOperand::Value(destination),
                };

                self.emit(HirInstruction::BinaryOperator {
                    destination: destination.clone(),
                    left,
                    operator,
                    right,
                });
                destination
            }
            TypedExpression::FunctionCall {
                name, arguments, ..
            } => {
                let arguments: Vec<_> = arguments
                    .into_iter()
                    .map(|arg| self.generate_expression(arg))
                    .collect();
                let destination = self.new_register();
                let destination = match expression_type {
                    Type::Class(_) => HirOperand::Link(destination),
                    _ => HirOperand::Value(destination),
                };

                let block = self.functions[&name];
                self.emit(HirInstruction::Call {
                    destination: destination.clone(),
                    block,
                    arguments,
                });
                destination
            }
            TypedExpression::Assign { name, value, .. } => {
                let value = self.generate_expression(*value);

                let is_local = self.scopes.iter().any(|scope| scope.contains_key(&name));
                if is_local {
                    let slot = self.resolve_variable_address(&name);
                    self.emit(HirInstruction::StoreStack { slot, value });
                } else if let Some(&id) = self.globals.get(&name) {
                    self.emit(HirInstruction::StoreGlobal { id, value });
                } else {
                    panic!("Variable {} not found", name);
                }
                HirOperand::Void
            }
            TypedExpression::Increment {
                expression,
                postfix,
                ..
            } => self.generate_increment_or_decrement(
                *expression,
                postfix,
                ExpressionBinaryOperator::Add,
            ),
            TypedExpression::Decrement {
                expression,
                postfix,
                ..
            } => self.generate_increment_or_decrement(
                *expression,
                postfix,
                ExpressionBinaryOperator::Sub,
            ),
            TypedExpression::Negate { expression, .. } => {
                let operand = self.generate_expression(*expression);
                let destination = self.new_register();
                let destination = match expression_type {
                    Type::Class(_) => HirOperand::Link(destination),
                    _ => HirOperand::Value(destination),
                };
                let zero_const = HirOperand::Constant("0".to_string());
                self.emit(HirInstruction::BinaryOperator {
                    destination: destination.clone(),
                    left: zero_const,
                    operator: ExpressionBinaryOperator::Sub,
                    right: operand,
                });
                destination
            }
            TypedExpression::Not { expression, .. } => {
                let operand = self.generate_expression(*expression);
                let destination = self.new_register();
                let destination = match expression_type {
                    Type::Class(_) => HirOperand::Link(destination),
                    _ => HirOperand::Value(destination),
                };
                let false_const = HirOperand::Constant("false".to_string());
                self.emit(HirInstruction::BinaryOperator {
                    destination: destination.clone(),
                    left: operand,
                    operator: ExpressionBinaryOperator::Equal,
                    right: false_const,
                });
                destination
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
                let object = self.generate_expression(*object);

                let arguments_iterator = arguments
                    .into_iter()
                    .map(|arg| self.generate_expression(arg));
                let combined_iterator = iter::once(object).chain(arguments_iterator);
                let arguments = combined_iterator.collect();

                let block = self.classes[&class_name].methods[&name];
                let destination = self.new_register();
                let destination = match expression_type {
                    Type::Class(_) => HirOperand::Link(destination),
                    _ => HirOperand::Value(destination),
                };

                self.emit(HirInstruction::Call {
                    destination: destination.clone(),
                    block,
                    arguments,
                });
                destination
            }
            TypedExpression::AssignField {
                object,
                name,
                value,
                ..
            } => {
                let offset = self.resolve_field_offset(&object.get_type(), &name);
                let object = self.generate_expression(*object);
                let value = self.generate_expression(*value);

                self.emit(HirInstruction::StoreField {
                    object,
                    offset,
                    value,
                });
                HirOperand::Void
            }
            TypedExpression::New { class_name, .. } => {
                let object = HirOperand::Link(self.new_register());

                self.emit(HirInstruction::AllocateObject {
                    destination: object.clone(),
                    class_name: class_name.clone(),
                });

                let class_info = &self.classes[&class_name];
                let mut fields: Vec<_> = class_info
                    .fields
                    .values()
                    .filter_map(|(expr, off)| expr.as_ref().map(|e| (e.clone(), *off)))
                    .collect();
                fields.sort_by_key(|(_, off)| *off);

                for (expression, offset) in fields {
                    let value = self.generate_expression(expression);
                    self.emit(HirInstruction::StoreField {
                        object: object.clone(),
                        offset,
                        value,
                    });
                }
                object
            }
            TypedExpression::Field { object, name, .. } => {
                let offset = self.resolve_field_offset(&object.get_type(), &name);
                let object = self.generate_expression(*object);
                let destination = self.new_register();
                let destination = match expression_type {
                    Type::Class(_) => HirOperand::Link(destination),
                    _ => HirOperand::Value(destination),
                };
                self.emit(HirInstruction::LoadField {
                    destination: destination.clone(),
                    object,
                    offset,
                });
                destination
            }
            TypedExpression::This { .. } => self
                .this_register
                .clone()
                .expect("Usage of 'this' outside of method"),
        }
    }

    fn generate_increment_or_decrement(
        &mut self,
        expression: TypedExpression,
        postfix: bool,
        operator: ExpressionBinaryOperator,
    ) -> HirOperand {
        match expression {
            TypedExpression::Variable { name, .. } => {
                let slot = self.resolve_variable_address(&name);
                let old_value = HirOperand::Value(self.new_register());
                self.emit(HirInstruction::LoadStack {
                    destination: old_value.clone(),
                    slot,
                });

                let new_value = HirOperand::Value(self.new_register());
                let one_const = HirOperand::Constant("1".to_string());
                self.emit(HirInstruction::BinaryOperator {
                    destination: new_value.clone(),
                    left: old_value.clone(),
                    operator,
                    right: one_const,
                });
                self.emit(HirInstruction::StoreStack {
                    slot,
                    value: new_value.clone(),
                });

                if postfix { old_value } else { new_value }
            }
            TypedExpression::Field { object, name, .. } => {
                let typ = object.get_type();
                let object = self.generate_expression(*object);
                let Type::Class(class_name) = typ else {
                    unreachable!()
                };
                let offset = self.classes[&class_name].fields[&name].1;

                let old_value = HirOperand::Value(self.new_register());
                self.emit(HirInstruction::LoadField {
                    destination: old_value.clone(),
                    object: object.clone(),
                    offset,
                });

                let new_value = HirOperand::Value(self.new_register());
                let one_const = HirOperand::Constant("1".to_string());
                self.emit(HirInstruction::BinaryOperator {
                    destination: new_value.clone(),
                    left: old_value.clone(),
                    operator,
                    right: one_const,
                });
                self.emit(HirInstruction::StoreField {
                    object,
                    offset,
                    value: new_value.clone(),
                });

                if postfix { old_value } else { new_value }
            }
            _ => unreachable!(),
        }
    }
}

impl HirContext {
    fn scan_signatures(&mut self, ast: &TypedAbstractSyntaxTree) {
        match &ast.node {
            AbstractSyntaxNode::File => {
                for child in &ast.children {
                    self.scan_signatures(child);
                }
            }
            AbstractSyntaxNode::Class { name } => {
                let mut class_info = ClassInfo::default();
                let mut field_counter = 0;

                for child in &ast.children {
                    match &child.node {
                        AbstractSyntaxNode::Callable { name, .. } => {
                            let block_id = self.create_block();
                            class_info.methods.insert(name.clone(), block_id);
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
                match name.as_str() {
                    "Main" => self.main_block = Some(block_id),
                    s if s.starts_with("interrupt") => {
                        if let Ok(number) = s.replace("interrupt", "").parse::<u8>()
                            && number <= 7
                        {
                            self.interrupt_block[number as usize] = block_id;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
pub struct ControlFlowGraph {
    pub blocks: Vec<HirBlock>,
    pub entry_block: BlockId,
    pub interrupt_blocks: [BlockId; 8],
    pub classes: HashMap<String, ClassInfo>,
}

pub fn compile_hir(ast: TypedAbstractSyntaxTree) -> ControlFlowGraph {
    let mut context = HirContext::new();

    context.scan_signatures(&ast);
    context.generate_statement(ast);

    ControlFlowGraph {
        blocks: context.blocks,
        entry_block: context.main_block.unwrap(),
        interrupt_blocks: context.interrupt_block,
        classes: context.classes,
    }
}
