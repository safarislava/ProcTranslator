use crate::isa::WordSize;
use crate::translator::common::ConstantAddress;
use crate::translator::expression::ExpressionBinaryOperator;
use crate::translator::hir::{
    BlockId, ClassInfo, ControlFlowGraph, HirInstruction, HirOperand, HirRegister, HirTerminator,
    StackSlot,
};
use std::collections::HashMap;
use std::vec;

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

    SetBool {
        condition: Condition,
        destination: LirOperand,
    },

    Jmp {
        label: BlockId,
    },
    Branch {
        condition: Condition,
        label: BlockId,
    },
    Call {
        label: BlockId,
    },
    Ret,

    Halt,

    AllocateStackFrame,
}

#[derive(Debug, Clone)]
pub struct LirBlock {
    pub id: BlockId,
    pub instructions: Vec<LirInstruction>,
}

pub struct LirContext {
    classes_size: HashMap<String, u32>,

    pub blocks: Vec<LirBlock>,
    virtual_register_counter: usize,

    stack_size: i64,
    stack_offsets: HashMap<StackSlot, i64>,

    pub constants_size: u64,
    pub constants: HashMap<String, ConstantAddress>,

    return_registers: LirOperand,
    offset_register: LirOperand,

    restore_data_registers: Vec<LirOperand>,
    restore_address_registers: Vec<LirOperand>,

    heap_pointer: LirOperand,
    frame_pointer: LirOperand,
    stack_pointer: LirOperand,

    allocated_data_registers: HashMap<usize, u8>,
    allocated_address_registers: HashMap<usize, u8>,

    spilled_data_registers: HashMap<usize, i64>,
    spilled_address_registers: HashMap<usize, i64>,
}

impl LirContext {
    pub fn new(register_counter: u64) -> Self {
        Self {
            classes_size: HashMap::new(),
            blocks: vec![],
            virtual_register_counter: register_counter as usize,
            stack_size: 0,
            stack_offsets: HashMap::new(),
            constants_size: 0,
            constants: HashMap::new(),
            return_registers: LirOperand::Register(0, RegisterType::Data),
            offset_register: LirOperand::Register(5, RegisterType::Data),
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
        }
    }

    fn next_virtual_register(&mut self, register_type: RegisterType) -> LirOperand {
        let id = self.virtual_register_counter;
        self.virtual_register_counter += 1;
        LirOperand::VirtualRegister(id, register_type)
    }

    fn get_virtual_data_register(&self, register: HirRegister) -> LirOperand {
        LirOperand::VirtualRegister(register.0 as usize, RegisterType::Data)
    }

    fn get_virtual_address_register(&self, register: HirRegister) -> LirOperand {
        LirOperand::VirtualRegister(register.0 as usize, RegisterType::Address)
    }

    fn get_constant_address(&mut self, value: String) -> u64 {
        if let Some(id) = self.constants.get(&value) {
            *id
        } else {
            let address = self.constants_size;
            self.constants.insert(value, address);
            self.constants_size += 8;
            address
        }
    }

