use crate::isa::WordSize;
use crate::translator::common::{Address, Type};
use crate::translator::hir::{
    BlockId, ControlFlowGraph, GlobalId, HirBinaryOperator, HirBlock, HirInstruction, HirOperand,
    HirRegister, HirTerminator, StackSlot,
};
use std::collections::{HashMap, hash_map};
use std::vec;

#[derive(Debug, Clone)]
pub struct ArrayConstant {
    pub values: Vec<String>,
    pub element_type: Type,
    pub base_address: Address,
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
        offset: Box<LirOperand>,
    },
    IndirectDirect(Address),
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
    Cmp {
        size: WordSize,
        that: LirOperand,
        with: LirOperand,
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

    Lsl {
        size: WordSize,
        count: LirOperand,
        destination: LirOperand,
    },
    Lsr {
        size: WordSize,
        count: LirOperand,
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
    Jmp {
        label: BlockId,
    },
    Branch {
        condition: Condition,
        label: BlockId,
    },
    VAdd {
        left: LirOperand,
        right: LirOperand,
    },
    VSub {
        left: LirOperand,
        right: LirOperand,
    },
    VMul {
        left: LirOperand,
        right: LirOperand,
    },
    VDiv {
        left: LirOperand,
        right: LirOperand,
    },
    VRem {
        left: LirOperand,
        right: LirOperand,
    },
    VAnd {
        left: LirOperand,
        right: LirOperand,
    },
    VOr {
        left: LirOperand,
        right: LirOperand,
    },
    VXor {
        left: LirOperand,
        right: LirOperand,
    },
    VEnd {
        destination: LirOperand,
    },
    VCmpBeq {
        left: LirOperand,
        right: LirOperand,
    },
    VCmpBne {
        left: LirOperand,
        right: LirOperand,
    },
    VCmpBgt {
        left: LirOperand,
        right: LirOperand,
    },
    VCmpBge {
        left: LirOperand,
        right: LirOperand,
    },
    VCmpBlt {
        left: LirOperand,
        right: LirOperand,
    },
    VCmpBle {
        left: LirOperand,
        right: LirOperand,
    },
    Call {
        label: BlockId,
    },
    Ret,
    IntRet,
    In {
        port: LirOperand,
        destination: LirOperand,
        word_size: WordSize,
    },
    Out {
        port: LirOperand,
        value: LirOperand,
        word_size: WordSize,
    },
    Halt,
    SetBool {
        condition: Condition,
        destination: LirOperand,
    },
    AllocateStackFrame,
}

#[derive(Debug, Clone)]
pub struct LirBlock {
    pub id: BlockId,
    pub instructions: Vec<LirInstruction>,
}

pub struct LirContext {
    virtual_register_counter: u64,

    block_to_function: HashMap<BlockId, BlockId>,
    register_to_function: HashMap<usize, BlockId>,

    pub blocks: Vec<LirBlock>,

    stack_offsets: HashMap<StackSlot, i64>,
    frame_sizes: HashMap<BlockId, i64>,
    current_function: Option<BlockId>,

    pub data_size: u64,
    pub constants: HashMap<(String, Type), (Address, WordSize)>,

    pub globals: HashMap<GlobalId, Address>,

    pub array_constants: Vec<ArrayConstant>,

    return_registers: LirOperand,

    restore_data_registers: Vec<LirOperand>,
    restore_address_registers: Vec<LirOperand>,

    heap_pointer: LirOperand,
    frame_pointer: LirOperand,
    stack_pointer: LirOperand,

    allocated_data_registers: HashMap<usize, u8>,
    allocated_address_registers: HashMap<usize, u8>,

    spilled_data_registers: HashMap<usize, i64>,
    spilled_address_registers: HashMap<usize, i64>,

    saved_registers: Vec<LirOperand>,
}

impl LirContext {
    fn new(register_counter: u64) -> Self {
        Self {
            virtual_register_counter: register_counter,
            block_to_function: HashMap::new(),
            register_to_function: HashMap::new(),
            blocks: vec![],
            frame_sizes: HashMap::new(),
            stack_offsets: HashMap::new(),
            current_function: None,
            data_size: 0,
            constants: HashMap::new(),
            globals: HashMap::new(),
            array_constants: vec![],
            return_registers: LirOperand::Register(0, RegisterType::Data),
            restore_data_registers: vec![
                LirOperand::Register(6, RegisterType::Data),
                LirOperand::Register(7, RegisterType::Data),
            ],
            restore_address_registers: vec![
                LirOperand::Register(3, RegisterType::Address),
                LirOperand::Register(4, RegisterType::Address),
            ],
            heap_pointer: LirOperand::Register(5, RegisterType::Address),
            frame_pointer: LirOperand::Register(6, RegisterType::Address),
            stack_pointer: LirOperand::Register(7, RegisterType::Address),
            allocated_data_registers: HashMap::new(),
            allocated_address_registers: HashMap::new(),
            spilled_data_registers: HashMap::new(),
            spilled_address_registers: HashMap::new(),
            saved_registers: vec![
                LirOperand::Register(1, RegisterType::Data),
                LirOperand::Register(2, RegisterType::Data),
                LirOperand::Register(3, RegisterType::Data),
                LirOperand::Register(4, RegisterType::Data),
                LirOperand::Register(5, RegisterType::Data),
                LirOperand::Register(0, RegisterType::Address),
                LirOperand::Register(1, RegisterType::Address),
                LirOperand::Register(2, RegisterType::Address),
            ],
        }
    }

    fn new_virtual_data_register(&mut self) -> LirOperand {
        let register = self.virtual_register_counter;
        self.virtual_register_counter += 1;
        LirOperand::VirtualRegister(register as usize, RegisterType::Data)
    }

    fn new_virtual_address_register(&mut self) -> LirOperand {
        let register = self.virtual_register_counter;
        self.virtual_register_counter += 1;
        LirOperand::VirtualRegister(register as usize, RegisterType::Address)
    }

    fn get_virtual_register(&mut self, operand: HirOperand) -> LirOperand {
        match operand {
            HirOperand::Value(register) => self.get_virtual_data_register(register),
            HirOperand::Link(register) => self.get_virtual_address_register(register),
            _ => panic!("Virtual register called for non-register"),
        }
    }

    fn get_virtual_data_register(&mut self, register: HirRegister) -> LirOperand {
        if let Some(function) = self.current_function {
            self.register_to_function
                .insert(register.0 as usize, function);
        }
        LirOperand::VirtualRegister(register.0 as usize, RegisterType::Data)
    }

    fn get_virtual_address_register(&mut self, register: HirRegister) -> LirOperand {
        if let Some(function) = self.current_function {
            self.register_to_function
                .insert(register.0 as usize, function);
        }
        LirOperand::VirtualRegister(register.0 as usize, RegisterType::Address)
    }

    fn get_constant_address(&mut self, value: String, typ: Type) -> Address {
        let word_size = match typ {
            Type::Int => WordSize::Long,
            Type::Bool | Type::Char => WordSize::Byte,
            _ => unreachable!(),
        };

        if let Some((address, size)) = self.constants.get(&(value.clone(), typ.clone()))
            && *size == word_size
        {
            *address
        } else {
            let address = self.data_size;

            self.constants
                .insert((value, typ), (address, word_size.clone()));
            self.data_size += 1;
            address
        }
    }

    fn get_global_address(&mut self, id: GlobalId) -> Address {
        if let Some(address) = self.globals.get(&id) {
            *address
        } else {
            let address = self.data_size;
            self.globals.insert(id, address);
            self.data_size += 1;
            address
        }
    }

    fn save_registers(&mut self, out: &mut Vec<LirInstruction>) {
        for register in &self.saved_registers {
            out.push(LirInstruction::Mov {
                size: WordSize::Long,
                source: register.clone(),
                destination: LirOperand::IndirectPreDecrement(Box::new(self.stack_pointer.clone())),
            });
        }
    }

    fn restore_registers(&mut self, out: &mut Vec<LirInstruction>) {
        for register in self.saved_registers.iter().rev() {
            out.push(LirInstruction::Mov {
                size: WordSize::Long,
                source: LirOperand::IndirectPostIncrement(Box::new(self.stack_pointer.clone())),
                destination: register.clone(),
            });
        }
    }

    fn operand_to_register(
        &mut self,
        operand: LirOperand,
        register_type: RegisterType,
        out: &mut Vec<LirInstruction>,
    ) -> LirOperand {
        if let LirOperand::VirtualRegister(_, _) = &operand {
            return operand;
        }

        let temp = match register_type {
            RegisterType::Address => self.new_virtual_address_register(),
            RegisterType::Data => self.new_virtual_data_register(),
        };
        out.push(LirInstruction::Mov {
            size: WordSize::Long,
            source: operand,
            destination: temp.clone(),
        });
        temp
    }

    fn lower_operand(&mut self, operand: HirOperand) -> LirOperand {
        match operand {
            HirOperand::Value(register) => self.get_virtual_data_register(register),
            HirOperand::Link(register) => self.get_virtual_address_register(register),
            HirOperand::Constant(value_str, typ) => {
                let normalized_value = normalize_constant(value_str);
                let address = self.get_constant_address(normalized_value, typ);
                LirOperand::IndirectDirect(address)
            }
            HirOperand::LocalVariable(slot) => {
                let offset = *self.stack_offsets.get(&slot).unwrap() as u64;
                LirOperand::IndirectOffset {
                    base: Box::new(self.frame_pointer.clone()),
                    offset: Box::new(LirOperand::Direct(offset)),
                }
            }
            HirOperand::GlobalVariable(id) => {
                let address = self.get_global_address(id);
                LirOperand::IndirectDirect(address)
            }
            HirOperand::Void => panic!("Cannot lower void operand"),
        }
    }

    fn lower(&mut self, hir_blocks: Vec<HirBlock>) {
        self.current_function = None;

        for hir_block in &hir_blocks {
            if hir_block.instructions.iter().any(|i| {
                matches!(
                    i,
                    HirInstruction::CallPrologue | HirInstruction::InterruptPrologue
                )
            }) {
                self.frame_sizes.insert(hir_block.id, 0);
                self.block_to_function.insert(hir_block.id, hir_block.id);
            }
        }

        let mut changed = true;
        while changed {
            changed = false;
            for hir_block in &hir_blocks {
                if let Some(&function) = self.block_to_function.get(&hir_block.id)
                    && let Some(terminator) = &hir_block.terminator
                {
                    let targets = match terminator {
                        HirTerminator::Jump(id) => vec![*id],
                        HirTerminator::Branch {
                            true_block,
                            false_block,
                            ..
                        } => vec![*true_block, *false_block],
                        HirTerminator::Return(_, _) => vec![],
                        HirTerminator::IntReturn => vec![],
                    };
                    for target in targets {
                        if let hash_map::Entry::Vacant(entry) = self.block_to_function.entry(target)
                        {
                            entry.insert(function);
                            changed = true;
                        }
                    }
                }
            }
        }

        for hir_block in hir_blocks {
            self.current_function = self.block_to_function.get(&hir_block.id).copied();

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
        self.current_function = None;
    }

    fn lower_instruction(&mut self, instruction: HirInstruction, out: &mut Vec<LirInstruction>) {
        match instruction {
            HirInstruction::LoadGlobal {
                destination,
                id,
                word_size,
            } => {
                let address = self.get_global_address(id);
                let destination = self.get_virtual_register(destination);
                out.push(LirInstruction::Mov {
                    size: word_size,
                    source: LirOperand::IndirectDirect(address),
                    destination,
                });
            }

            HirInstruction::StoreGlobal {
                id,
                value,
                word_size,
            } => {
                let value_operand = self.lower_operand(value);
                let address = self.get_global_address(id);
                out.push(LirInstruction::Mov {
                    size: word_size,
                    source: value_operand,
                    destination: LirOperand::IndirectDirect(address),
                });
            }
            HirInstruction::BinaryOperator {
                destination,
                left,
                operator,
                right,
                word_size,
            } => {
                let destination = self.get_virtual_register(destination);
                let left = self.lower_operand(left);
                let right = self.lower_operand(right);

                match &operator {
                    HirBinaryOperator::Assign => {}
                    HirBinaryOperator::Add => {
                        out.push(LirInstruction::Mov {
                            size: word_size.clone(),
                            source: left.clone(),
                            destination: destination.clone(),
                        });
                        out.push(LirInstruction::Add {
                            size: word_size,
                            source: right,
                            destination: destination.clone(),
                        });
                    }
                    HirBinaryOperator::Sub => {
                        out.push(LirInstruction::Mov {
                            size: word_size.clone(),
                            source: left.clone(),
                            destination: destination.clone(),
                        });
                        out.push(LirInstruction::Sub {
                            size: word_size,
                            source: right,
                            destination: destination.clone(),
                        });
                    }
                    HirBinaryOperator::Multiply => {
                        out.push(LirInstruction::Mov {
                            size: word_size.clone(),
                            source: left.clone(),
                            destination: destination.clone(),
                        });
                        out.push(LirInstruction::Mul {
                            size: word_size,
                            source: right,
                            destination: destination.clone(),
                        });
                    }
                    HirBinaryOperator::Divide => {
                        out.push(LirInstruction::Mov {
                            size: word_size.clone(),
                            source: left.clone(),
                            destination: destination.clone(),
                        });
                        out.push(LirInstruction::Div {
                            size: word_size,
                            source: right,
                            destination: destination.clone(),
                        });
                    }
                    HirBinaryOperator::Remainder => {
                        out.push(LirInstruction::Mov {
                            size: word_size.clone(),
                            source: left.clone(),
                            destination: destination.clone(),
                        });
                        out.push(LirInstruction::Rem {
                            size: word_size,
                            source: right,
                            destination: destination.clone(),
                        });
                    }
                    HirBinaryOperator::BitwiseAnd => {
                        out.push(LirInstruction::Mov {
                            size: word_size.clone(),
                            source: left.clone(),
                            destination: destination.clone(),
                        });
                        out.push(LirInstruction::And {
                            size: word_size,
                            source: right,
                            destination: destination.clone(),
                        });
                    }
                    HirBinaryOperator::BitwiseOr => {
                        out.push(LirInstruction::Mov {
                            size: word_size.clone(),
                            source: left.clone(),
                            destination: destination.clone(),
                        });
                        out.push(LirInstruction::Or {
                            size: word_size,
                            source: right,
                            destination: destination.clone(),
                        });
                    }
                    HirBinaryOperator::BitwiseXor => {
                        out.push(LirInstruction::Mov {
                            size: word_size.clone(),
                            source: left.clone(),
                            destination: destination.clone(),
                        });
                        out.push(LirInstruction::Xor {
                            size: word_size,
                            source: right,
                            destination: destination.clone(),
                        });
                    }
                    HirBinaryOperator::LeftShift => {
                        out.push(LirInstruction::Mov {
                            size: word_size.clone(),
                            source: right,
                            destination: destination.clone(),
                        });
                        out.push(LirInstruction::Lsl {
                            size: word_size,
                            count: left,
                            destination: destination.clone(),
                        });
                    }
                    HirBinaryOperator::RightShift => {
                        out.push(LirInstruction::Mov {
                            size: word_size.clone(),
                            source: right,
                            destination: destination.clone(),
                        });
                        out.push(LirInstruction::Lsr {
                            size: word_size,
                            count: left,
                            destination: destination.clone(),
                        });
                    }
                    HirBinaryOperator::VectorAdd
                    | HirBinaryOperator::VectorSub
                    | HirBinaryOperator::VectorMultiply
                    | HirBinaryOperator::VectorDivide
                    | HirBinaryOperator::VectorRemainder
                    | HirBinaryOperator::VectorAnd
                    | HirBinaryOperator::VectorOr
                    | HirBinaryOperator::VectorXor
                    | HirBinaryOperator::VectorEqual
                    | HirBinaryOperator::VectorNotEqual
                    | HirBinaryOperator::VectorLess
                    | HirBinaryOperator::VectorLessEqual
                    | HirBinaryOperator::VectorGreater
                    | HirBinaryOperator::VectorGreaterEqual => {
                        out.push(LirInstruction::Mov {
                            size: WordSize::Long,
                            source: self.heap_pointer.clone(),
                            destination: destination.clone(),
                        });
                        out.push(LirInstruction::Add {
                            size: WordSize::Long,
                            source: LirOperand::Direct(4),
                            destination: self.heap_pointer.clone(),
                        });
                        match &operator {
                            HirBinaryOperator::VectorAdd => {
                                out.push(LirInstruction::VAdd { left, right })
                            }
                            HirBinaryOperator::VectorSub => {
                                out.push(LirInstruction::VSub { left, right })
                            }
                            HirBinaryOperator::VectorMultiply => {
                                out.push(LirInstruction::VMul { left, right })
                            }
                            HirBinaryOperator::VectorDivide => {
                                out.push(LirInstruction::VDiv { left, right })
                            }
                            HirBinaryOperator::VectorRemainder => {
                                out.push(LirInstruction::VRem { left, right })
                            }
                            HirBinaryOperator::VectorAnd => {
                                out.push(LirInstruction::VAnd { left, right })
                            }
                            HirBinaryOperator::VectorOr => {
                                out.push(LirInstruction::VOr { left, right })
                            }
                            HirBinaryOperator::VectorXor => {
                                out.push(LirInstruction::VXor { left, right })
                            }
                            HirBinaryOperator::VectorEqual => {
                                out.push(LirInstruction::VCmpBeq { left, right })
                            }
                            HirBinaryOperator::VectorNotEqual => {
                                out.push(LirInstruction::VCmpBne { left, right })
                            }
                            HirBinaryOperator::VectorLess => {
                                out.push(LirInstruction::VCmpBlt { left, right })
                            }
                            HirBinaryOperator::VectorLessEqual => {
                                out.push(LirInstruction::VCmpBle { left, right })
                            }
                            HirBinaryOperator::VectorGreater => {
                                out.push(LirInstruction::VCmpBgt { left, right })
                            }
                            HirBinaryOperator::VectorGreaterEqual => {
                                out.push(LirInstruction::VCmpBge { left, right })
                            }
                            _ => unreachable!(),
                        }
                        out.push(LirInstruction::VEnd { destination })
                    }
                    HirBinaryOperator::And => {
                        out.push(LirInstruction::Mov {
                            size: word_size.clone(),
                            source: left.clone(),
                            destination: destination.clone(),
                        });
                        out.push(LirInstruction::And {
                            size: word_size,
                            source: right,
                            destination: destination.clone(),
                        });
                    }
                    HirBinaryOperator::Or => {
                        out.push(LirInstruction::Mov {
                            size: word_size.clone(),
                            source: left.clone(),
                            destination: destination.clone(),
                        });
                        out.push(LirInstruction::Or {
                            size: word_size,
                            source: right,
                            destination: destination.clone(),
                        });
                    }
                    HirBinaryOperator::Equal
                    | HirBinaryOperator::NotEqual
                    | HirBinaryOperator::Less
                    | HirBinaryOperator::LessEqual
                    | HirBinaryOperator::Greater
                    | HirBinaryOperator::GreaterEqual => {
                        out.push(LirInstruction::Cmp {
                            size: word_size,
                            that: right,
                            with: left,
                        });

                        let condition = match operator {
                            HirBinaryOperator::Equal => Condition::Equal,
                            HirBinaryOperator::NotEqual => Condition::NotEqual,
                            HirBinaryOperator::Less => Condition::Lower,
                            HirBinaryOperator::LessEqual => Condition::LowerEqual,
                            HirBinaryOperator::Greater => Condition::Greater,
                            HirBinaryOperator::GreaterEqual => Condition::GreaterEqual,
                            _ => unreachable!(),
                        };

                        out.push(LirInstruction::SetBool {
                            condition,
                            destination: destination.clone(),
                        });
                    }
                }
            }
            HirInstruction::Call {
                destination,
                block,
                arguments,
            } => {
                self.save_registers(out);

                let arguments_count = arguments.len();
                let mut arguments_size = 0;
                for (argument, word_size) in arguments.into_iter().rev() {
                    let operand = self.lower_operand(argument);
                    arguments_size += 1;
                    out.push(LirInstruction::Mov {
                        size: word_size,
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
                        source: LirOperand::Direct(arguments_size),
                        destination: self.stack_pointer.clone(),
                    });
                }

                self.restore_registers(out);

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: self.return_registers.clone(),
                    destination: self.get_virtual_register(destination),
                });
            }
            HirInstruction::CallPrologue => {
                if let Some(entry) = self.current_function {
                    self.frame_sizes.entry(entry).or_insert(0);
                }

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: self.frame_pointer.clone(),
                    destination: LirOperand::IndirectPreDecrement(Box::new(
                        self.stack_pointer.clone(),
                    )),
                });

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: self.stack_pointer.clone(),
                    destination: self.frame_pointer.clone(),
                });

                out.push(LirInstruction::AllocateStackFrame);
            }
            HirInstruction::InterruptPrologue => {
                if let Some(entry) = self.current_function {
                    self.frame_sizes.entry(entry).or_insert(0);
                }

                self.save_registers(out);

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: self.frame_pointer.clone(),
                    destination: LirOperand::IndirectPreDecrement(Box::new(
                        self.stack_pointer.clone(),
                    )),
                });

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: self.stack_pointer.clone(),
                    destination: self.frame_pointer.clone(),
                });

                out.push(LirInstruction::AllocateStackFrame);
            }
            HirInstruction::LoadParameter {
                destination,
                offset,
                word_size,
            } => {
                out.push(LirInstruction::Mov {
                    size: word_size,
                    source: LirOperand::IndirectOffset {
                        base: Box::new(self.frame_pointer.clone()),
                        offset: Box::new(LirOperand::Direct(offset + 1)),
                    },
                    destination: self.get_virtual_register(destination),
                });
            }
            HirInstruction::AllocateStack { slot } => {
                let entry = self
                    .current_function
                    .expect("Stack allocation outside of function context");
                let size = self.frame_sizes.get_mut(&entry).unwrap();
                *size += 1;
                self.stack_offsets.insert(slot, -*size);
            }
            HirInstruction::StoreStack {
                slot,
                value,
                word_size,
            } => {
                let offset = *self.stack_offsets.get(&slot).unwrap() as u64;
                let value_operand = self.lower_operand(value);
                out.push(LirInstruction::Mov {
                    size: word_size,
                    source: value_operand,
                    destination: LirOperand::IndirectOffset {
                        base: Box::new(self.frame_pointer.clone()),
                        offset: Box::new(LirOperand::Direct(offset)),
                    },
                });
            }
            HirInstruction::LoadStack {
                destination,
                slot,
                word_size,
            } => {
                let offset = *self.stack_offsets.get(&slot).unwrap() as u64;
                out.push(LirInstruction::Mov {
                    size: word_size,
                    source: LirOperand::IndirectOffset {
                        base: Box::new(self.frame_pointer.clone()),
                        offset: Box::new(LirOperand::Direct(offset)),
                    },
                    destination: self.get_virtual_register(destination),
                });
            }
            HirInstruction::LoadField {
                destination,
                object,
                offset,
                word_size,
            } => {
                let object = self.lower_operand(object);
                let base = self.operand_to_register(object, RegisterType::Address, out);
                out.push(LirInstruction::Mov {
                    size: word_size,
                    source: LirOperand::IndirectOffset {
                        base: Box::new(base),
                        offset: Box::new(LirOperand::Direct(offset)),
                    },
                    destination: self.get_virtual_register(destination),
                });
            }
            HirInstruction::StoreField {
                object,
                offset,
                value,
                word_size,
            } => {
                let object = self.lower_operand(object);
                let value = self.lower_operand(value);
                let base = self.operand_to_register(object, RegisterType::Address, out);
                out.push(LirInstruction::Mov {
                    size: word_size,
                    source: value,
                    destination: LirOperand::IndirectOffset {
                        base: Box::new(base),
                        offset: Box::new(LirOperand::Direct(offset)),
                    },
                });
            }
            HirInstruction::AllocateObject { destination, size } => {
                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: self.heap_pointer.clone(),
                    destination: self.get_virtual_register(destination),
                });

                out.push(LirInstruction::Add {
                    size: WordSize::Long,
                    source: LirOperand::Direct(size),
                    destination: self.heap_pointer.clone(),
                });
            }
            HirInstruction::Input {
                destination,
                port,
                word_size,
            } => {
                let port = Self::lower_constant_to_direct(port);
                let destination = self.lower_operand(destination);
                out.push(LirInstruction::In {
                    port,
                    destination,
                    word_size,
                })
            }
            HirInstruction::Output {
                port,
                value,
                word_size,
            } => {
                let port = Self::lower_constant_to_direct(port);
                let value = self.lower_operand(value);
                out.push(LirInstruction::Out {
                    port,
                    value,
                    word_size,
                })
            }
            HirInstruction::LoadIndex {
                destination,
                array,
                index,
                word_size,
            } => {
                let destination = self.get_virtual_register(destination);
                let array = self.lower_operand(array);
                let index = self.lower_operand(index);

                let base = self.operand_to_register(array, RegisterType::Address, out);
                let offset = self.operand_to_register(index, RegisterType::Data, out);
                out.push(LirInstruction::Mov {
                    size: word_size,
                    source: LirOperand::IndirectOffset {
                        base: Box::new(base),
                        offset: Box::new(offset),
                    },
                    destination,
                })
            }
            HirInstruction::StoreIndex {
                array,
                index,
                value,
                word_size,
            } => {
                let array = self.lower_operand(array);
                let index = self.lower_operand(index);
                let value = self.lower_operand(value);

                let base = self.operand_to_register(array, RegisterType::Address, out);
                let offset = self.operand_to_register(index, RegisterType::Data, out);
                out.push(LirInstruction::Mov {
                    size: word_size,
                    source: value,
                    destination: LirOperand::IndirectOffset {
                        base: Box::new(base),
                        offset: Box::new(offset),
                    },
                })
            }
            HirInstruction::AllocateArray { destination, size } => {
                let destination = self.get_virtual_register(destination);

                out.push(LirInstruction::Add {
                    size: WordSize::Long,
                    source: LirOperand::Direct(3),
                    destination: self.heap_pointer.clone(),
                });
                out.push(LirInstruction::And {
                    size: WordSize::Long,
                    source: LirOperand::Direct(0xFFFFFFFFFFFFFFFC),
                    destination: self.heap_pointer.clone(),
                });

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: self.heap_pointer.clone(),
                    destination,
                });
                out.push(LirInstruction::Add {
                    size: WordSize::Long,
                    source: LirOperand::Direct(size),
                    destination: self.heap_pointer.clone(),
                });
            }
            HirInstruction::StoreSlice {
                target,
                start,
                value,
                size,
                word_size,
            } => {
                let target = self.lower_operand(target);
                let value = self.lower_operand(value);
                let start = self.lower_operand(start);

                let destination = self.new_virtual_address_register();
                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: target,
                    destination: destination.clone(),
                });
                out.push(LirInstruction::Add {
                    size: WordSize::Long,
                    source: start,
                    destination: destination.clone(),
                });

                let base = self.operand_to_register(value, RegisterType::Address, out);
                for i in 0..size {
                    out.push(LirInstruction::Mov {
                        size: word_size.clone(),
                        source: LirOperand::IndirectOffset {
                            base: Box::new(base.clone()),
                            offset: Box::new(LirOperand::Direct(i)),
                        },
                        destination: LirOperand::IndirectOffset {
                            base: Box::new(destination.clone()),
                            offset: Box::new(LirOperand::Direct(i)),
                        },
                    });
                }
            }
            HirInstruction::LoadSlice {
                destination,
                array,
                start,
                ..
            } => {
                let destination = self.get_virtual_register(destination);
                let array = self.lower_operand(array);
                let start = self.lower_operand(start);
                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: array,
                    destination: destination.clone(),
                });
                out.push(LirInstruction::Add {
                    size: WordSize::Long,
                    source: start,
                    destination,
                })
            }
            HirInstruction::Not {
                destination,
                operand,
                word_size,
            } => {
                let destination = self.get_virtual_register(destination);
                let operand = self.lower_operand(operand);

                out.push(LirInstruction::Not {
                    size: word_size,
                    source: operand,
                    destination,
                });
            }
            HirInstruction::CopyConstantArray {
                destination,
                id,
                word_size,
            } => {
                let destination = self.get_virtual_register(destination);
                let array_const = &self.array_constants[id];
                let base = array_const.base_address;
                let size = array_const.values.len() as u64;

                for i in 0..size {
                    out.push(LirInstruction::Mov {
                        size: word_size.clone(),
                        source: LirOperand::IndirectDirect(base + i),
                        destination: LirOperand::IndirectOffset {
                            base: Box::new(destination.clone()),
                            offset: Box::new(LirOperand::Direct(i)),
                        },
                    });
                }
            }
        }
    }

    fn lower_constant_to_direct(constant: HirOperand) -> LirOperand {
        if let HirOperand::Constant(constant, _) = constant {
            LirOperand::Direct(constant.parse::<u64>().unwrap())
        } else {
            panic!("Operand is not a constant");
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
                    size: WordSize::Byte,
                    that: condition,
                    with: LirOperand::Direct(1),
                });

                out.push(LirInstruction::Branch {
                    condition: Condition::Equal,
                    label: true_block,
                });

                out.push(LirInstruction::Jmp { label: false_block });
            }
            HirTerminator::Return(operand, word_size) => {
                if let Some(operand) = operand {
                    let return_value = self.lower_operand(operand);
                    out.push(LirInstruction::Mov {
                        size: word_size,
                        source: return_value,
                        destination: self.return_registers.clone(),
                    });
                }

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: self.frame_pointer.clone(),
                    destination: self.stack_pointer.clone(),
                });

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: LirOperand::IndirectPostIncrement(Box::new(self.stack_pointer.clone())),
                    destination: self.frame_pointer.clone(),
                });

                out.push(LirInstruction::Ret);
            }
            HirTerminator::IntReturn => {
                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: self.frame_pointer.clone(),
                    destination: self.stack_pointer.clone(),
                });

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: LirOperand::IndirectPostIncrement(Box::new(self.stack_pointer.clone())),
                    destination: self.frame_pointer.clone(),
                });

                self.restore_registers(out);

                out.push(LirInstruction::IntRet);
            }
        }
    }

    fn create_entry_point(&mut self, main_block_id: BlockId) {
        let instructions = vec![
            LirInstruction::Mov {
                size: WordSize::Long,
                source: LirOperand::Direct(99999),
                destination: self.stack_pointer.clone(),
            },
            LirInstruction::Mov {
                size: WordSize::Long,
                source: LirOperand::Direct(1000),
                destination: self.heap_pointer.clone(),
            },
            LirInstruction::Call {
                label: main_block_id,
            },
            LirInstruction::Halt,
        ];

        let entry_block = LirBlock {
            id: BlockId::MAX,
            instructions,
        };

        self.blocks.insert(0, entry_block);
    }
}

