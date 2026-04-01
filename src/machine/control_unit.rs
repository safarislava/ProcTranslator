use crate::isa::{Mode, Operand, Operator, WordSize};
use crate::machine::alu::AluOperator;
use crate::machine::data_path::{
    AluInputSelector, BufferSelector, DataPath, DataSelector, ExternalSelector, PostModeSelector,
    PreModeSelector,
};
use crate::machine::instruction_parser::InstructionParser;
use crate::machine::memory::Memory;
use crate::machine::stack::Stack;
use crate::translator::common::Address;
use std::collections::HashMap;
use tracing::{debug, info};

pub enum Order {
    Master,
    Slave,
}

pub enum PcSelector {
    NextByte,
    NextWord,
    NextTwoWords,
    SetByDecoder,
    SetByStack,
}

enum StackSelector {
    Current,
    Next,
}

enum ExecutionState {
    Fetch,
    Execute(u8),
    Done,
    Stop,
}

pub struct ControlUnit {
    instruction_parser: InstructionParser,
    data_path: DataPath,
    program_memory: Memory,
    stack: Stack,
    decoder_output: u64,
    pc: u64,
    read_data: u64,
    ir: u32,
    word_size: WordSize,
    operator: Operator,
    operands: Vec<Operand>,
    execution_state: ExecutionState,
    tick: u64,
}

impl ControlUnit {
    fn tick(&mut self) {
        info!("TICK {}", self.tick);
        self.tick += 1;
    }
    pub fn set_pc(&mut self, pc: u64) {
        self.pc = pc;
    }

    pub fn latch_pc(&mut self, signal: PcSelector) {
        self.pc = match signal {
            PcSelector::NextByte => self.pc + 1,
            PcSelector::NextWord => self.pc + 4,
            PcSelector::NextTwoWords => self.pc + 8,
            PcSelector::SetByDecoder => self.decoder_output,
            PcSelector::SetByStack => self.stack.pop(),
        };
    }

    pub fn latch_read_data(&mut self) {
        self.read_data = self.program_memory.read_u64(self.pc);
    }

    pub fn latch_ir(&mut self) {
        self.ir = self.program_memory.read_u32(self.pc);
    }

    pub fn step(&mut self) -> bool {
        match self.execution_state {
            ExecutionState::Fetch => {
                self.fetch();
                false
            }
            ExecutionState::Execute(step) => {
                self.execute_step(step);
                false
            }
            ExecutionState::Done => {
                debug!("D {:?}", self.data_path.data_registers);
                debug!("A {:?}", self.data_path.address_registers);
                self.execution_state = ExecutionState::Fetch;
                false
            }
            ExecutionState::Stop => true,
        }
    }

    fn fetch(&mut self) -> bool {
        self.tick();
        self.latch_ir();

        let (operator, word_size) = self.instruction_parser.parse_operator(self.ir);

        self.word_size = word_size;
        self.operator = operator;

        debug!(
            "PC={} : {}.{}",
            self.pc,
            self.operator,
            match self.word_size {
                WordSize::Byte => "b",
                WordSize::Long => "w",
            }
        );

        match self.operator {
            Operator::Hlt => {}
            Operator::Mov
            | Operator::Mova
            | Operator::Add
            | Operator::Adc
            | Operator::Sub
            | Operator::Mul
            | Operator::Div
            | Operator::Rem
            | Operator::And
            | Operator::Or
            | Operator::Xor
            | Operator::Not
            | Operator::Lsl
            | Operator::Lsr
            | Operator::Asl
            | Operator::Asr
            | Operator::Cmp => self.latch_pc(PcSelector::NextWord),
            Operator::Jmp
            | Operator::Call
            | Operator::Ret
            | Operator::Beq
            | Operator::Bne
            | Operator::Bgt
            | Operator::Bge
            | Operator::Blt
            | Operator::Ble
            | Operator::Bcs
            | Operator::Bcc
            | Operator::Bvs
            | Operator::Bvc => self.latch_pc(PcSelector::NextByte),
        }

        self.execution_state = ExecutionState::Execute(0);
        false
    }

