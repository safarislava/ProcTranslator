use crate::isa::{Mode, Operator, WordSize};
use crate::translator::common::Address;
use crate::translator::hir::BlockId;
use crate::translator::lir::{Condition, LirBlock, LirInstruction, LirOperand, RegisterType};
use std::collections::HashMap;

pub struct AsmTranslator {
    operators: HashMap<Operator, u8>,
    modes: HashMap<Mode, u8>,
    word_sizes: HashMap<WordSize, u8>,
    pub data: Vec<u8>,
    block_address: HashMap<BlockId, Address>,
    jumps: Vec<(usize, BlockId)>,
}

impl AsmTranslator {
    fn new(
        operators: HashMap<Operator, u8>,
        modes: HashMap<Mode, u8>,
        word_sizes: HashMap<WordSize, u8>,
    ) -> Self {
        Self {
            operators,
            modes,
            word_sizes,
            data: Vec::new(),
            block_address: HashMap::new(),
            jumps: Vec::new(),
        }
    }

    fn add_blocks(&mut self, blocks: Vec<LirBlock>) {
        for block in blocks {
            self.add_block(block);
        }
        self.patch_jumps();
    }

    fn add_block(&mut self, block: LirBlock) {
        self.block_address
            .insert(block.id, self.data.len() as Address);

        for instruction in block.instructions {
            match instruction {
                LirInstruction::Mov {
                    size,
                    source,
                    destination,
                } => {
                    self.translate_standard_instruction(Operator::Mov, size, &source, &destination);
                }
                LirInstruction::Mova {
                    size,
                    source,
                    destination,
                } => {
                    self.translate_standard_instruction(
                        Operator::Mova,
                        size,
                        &source,
                        &destination,
                    );
                }
                LirInstruction::Add {
                    size,
                    source,
                    destination,
                } => {
                    self.translate_standard_instruction(Operator::Add, size, &source, &destination);
                }
                LirInstruction::Sub {
                    size,
                    source,
                    destination,
                } => {
                    self.translate_standard_instruction(Operator::Sub, size, &source, &destination);
                }
                LirInstruction::Mul {
                    size,
                    source,
                    destination,
                } => {
                    self.translate_standard_instruction(Operator::Mul, size, &source, &destination);
                }
                LirInstruction::Div {
                    size,
                    source,
                    destination,
                } => {
                    self.translate_standard_instruction(Operator::Div, size, &source, &destination);
                }
                LirInstruction::Rem {
                    size,
                    source,
                    destination,
                } => {
                    self.translate_standard_instruction(Operator::Rem, size, &source, &destination);
                }
                LirInstruction::And {
                    size,
                    source,
                    destination,
                } => {
                    self.translate_standard_instruction(Operator::And, size, &source, &destination);
                }
                LirInstruction::Or {
                    size,
                    source,
                    destination,
                } => {
                    self.translate_standard_instruction(Operator::Or, size, &source, &destination);
                }
                LirInstruction::Xor {
                    size,
                    source,
                    destination,
                } => {
                    self.translate_standard_instruction(Operator::Xor, size, &source, &destination);
                }
                LirInstruction::Not {
                    size,
                    source,
                    destination,
                } => {
                    self.translate_standard_instruction(Operator::Not, size, &source, &destination);
                }
                LirInstruction::Cmp { size, that, with } => {
                    self.translate_standard_instruction(Operator::Cmp, size, &that, &with);
                }
                LirInstruction::Jmp { label } => {
                    self.translate_branch(Operator::Jmp, label);
                }
                LirInstruction::Branch { condition, label } => {
                    let operator = self.condition_to_operator(condition);
                    self.translate_branch(operator, label);
                }
                LirInstruction::Call { label } => {
                    self.translate_branch(Operator::Call, label);
                }
                LirInstruction::Ret => {
                    let operator_code = self.operators[&Operator::Ret];
                    self.data.push(operator_code << 1);
                }
                LirInstruction::IntRet => {
                    let operator_code = self.operators[&Operator::IntRet];
                    self.data.push(operator_code << 1);
                }
                LirInstruction::SetBool {
                    condition,
                    destination,
                } => {
                    let condition = self.condition_to_operator(condition);

                    let branch_opcode = self.operators[&condition];
                    self.data.push(branch_opcode << 1);
                    let true_address = self.data.len();
                    self.data.extend(vec![0; 8]);

                    self.translate_standard_instruction(
                        Operator::Mov,
                        WordSize::Long,
                        &LirOperand::Direct(0),
                        &destination,
                    );

                    let operator_code = self.operators[&Operator::Jmp];
                    self.data.push(operator_code << 1);
                    let jump_address = self.data.len();
                    self.data.extend(vec![0; 8]);

                    let current_address = self.data.len() as u64;
                    self.data[true_address..true_address + 8]
                        .copy_from_slice(&current_address.to_be_bytes());

                    self.translate_standard_instruction(
                        Operator::Mov,
                        WordSize::Long,
                        &LirOperand::Direct(1),
                        &destination,
                    );

                    let current_address = self.data.len() as u64;
                    self.data[jump_address..jump_address + 8]
                        .copy_from_slice(&current_address.to_be_bytes());
                }
                LirInstruction::In { port, destination } => {
                    self.translate_io(Operator::In, &port, &destination);
                }
                LirInstruction::Out { port, value } => {
                    self.translate_io(Operator::Out, &port, &value);
                }
                LirInstruction::Halt => {
                    let operator_code = self.operators[&Operator::Hlt];
                    self.data.push(operator_code << 1);
                }
                LirInstruction::AllocateStackFrame => {
                    panic!("AllocateStackFrame should be lowered");
                }
            }
        }
    }