fn normalize_constant(value_str: String) -> String {
    match value_str.as_str() {
        "true" => "1".to_string(),
        "false" => "0".to_string(),
        s if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 3 => {
            let inner = &s[1..s.len() - 1];
            let char_value = match inner {
                "\\n" => '\n' as u64,
                "\\r" => '\r' as u64,
                "\\t" => '\t' as u64,
                "\\\\" => '\\' as u64,
                "\\'" => '\'' as u64,
                "\\0" => 0_u64,
                _ => inner.chars().next().unwrap_or('\0') as u64,
            };
            char_value.to_string()
        }
        _ => value_str,
    }
}

enum MemorySignal {
    Read,
    Write,
    ReadWrite,
}

#[derive(Clone)]
struct LifeInterval {
    pub start: usize,
    pub end: usize,
}

struct RegisterBatch {
    registers: Vec<Option<(usize, LifeInterval)>>,
    active_count: usize,
    max_count: usize,
    offset: usize,
}

pub struct AllocateContext<'a> {
    pub data_restore: usize,
    pub address_restore: usize,
    pub pre: &'a mut Vec<LirInstruction>,
    pub post: &'a mut Vec<LirInstruction>,
}

impl RegisterBatch {
    fn new(max_count: usize, offset: usize) -> Self {
        Self {
            registers: vec![None; max_count],
            active_count: 0,
            max_count,
            offset,
        }
    }

