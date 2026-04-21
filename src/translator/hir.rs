use crate::isa::WordSize;
use crate::translator::analyzer::{TypedAbstractSyntaxNode, TypedAbstractSyntaxTree};
use crate::translator::common::{Type, TypedExpression};
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
    Constant(String, Type),
    Void,
    Variable(StackSlot),
    Parameter(u64),
    GlobalVariable(GlobalId),
}

#[derive(Debug, Clone)]
pub enum HirBinaryOperator {
    Assign,
    Or,
    And,
    BitwiseOr,
    BitwiseXor,
    BitwiseAnd,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Sub,
    Multiply,
    Divide,
    Remainder,
    LeftShift,
    RightShift,
    VectorAdd,
    VectorSub,
    VectorMultiply,
    VectorDivide,
    VectorRemainder,
    VectorAnd,
    VectorOr,
    VectorXor,
    VectorEqual,
    VectorNotEqual,
    VectorLess,
    VectorLessEqual,
    VectorGreater,
    VectorGreaterEqual,
}

fn translate_binary_operator(operator: ExpressionBinaryOperator) -> HirBinaryOperator {
    match operator {
        ExpressionBinaryOperator::Assign => HirBinaryOperator::Assign,
        ExpressionBinaryOperator::AssignAdd
        | ExpressionBinaryOperator::AssignSub
        | ExpressionBinaryOperator::AssignMul
        | ExpressionBinaryOperator::AssignDiv
        | ExpressionBinaryOperator::AssignAnd
        | ExpressionBinaryOperator::AssignOr
        | ExpressionBinaryOperator::AssignXor => unreachable!(),
        ExpressionBinaryOperator::Or => HirBinaryOperator::Or,
        ExpressionBinaryOperator::And => HirBinaryOperator::And,
        ExpressionBinaryOperator::Equal => HirBinaryOperator::Equal,
        ExpressionBinaryOperator::NotEqual => HirBinaryOperator::NotEqual,
        ExpressionBinaryOperator::Less => HirBinaryOperator::Less,
        ExpressionBinaryOperator::LessEqual => HirBinaryOperator::LessEqual,
        ExpressionBinaryOperator::Greater => HirBinaryOperator::Greater,
        ExpressionBinaryOperator::GreaterEqual => HirBinaryOperator::GreaterEqual,
        ExpressionBinaryOperator::Add => HirBinaryOperator::Add,
        ExpressionBinaryOperator::Sub => HirBinaryOperator::Sub,
        ExpressionBinaryOperator::Multiply => HirBinaryOperator::Multiply,
        ExpressionBinaryOperator::Divide => HirBinaryOperator::Divide,
        ExpressionBinaryOperator::Remainder => HirBinaryOperator::Remainder,
        ExpressionBinaryOperator::LeftShift => HirBinaryOperator::LeftShift,
        ExpressionBinaryOperator::RightShift => HirBinaryOperator::RightShift,
        ExpressionBinaryOperator::BitwiseOr => HirBinaryOperator::BitwiseOr,
        ExpressionBinaryOperator::BitwiseXor => HirBinaryOperator::BitwiseXor,
        ExpressionBinaryOperator::BitwiseAnd => HirBinaryOperator::BitwiseAnd,
    }
}

#[derive(Debug, Clone)]
pub enum HirInstruction {
    BinaryOperator {
        destination: HirOperand,
        left: HirOperand,
        operator: HirBinaryOperator,
        right: HirOperand,
        word_size: WordSize,
    },
    Not {
        destination: HirOperand,
        operand: HirOperand,
        word_size: WordSize,
    },
    Call {
        destination: HirOperand,
        block: BlockId,
        arguments: Vec<(HirOperand, WordSize)>,
    },
    CallPrologue,
    InterruptPrologue,

    LoadGlobal {
        destination: HirOperand,
        id: GlobalId,
        word_size: WordSize,
    },
    StoreGlobal {
        id: GlobalId,
        value: HirOperand,
        word_size: WordSize,
    },