    fn translate_operand(&self, operand: &LirOperand) -> (u8, Vec<u8>) {
        match operand {
            LirOperand::Direct(value) => {
                let mode_code = self.modes[&Mode::Direct];
                (mode_code << 5, value.to_be_bytes().to_vec())
            }
            LirOperand::Register(register, RegisterType::Data) => {
                let mode_code = self.modes[&Mode::DataRegister];
                ((mode_code << 5) | (register << 2), vec![])
            }
            LirOperand::Register(register, RegisterType::Address) => {
                let mode_code = self.modes[&Mode::AddressRegister];
                ((mode_code << 5) | (register << 2), vec![])
            }
            LirOperand::Indirect(inner)
            | LirOperand::IndirectPostIncrement(inner)
            | LirOperand::IndirectPreDecrement(inner) => {
                let mode = match operand {
                    LirOperand::Indirect(_) => Mode::Indirect,
                    LirOperand::IndirectPostIncrement(_) => Mode::IndirectPostIncrement,
                    LirOperand::IndirectPreDecrement(_) => Mode::IndirectPreDecrement,
                    _ => unreachable!(),
                };
                if let LirOperand::Register(register, RegisterType::Address) = **inner {
                    let mode_code = self.modes[&mode];
                    ((mode_code << 5) | (register << 2), vec![])
                } else {
                    panic!("Invalid indirect operand");
                }
            }
            LirOperand::IndirectOffset {
                base,
                offset: offset_register,
            } => {
                if let LirOperand::Register(base, RegisterType::Address) = **base {
                    if let LirOperand::Register(offset, RegisterType::Data) = **offset_register {
                        let mode_code = self.modes[&Mode::IndirectOffset];
                        assert!((5..=7).contains(&offset), "Offset register must be D5-D7");
                        let offset_normalized = offset - 4;
                        ((mode_code << 5) | (base << 2) | offset_normalized, vec![])
                    } else if let LirOperand::Direct(offset) = **offset_register {
                        let mode_code = self.modes[&Mode::IndirectOffset];
                        (
                            (mode_code << 5) | (base << 2),
                            offset.to_be_bytes().to_vec(),
                        )
                    } else {
                        panic!("Wrong offset register");
                    }
                } else {
                    panic!("Wrong base register");
                }
            }
            LirOperand::IndirectDirect(address) => {
                let mode_code = self.modes[&Mode::IndirectDirect];
                ((mode_code << 5), address.to_be_bytes().to_vec())
            }
            LirOperand::VirtualRegister(_, _) => unreachable!(),
        }
    }

    fn translate_standard_instruction(
        &mut self,
        operator: Operator,
        size: WordSize,
        source: &LirOperand,
        destination: &LirOperand,
    ) {
        let operator_code = self.operators[&operator];
        let size_code = self.word_sizes[&size];

        let (source_code, source_postcode) = self.translate_operand(source);
        let (destination_code, destination_postcode) = self.translate_operand(destination);

        self.data.push((operator_code << 1) | size_code);
        self.data.push(source_code);
        self.data.push(destination_code);
        self.data.push(0);
        self.data.extend(source_postcode);
        self.data.extend(destination_postcode);
    }

    fn translate_branch(&mut self, operator: Operator, label: BlockId) {
        let operator_code = self.operators[&operator];
        self.data.push(operator_code << 1);

        let patch_offset = self.data.len();
        self.jumps.push((patch_offset, label));

        self.data.extend(vec![0; 8]);
    }