    fn clear_old_registers(&mut self, instruction_counter: usize) {
        for slot in self.registers.iter_mut() {
            if let Some((_, register_interval)) = slot
                && register_interval.end < instruction_counter
            {
                *slot = None;
                self.active_count = self.active_count.saturating_sub(1);
            }
        }
    }
}

impl LirContext {
    fn analyze_instruction(
        counter: usize,
        instruction: &LirInstruction,
        data_interval: &mut HashMap<usize, LifeInterval>,
        address_interval: &mut HashMap<usize, LifeInterval>,
    ) {
        let mut add_interval = |operand: &LirOperand| {
            Self::record_operand_life_interval(operand, counter, data_interval, address_interval);
        };

        match instruction {
            LirInstruction::Mov {
                source,
                destination,
                ..
            }
            | LirInstruction::Add {
                source,
                destination,
                ..
            }
            | LirInstruction::Sub {
                source,
                destination,
                ..
            }
            | LirInstruction::Mul {
                source,
                destination,
                ..
            }
            | LirInstruction::Div {
                source,
                destination,
                ..
            }
            | LirInstruction::Rem {
                source,
                destination,
                ..
            }
            | LirInstruction::And {
                source,
                destination,
                ..
            }
            | LirInstruction::Or {
                source,
                destination,
                ..
            }
            | LirInstruction::Xor {
                source,
                destination,
                ..
            }
            | LirInstruction::Not {
                source,
                destination,
                ..
            } => {
                add_interval(source);
                add_interval(destination);
            }
            LirInstruction::Cmp { that, with, .. } => {
                add_interval(that);
                add_interval(with);
            }
            LirInstruction::SetBool { destination, .. }
            | LirInstruction::In { destination, .. } => {
                add_interval(destination);
            }
            LirInstruction::VAdd { left, right, .. }
            | LirInstruction::VSub { left, right, .. }
            | LirInstruction::VMul { left, right, .. }
            | LirInstruction::VDiv { left, right, .. }
            | LirInstruction::VRem { left, right, .. }
            | LirInstruction::VAnd { left, right, .. }
            | LirInstruction::VOr { left, right, .. }
            | LirInstruction::VXor { left, right, .. }
            | LirInstruction::VCmpBeq { left, right, .. }
            | LirInstruction::VCmpBne { left, right, .. }
            | LirInstruction::VCmpBlt { left, right, .. }
            | LirInstruction::VCmpBle { left, right, .. }
            | LirInstruction::VCmpBgt { left, right, .. }
            | LirInstruction::VCmpBge { left, right, .. } => {
                add_interval(left);
                add_interval(right);
            }
            LirInstruction::VEnd { destination, .. } => {
                add_interval(destination);
            }
            LirInstruction::Out { value, .. } => {
                add_interval(value);
            }
            LirInstruction::Lsl {
                count, destination, ..
            }
            | LirInstruction::Lsr {
                count, destination, ..
            } => {
                add_interval(count);
                add_interval(destination);
            }
            LirInstruction::Jmp { .. }
            | LirInstruction::Branch { .. }
            | LirInstruction::Call { .. }
            | LirInstruction::Ret
            | LirInstruction::IntRet
            | LirInstruction::Halt
            | LirInstruction::AllocateStackFrame => {}
        }
    }