    AllocateStack {
        slot: StackSlot,
    },
    LoadStack {
        destination: HirOperand,
        slot: StackSlot,
        word_size: WordSize,
    },
    StoreStack {
        slot: StackSlot,
        value: HirOperand,
        word_size: WordSize,
    },

    LoadField {
        destination: HirOperand,
        object: HirOperand,
        offset: u64,
        word_size: WordSize,
    },
    StoreField {
        object: HirOperand,
        offset: u64,
        value: HirOperand,
        word_size: WordSize,
    },
    LoadIndex {
        destination: HirOperand,
        array: HirOperand,
        index: HirOperand,
        word_size: WordSize,
    },
    LoadSlice {
        destination: HirOperand,
        array: HirOperand,
        start: HirOperand,
    },
    StoreSlice {
        target: HirOperand,
        start: HirOperand,
        value: HirOperand,
        size: u64,
        word_size: WordSize,
    },
    StoreIndex {
        array: HirOperand,
        index: HirOperand,
        value: HirOperand,
        word_size: WordSize,
    },
    AllocateObject {
        destination: HirOperand,
        size: u64,
    },
    AllocateArray {
        destination: HirOperand,
        size: u64,
    },

    Input {
        port: HirOperand,
        destination: HirOperand,
        word_size: WordSize,
    },
    Output {
        port: HirOperand,
        value: HirOperand,
        word_size: WordSize,
    },
    CopyConstantArray {
        destination: HirOperand,
        id: usize,
        word_size: WordSize,
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
    Return(Option<HirOperand>, WordSize),
    IntReturn,
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
    pub fields: HashMap<String, (Option<TypedExpression>, Type, u64)>,
    pub methods: HashMap<String, BlockId>,
}

impl ClassInfo {
    pub fn new(
        fields: HashMap<String, (Option<TypedExpression>, Type, u64)>,
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

enum LocalVariable {
    Variable(StackSlot),
    Parameter(u64),
}

struct HirContext {
    pub blocks: Vec<HirBlock>,
    current_block: Option<BlockId>,
    register_counter: u64,
    slot_counter: u64,
    global_counter: usize,
    scopes: Vec<HashMap<String, LocalVariable>>,
    loop_stack: Vec<(BlockId, BlockId)>,
    functions: HashMap<String, BlockId>,
    classes: HashMap<String, ClassInfo>,
    current_class: Option<String>,
    current_function: Option<String>,
    this_register: Option<HirOperand>,
    main_block: Option<BlockId>,
    interrupt_block: [BlockId; 8],
    globals: HashMap<String, GlobalId>,
    array_constants: Vec<(Vec<String>, Type)>,
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
            current_function: None,
            this_register: None,
            main_block: None,
            interrupt_block: [0; 8],
            globals: HashMap::new(),
            array_constants: vec![],
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
        current_scope.insert(name, LocalVariable::Variable(slot));
        slot
    }

    fn declare_parameter(&mut self, name: String, offset: u64) {
        let current_scope = self.scopes.last_mut().expect("No scope active");
        current_scope.insert(name, LocalVariable::Parameter(offset));
    }

    fn resolve_local_variable_address(&self, name: &str) -> &LocalVariable {
        for scope in self.scopes.iter().rev() {
            if let Some(variable) = scope.get(name) {
                return variable;
            }
        }
        panic!("Variable {} not found", name);
    }

    fn resolve_field_offset(&self, object_type: &Type, field_name: &str) -> u64 {
        if let Type::Class(class_name) = object_type {
            self.classes.get(class_name).unwrap().fields[field_name].2
        } else {
            unreachable!("Type is not a class")
        }
    }

    fn new_constant_array(&mut self, values: Vec<String>, elem_type: Type) -> usize {
        let id = self.array_constants.len();
        self.array_constants.push((values, elem_type));
        id
    }
}

impl HirContext {
    fn generate_statement(&mut self, ast: TypedAbstractSyntaxTree) {
        match ast.node {
            TypedAbstractSyntaxNode::If { condition } => {
                let condition = self.generate_expression(condition);
                let true_block = self.create_block();
                let false_block = self.create_block();

                self.emit_terminator(HirTerminator::Branch {
                    condition,
                    true_block,
                    false_block,
                });

                let else_node = ast
                    .children
                    .iter()
                    .find(|child| matches!(child.node, TypedAbstractSyntaxNode::Else));

                self.set_current_block(true_block);
                self.enter_scope();
                for child in &ast.children {
                    if !matches!(child.node, TypedAbstractSyntaxNode::Else) {
                        self.generate_statement(child.clone());
                    }
                }
                self.exit_scope();

                let mut optional_merge_block = if !self.is_current_terminated() {
                    let merge_block = self.create_block();
                    self.emit_terminator(HirTerminator::Jump(merge_block));
                    Some(merge_block)
                } else {
                    None
                };

                self.set_current_block(false_block);

                if let Some(else_branch) = else_node {
                    let is_else_if = else_branch.children.len() == 1
                        && matches!(
                            else_branch.children[0].node,
                            TypedAbstractSyntaxNode::If { .. }
                        );

                    if is_else_if {
                        self.generate_statement(else_branch.children[0].clone());
                    } else {
                        self.enter_scope();
                        for statement in &else_branch.children {
                            self.generate_statement(statement.clone());
                        }
                        self.exit_scope();
                    }
                }

                if !self.is_current_terminated() {
                    let final_merge_block = match optional_merge_block {
                        Some(block_id) => block_id,
                        None => self.create_block(),
                    };
                    self.emit_terminator(HirTerminator::Jump(final_merge_block));
                    optional_merge_block = Some(final_merge_block);
                }

                if let Some(merge_block) = optional_merge_block {
                    self.set_current_block(merge_block);
                }
            }
            TypedAbstractSyntaxNode::While { condition } => {
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
            TypedAbstractSyntaxNode::Expression { expression } => {
                self.generate_expression(expression);
            }
            TypedAbstractSyntaxNode::Declaration {
                name,
                expression,
                typ,
            } => {
                if self.current_class.is_some() && self.scopes.len() == 1 {
                    return;
                }

                let word_size = self.get_word_size(&typ);
                if self.scopes.len() > 1 {
                    let slot = self.declare_variable(name);
                    let value = if let Some(expression) = expression {
                        self.generate_expression(expression)
                    } else {
                        HirOperand::Void
                    };

                    if !matches!(value, HirOperand::Void) {
                        self.emit(HirInstruction::StoreStack {
                            slot,
                            value,
                            word_size,
                        });
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
                            word_size,
                        });
                    } else {
                        self.emit(HirInstruction::StoreGlobal {
                            id: global_id,
                            value: HirOperand::Constant("0".to_string(), typ),
                            word_size,
                        });
                    }
                }
            }
            TypedAbstractSyntaxNode::Callable {
                name, arguments, ..
            } => {
                if name == "in" || name == "out" {
                    return;
                }

                let block_id = match &self.current_class {
                    Some(current_class) => self.classes[current_class].methods[&name],
                    None => {
                        self.current_function = Some(name.clone());
                        self.functions[&name]
                    }
                };

                self.set_current_block(block_id);
                self.enter_scope();
                let is_interrupt = name.starts_with("interrupt");
                if is_interrupt {
                    self.emit(HirInstruction::InterruptPrologue);
                } else {
                    self.emit(HirInstruction::CallPrologue);
                }

                let mut parameter_offset = 2;
                if self.current_class.is_some() {
                    self.declare_parameter("this".to_string(), parameter_offset);
                    self.this_register = Some(HirOperand::Parameter(parameter_offset));
                    parameter_offset += 1;
                }
                for argument in arguments.iter() {
                    self.declare_parameter(argument.name.clone(), parameter_offset);
                    parameter_offset += 1;
                }

                for child in ast.children {
                    self.generate_statement(child);
                }

                self.exit_scope();
                self.this_register = None;
                self.current_function = None;
            }
            TypedAbstractSyntaxNode::Class { name } => {
                self.current_class = Some(name);
                for child in ast.children {
                    self.generate_statement(child);
                }
                self.current_class = None;
            }
            TypedAbstractSyntaxNode::Return { value } => {
                let is_interrupt = if let Some(function) = &self.current_function {
                    function.starts_with("interrupt")
                } else {
                    false
                };

                if is_interrupt {
                    self.emit_terminator(HirTerminator::IntReturn);
                } else {
                    let word_size = value
                        .as_ref()
                        .map(|v| self.get_word_size(&v.get_type()))
                        .unwrap_or(WordSize::Long);
                    let operand = value.map(|v| self.generate_expression(v));
                    self.emit_terminator(HirTerminator::Return(operand, word_size));
                }
            }
            TypedAbstractSyntaxNode::Break => {
                if let Some((_, break_target)) = self.loop_stack.last() {
                    self.emit_terminator(HirTerminator::Jump(*break_target));
                } else {
                    unreachable!();
                }
            }
            TypedAbstractSyntaxNode::Continue => {
                if let Some((continue_target, _)) = self.loop_stack.last() {
                    self.emit_terminator(HirTerminator::Jump(*continue_target));
                } else {
                    unreachable!();
                }
            }
            TypedAbstractSyntaxNode::Scope => {
                self.enter_scope();
                for child in ast.children {
                    self.generate_statement(child);
                }
                self.exit_scope();
            }
            TypedAbstractSyntaxNode::File => {
                for child in ast.children {
                    self.generate_statement(child);
                }
            }
            TypedAbstractSyntaxNode::ElseIf { .. }
            | TypedAbstractSyntaxNode::Else
            | TypedAbstractSyntaxNode::For { .. } => unreachable!(),
        }
    }