    fn condition_to_operator(&self, condition: Condition) -> Operator {
        match condition {
            Condition::Equal => Operator::Beq,
            Condition::NotEqual => Operator::Bne,
            Condition::Greater => Operator::Bgt,
            Condition::GreaterEqual => Operator::Bge,
            Condition::Lower => Operator::Blt,
            Condition::LowerEqual => Operator::Ble,
            Condition::CarrySet => Operator::Bcs,
            Condition::CarryClear => Operator::Bcc,
            Condition::OverflowSet => Operator::Bvs,
            Condition::OverflowClear => Operator::Bvc,
        }
    }

    fn translate_io(&mut self, operator: Operator, port: &LirOperand, operand: &LirOperand) {
        let operator_code = self.operators[&operator];
        let size_code = self.word_sizes[&WordSize::Long];

        if let LirOperand::Direct(port) = port
            && *port <= u8::MAX as u64
        {
            let port_code = port.to_le_bytes()[0];
            let (value_code, value_postcode) = self.translate_operand(operand);

            self.data.push((operator_code << 1) | size_code);
            self.data.push(port_code);
            self.data.push(value_code);
            self.data.push(0);
            self.data.extend(value_postcode);
        } else {
            panic!("Invalid port")
        }
    }

    fn patch_jumps(&mut self) {
        for (offset, block_id) in &self.jumps {
            if let Some(&target_address) = self.block_address.get(block_id) {
                let address_bytes = target_address.to_be_bytes();
                self.data[*offset..*offset + 8].copy_from_slice(&address_bytes);
            } else {
                panic!(
                    "Label/BlockId {:?} not found during jump patching",
                    block_id
                );
            }
        }
    }

    fn get_interrupt_vectors(&self, interrupt_blocks: [BlockId; 8]) -> [Address; 8] {
        *interrupt_blocks
            .map(|block_id| self.block_address[&block_id] as Address)
            .as_array::<8>()
            .expect("Interrupt vectors aren't valid")
    }
}

pub struct ControlUnitPackage {
    pub program: Vec<u8>,
    pub data: HashMap<Address, u64>,
    pub interrupt_vectors: [Address; 8],
}

impl ControlUnitPackage {
    fn new(program: Vec<u8>, data: HashMap<Address, u64>, interrupt_vectors: [Address; 8]) -> Self {
        Self {
            program,
            data,
            interrupt_vectors,
        }
    }
}

pub fn translate(
    blocks: Vec<LirBlock>,
    data: HashMap<Address, u64>,
    interrupt_blocks: [BlockId; 8],
) -> ControlUnitPackage {
    let mut translator = AsmTranslator::default();
    translator.add_blocks(blocks);
    let interrupt_vectors = translator.get_interrupt_vectors(interrupt_blocks);
    ControlUnitPackage::new(translator.data, data, interrupt_vectors)
}

impl Default for AsmTranslator {
    fn default() -> Self {
        let operators = HashMap::from([
            (Operator::Hlt, 0x00),
            (Operator::Mov, 0x01),
            (Operator::Mova, 0x02),
            (Operator::Add, 0x10),
            (Operator::Adc, 0x11),
            (Operator::Sub, 0x12),
            (Operator::Mul, 0x13),
            (Operator::Div, 0x14),
            (Operator::Rem, 0x15),
            (Operator::And, 0x20),
            (Operator::Or, 0x21),
            (Operator::Xor, 0x22),
            (Operator::Not, 0x23),
            (Operator::Lsl, 0x24),
            (Operator::Lsr, 0x31),
            (Operator::Asl, 0x32),
            (Operator::Asr, 0x33),
            (Operator::Jmp, 0x40),
            (Operator::Call, 0x41),
            (Operator::Ret, 0x42),
            (Operator::IntRet, 0x43),
            (Operator::Beq, 0x50),
            (Operator::Bne, 0x51),
            (Operator::Bgt, 0x52),
            (Operator::Bge, 0x53),
            (Operator::Blt, 0x54),
            (Operator::Ble, 0x55),
            (Operator::Bcs, 0x56),
            (Operator::Bcc, 0x57),
            (Operator::Bvs, 0x58),
            (Operator::Bvc, 0x59),
            (Operator::Cmp, 0x60),
            (Operator::In, 0x70),
            (Operator::Out, 0x71),
            (Operator::EI, 0x72),
            (Operator::DI, 0x73),
        ]);
        let modes = HashMap::from([
            (Mode::Direct, 0x0),
            (Mode::DataRegister, 0x1),
            (Mode::AddressRegister, 0x2),
            (Mode::Indirect, 0x3),
            (Mode::IndirectPostIncrement, 0x4),
            (Mode::IndirectPreDecrement, 0x5),
            (Mode::IndirectOffset, 0x6),
            (Mode::IndirectDirect, 0x7),
        ]);
        let word_sizes = HashMap::from([(WordSize::Byte, 0b0), (WordSize::Long, 0b1)]);
        Self::new(operators, modes, word_sizes)
    }
}