    fn execute_step(&mut self, step: u8) {
        match self.operator {
            Operator::Hlt => {
                self.execution_state = ExecutionState::Stop;
                info!("Result : {}", self.data_path.data_registers[0]);
            }
            Operator::Mov => self.execute_operator(step, AluOperator::Trr),
            Operator::Mova => self.execute_operator(step, AluOperator::Trr), // todo
            Operator::Add => self.execute_operator(step, AluOperator::Add),
            Operator::Adc => self.execute_operator(step, AluOperator::Adc),
            Operator::Sub => self.execute_operator(step, AluOperator::Sub),
            Operator::Mul => self.execute_operator(step, AluOperator::Mul),
            Operator::Div => self.execute_operator(step, AluOperator::Div),
            Operator::Rem => self.execute_operator(step, AluOperator::Rem),
            Operator::And => self.execute_operator(step, AluOperator::And),
            Operator::Or => self.execute_operator(step, AluOperator::Or),
            Operator::Xor => self.execute_operator(step, AluOperator::Xor),
            Operator::Not => self.execute_operator(step, AluOperator::Not),
            Operator::Lsl => self.execute_operator(step, AluOperator::Lsl),
            Operator::Lsr => self.execute_operator(step, AluOperator::Lsr),
            Operator::Asl => self.execute_operator(step, AluOperator::Asl),
            Operator::Asr => self.execute_operator(step, AluOperator::Asr),
            Operator::Jmp => self.execute_jump(step),
            Operator::Call => self.execute_call(step, StackSelector::Next),
            Operator::Ret => self.execute_return(step),
            Operator::Beq => self.execute_branch(step, self.data_path.transmit_nzcv().zero),
            Operator::Bne => self.execute_branch(step, !self.data_path.transmit_nzcv().zero),
            Operator::Bgt => {
                let nzcv = self.data_path.transmit_nzcv();
                self.execute_branch(step, !nzcv.zero && nzcv.negative == nzcv.overflow)
            }
            Operator::Bge => {
                let nzcv = self.data_path.transmit_nzcv();
                self.execute_branch(step, nzcv.negative == nzcv.overflow)
            }
            Operator::Blt => {
                let nzcv = self.data_path.transmit_nzcv();
                self.execute_branch(step, nzcv.negative != nzcv.overflow);
            }
            Operator::Ble => {
                let nzcv = self.data_path.transmit_nzcv();
                self.execute_branch(step, nzcv.zero && nzcv.negative != nzcv.overflow);
            }
            Operator::Bcs => self.execute_branch(step, self.data_path.transmit_nzcv().carry),
            Operator::Bcc => self.execute_branch(step, !self.data_path.transmit_nzcv().carry),
            Operator::Bvs => self.execute_branch(step, self.data_path.transmit_nzcv().overflow),
            Operator::Bvc => self.execute_branch(step, !self.data_path.transmit_nzcv().overflow),
            Operator::Cmp => self.execute_cmp(step),
        }
    }

    pub fn parse_register(&mut self, byte: u8) -> Operand {
        let offset = (3 - byte) * 8;
        let operand = ((self.ir & (0xff << offset)) >> offset) as u8;
        let operand = self.instruction_parser.parse_operand(operand);
        assert!(operand.mode == Mode::AddressRegister || operand.mode == Mode::DataRegister);
        operand
    }

    pub fn parse_data_readable(&mut self, byte: u8) -> Operand {
        let offset = (3 - byte) * 8;
        let operand = ((self.ir & (0xff << offset)) >> offset) as u8;
        self.instruction_parser.parse_operand(operand)
    }

    pub fn parse_data_writable(&mut self, byte: u8) -> Operand {
        let offset = (3 - byte) * 8;
        let operand = ((self.ir & (0xff << offset)) >> offset) as u8;
        let operand = self.instruction_parser.parse_operand(operand);
        assert!(operand.mode != Mode::Direct);
        operand
    }