    fn lower_operand(&mut self, operand: HirOperand) -> LirOperand {
        match operand {
            HirOperand::Value(register) => self.get_virtual_data_register(register),
            HirOperand::Link(register) => self.get_virtual_address_register(register),
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
                    ExpressionBinaryOperator::Assign => out.push(LirInstruction::Mov {
                        size: WordSize::Long,
                        source: left_operand,
                        destination,
                    }),
                    ExpressionBinaryOperator::Add => out.push(LirInstruction::Add {
                        size: WordSize::Long,
                        source: left_operand,
                        destination,
                    }),
                    ExpressionBinaryOperator::Sub => out.push(LirInstruction::Sub {
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
                    ExpressionBinaryOperator::Remainder => out.push(LirInstruction::Rem {
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

                        out.push(LirInstruction::SetBool {
                            condition,
                            destination,
                        });
                    }
                    _ => unreachable!(),
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
                    source: self.return_registers.clone(),
                    destination: self.get_virtual_data_register(destination),
                });
            }
            HirInstruction::CallPrologue => {
                self.stack_size = 0;

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: self.frame_pointer.clone(),
                    destination: LirOperand::IndirectPreDecrement(Box::new(
                        self.stack_pointer.clone(),
                    )),
                });

                out.push(LirInstruction::Mova {
                    size: WordSize::Long,
                    source: self.stack_pointer.clone(),
                    destination: self.frame_pointer.clone(),
                });

                out.push(LirInstruction::AllocateStackFrame);
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
                        offset: Box::new(offset_register),
                    },
                    destination: self.get_virtual_data_register(destination),
                });
            }
            HirInstruction::StackAllocate { slot } => {
                self.stack_size += 8;
                self.stack_offsets.insert(slot, -self.stack_size);
            }
            HirInstruction::StackStore { slot, value } => {
                let offset = *self.stack_offsets.get(&slot).unwrap() as u64;
                let offset_register = self.next_virtual_register(RegisterType::Data);
                let value_operand = self.lower_operand(value);

                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: LirOperand::Direct(offset),
                    destination: offset_register.clone(),
                });
                out.push(LirInstruction::Mov {
                    size: WordSize::Long,
                    source: value_operand,
                    destination: LirOperand::IndirectOffset {
                        base: Box::new(self.frame_pointer.clone()),
                        offset: Box::new(offset_register),
                    },
                });
            }
            HirInstruction::StackLoad { destination, slot } => {
                let offset = *self.stack_offsets.get(&slot).unwrap() as u64;
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
                        offset: Box::new(offset_register),
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
                let offset = offset as u64 * 8;

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
                        offset: Box::new(offset_register),
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
                let offset = offset as u64 * 8;

                let object_address_register = self.next_virtual_register(RegisterType::Address);
                let offset_register = self.next_virtual_register(RegisterType::Data);

                out.push(LirInstruction::Mov {
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
                        offset: Box::new(offset_register),
                    },
                });
            }
            HirInstruction::AllocateObject {
                destination,
                class_name,
            } => {
                out.push(LirInstruction::Mova {
                    size: WordSize::Long,
                    source: self.heap_pointer.clone(),
                    destination: self.get_virtual_address_register(destination),
                });

                let size = *self.classes_size.get(&class_name).unwrap() as u64;
                out.push(LirInstruction::Add {
                    size: WordSize::Long,
                    source: LirOperand::Direct(size),
                    destination: self.heap_pointer.clone(),
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
                    condition: Condition::Equal,
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
                        destination: self.return_registers.clone(),
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

    fn create_entry_point(&mut self, main_block_id: BlockId) {
        let instructions = vec![
            LirInstruction::Mov {
                size: WordSize::Long,
                source: LirOperand::Direct(99999),
                destination: self.stack_pointer.clone(),
            },
            LirInstruction::Mov {
                size: WordSize::Long,
                source: LirOperand::Direct(40000),
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

impl RegisterBatch {
    pub fn new(max_count: usize, offset: usize) -> Self {
        Self {
            registers: vec![None; max_count],
            active_count: 0,
            max_count,
            offset,
        }
    }

    pub fn clear_old_registers(&mut self, instruction_counter: usize) {
        for register in self.registers.iter_mut() {
            if let Some((_, register_interval)) = register
                && register_interval.end < instruction_counter
            {
                *register = None;
                self.active_count -= 1;
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
            | LirInstruction::Mova {
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
            LirInstruction::SetBool { destination, .. } => {
                add_interval(destination);
            }
            _ => {}
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
        let (allocated_registers, spilled_registers) = match register_type {
            RegisterType::Data => (
                &mut self.allocated_data_registers,
                &mut self.spilled_data_registers,
            ),
            RegisterType::Address => (
                &mut self.allocated_address_registers,
                &mut self.spilled_address_registers,
            ),
        };

        for (virtual_register, interval) in intervals {
            register_batch.clear_old_registers(interval.start);

            if register_batch.active_count < register_batch.max_count {
                for (i, slot) in register_batch.registers.iter_mut().enumerate() {
                    if slot.is_none() {
                        *slot = Some((*virtual_register, interval.clone()));
                        register_batch.active_count += 1;
                        allocated_registers
                            .insert(*virtual_register, (i + register_batch.offset) as u8);
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
                    allocated_registers
                        .insert(*virtual_register, (i + register_batch.offset) as u8);
                    allocated_registers.remove(&spilled_virtual_register);

                    self.stack_size += 8;
                    spilled_registers.insert(spilled_virtual_register, -self.stack_size);
                } else {
                    self.stack_size += 8;
                    spilled_registers.insert(*virtual_register, -self.stack_size);
                }
            }
        }
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

        let mut data_register_batch = RegisterBatch::new(4, 1);
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
            for instruction in &mut block.instructions {
                if matches!(instruction, LirInstruction::AllocateStackFrame) {
                    *instruction = LirInstruction::Sub {
                        size: WordSize::Long,
                        source: LirOperand::Direct(self.stack_size as u64),
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
        let mut next_data_restore = 0;
        let mut next_address_restore = 0;

        let mut allocate_operand = |operand: &mut LirOperand, signal: MemorySignal| {
            self.allocate_operand(
                operand,
                signal,
                &mut next_data_restore,
                &mut next_address_restore,
                pre,
                post,
            );
        };

        match instruction {
            LirInstruction::Mov {
                source,
                destination,
                ..
            }
            | LirInstruction::Mova {
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
            LirInstruction::SetBool { destination, .. } => {
                allocate_operand(destination, MemorySignal::Write);
            }
            _ => {}
        }
    }

    fn allocate_operand(
        &self,
        operand: &mut LirOperand,
        signal: MemorySignal,
        data_restore_register: &mut usize,
        address_restore_register: &mut usize,
        pre: &mut Vec<LirInstruction>,
        post: &mut Vec<LirInstruction>,
    ) {
        match operand {
            LirOperand::VirtualRegister(virtual_register, register_type) => {
                let (register_map, spilled_map, restore_registers, next_restore_register) =
                    match register_type {
                        RegisterType::Data => (
                            &self.allocated_data_registers,
                            &self.spilled_data_registers,
                            &self.restore_data_registers,
                            data_restore_register,
                        ),
                        RegisterType::Address => (
                            &self.allocated_address_registers,
                            &self.spilled_address_registers,
                            &self.restore_address_registers,
                            address_restore_register,
                        ),
                    };

                if let Some(&register) = register_map.get(virtual_register) {
                    *operand = LirOperand::Register(register, *register_type);
                } else if let Some(&offset) = spilled_map.get(virtual_register) {
                    if *next_restore_register >= restore_registers.len() {
                        panic!("Not enough restore registers for this instruction!");
                    }
                    let restore_register = restore_registers[*next_restore_register].clone();
                    *next_restore_register += 1;

                    let load_spilled = vec![
                        LirInstruction::Mov {
                            size: WordSize::Long,
                            source: LirOperand::Direct(offset as u64),
                            destination: self.offset_register.clone(),
                        },
                        LirInstruction::Mov {
                            size: WordSize::Long,
                            source: LirOperand::IndirectOffset {
                                base: Box::new(self.frame_pointer.clone()),
                                offset: Box::new(self.offset_register.clone()),
                            },
                            destination: restore_register.clone(),
                        },
                    ];

                    let store_spilled = vec![
                        LirInstruction::Mov {
                            size: WordSize::Long,
                            source: LirOperand::Direct(offset as u64),
                            destination: self.offset_register.clone(),
                        },
                        LirInstruction::Mov {
                            size: WordSize::Long,
                            source: restore_register.clone(),
                            destination: LirOperand::IndirectOffset {
                                base: Box::new(self.frame_pointer.clone()),
                                offset: Box::new(self.offset_register.clone()),
                            },
                        },
                    ];

                    match signal {
                        MemorySignal::Read => {
                            pre.extend(load_spilled);
                        }
                        MemorySignal::Write => {
                            post.extend(store_spilled);
                        }
                        MemorySignal::ReadWrite => {
                            pre.extend(load_spilled);
                            post.extend(store_spilled);
                        }
                    }
                    *operand = restore_register;
                }
            }
            LirOperand::Indirect(register) => {
                self.allocate_operand(
                    register,
                    MemorySignal::Read,
                    data_restore_register,
                    address_restore_register,
                    pre,
                    post,
                );
            }
            LirOperand::IndirectPostIncrement(register)
            | LirOperand::IndirectPreDecrement(register) => {
                self.allocate_operand(
                    register,
                    MemorySignal::ReadWrite,
                    data_restore_register,
                    address_restore_register,
                    pre,
                    post,
                );
            }
            LirOperand::IndirectOffset {
                base,
                offset: offset_register,
            } => {
                self.allocate_operand(
                    base,
                    MemorySignal::Read,
                    data_restore_register,
                    address_restore_register,
                    pre,
                    post,
                );
                self.allocate_operand(
                    offset_register,
                    MemorySignal::Read,
                    data_restore_register,
                    address_restore_register,
                    pre,
                    post,
                );
                if let LirOperand::Register(register, RegisterType::Data) = **offset_register
                    && register < 4
                {
                    if *data_restore_register >= self.restore_data_registers.len() {
                        panic!("Not enough restore registers to legalize IndirectOffset index!");
                    }
                    let valid_register =
                        self.restore_data_registers[*data_restore_register].clone();
                    *data_restore_register += 1;
                    pre.push(LirInstruction::Mov {
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

impl LirContext {
    fn calculate_classes_size(&mut self, classes: HashMap<String, ClassInfo>) {
        for (name, class) in &classes {
            let class_size = (class.fields.len() * 8) as u32;
            self.classes_size.insert(name.clone(), class_size);
        }
    }
}

pub fn compile_lir(
    control_flow_graph: ControlFlowGraph,
    classes: HashMap<String, ClassInfo>,
) -> (Vec<LirBlock>, HashMap<String, ConstantAddress>) {
    let mut context = LirContext::new(control_flow_graph.register_counter);
    context.calculate_classes_size(classes);

    let entry_point = control_flow_graph.entry_block;
    context.lower(control_flow_graph);
    context.create_entry_point(entry_point);

    context.compile_virtual_registers();

    (context.blocks, context.constants)
}
