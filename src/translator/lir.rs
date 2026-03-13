use crate::translator::expression::ExpressionBinaryOperator;
use crate::translator::hir::{
    BlockId, ControlFlowGraph, HirInstruction, HirOperand, HirRegister, HirTerminator, StackSlot,
};
use std::collections::HashMap;

pub struct ConstantId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordSize {
    Byte = 0,
    Long = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterType {
    Data,
    Address,
}

#[derive(Debug, Clone)]
pub enum LirOperand {
    Direct(u64),
    VirtualRegister(usize, RegisterType),
    Register(u8, RegisterType),
    Indirect(Box<LirOperand>),
    IndirectPostIncrement(Box<LirOperand>),
    IndirectPreDecrement(Box<LirOperand>),
    IndirectOffset {
        base: Box<LirOperand>,
        offset_register: Box<LirOperand>,
    },
    IndirectDirect(u64),
}

#[derive(Debug, Clone)]
pub enum Condition {
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Lower,
    LowerEqual,
    CarrySet,
    CarryClear,
    OverflowSet,
    OverflowClear,
}

#[derive(Debug, Clone)]
pub enum LirInstruction {
    Mov {
        size: WordSize,
        source: LirOperand,
        destination: LirOperand,
    },
    Mova {
        size: WordSize,
        source: LirOperand,
        destination: LirOperand,
    },

    Add {
        size: WordSize,
        source: LirOperand,
        destination: LirOperand,
    },
    Sub {
        size: WordSize,
        source: LirOperand,
        destination: LirOperand,
    },
    Mul {
        size: WordSize,
        source: LirOperand,
        destination: LirOperand,
    },
    Div {
        size: WordSize,
        source: LirOperand,
        destination: LirOperand,
    },
    Rem {
        size: WordSize,
        source: LirOperand,
        destination: LirOperand,
    },

    And {
        size: WordSize,
        source: LirOperand,
        destination: LirOperand,
    },
    Or {
        size: WordSize,
        source: LirOperand,
        destination: LirOperand,
    },
    Xor {
        size: WordSize,
        source: LirOperand,
        destination: LirOperand,
    },
    Not {
        size: WordSize,
        source: LirOperand,
        destination: LirOperand,
    },

    Cmp {
        size: WordSize,
        that: LirOperand,
        with: LirOperand,
    },

    SetCond {
        condition: Condition,
        destination: LirOperand,
    },

    Jmp {
        label: BlockId,
    },
    Branch {
        cond: Condition,
        label: BlockId,
    },
    Call {
        label: BlockId,
    },
    Ret,
}

#[derive(Debug)]
pub struct LirBlock {
    pub id: BlockId,
    pub instructions: Vec<LirInstruction>,
}

pub struct LirContext {
    pub blocks: Vec<LirBlock>,
    virtual_register_counter: usize,

    stack_size: i32,
    stack_offsets: HashMap<StackSlot, i32>,

    pub constants_size: u64,
    pub constants: HashMap<String, ConstantId>,

    frame_pointer: LirOperand,
    stack_pointer: LirOperand,
}

impl LirContext {
    pub fn new() -> Self {
        Self {
            blocks: vec![],
            virtual_register_counter: 0,
            stack_size: 0,
            stack_offsets: HashMap::new(),
            constants_size: 0,
            constants: HashMap::new(),
            frame_pointer: LirOperand::Register(6, RegisterType::Address),
            stack_pointer: LirOperand::Register(7, RegisterType::Address),
        }
    }

    fn next_virtual_register(&mut self, reg_type: RegisterType) -> LirOperand {
        let id = self.virtual_register_counter;
        self.virtual_register_counter += 1;
        LirOperand::VirtualRegister(id, reg_type)
    }

    fn get_virtual_data_register(&self, reg: HirRegister) -> LirOperand {
        LirOperand::VirtualRegister(reg.0 as usize, RegisterType::Data)
    }

    fn get_constant_address(&mut self, value: String) -> u64 {
        if let Some(id) = self.constants.get(&value) {
            id.0
        } else {
            let address = self.constants_size;
            self.constants.insert(value, ConstantId(address));
            self.constants_size += 8;
            address
        }
    }

    fn lower_operand(&mut self, operand: HirOperand) -> LirOperand {
        match operand {
            HirOperand::Value(register) => self.get_virtual_data_register(register),
            HirOperand::Constant(value_str) => {
                let normalized_value = match value_str.as_str() {
                    "true" => "1".to_string(),
                    "false" => "0".to_string(),
                    _ => value_str,
                };
                let address = self.get_constant_address(normalized_value);
                LirOperand::IndirectDirect(address)
            }
            HirOperand::Void => panic!("Cannot lower void operand"),
        }
    }

    pub fn lower(&mut self, control_flow_graph: ControlFlowGraph) {
        for hir_block in control_flow_graph.blocks {
            let mut lir_instructions = Vec::new();

            for instruction in hir_block.instructions {
                self.lower_instruction(instruction, &mut lir_instructions);
            }

            if let Some(terminator) = hir_block.terminator {
                self.lower_terminator(terminator, &mut lir_instructions);
            }

            self.blocks.push(LirBlock {
                id: hir_block.id,
                instructions: lir_instructions,
            });
        }
    }