    fn record_operand_life_interval(
        operand: &LirOperand,
        counter: usize,
        data_interval: &mut HashMap<usize, LifeInterval>,
        address_interval: &mut HashMap<usize, LifeInterval>,
    ) {
        match operand {
            LirOperand::VirtualRegister(register, register_type) => {
                let events = match register_type {
                    RegisterType::Data => &mut *data_interval,
                    RegisterType::Address => &mut *address_interval,
                };
                events
                    .entry(*register)
                    .and_modify(|event| event.end = counter)
                    .or_insert(LifeInterval {
                        start: counter,
                        end: counter,
                    });
            }
            LirOperand::Indirect(register)
            | LirOperand::IndirectPostIncrement(register)
            | LirOperand::IndirectPreDecrement(register) => {
                Self::record_operand_life_interval(
                    register,
                    counter,
                    data_interval,
                    address_interval,
                );
            }
            LirOperand::IndirectOffset {
                base,
                offset: offset_register,
            } => {
                Self::record_operand_life_interval(base, counter, data_interval, address_interval);
                Self::record_operand_life_interval(
                    offset_register,
                    counter,
                    data_interval,
                    address_interval,
                );
            }
            _ => {}
        }
    }

    fn process_intervals(
        &mut self,
        intervals: &[(usize, LifeInterval)],
        register_batch: &mut RegisterBatch,
        register_type: RegisterType,
    ) {
        for (virtual_register, interval) in intervals {
            register_batch.clear_old_registers(interval.start);

            if register_batch.active_count < register_batch.max_count {
                for (i, slot) in register_batch.registers.iter_mut().enumerate() {
                    if slot.is_none() {
                        *slot = Some((*virtual_register, interval.clone()));
                        register_batch.active_count += 1;

                        let register = (i + register_batch.offset) as u8;

                        match register_type {
                            RegisterType::Data => self
                                .allocated_data_registers
                                .insert(*virtual_register, register),
                            RegisterType::Address => self
                                .allocated_address_registers
                                .insert(*virtual_register, register),
                        };
                        break;
                    }
                }
            } else {
                let mut spill_candidate = None;
                let mut farthest_end = 0;

                for (i, register) in register_batch.registers.iter().enumerate() {
                    if let Some((_, interval)) = register
                        && interval.end > farthest_end
                    {
                        farthest_end = interval.end;
                        spill_candidate = Some(i);
                    }
                }

                if farthest_end > interval.end {
                    let i = spill_candidate.unwrap();
                    let (spilled_virtual_register, _) = register_batch.registers[i].take().unwrap();

                    register_batch.registers[i] = Some((*virtual_register, interval.clone()));

                    let phys_reg = (i + register_batch.offset) as u8;

                    match register_type {
                        RegisterType::Data => {
                            self.allocated_data_registers
                                .remove(&spilled_virtual_register);
                            self.allocated_data_registers
                                .insert(*virtual_register, phys_reg);
                            self.allocate_spill_register(
                                spilled_virtual_register,
                                RegisterType::Data,
                            );
                        }
                        RegisterType::Address => {
                            self.allocated_address_registers
                                .remove(&spilled_virtual_register);
                            self.allocated_address_registers
                                .insert(*virtual_register, phys_reg);
                            self.allocate_spill_register(
                                spilled_virtual_register,
                                RegisterType::Address,
                            );
                        }
                    }
                } else {
                    self.allocate_spill_register(*virtual_register, register_type);
                }
            }
        }
    }

