use crate::isa::{Mode, Operand, Operator, WordSize};
use crate::machine::alu::AluOperator;
use crate::machine::data_path::{
    AluInputSelector, BufferSelector, DataPath, DataSelector, PostModeSelector, PreModeSelector,
};
use crate::machine::instruction_parser::InstructionParser;
use crate::machine::memory::Memory;
use crate::machine::stack::Stack;
use crate::translator::common::ConstantAddress;
use std::collections::HashMap;
use tracing::{debug, info};

pub enum Order {
    First,
    Second,
}

pub enum PcSelector {
    NextByte,
    NextWord,
    NextTwoWords,
    SetByDataPath,
    SetByDecoder,
    SetByStack,
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

    pc: u64,
    read_data: u64,
    data_path_output: i64,
    decoder_output: u64,
    word_size: WordSize,

    current_operator: Operator,
    current_operands: Vec<Operand>,
    execution_state: ExecutionState,
}

impl ControlUnit {
    pub fn set_pc(&mut self, pc: u64) {
        self.pc = pc;
    }

    pub fn latch_pc(&mut self, signal: PcSelector) {
        self.pc = match signal {
            PcSelector::NextByte => self.pc + 1,
            PcSelector::NextWord => self.pc + 4,
            PcSelector::NextTwoWords => self.pc + 8,
            PcSelector::SetByDataPath => self.data_path_output as u64,
            PcSelector::SetByDecoder => self.decoder_output,
            PcSelector::SetByStack => self.stack.pop(),
        };
    }