    fn is_operand_needed_second_step(operand: &Operand) -> bool {
        matches!(
            operand.mode,
            Mode::Indirect
                | Mode::IndirectPreDecrement
                | Mode::IndirectPostIncrement
                | Mode::IndirectOffset
                | Mode::IndirectDirect
        )
    }

    fn execute_operator(&mut self, step: u8, alu_op: AluOperator) {
        self.tick();
        match step {
            0 => {
                let first = self.parse_data_readable(1);
                let second = self.parse_data_writable(2);
                self.operands = vec![first.clone(), second];

                self.prepare_operand(Order::Slave, &first);

                if Self::is_operand_needed_second_step(&first) {
                    self.execution_state = ExecutionState::Execute(1);
                } else {
                    self.execution_state = ExecutionState::Execute(2);
                }
            }
            1 => {
                self.prepare_operand_read_data(Order::Slave);
                self.execution_state = ExecutionState::Execute(2);
            }
            2 => {
                let second = self.operands[1].clone();
                self.prepare_operand(Order::Master, &second);

                if Self::is_operand_needed_second_step(&second) {
                    self.execution_state = ExecutionState::Execute(3);
                } else {
                    self.data_path.execute_alu(alu_op);
                    self.store_by_second_operand(second);

                    self.execution_state = ExecutionState::Done;
                }
            }
            3 => {
                let second = self.operands[1].clone();
                self.prepare_operand_read_data(Order::Master);

                self.data_path.execute_alu(alu_op);
                self.store_by_second_operand(second);

                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn execute_cmp(&mut self, step: u8) {
        match step {
            0 => {
                let first = self.parse_data_readable(1);
                let second = self.parse_data_readable(2);
                self.operands = vec![first.clone(), second];

                self.prepare_operand(Order::Slave, &first);

                if Self::is_operand_needed_second_step(&first) {
                    self.execution_state = ExecutionState::Execute(1);
                } else {
                    self.execution_state = ExecutionState::Execute(2);
                }
            }
            1 => {
                self.prepare_operand_read_data(Order::Slave);
                self.execution_state = ExecutionState::Execute(2);
            }
            2 => {
                let second = self.operands[1].clone();
                self.prepare_operand(Order::Master, &second);

                if Self::is_operand_needed_second_step(&second) {
                    self.execution_state = ExecutionState::Execute(3);
                } else {
                    self.data_path.execute_alu(AluOperator::Sub);
                    self.execution_state = ExecutionState::Done;
                }
            }
            3 => {
                self.prepare_operand_read_data(Order::Master);
                self.data_path.execute_alu(AluOperator::Sub);
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    pub fn execute_jump(&mut self, step: u8) {
        match step {
            0 => {
                self.latch_read_data();
                self.decoder_output = self.read_data;
                self.latch_pc(PcSelector::SetByDecoder);
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn execute_call(&mut self, step: u8, selector: StackSelector) {
        match step {
            0 => {
                self.latch_read_data();
                self.decoder_output = self.read_data;
                let offset = match selector {
                    StackSelector::Current => 0,
                    StackSelector::Next => 8,
                };
                self.stack.push(self.pc + offset);
                self.latch_pc(PcSelector::SetByDecoder);
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    pub fn execute_return(&mut self, step: u8) {
        match step {
            0 => {
                self.latch_pc(PcSelector::SetByStack);
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    pub fn execute_branch(&mut self, step: u8, condition: bool) {
        match step {
            0 => {
                self.latch_read_data();
                self.decoder_output = self.read_data;
                if condition {
                    self.latch_pc(PcSelector::SetByDecoder);
                } else {
                    self.latch_pc(PcSelector::NextTwoWords)
                }
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    pub fn prepare_operand(&mut self, order: Order, operand: &Operand) {
        match operand.mode {
            Mode::Direct => {
                self.latch_read_data();
                self.latch_pc(PcSelector::NextTwoWords);
                self.decoder_output = self.read_data;
                self.data_path.control_unit_output = self.decoder_output as i64;
                self.data_path.external_selector = ExternalSelector::ControlUnit;
                match order {
                    Order::Master => self.data_path.update_left_data(DataSelector::External),
                    Order::Slave => self.data_path.update_right_data(DataSelector::External),
                }
                debug!("#{}", self.decoder_output);
            }
            Mode::DataRegister => {
                self.data_path.read_data_register(operand.main_register);
                match order {
                    Order::Master => self.data_path.update_left_data(DataSelector::DataRegister),
                    Order::Slave => self.data_path.update_right_data(DataSelector::DataRegister),
                }
                debug!("D{}", operand.main_register);
            }
            Mode::AddressRegister => {
                self.data_path.read_address_register(operand.main_register);
                match order {
                    Order::Master => self
                        .data_path
                        .update_left_data(DataSelector::AddressRegister),
                    Order::Slave => self
                        .data_path
                        .update_right_data(DataSelector::AddressRegister),
                }
                debug!("A{}", operand.main_register);
            }
            Mode::Indirect | Mode::IndirectPostIncrement | Mode::IndirectPreDecrement => {
                self.data_path.read_address_register(operand.main_register);
                match order {
                    Order::Master => {
                        self.data_path
                            .update_left_data(DataSelector::AddressRegister);
                        self.data_path.update_left_alu_input(AluInputSelector::Data);
                        self.data_path.execute_alu(AluOperator::Trl);
                    }
                    Order::Slave => {
                        self.data_path
                            .update_right_data(DataSelector::AddressRegister);
                        self.data_path
                            .update_right_alu_input(AluInputSelector::Data);
                        self.data_path.execute_alu(AluOperator::Trr);
                    }
                }
                match operand.mode {
                    Mode::Indirect => {}
                    Mode::IndirectPreDecrement => {
                        self.data_path.pre_mode_selector = match self.word_size {
                            WordSize::Byte => PreModeSelector::DecrementByte,
                            WordSize::Long => PreModeSelector::DecrementWord,
                        };
                    }
                    Mode::IndirectPostIncrement => {
                        self.data_path.post_mode_selector = match self.word_size {
                            WordSize::Byte => PostModeSelector::IncrementByte,
                            WordSize::Long => PostModeSelector::IncrementWord,
                        };
                    }
                    _ => unreachable!(),
                }
                self.data_path.latch_data_address();
                self.data_path.read_data_memory();
                self.data_path
                    .latch_address_register(operand.main_register, &self.word_size);

                match operand.mode {
                    Mode::Indirect => debug!("(A{})", operand.main_register),
                    Mode::IndirectPreDecrement => debug!("(A{})+", operand.main_register),
                    Mode::IndirectPostIncrement => debug!("-(A{})", operand.main_register),
                    _ => unreachable!(),
                }
            }
            Mode::IndirectOffset => {
                self.data_path.read_address_register(operand.main_register);
                match order {
                    Order::Master => {
                        self.data_path
                            .update_left_data(DataSelector::AddressRegister);
                    }
                    Order::Slave => {
                        self.data_path
                            .update_right_data(DataSelector::AddressRegister);
                    }
                }

                if operand.offset == 0 {
                    self.latch_read_data();
                    self.latch_pc(PcSelector::NextTwoWords);
                    self.decoder_output = self.read_data;
                    self.data_path.control_unit_output = self.decoder_output as i64;
                    self.data_path.external_selector = ExternalSelector::ControlUnit;
                    match order {
                        Order::Master => {
                            self.data_path.update_right_buffer(BufferSelector::External);
                        }
                        Order::Slave => {
                            self.data_path.update_left_buffer(BufferSelector::External);
                        }
                    }
                    debug!(
                        "(A{}:#{})",
                        operand.main_register, self.decoder_output as i64
                    );
                } else {
                    let offset_register = operand.offset + 4;
                    self.data_path.read_data_register(offset_register);
                    debug!("(A{}:D{})", operand.main_register, offset_register);
                }

                match order {
                    Order::Master => {
                        self.data_path.update_left_alu_input(AluInputSelector::Data);
                        self.data_path
                            .update_right_alu_input(AluInputSelector::Buffer);
                    }
                    Order::Slave => {
                        self.data_path
                            .update_left_alu_input(AluInputSelector::Buffer);
                        self.data_path
                            .update_right_alu_input(AluInputSelector::Data);
                    }
                }
                self.data_path.execute_alu(AluOperator::Add);
                self.data_path.latch_data_address();
                self.data_path.read_data_memory();
            }
            Mode::IndirectDirect => {
                self.latch_read_data();
                self.latch_pc(PcSelector::NextTwoWords);
                self.decoder_output = self.read_data;
                self.data_path.control_unit_output = self.decoder_output as i64;
                match order {
                    Order::Master => {
                        self.data_path.external_selector = ExternalSelector::ControlUnit;
                        self.data_path.update_left_data(DataSelector::External);
                        self.data_path.update_left_alu_input(AluInputSelector::Data);
                        self.data_path.execute_alu(AluOperator::Trl);
                    }
                    Order::Slave => {
                        self.data_path.external_selector = ExternalSelector::ControlUnit;
                        self.data_path.update_right_data(DataSelector::External);
                        self.data_path
                            .update_right_alu_input(AluInputSelector::Data);
                        self.data_path.execute_alu(AluOperator::Trr);
                    }
                }
                self.data_path.latch_data_address();
                self.data_path.read_data_memory();

                debug!("(#{})", self.decoder_output);
            }
        }
        self.data_path.update_left_alu_input(AluInputSelector::Data);
        self.data_path
            .update_right_alu_input(AluInputSelector::Data);
        self.data_path.pre_mode_selector = PreModeSelector::None;
        self.data_path.post_mode_selector = PostModeSelector::None;
    }

    pub fn prepare_operand_read_data(&mut self, order: Order) {
        self.data_path.latch_read_data();
        match order {
            Order::Master => {
                self.data_path.update_left_data(DataSelector::ReadData);
            }
            Order::Slave => {
                self.data_path.update_right_data(DataSelector::ReadData);
            }
        }
        debug!("Value={}", self.data_path.read_data);

        self.data_path.update_left_alu_input(AluInputSelector::Data);
        self.data_path
            .update_right_alu_input(AluInputSelector::Data);
        self.data_path.pre_mode_selector = PreModeSelector::None;
        self.data_path.post_mode_selector = PostModeSelector::None;
    }

    pub fn store_by_second_operand(&mut self, operand: Operand) {
        match operand.mode {
            Mode::DataRegister => {
                self.data_path
                    .latch_data_register(operand.main_register, &self.word_size);
            }
            Mode::AddressRegister => {
                self.data_path
                    .latch_address_register(operand.main_register, &self.word_size);
            }
            Mode::Indirect
            | Mode::IndirectPostIncrement
            | Mode::IndirectPreDecrement
            | Mode::IndirectOffset
            | Mode::IndirectDirect => {
                self.data_path.latch_write_data();
                self.data_path.write_data_memory(&self.word_size);
                debug!(
                    "Store ({}) = {}",
                    self.data_path.data_address, self.data_path.write_data
                )
            }
            _ => unreachable!(),
        }
    }
}

impl ControlUnit {
    pub fn load_program(&mut self, program: &[u8]) {
        for (i, word) in program.iter().enumerate() {
            self.program_memory.write_u8(i as u64, *word);
        }
    }

    pub fn load_data_section(&mut self, data_section: HashMap<Address, u64>) {
        for (address, value) in data_section {
            self.data_path.data_memory.write_u64(address, value);
        }
    }
}

impl Default for ControlUnit {
    fn default() -> Self {
        Self {
            instruction_parser: InstructionParser::new(),
            data_path: DataPath::default(),
            program_memory: Memory::new(100000),
            stack: Stack::new(),
            read_data: 0,
            ir: 0,
            pc: 0,
            decoder_output: 0,
            word_size: WordSize::Long,
            operator: Operator::Hlt,
            operands: vec![],
            execution_state: ExecutionState::Fetch,
            tick: 0,
        }
    }
}