    fn allocate_spill_register(&mut self, virtual_register: usize, register_type: RegisterType) {
        let entry_id = *self
            .register_to_function
            .get(&virtual_register)
            .expect("Unknown register function origin");
        let frame_size = self.frame_sizes.get_mut(&entry_id).unwrap();
        *frame_size += 1;
        let offset = -(*frame_size);

        match register_type {
            RegisterType::Data => self.spilled_data_registers.insert(virtual_register, offset),
            RegisterType::Address => self
                .spilled_address_registers
                .insert(virtual_register, offset),
        };
    }

    fn compile_virtual_registers(&mut self) {
        let mut instruction_counter = 0;
        let mut data_register_life_interval: HashMap<usize, LifeInterval> = HashMap::new();
        let mut address_register_life_interval: HashMap<usize, LifeInterval> = HashMap::new();

        for block in &mut self.blocks {
            for instruction in &mut block.instructions {
                instruction_counter += 10;
                Self::analyze_instruction(
                    instruction_counter,
                    instruction,
                    &mut data_register_life_interval,
                    &mut address_register_life_interval,
                );
            }
        }

        let mut data_intervals: Vec<(usize, LifeInterval)> =
            data_register_life_interval.into_iter().collect();
        data_intervals.sort_by_key(|(_, event)| event.start);

        let mut address_intervals: Vec<(usize, LifeInterval)> =
            address_register_life_interval.into_iter().collect();
        address_intervals.sort_by_key(|(_, event)| event.start);

        let mut data_register_batch = RegisterBatch::new(5, 1);
        let mut address_register_batch = RegisterBatch::new(3, 0);

        self.process_intervals(
            &data_intervals,
            &mut data_register_batch,
            RegisterType::Data,
        );

        self.process_intervals(
            &address_intervals,
            &mut address_register_batch,
            RegisterType::Address,
        );

        for block in &mut self.blocks {
            let entry_id = *self.block_to_function.get(&block.id).unwrap_or(&block.id);
            let frame_size = *self.frame_sizes.get(&entry_id).unwrap_or(&0);

            for instruction in &mut block.instructions {
                if matches!(instruction, LirInstruction::AllocateStackFrame) {
                    *instruction = LirInstruction::Sub {
                        size: WordSize::Long,
                        source: LirOperand::Direct(frame_size as u64),
                        destination: self.stack_pointer.clone(),
                    };
                }
            }
        }

        let mut blocks = self.blocks.clone();
        for block in &mut blocks {
            let mut new_instructions = Vec::new();
            for mut instruction in block.instructions.drain(..) {
                let mut pre_instructions = Vec::new();
                let mut post_instructions = Vec::new();

                self.allocate_instruction(
                    &mut instruction,
                    &mut pre_instructions,
                    &mut post_instructions,
                );

                new_instructions.extend(pre_instructions);
                new_instructions.push(instruction);
                new_instructions.extend(post_instructions);
            }
            block.instructions = new_instructions;
        }
        self.blocks = blocks;
    }