    pub fn latch_read_data(&mut self) {
        self.read_data = self.program_memory.read_u64(self.pc);
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
                debug!("{:?}", self.data_path.data_registers);
                debug!("{:?}", self.data_path.address_registers);

                self.execution_state = ExecutionState::Fetch;
                false
            }
            ExecutionState::Stop => true,
        }
    }

    fn fetch(&mut self) -> bool {
        self.latch_read_data();

        let (operator, word_size) = self
            .instruction_parser
            .parse_operator((self.read_data >> 32) as u32);

        self.word_size = word_size;
        self.current_operator = operator;

        info!(
            "PC={} : {}.{}",
            self.pc,
            self.current_operator,
            match self.word_size {
                WordSize::Byte => "b",
                WordSize::Long => "w",
            }
        );

        self.execution_state = ExecutionState::Execute(0);
        false
    }

    fn execute_step(&mut self, step: u8) {
        match self.current_operator {
            Operator::Hlt => {
                self.execution_state = ExecutionState::Stop;
                info!("Result : {}", self.data_path.data_registers[0]);
            }
            Operator::Mov => self.execute_standard_alu(step, AluOperator::Trr),
            Operator::Mova => self.execute_standard_alu(step, AluOperator::Trr), // todo
            Operator::Add => self.execute_standard_alu(step, AluOperator::Add),
            Operator::Adc => self.execute_standard_alu(step, AluOperator::Adc),
            Operator::Sub => self.execute_standard_alu(step, AluOperator::Sub),
            Operator::Mul => self.execute_standard_alu(step, AluOperator::Mul),
            Operator::Div => self.execute_standard_alu(step, AluOperator::Div),
            Operator::Rem => self.execute_standard_alu(step, AluOperator::Rem),
            Operator::And => self.execute_standard_alu(step, AluOperator::And),
            Operator::Or => self.execute_standard_alu(step, AluOperator::Or),
            Operator::Xor => self.execute_standard_alu(step, AluOperator::Xor),
            Operator::Not => self.execute_standard_alu(step, AluOperator::Not),
            Operator::Lsl => self.execute_standard_alu(step, AluOperator::Lsl),
            Operator::Lsr => self.execute_standard_alu(step, AluOperator::Lsr),
            Operator::Asl => self.execute_standard_alu(step, AluOperator::Asl),
            Operator::Asr => self.execute_standard_alu(step, AluOperator::Asr),
            Operator::Jmp => self.execute_jump(step),
            Operator::Call => self.execute_call(step),
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
        let operand = ((self.read_data & (0xff << offset)) >> offset) as u8;
        let operand = self.instruction_parser.parse_operand(operand);
        assert!(operand.mode == Mode::AddressRegister || operand.mode == Mode::DataRegister);
        operand
    }

    pub fn parse_data_readable(&mut self, byte: u8) -> Operand {
        let offset = (7 - byte) * 8;
        let operand = ((self.read_data & (0xff << offset)) >> offset) as u8;
        self.instruction_parser.parse_operand(operand)
    }

    pub fn parse_data_writable(&mut self, byte: u8) -> Operand {
        let offset = (7 - byte) * 8;
        let operand = ((self.read_data & (0xff << offset)) >> offset) as u8;
        let operand = self.instruction_parser.parse_operand(operand);
        assert!(operand.mode != Mode::Direct);
        operand
    }

    fn execute_standard_alu(&mut self, step: u8, alu_op: AluOperator) {
        match step {
            0 => {
                let first = self.parse_data_readable(1);
                let second = self.parse_data_writable(2);
                self.current_operands = vec![first.clone(), second];
                self.latch_pc(PcSelector::NextWord);
                self.prepare_operand(Order::Second, &first);
                self.execution_state = ExecutionState::Execute(1);
            }
            1 => {
                let second = self.current_operands[1].clone();
                self.prepare_operand(Order::First, &second);
                self.execution_state = ExecutionState::Execute(2);
            }
            2 => {
                let second = self.current_operands[1].clone();
                self.data_path.execute_alu(alu_op);
                self.store_by_second_operand(second);
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    pub fn execute_jump(&mut self, step: u8) {
        match step {
            0 => {
                self.latch_pc(PcSelector::NextByte);
                self.execution_state = ExecutionState::Execute(1);
            }
            1 => {
                self.latch_read_data();
                self.decoder_output = self.read_data;
                self.latch_pc(PcSelector::SetByDecoder);
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    pub fn execute_call(&mut self, step: u8) {
        match step {
            0 => {
                self.latch_pc(PcSelector::NextByte);
                self.stack.push(self.pc + 8);

                self.execution_state = ExecutionState::Execute(1);
            }
            1 => {
                self.latch_read_data();
                self.decoder_output = self.read_data;
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

    fn execute_cmp(&mut self, step: u8) {
        match step {
            0 => {
                let first = self.parse_data_readable(1);
                let second = self.parse_data_readable(2);
                self.current_operands = vec![first.clone(), second];
                self.latch_pc(PcSelector::NextWord);
                self.prepare_operand(Order::First, &first);
                self.execution_state = ExecutionState::Execute(1);
            }
            1 => {
                let second = self.current_operands[1].clone();
                self.prepare_operand(Order::Second, &second);
                self.execution_state = ExecutionState::Execute(2);
            }
            2 => {
                let second = self.current_operands[1].clone();
                self.data_path.execute_alu(AluOperator::Sub);
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    pub fn execute_branch(&mut self, step: u8, condition: bool) {
        match step {
            0 => {
                self.latch_pc(PcSelector::NextByte);

                self.execution_state = ExecutionState::Execute(1);
            }
            1 => {
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
                match order {
                    Order::First => self.data_path.update_left_data(DataSelector::ControlUnit),
                    Order::Second => self.data_path.update_right_data(DataSelector::ControlUnit),
                }
            }
            Mode::DataRegister => {
                self.data_path.read_data_register(operand.main_register);
                match order {
                    Order::First => self.data_path.update_left_data(DataSelector::DataRegister),
                    Order::Second => self.data_path.update_right_data(DataSelector::DataRegister),
                }
            }
            Mode::AddressRegister => {
                self.data_path.read_address_register(operand.main_register);
                match order {
                    Order::First => self
                        .data_path
                        .update_left_data(DataSelector::AddressRegister),
                    Order::Second => self
                        .data_path
                        .update_right_data(DataSelector::AddressRegister),
                }
            }
            Mode::Indirect | Mode::IndirectPostIncrement | Mode::IndirectPreDecrement => {
                self.data_path.read_address_register(operand.main_register);
                match order {
                    Order::First => {
                        self.data_path
                            .update_left_data(DataSelector::AddressRegister);
                        self.data_path.update_left_alu_input(AluInputSelector::Data);
                        self.data_path.execute_alu(AluOperator::Trl);
                    }
                    Order::Second => {
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
                self.data_path.latch_read_data();
                self.data_path
                    .latch_address_register(operand.main_register, &self.word_size);
                match order {
                    Order::First => {
                        self.data_path.update_left_data(DataSelector::ReadData);
                    }
                    Order::Second => {
                        self.data_path.update_right_data(DataSelector::ReadData);
                    }
                }
            }
            Mode::IndirectOffset => {
                self.data_path.read_address_register(operand.main_register);
                match order {
                    Order::First => {
                        self.data_path
                            .update_left_data(DataSelector::AddressRegister);
                    }
                    Order::Second => {
                        self.data_path
                            .update_right_data(DataSelector::AddressRegister);
                    }
                }

                if operand.offset == 0 {
                    self.latch_read_data();
                    self.latch_pc(PcSelector::NextTwoWords);
                    self.decoder_output = self.read_data;
                    self.data_path.control_unit_output = self.decoder_output as i64;
                    match order {
                        Order::First => self
                            .data_path
                            .update_right_buffer(BufferSelector::ControlUnit),
                        Order::Second => self
                            .data_path
                            .update_left_buffer(BufferSelector::ControlUnit),
                    }
                } else {
                    let offset_register = operand.offset + 4;
                    self.data_path.read_data_register(offset_register);
                    match order {
                        Order::First => self
                            .data_path
                            .update_right_buffer(BufferSelector::DataRegister),
                        Order::Second => self
                            .data_path
                            .update_left_buffer(BufferSelector::DataRegister),
                    }
                }

                match order {
                    Order::First => {
                        self.data_path.update_left_alu_input(AluInputSelector::Data);
                        self.data_path
                            .update_right_alu_input(AluInputSelector::Buffer);
                    }
                    Order::Second => {
                        self.data_path
                            .update_left_alu_input(AluInputSelector::Buffer);
                        self.data_path
                            .update_right_alu_input(AluInputSelector::Data);
                    }
                }
                self.data_path.execute_alu(AluOperator::Add);

                self.data_path.latch_data_address();
                self.data_path.read_data_memory();
                self.data_path.latch_read_data();

                match order {
                    Order::First => {
                        self.data_path.update_left_data(DataSelector::ReadData);
                    }
                    Order::Second => {
                        self.data_path.update_right_data(DataSelector::ReadData);
                    }
                }
            }
            Mode::IndirectDirect => {
                self.latch_read_data();
                self.latch_pc(PcSelector::NextTwoWords);
                self.decoder_output = self.read_data;
                self.data_path.control_unit_output = self.decoder_output as i64;
                match order {
                    Order::First => {
                        self.data_path.update_left_data(DataSelector::ControlUnit);
                        self.data_path.update_left_alu_input(AluInputSelector::Data);
                        self.data_path.execute_alu(AluOperator::Trl);
                    }
                    Order::Second => {
                        self.data_path.update_right_data(DataSelector::ControlUnit);
                        self.data_path
                            .update_right_alu_input(AluInputSelector::Data);
                        self.data_path.execute_alu(AluOperator::Trr);
                    }
                }
                self.data_path.latch_data_address();
                self.data_path.read_data_memory();
                self.data_path.latch_read_data();
                match order {
                    Order::First => {
                        self.data_path.update_left_data(DataSelector::ReadData);
                    }
                    Order::Second => {
                        self.data_path.update_right_data(DataSelector::ReadData);
                    }
                }
            }
        }
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
                    "Store: address = {}, value = {}",
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

    pub fn load_constants(&mut self, constants: HashMap<String, ConstantAddress>) {
        for (name, address) in constants {
            let value = name.parse::<u64>().unwrap();
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
            pc: 0,
            data_path_output: 0,
            decoder_output: 0,
            word_size: WordSize::Long,
            current_operator: Operator::Hlt,
            current_operands: vec![],
            execution_state: ExecutionState::Fetch,
        }
    }
}