    fn generate_expression(&mut self, expression: TypedExpression) -> HirOperand {
        let expression_type = expression.get_type();
        match expression {
            TypedExpression::Literal { value, typ } => HirOperand::Constant(value, typ),
            TypedExpression::Variable { name, .. } => {
                let is_local = self.scopes.iter().any(|scope| scope.contains_key(&name));
                if is_local {
                    let variable = self.resolve_local_variable_address(&name);
                    match variable {
                        LocalVariable::Variable(slot) => HirOperand::Variable(*slot),
                        LocalVariable::Parameter(offset) => HirOperand::Parameter(*offset),
                    }
                } else if let Some(&id) = self.globals.get(&name) {
                    HirOperand::GlobalVariable(id)
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
                let typ = left.get_type();
                let word_size = self.get_word_size(&typ);
                let left = self.generate_expression(*left);
                let right = self.generate_expression(*right);
                let destination = self.new_register();
                let destination = match expression_type {
                    Type::Class(_) | Type::Array(_, _) => HirOperand::Link(destination),
                    _ => HirOperand::Value(destination),
                };

                if typ == Type::Array(Box::new(Type::Int), 4) {
                    let operator = match operator {
                        ExpressionBinaryOperator::Add => HirBinaryOperator::VectorAdd,
                        ExpressionBinaryOperator::Sub => HirBinaryOperator::VectorSub,
                        ExpressionBinaryOperator::Multiply => HirBinaryOperator::VectorMultiply,
                        ExpressionBinaryOperator::Divide => HirBinaryOperator::VectorDivide,
                        ExpressionBinaryOperator::Remainder => HirBinaryOperator::VectorRemainder,
                        ExpressionBinaryOperator::BitwiseAnd => HirBinaryOperator::VectorAnd,
                        ExpressionBinaryOperator::BitwiseOr => HirBinaryOperator::VectorOr,
                        ExpressionBinaryOperator::BitwiseXor => HirBinaryOperator::VectorXor,
                        ExpressionBinaryOperator::Equal => HirBinaryOperator::VectorEqual,
                        ExpressionBinaryOperator::NotEqual => HirBinaryOperator::VectorNotEqual,
                        ExpressionBinaryOperator::Less => HirBinaryOperator::VectorLess,
                        ExpressionBinaryOperator::LessEqual => HirBinaryOperator::VectorLessEqual,
                        ExpressionBinaryOperator::Greater => HirBinaryOperator::VectorGreater,
                        ExpressionBinaryOperator::GreaterEqual => {
                            HirBinaryOperator::VectorGreaterEqual
                        }
                        _ => unreachable!(),
                    };
                    self.emit(HirInstruction::BinaryOperator {
                        destination: destination.clone(),
                        left,
                        operator,
                        right,
                        word_size,
                    });
                    destination
                } else {
                    self.emit(HirInstruction::BinaryOperator {
                        destination: destination.clone(),
                        left,
                        operator: translate_binary_operator(operator),
                        right,
                        word_size,
                    });
                    destination
                }
            }
            TypedExpression::FunctionCall {
                name,
                arguments,
                typ,
            } => {
                let arguments: Vec<_> = arguments
                    .into_iter()
                    .map(|arg| {
                        (
                            self.generate_expression(arg.clone()),
                            self.get_word_size(&arg.get_type()),
                        )
                    })
                    .collect();
                let destination = self.new_register();
                let destination = match expression_type {
                    Type::Class(_) | Type::Array(_, _) => HirOperand::Link(destination),
                    _ => HirOperand::Value(destination),
                };

                if name == "iin" || name == "cin" {
                    self.emit(HirInstruction::Input {
                        destination: destination.clone(),
                        port: arguments[0].0.clone(),
                        word_size: self.get_word_size(&typ),
                    })
                } else if name == "iout" || name == "cout" {
                    self.emit(HirInstruction::Output {
                        port: arguments[0].0.clone(),
                        value: arguments[1].0.clone(),
                        word_size: arguments[1].1.clone(),
                    })
                } else {
                    let block = self.functions[&name];
                    self.emit(HirInstruction::Call {
                        destination: destination.clone(),
                        block,
                        arguments,
                    });
                }
                destination
            }
            TypedExpression::Assign { name, value, typ } => {
                let value = self.generate_expression(*value);
                let word_size = self.get_word_size(&typ);

                let is_local = self.scopes.iter().any(|scope| scope.contains_key(&name));
                if is_local {
                    let variable = self.resolve_local_variable_address(&name);
                    let slot = match variable {
                        LocalVariable::Variable(slot) => *slot,
                        LocalVariable::Parameter(_) => panic!("Can't change parameter"),
                    };
                    self.emit(HirInstruction::StoreStack {
                        slot,
                        value,
                        word_size,
                    });
                } else if let Some(&id) = self.globals.get(&name) {
                    self.emit(HirInstruction::StoreGlobal {
                        id,
                        value,
                        word_size,
                    });
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
            TypedExpression::Negate { expression, typ } => {
                let word_size = self.get_word_size(&typ);
                let operand = self.generate_expression(*expression);
                let destination = self.new_register();
                let destination = match expression_type {
                    Type::Class(_) | Type::Array(_, _) => HirOperand::Link(destination),
                    _ => HirOperand::Value(destination),
                };
                let zero_const = HirOperand::Constant("0".to_string(), typ);
                self.emit(HirInstruction::BinaryOperator {
                    destination: destination.clone(),
                    left: zero_const,
                    operator: HirBinaryOperator::Sub,
                    right: operand,
                    word_size,
                });
                destination
            }
            TypedExpression::Not { expression, typ } => {
                let word_size = self.get_word_size(&typ);
                let operand = self.generate_expression(*expression);
                let destination = self.new_register();
                let destination = match expression_type {
                    Type::Class(_) | Type::Array(_, _) => HirOperand::Link(destination),
                    _ => HirOperand::Value(destination),
                };
                let false_const = HirOperand::Constant("false".to_string(), typ);
                self.emit(HirInstruction::BinaryOperator {
                    destination: destination.clone(),
                    left: operand,
                    operator: HirBinaryOperator::Equal,
                    right: false_const,
                    word_size,
                });
                destination
            }
            TypedExpression::BitwiseNot { expression, typ } => {
                let word_size = self.get_word_size(&typ);
                let operand = self.generate_expression(*expression);
                let destination = self.new_register();
                let destination = match expression_type {
                    Type::Class(_) | Type::Array(_, _) => HirOperand::Link(destination),
                    _ => HirOperand::Value(destination),
                };
                self.emit(HirInstruction::Not {
                    destination: destination.clone(),
                    operand,
                    word_size,
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

                let arguments_iterator = arguments.into_iter().map(|arg| {
                    (
                        self.generate_expression(arg.clone()),
                        self.get_word_size(&arg.get_type()),
                    )
                });
                let combined_iterator =
                    iter::once((object, WordSize::Long)).chain(arguments_iterator);
                let arguments = combined_iterator.collect();

                let block = self.classes[&class_name].methods[&name];
                let destination = self.new_register();
                let destination = match expression_type {
                    Type::Class(_) | Type::Array(_, _) => HirOperand::Link(destination),
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
                typ,
            } => {
                let word_size = self.get_word_size(&typ);
                let offset = self.resolve_field_offset(&object.get_type(), &name);
                let object = self.generate_expression(*object);
                let value = self.generate_expression(*value);

                self.emit(HirInstruction::StoreField {
                    object,
                    offset,
                    value,
                    word_size,
                });
                HirOperand::Void
            }
            TypedExpression::AssignIndex {
                expression,
                index,
                value,
                typ,
            } => {
                let word_size = self.get_word_size(&typ);
                let array = self.generate_expression(*expression);
                let index = self.generate_expression(*index);
                let value = self.generate_expression(*value);

                self.emit(HirInstruction::StoreIndex {
                    array,
                    index,
                    value,
                    word_size,
                });
                HirOperand::Void
            }
            TypedExpression::New { class_name, typ } => {
                let object = HirOperand::Link(self.new_register());

                self.emit(HirInstruction::AllocateObject {
                    destination: object.clone(),
                    size: self.get_type_allocation_size(&typ),
                });

                let class_info = &self.classes[&class_name];
                let mut fields: Vec<_> = class_info
                    .fields
                    .values()
                    .filter_map(|(expr, _, off)| expr.as_ref().map(|e| (e.clone(), *off)))
                    .collect();
                fields.sort_by_key(|(_, offset)| *offset);

                for (expression, offset) in fields {
                    let value = self.generate_expression(expression.clone());
                    let word_size = self.get_word_size(&expression.get_type());
                    self.emit(HirInstruction::StoreField {
                        object: object.clone(),
                        offset,
                        value,
                        word_size,
                    });
                }
                object
            }
            TypedExpression::NewArray { size, .. } => {
                let destination = HirOperand::Link(self.new_register());
                self.emit(HirInstruction::AllocateArray {
                    destination: destination.clone(),
                    size,
                });
                destination
            }
            TypedExpression::Field { object, name, typ } => {
                let word_size = self.get_word_size(&typ);
                let offset = self.resolve_field_offset(&object.get_type(), &name);
                let object = self.generate_expression(*object);
                let destination = self.new_register();
                let destination = match expression_type {
                    Type::Class(_) | Type::Array(_, _) => HirOperand::Link(destination),
                    _ => HirOperand::Value(destination),
                };
                self.emit(HirInstruction::LoadField {
                    destination: destination.clone(),
                    object,
                    offset,
                    word_size,
                });
                destination
            }
            TypedExpression::Index {
                expression,
                index,
                typ,
            } => {
                let array = self.generate_expression(*expression);
                let index = self.generate_expression(*index);
                let destination = self.new_register();
                let destination = match expression_type {
                    Type::Class(_) | Type::Array(_, _) => HirOperand::Link(destination),
                    _ => HirOperand::Value(destination),
                };
                self.emit(HirInstruction::LoadIndex {
                    destination: destination.clone(),
                    array,
                    index,
                    word_size: self.get_word_size(&typ),
                });
                destination
            }
            TypedExpression::AssignSlice {
                expression,
                start,
                value,
                ..
            } => {
                let value_type = value.get_type();
                let size = if let Type::Array(_, size) = value_type {
                    size
                } else {
                    panic!("Slice can be assigned only for sized array")
                };

                let target = self.generate_expression(*expression);
                let start = self.generate_expression(*start);
                let value = self.generate_expression(*value);

                self.emit(HirInstruction::StoreSlice {
                    target,
                    start,
                    value,
                    size,
                    word_size: self.get_word_size(&value_type),
                });
                HirOperand::Void
            }
            TypedExpression::Slice {
                expression, start, ..
            } => {
                let array = self.generate_expression(*expression);
                let start = self.generate_expression(*start);

                let destination = HirOperand::Link(self.new_register());

                self.emit(HirInstruction::LoadSlice {
                    destination: destination.clone(),
                    array,
                    start,
                });

                destination
            }
            TypedExpression::This { .. } => self
                .this_register
                .clone()
                .expect("Usage of 'this' outside of method"),
            TypedExpression::ArrayLiteral { elements, typ } => {
                let inner_type = if let Type::Array(inner, _) = &typ {
                    inner.as_ref().clone()
                } else {
                    unreachable!()
                };

                let mut values = Vec::with_capacity(elements.len());
                for element in elements {
                    if let TypedExpression::Literal { value, .. } = element {
                        values.push(value);
                    } else {
                        unreachable!();
                    }
                }

                let size = values.len() as u64;
                let const_id = self.new_constant_array(values, inner_type.clone());
                let destination = HirOperand::Link(self.new_register());
                let word_size = self.get_word_size(&inner_type);

                self.emit(HirInstruction::AllocateArray {
                    destination: destination.clone(),
                    size,
                });
                self.emit(HirInstruction::CopyConstantArray {
                    destination: destination.clone(),
                    id: const_id,
                    word_size,
                });
                destination
            }
        }
    }

    fn generate_increment_or_decrement(
        &mut self,
        expression: TypedExpression,
        postfix: bool,
        operator: ExpressionBinaryOperator,
    ) -> HirOperand {
        match expression {
            TypedExpression::Variable { name, typ } => {
                let word_size = self.get_word_size(&typ);
                let variable = self.resolve_local_variable_address(&name);
                let slot = match variable {
                    LocalVariable::Variable(slot) => *slot,
                    LocalVariable::Parameter(_) => panic!("Can't change parameter"),
                };
                let old_value = HirOperand::Value(self.new_register());
                self.emit(HirInstruction::LoadStack {
                    destination: old_value.clone(),
                    slot,
                    word_size: word_size.clone(),
                });

                let new_value = HirOperand::Value(self.new_register());
                let one_const = HirOperand::Constant("1".to_string(), typ);
                self.emit(HirInstruction::BinaryOperator {
                    destination: new_value.clone(),
                    left: old_value.clone(),
                    operator: translate_binary_operator(operator),
                    right: one_const,
                    word_size: word_size.clone(),
                });
                self.emit(HirInstruction::StoreStack {
                    slot,
                    value: new_value.clone(),
                    word_size,
                });

                if postfix { old_value } else { new_value }
            }
            TypedExpression::Field { object, name, typ } => {
                let word_size = self.get_word_size(&typ);
                let object_type = object.get_type();
                let object = self.generate_expression(*object);
                let offset = self.resolve_field_offset(&object_type, &name);

                let old_value = HirOperand::Value(self.new_register());
                self.emit(HirInstruction::LoadField {
                    destination: old_value.clone(),
                    object: object.clone(),
                    offset,
                    word_size: word_size.clone(),
                });

                let new_value = HirOperand::Value(self.new_register());
                let one_const = HirOperand::Constant("1".to_string(), typ);
                self.emit(HirInstruction::BinaryOperator {
                    destination: new_value.clone(),
                    left: old_value.clone(),
                    operator: translate_binary_operator(operator),
                    right: one_const,
                    word_size: word_size.clone(),
                });
                self.emit(HirInstruction::StoreField {
                    object,
                    offset,
                    value: new_value.clone(),
                    word_size,
                });

                if postfix { old_value } else { new_value }
            }
            TypedExpression::Index {
                expression,
                index,
                typ,
            } => {
                let array = self.generate_expression(*expression);
                let index = self.generate_expression(*index);

                let old_value = HirOperand::Value(self.new_register());
                self.emit(HirInstruction::LoadIndex {
                    destination: old_value.clone(),
                    array: array.clone(),
                    index: index.clone(),
                    word_size: self.get_word_size(&typ),
                });

                let new_value = HirOperand::Value(self.new_register());
                let one_const = HirOperand::Constant("1".to_string(), typ.clone());
                self.emit(HirInstruction::BinaryOperator {
                    destination: new_value.clone(),
                    left: old_value.clone(),
                    operator: translate_binary_operator(operator),
                    right: one_const,
                    word_size: self.get_word_size(&typ),
                });
                self.emit(HirInstruction::StoreIndex {
                    array,
                    index,
                    value: new_value.clone(),
                    word_size: self.get_word_size(&typ),
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
            TypedAbstractSyntaxNode::File => {
                for child in &ast.children {
                    self.scan_signatures(child);
                }
            }
            TypedAbstractSyntaxNode::Class { name } => {
                let mut class_info = ClassInfo::default();
                let mut offset = 0;

                for child in &ast.children {
                    match &child.node {
                        TypedAbstractSyntaxNode::Callable { name, .. } => {
                            let block_id = self.create_block();
                            class_info.methods.insert(name.clone(), block_id);
                        }
                        TypedAbstractSyntaxNode::Declaration {
                            name,
                            expression,
                            typ,
                        } => {
                            class_info
                                .fields
                                .insert(name.clone(), (expression.clone(), typ.clone(), offset));
                            offset += 1;
                        }
                        _ => {}
                    }
                }
                self.classes.insert(name.clone(), class_info);
            }
            TypedAbstractSyntaxNode::Callable { name, .. } => {
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

    fn get_type_allocation_size(&self, typ: &Type) -> u64 {
        match typ {
            Type::Void => 0,
            Type::Bool => 1,
            Type::Char => 1,
            Type::Int => 1,
            Type::Array(_, size) => *size,
            Type::Class(name) => self.classes[name].fields.iter().len() as u64,
        }
    }

    fn get_word_size(&self, typ: &Type) -> WordSize {
        match typ {
            Type::Bool | Type::Char => WordSize::Byte,
            _ => WordSize::Long,
        }
    }
}

#[derive(Debug)]
pub struct ControlFlowGraph {
    pub blocks: Vec<HirBlock>,
    pub entry_block: BlockId,
    pub interrupt_blocks: [BlockId; 8],
    pub classes: HashMap<String, ClassInfo>,
    pub register_counter: u64,
    pub array_constants: Vec<(Vec<String>, Type)>,
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
        register_counter: context.register_counter,
        array_constants: context.array_constants,
    }
}