    fn allocate_instruction(
        &self,
        instruction: &mut LirInstruction,
        pre: &mut Vec<LirInstruction>,
        post: &mut Vec<LirInstruction>,
    ) {
        let mut context = AllocateContext {
            data_restore: 0,
            address_restore: 0,
            pre,
            post,
        };

        let mut allocate_operand = |operand: &mut LirOperand, signal: MemorySignal| {
            self.allocate_operand(operand, signal, &mut context, true);
        };

        match instruction {
            LirInstruction::Mov {
                source,
                destination,
                ..
            } => {
                allocate_operand(source, MemorySignal::Read);
                allocate_operand(destination, MemorySignal::Write);
            }
            LirInstruction::Add {
                source,
                destination,
                ..
            }
            | LirInstruction::Sub {
                source,
                destination,
                ..
            }
            | LirInstruction::Mul {
                source,
                destination,
                ..
            }
            | LirInstruction::Div {
                source,
                destination,
                ..
            }
            | LirInstruction::Rem {
                source,
                destination,
                ..
            }
            | LirInstruction::And {
                source,
                destination,
                ..
            }
            | LirInstruction::Or {
                source,
                destination,
                ..
            }
            | LirInstruction::Xor {
                source,
                destination,
                ..
            } => {
                allocate_operand(source, MemorySignal::Read);
                allocate_operand(destination, MemorySignal::ReadWrite);
            }
            LirInstruction::Not {
                source,
                destination,
                ..
            } => {
                allocate_operand(source, MemorySignal::Read);
                allocate_operand(destination, MemorySignal::Write);
            }
            LirInstruction::Cmp { that, with, .. } => {
                allocate_operand(that, MemorySignal::Read);
                allocate_operand(with, MemorySignal::Read);
            }
            LirInstruction::VAdd { left, right, .. }
            | LirInstruction::VSub { left, right, .. }
            | LirInstruction::VMul { left, right, .. }
            | LirInstruction::VDiv { left, right, .. }
            | LirInstruction::VRem { left, right, .. }
            | LirInstruction::VAnd { left, right, .. }
            | LirInstruction::VOr { left, right, .. }
            | LirInstruction::VXor { left, right, .. }
            | LirInstruction::VCmpBeq { left, right, .. }
            | LirInstruction::VCmpBne { left, right, .. }
            | LirInstruction::VCmpBlt { left, right, .. }
            | LirInstruction::VCmpBle { left, right, .. }
            | LirInstruction::VCmpBgt { left, right, .. }
            | LirInstruction::VCmpBge { left, right, .. } => {
                allocate_operand(left, MemorySignal::Read);
                allocate_operand(right, MemorySignal::Read);
            }
            LirInstruction::VEnd { destination, .. } => {
                allocate_operand(destination, MemorySignal::Write);
            }
            LirInstruction::SetBool { destination, .. }
            | LirInstruction::In { destination, .. } => {
                allocate_operand(destination, MemorySignal::Write);
            }
            LirInstruction::Out { value, .. } => {
                allocate_operand(value, MemorySignal::Read);
            }
            LirInstruction::Lsl {
                count, destination, ..
            }
            | LirInstruction::Lsr {
                count, destination, ..
            } => {
                allocate_operand(count, MemorySignal::Read);
                allocate_operand(destination, MemorySignal::ReadWrite);
            }
            LirInstruction::Jmp { .. }
            | LirInstruction::Branch { .. }
            | LirInstruction::Call { .. }
            | LirInstruction::Ret
            | LirInstruction::IntRet
            | LirInstruction::Halt
            | LirInstruction::AllocateStackFrame => {}
        }
    }

