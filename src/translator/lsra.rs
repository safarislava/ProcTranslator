use crate::isa::WordSize;
use crate::translator::lir::{LirContext, LirInstruction, LirOperand, RegisterType};
use std::collections::HashMap;

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

    pub fn compile_virtual_registers(&mut self) {
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