    fn lower_instruction(&mut self, instruction: HirInstruction, out: &mut Vec<LirInstruction>) {
        match instruction {
            HirInstruction::LoadConst { destination, value } => {
                let address = self.get_constant_address(value);

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: LirOperand::IndirectDirect(address),
                    destination: self.get_virtual_data_register(destination),
                });
            }
            HirInstruction::BinaryOperator {
                destination,
                left,
                operator,
                right,
            } => {
                let destination = self.get_virtual_data_register(destination);
                let left_operand = self.lower_operand(left);
                let right_operand = self.lower_operand(right);

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: right_operand.clone(),
                    destination: destination.clone(),
                });

                match operator {
                    ExpressionBinaryOperator::Plus => out.push(LirInstruction::Add {
                        size: WordSize::Long,
                        source: left_operand,
                        destination,
                    }),
                    ExpressionBinaryOperator::Minus => out.push(LirInstruction::Sub {
                        size: WordSize::Long,
                        source: left_operand,
                        destination,
                    }),
                    ExpressionBinaryOperator::Multiply => out.push(LirInstruction::Mul {
                        size: WordSize::Long,
                        source: left_operand,
                        destination,
                    }),
                    ExpressionBinaryOperator::Divide => out.push(LirInstruction::Div {
                        size: WordSize::Long,
                        source: left_operand,
                        destination,
                    }),
                    ExpressionBinaryOperator::Modulo => out.push(LirInstruction::Rem {
                        size: WordSize::Long,
                        source: left_operand,
                        destination,
                    }),

                    operator @ (ExpressionBinaryOperator::Equal
                    | ExpressionBinaryOperator::NotEqual
                    | ExpressionBinaryOperator::Less
                    | ExpressionBinaryOperator::LessEqual
                    | ExpressionBinaryOperator::Greater
                    | ExpressionBinaryOperator::GreaterEqual) => {
                        out.push(LirInstruction::Cmp {
                            size: WordSize::Long,
                            that: left_operand,
                            with: right_operand,
                        });

                        let condition = match operator {
                            ExpressionBinaryOperator::Equal => Condition::Equal,
                            ExpressionBinaryOperator::NotEqual => Condition::NotEqual,
                            ExpressionBinaryOperator::Less => Condition::Lower,
                            ExpressionBinaryOperator::LessEqual => Condition::LowerEqual,
                            ExpressionBinaryOperator::Greater => Condition::Greater,
                            ExpressionBinaryOperator::GreaterEqual => Condition::GreaterEqual,
                            _ => unreachable!(),
                        };

                        out.push(LirInstruction::SetCond {
                            condition,
                            destination,
                        });
                    },
                    _ => unreachable!()
                }
            }
            HirInstruction::Call {
                destination,
                block,
                arguments,
            } => {
                let arguments_count = arguments.len();

                for argument in arguments.into_iter().rev() {
                    let operand = self.lower_operand(argument);
                    out.push(LirInstruction::Mov {
                        size: WordSize::Long,
                        source: operand,
                        destination: LirOperand::IndirectPreDecrement(Box::new(
                            self.stack_pointer.clone(),
                        )),
                    });
                }

                out.push(LirInstruction::Call { label: block });

                if arguments_count > 0 {
                    out.push(LirInstruction::Add {
                        size: WordSize::Long,
                        source: LirOperand::Direct((arguments_count * 8) as u64),
                        destination: self.stack_pointer.clone(),
                    });
                }

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: LirOperand::Register(0, RegisterType::Data),
                    destination: self.get_virtual_data_register(destination),
                });
            }
            HirInstruction::CallPrologue => {
                self.stack_size = 0;

                let temp_register = self.next_virtual_register(RegisterType::Data);
                out.push(LirInstruction::Mova {
                    size: WordSize::Long,
                    source: self.frame_pointer.clone(),
                    destination: temp_register.clone(),
                });
                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: temp_register,
                    destination: LirOperand::IndirectPreDecrement(Box::new(self.stack_pointer.clone())),
                });

                out.push(LirInstruction::Mova {
                    size: WordSize::Long,
                    source: self.stack_pointer.clone(),
                    destination: self.frame_pointer.clone(),
                });
            }
            HirInstruction::LoadParameter { destination, index } => {
                let offset = index as u64 * 8 + 8;
                let offset_register = self.next_virtual_register(RegisterType::Data);

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: LirOperand::Direct(offset),
                    destination: offset_register.clone(),
                });

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: LirOperand::IndirectOffset {
                        base: Box::new(self.frame_pointer.clone()),
                        offset_register: Box::new(offset_register),
                    },
                    destination: self.get_virtual_data_register(destination),
                });
            }
            HirInstruction::StackAllocate { slot } => {
                self.stack_size += 8;
                self.stack_offsets.insert(slot, -self.stack_size);

                out.push(LirInstruction::Sub {
                    size: WordSize::Long,
                    source: LirOperand::Direct(8),
                    destination: self.stack_pointer.clone(),
                });
            }
            HirInstruction::StackStore { slot, value } => {
                let offset = *self.stack_offsets.get(&slot).unwrap() as i64;
                let offset_register = self.next_virtual_register(RegisterType::Data);
                let value_operand = self.lower_operand(value);

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: LirOperand::Direct(offset as u64),
                    destination: offset_register.clone(),
                });
                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: value_operand,
                    destination: LirOperand::IndirectOffset {
                        base: Box::new(self.frame_pointer.clone()),
                        offset_register: Box::new(offset_register),
                    },
                });
            }
            HirInstruction::StackLoad { destination, slot } => {
                let offset = *self.stack_offsets.get(&slot).unwrap() as i64;
                let offset_register = self.next_virtual_register(RegisterType::Data);

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: LirOperand::Direct(offset as u64),
                    destination: offset_register.clone(),
                });
                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: LirOperand::IndirectOffset {
                        base: Box::new(self.frame_pointer.clone()),
                        offset_register: Box::new(offset_register),
                    },
                    destination: self.get_virtual_data_register(destination),
                });
            }
            HirInstruction::GetField {
                destination,
                object,
                offset,
            } => {
                let object = self.lower_operand(object);
                let offset = (offset * 8) as u64;

                let object_address_register = self.next_virtual_register(RegisterType::Address);
                let offset_register = self.next_virtual_register(RegisterType::Data);

                out.push(LirInstruction::Mova {
                    size: WordSize::Long,
                    source: object,
                    destination: object_address_register.clone(),
                });
                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: LirOperand::Direct(offset),
                    destination: offset_register.clone(),
                });

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: LirOperand::IndirectOffset {
                        base: Box::new(object_address_register),
                        offset_register: Box::new(offset_register),
                    },
                    destination: self.get_virtual_data_register(destination),
                });
            }
            HirInstruction::PutField {
                object,
                offset,
                value,
            } => {
                let object = self.lower_operand(object);
                let value = self.lower_operand(value);
                let offset = (offset * 8) as u64;

                let object_address_register = self.next_virtual_register(RegisterType::Address);
                let offset_register = self.next_virtual_register(RegisterType::Data);

                out.push(LirInstruction::Mova {
                    size: WordSize::Long,
                    source: object,
                    destination: object_address_register.clone(),
                });
                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: LirOperand::Direct(offset),
                    destination: offset_register.clone(),
                });

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: value,
                    destination: LirOperand::IndirectOffset {
                        base: Box::new(object_address_register),
                        offset_register: Box::new(offset_register),
                    },
                });
            }
            HirInstruction::AllocateObject {
                destination,
                class_name: _,
            } => {
                let destination = self.get_virtual_data_register(destination);
                out.push(LirInstruction::Call { label: 0 });
                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: LirOperand::Register(0, RegisterType::Data),
                    destination,
                });
            }
        }
    }

    fn lower_terminator(&mut self, terminator: HirTerminator, out: &mut Vec<LirInstruction>) {
        match terminator {
            HirTerminator::Jump(block_id) => {
                out.push(LirInstruction::Jmp { label: block_id });
            }
            HirTerminator::Branch {
                condition,
                true_block,
                false_block,
            } => {
                let condition = self.lower_operand(condition);

                out.push(LirInstruction::Cmp {
                    size: WordSize::Long,
                    that: condition,
                    with: LirOperand::Direct(0),
                });

                out.push(LirInstruction::Branch {
                    cond: Condition::Equal,
                    label: false_block,
                });

                out.push(LirInstruction::Jmp { label: true_block });
            }
            HirTerminator::Return(operand) => {
                if let Some(operand) = operand {
                    let return_value = self.lower_operand(operand);
                    out.push(LirInstruction::Mov {
                        size: WordSize::Long,
                        source: return_value,
                        destination: LirOperand::Register(0, RegisterType::Data),
                    });
                }

                out.push(LirInstruction::Mova {
                    size: WordSize::Long,
                    source: self.frame_pointer.clone(),
                    destination: self.stack_pointer.clone(),
                });

                let temp_register = self.next_virtual_register(RegisterType::Data);
                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: LirOperand::IndirectPostIncrement(Box::new(self.stack_pointer.clone())),
                    destination: temp_register.clone(),
                });
                out.push(LirInstruction::Mova {
                    size: WordSize::Long,
                    source: temp_register,
                    destination: self.frame_pointer.clone(),
                });

                out.push(LirInstruction::Ret);
            }
        }
    }
}

pub fn compile_lir(
    control_flow_graph: ControlFlowGraph,
) -> (Vec<LirBlock>, HashMap<String, ConstantId>) {
    let mut context = LirContext::new();
    context.lower(control_flow_graph);

    (context.blocks, context.constants)
}

// A5 - heap, of course without gc
// Check that new virtual registers don't conflict with elder
// SetCond