    fn allocate_operand(
        &self,
        operand: &mut LirOperand,
        signal: MemorySignal,
        context: &mut AllocateContext,
        allow_memory: bool,
    ) {
        match operand {
            LirOperand::VirtualRegister(virtual_register, register_type) => {
                let (register_map, spilled_map, restore_registers) = match register_type {
                    RegisterType::Data => (
                        &self.allocated_data_registers,
                        &self.spilled_data_registers,
                        &self.restore_data_registers,
                    ),
                    RegisterType::Address => (
                        &self.allocated_address_registers,
                        &self.spilled_address_registers,
                        &self.restore_address_registers,
                    ),
                };

                if let Some(&register) = register_map.get(virtual_register) {
                    *operand = LirOperand::Register(register, *register_type);
                } else if let Some(&offset) = spilled_map.get(virtual_register) {
                    if allow_memory {
                        *operand = LirOperand::IndirectOffset {
                            base: Box::new(self.frame_pointer.clone()),
                            offset: Box::new(LirOperand::Direct(offset as u64)),
                        };
                    } else {
                        let next_restore_register = match register_type {
                            RegisterType::Data => context.data_restore,
                            RegisterType::Address => context.address_restore,
                        };

                        if next_restore_register >= restore_registers.len() {
                            panic!("Not enough restore registers for this instruction!");
                        }
                        let restore_register = restore_registers[next_restore_register].clone();
                        match register_type {
                            RegisterType::Data => context.data_restore += 1,
                            RegisterType::Address => context.address_restore += 1,
                        }

                        let load_spilled = vec![LirInstruction::Mov {
                            size: WordSize::Long,
                            source: LirOperand::IndirectOffset {
                                base: Box::new(self.frame_pointer.clone()),
                                offset: Box::new(LirOperand::Direct(offset as u64)),
                            },
                            destination: restore_register.clone(),
                        }];

                        let store_spilled = vec![LirInstruction::Mov {
                            size: WordSize::Long,
                            source: restore_register.clone(),
                            destination: LirOperand::IndirectOffset {
                                base: Box::new(self.frame_pointer.clone()),
                                offset: Box::new(LirOperand::Direct(offset as u64)),
                            },
                        }];

                        match signal {
                            MemorySignal::Read => {
                                context.pre.extend(load_spilled);
                            }
                            MemorySignal::Write => {
                                context.post.extend(store_spilled);
                            }
                            MemorySignal::ReadWrite => {
                                context.pre.extend(load_spilled);
                                context.post.extend(store_spilled);
                            }
                        }
                        *operand = restore_register;
                    }
                }
            }
            LirOperand::Indirect(register) => {
                self.allocate_operand(register, MemorySignal::Read, context, false);
            }
            LirOperand::IndirectPostIncrement(register)
            | LirOperand::IndirectPreDecrement(register) => {
                self.allocate_operand(register, MemorySignal::ReadWrite, context, false);
            }
            LirOperand::IndirectOffset {
                base,
                offset: offset_register,
            } => {
                self.allocate_operand(base, MemorySignal::Read, context, false);
                self.allocate_operand(offset_register, MemorySignal::Read, context, false);
                if let LirOperand::Register(register, RegisterType::Data) = **offset_register
                    && register < 5
                {
                    if context.data_restore >= self.restore_data_registers.len() {
                        panic!("Not enough restore registers to legalize IndirectOffset index!");
                    }
                    let valid_register = self.restore_data_registers[context.data_restore].clone();
                    context.data_restore += 1;
                    context.pre.push(LirInstruction::Mov {
                        size: WordSize::Long,
                        source: LirOperand::Register(register, RegisterType::Data),
                        destination: valid_register.clone(),
                    });
                    **offset_register = valid_register;
                }
            }
            _ => {}
        }
    }
}

pub struct LirPackage {
    pub text_section: Vec<LirBlock>,
    pub data_section: HashMap<Address, (u64, WordSize)>,
    pub interrupt_blocks: [BlockId; 8],
}

pub fn compile_lir(control_flow_graph: ControlFlowGraph) -> LirPackage {
    let mut context = LirContext::new(control_flow_graph.register_counter);

    for (values, element_type) in control_flow_graph.array_constants {
        let length = values.len() as u64;
        context.array_constants.push(ArrayConstant {
            values,
            element_type,
            base_address: context.data_size,
        });
        context.data_size += length;
    }

    let entry_point = control_flow_graph.entry_block;
    context.lower(control_flow_graph.blocks);
    context.create_entry_point(entry_point);

    context.compile_virtual_registers();

    let mut data_section = HashMap::new();
    for ((name, _), (address, word_size)) in context.constants {
        let value = name.parse::<i64>().unwrap() as u64;
        data_section.insert(address, (value, word_size));
    }

    for array in &context.array_constants {
        let word_size = match array.element_type {
            Type::Bool | Type::Char => WordSize::Byte,
            _ => WordSize::Long,
        };
        for (i, value_str) in array.values.iter().enumerate() {
            let normalized = normalize_constant(value_str.clone());
            let value = normalized.parse::<i64>().unwrap_or(0) as u64;
            data_section.insert(array.base_address + i as u64, (value, word_size.clone()));
        }
    }

    LirPackage {
        text_section: context.blocks,
        data_section,
        interrupt_blocks: control_flow_graph.interrupt_blocks,
    }
}
