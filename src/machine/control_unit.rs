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
    ByDecoder,
    ByStack,
    ByInterrupt,
}

enum ExecutionState {
    Start,
    Execute(u8),
    Interrupt(u8),
    Done,
    Stop,
}

pub struct ControlUnit {
    instruction_parser: InstructionParser,
    pub data_path: DataPath,
    program_memory: Memory,
    stack: Stack,
    decoder_output: u64,
    pc: Address,
    read_data: u64,
    ir: u32,
    word_size: WordSize,
    operator: Operator,
    execution_state: ExecutionState,
    interrupt_flag: bool,
    vector_table: HashMap<u8, Address>,
    pub tick: u64,
    log: String,
}

impl ControlUnit {
    fn tick(&mut self) {
        self.log = format!("TICK {} ", self.tick);
        self.tick += 1;
    }

    fn latch_pc(&mut self, signal: PcSelector) {
        self.pc = match signal {
            PcSelector::NextByte => self.pc + 1,
            PcSelector::NextWord => self.pc + 4,
            PcSelector::NextTwoWords => self.pc + 8,
            PcSelector::ByDecoder => self.decoder_output,
            PcSelector::ByStack => self.stack.pop(),
            PcSelector::ByInterrupt => self.vector_table[&self.data_path.io.interrupt_vector],
        };
    }

    fn latch_read_data(&mut self) {
        self.read_data = self.program_memory.read_u64(self.pc);
    }

    fn latch_ir(&mut self) {
        self.ir = self.program_memory.read_u32(self.pc);
    }

    pub fn step(&mut self) -> bool {
        match self.execution_state {
            ExecutionState::Start => {
                if self.interrupt_flag && self.data_path.io.check_interrupt() {
                    self.execution_state = ExecutionState::Interrupt(0);
                    return false;
                }
                self.fetch();
                info!("{}", self.log);
                false
            }
            ExecutionState::Execute(step) => {
                self.execute_step(step);
                info!("{}", self.log);
                false
            }
            ExecutionState::Interrupt(step) => {
                self.interrupt(step);
                info!("{}", self.log);
                false
            }
            ExecutionState::Done => {
                debug!("D {:?}", self.data_path.data_registers);
                debug!("A {:?}", self.data_path.address_registers);
                self.execution_state = ExecutionState::Start;
                false
            }
            ExecutionState::Stop => true,
        }
    }

    fn interrupt(&mut self, step: u8) {
        self.tick();
        match step {
            0 => {
                self.log += "| INTERRUPT ";
                self.interrupt_flag = false;
                self.stack.push(self.pc);
                self.latch_pc(PcSelector::ByInterrupt);

                let nzcv = self.data_path.transmit_nzcv();
                self.data_path.write_data = nzcv.to_byte() as i64;

                self.execution_state = ExecutionState::Interrupt(1);
            }
            1 => {
                self.data_path.read_address_register(7);
                self.data_path
                    .update_left_data(DataSelector::AddressRegister);
                self.data_path.update_left_alu_input(AluInputSelector::Data);
                self.data_path.execute_alu(AluOperator::Trl);
                self.data_path.pre_mode_selector = PreModeSelector::DecrementByte;
                self.data_path.latch_data_address();
                self.data_path.latch_address_register(7, &WordSize::Long);
                self.data_path.write_data_memory(&WordSize::Byte);

                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn fetch(&mut self) {
        self.tick();
        self.latch_ir();

        let (operator, word_size) = self.instruction_parser.parse_operator(self.ir);

        self.word_size = word_size;
        self.operator = operator;

        self.log += &format!(
            "| PC={} | {}.{} ",
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
            | Operator::Cmp
            | Operator::In
            | Operator::Out => self.latch_pc(PcSelector::NextWord),
            Operator::Jmp
            | Operator::Call
            | Operator::Ret
            | Operator::IntRet
            | Operator::Beq
            | Operator::Bne
            | Operator::Bgt
            | Operator::Bge
            | Operator::Blt
            | Operator::Ble
            | Operator::Bcs
            | Operator::Bcc
            | Operator::Bvs
            | Operator::Bvc
            | Operator::EI
            | Operator::DI => self.latch_pc(PcSelector::NextByte),
        }

        self.execution_state = ExecutionState::Execute(0);
    }

    fn execute_step(&mut self, step: u8) {
        self.tick();
        match self.operator {
            Operator::Hlt => {
                self.execution_state = ExecutionState::Stop;
                self.log += &format!("| Result : {}", self.data_path.data_registers[0]);
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
            Operator::Call => self.execute_call(step),
            Operator::Ret => self.execute_return(step),
            Operator::IntRet => self.execute_interrupt_return(step),
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
                self.execute_branch(step, nzcv.zero || nzcv.negative != nzcv.overflow);
            }
            Operator::Bcs => self.execute_branch(step, self.data_path.transmit_nzcv().carry),
            Operator::Bcc => self.execute_branch(step, !self.data_path.transmit_nzcv().carry),
            Operator::Bvs => self.execute_branch(step, self.data_path.transmit_nzcv().overflow),
            Operator::Bvc => self.execute_branch(step, !self.data_path.transmit_nzcv().overflow),
            Operator::Cmp => self.execute_cmp(step),
            Operator::In => self.execute_input(step),
            Operator::Out => self.execute_output(step),
            Operator::EI => self.enable_interrupt(step),
            Operator::DI => self.disable_interrupt(step),
        }
    }

    #[allow(dead_code)]
    fn parse_register(&mut self, byte: u8) -> Operand {
        let offset = (3 - byte) * 8;
        let operand = ((self.ir & (0xff << offset)) >> offset) as u8;
        let operand = self.instruction_parser.parse_operand(operand);
        assert!(operand.mode == Mode::AddressRegister || operand.mode == Mode::DataRegister);
        operand
    }

    fn parse_data_readable(&mut self, byte: u8) -> Operand {
        let offset = (3 - byte) * 8;
        let operand = ((self.ir & (0xff << offset)) >> offset) as u8;
        self.instruction_parser.parse_operand(operand)
    }

    fn parse_data_writable(&mut self, byte: u8) -> Operand {
        let offset = (3 - byte) * 8;
        let operand = ((self.ir & (0xff << offset)) >> offset) as u8;
        let operand = self.instruction_parser.parse_operand(operand);
        assert!(operand.mode != Mode::Direct);
        operand
    }

    fn parse_port(&mut self, byte: u8) -> u8 {
        let offset = (3 - byte) * 8;
        ((self.ir & (0xff << offset)) >> offset) as u8
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
        match step {
            0 => {
                let first = self.parse_data_readable(1);
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
                let second = self.parse_data_writable(2);
                self.prepare_operand(Order::Master, &second);

                if Self::is_operand_needed_second_step(&second) {
                    self.execution_state = ExecutionState::Execute(3);
                } else {
                    self.data_path.execute_alu(alu_op);
                    self.store_operand(second);

                    self.execution_state = ExecutionState::Done;
                }
            }
            3 => {
                let second = self.parse_data_writable(2);
                self.prepare_operand_read_data(Order::Master);

                self.data_path.execute_alu(alu_op);
                self.store_operand(second);

                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn execute_cmp(&mut self, step: u8) {
        match step {
            0 => {
                let first = self.parse_data_readable(1);
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
                let second = self.parse_data_readable(2);
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

    fn execute_jump(&mut self, step: u8) {
        match step {
            0 => {
                self.latch_read_data();
                self.decoder_output = self.read_data;
                self.latch_pc(PcSelector::ByDecoder);
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn execute_call(&mut self, step: u8) {
        match step {
            0 => {
                self.latch_read_data();
                self.decoder_output = self.read_data;
                self.stack.push(self.pc + 8);
                self.latch_pc(PcSelector::ByDecoder);
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn execute_return(&mut self, step: u8) {
        match step {
            0 => {
                self.latch_pc(PcSelector::ByStack);
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn execute_interrupt_return(&mut self, step: u8) {
        match step {
            0 => {
                self.latch_pc(PcSelector::ByStack);

                self.data_path.read_address_register(7);
                self.data_path
                    .update_left_data(DataSelector::AddressRegister);
                self.data_path.update_left_alu_input(AluInputSelector::Data);
                self.data_path.execute_alu(AluOperator::Trl);
                self.data_path.latch_data_address();
                self.data_path.post_mode_selector = PostModeSelector::IncrementByte;
                self.data_path.latch_address_register(7, &WordSize::Long);
                self.data_path.read_data_memory(&WordSize::Byte);

                self.execution_state = ExecutionState::Execute(1);
            }
            1 => {
                self.latch_read_data();
                self.data_path.update_left_data(DataSelector::ReadData);
                self.data_path.update_left_alu_input(AluInputSelector::Data);
                self.data_path.execute_alu(AluOperator::Trl);
                self.data_path.restore_nzcv();
                self.interrupt_flag = true;

                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn execute_branch(&mut self, step: u8, condition: bool) {
        match step {
            0 => {
                self.latch_read_data();
                self.decoder_output = self.read_data;
                if condition {
                    self.latch_pc(PcSelector::ByDecoder);
                } else {
                    self.latch_pc(PcSelector::NextTwoWords)
                }
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn execute_input(&mut self, step: u8) {
        match step {
            0 => {
                let port = self.parse_port(1);
                self.data_path.read_io(port);

                self.data_path.io.update_interrupt_vector();

                self.data_path.external_selector = ExternalSelector::IO;
                self.data_path.update_right_data(DataSelector::External);
                self.log += &format!("| IN #{} ", self.data_path.io.output);

                let operand = self.parse_data_writable(2);
                self.prepare_operand(Order::Master, &operand);

                if Self::is_operand_needed_second_step(&operand) {
                    self.execution_state = ExecutionState::Execute(1);
                } else {
                    self.data_path.execute_alu(AluOperator::Trr);
                    self.store_operand(operand);
                    self.execution_state = ExecutionState::Done;
                }
            }
            1 => {
                let operand = self.parse_data_writable(2);
                self.prepare_operand_read_data(Order::Master);
                self.data_path.execute_alu(AluOperator::Trr);
                self.store_operand(operand);
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn execute_output(&mut self, step: u8) {
        match step {
            0 => {
                let port = self.parse_port(1);
                let operand = self.parse_data_readable(2);
                self.prepare_operand(Order::Slave, &operand);

                if Self::is_operand_needed_second_step(&operand) {
                    self.execution_state = ExecutionState::Execute(1);
                } else {
                    self.data_path.execute_alu(AluOperator::Trr);
                    self.data_path.write_io(port);
                    self.execution_state = ExecutionState::Done;
                }
            }
            1 => {
                let port = self.parse_port(1);
                self.prepare_operand_read_data(Order::Slave);
                self.data_path.execute_alu(AluOperator::Trr);
                self.data_path.write_io(port);
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn enable_interrupt(&mut self, step: u8) {
        match step {
            0 => self.interrupt_flag = true,
            _ => unreachable!(),
        }
    }

    fn disable_interrupt(&mut self, step: u8) {
        match step {
            0 => self.interrupt_flag = false,
            _ => unreachable!(),
        }
    }

    fn prepare_operand(&mut self, order: Order, operand: &Operand) {
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
                self.log += &format!("| #{} ", self.decoder_output);
            }
            Mode::DataRegister => {
                self.data_path.read_data_register(operand.main_register);
                match order {
                    Order::Master => self.data_path.update_left_data(DataSelector::DataRegister),
                    Order::Slave => self.data_path.update_right_data(DataSelector::DataRegister),
                }
                self.log += &format!("| D{} ", operand.main_register);
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
                self.log += &format!("| A{} ", operand.main_register);
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
                self.data_path.read_data_memory(&self.word_size);
                self.data_path
                    .latch_address_register(operand.main_register, &WordSize::Long);

                match operand.mode {
                    Mode::Indirect => self.log += &format!("| (A{}) ", operand.main_register),
                    Mode::IndirectPreDecrement => {
                        self.log += &format!("| -(A{}) ", operand.main_register)
                    }
                    Mode::IndirectPostIncrement => {
                        self.log += &format!("| (A{})+ ", operand.main_register)
                    }
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
                    self.log += &format!(
                        "| (A{}:#{}) ",
                        operand.main_register, self.decoder_output as i64
                    );
                } else {
                    let offset_register = operand.offset + 4;
                    self.data_path.read_data_register(offset_register);
                    match order {
                        Order::Master => {
                            self.data_path
                                .update_right_buffer(BufferSelector::DataRegister);
                        }
                        Order::Slave => {
                            self.data_path
                                .update_left_buffer(BufferSelector::DataRegister);
                        }
                    }
                    self.log += &format!("| (A{}:D{}) ", operand.main_register, offset_register);
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
                self.data_path.read_data_memory(&self.word_size);
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
                self.data_path.read_data_memory(&self.word_size);

                self.log += &format!("| (#{}) ", self.decoder_output);
            }
        }
        self.data_path.update_left_alu_input(AluInputSelector::Data);
        self.data_path
            .update_right_alu_input(AluInputSelector::Data);
        self.data_path.pre_mode_selector = PreModeSelector::None;
        self.data_path.post_mode_selector = PostModeSelector::None;
    }

    fn prepare_operand_read_data(&mut self, order: Order) {
        self.data_path.latch_read_data();
        match order {
            Order::Master => {
                self.data_path.update_left_data(DataSelector::ReadData);
            }
            Order::Slave => {
                self.data_path.update_right_data(DataSelector::ReadData);
            }
        }
        self.log += &format!(
            "| Load (#{}) = {} ",
            self.data_path.data_address, self.data_path.read_data
        );

        self.data_path.update_left_alu_input(AluInputSelector::Data);
        self.data_path
            .update_right_alu_input(AluInputSelector::Data);
        self.data_path.pre_mode_selector = PreModeSelector::None;
        self.data_path.post_mode_selector = PostModeSelector::None;
    }

    fn store_operand(&mut self, operand: Operand) {
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
                self.log += &format!(
                    "| Store (#{}) = {} ",
                    self.data_path.data_address, self.data_path.write_data
                )
            }
            _ => unreachable!(),
        }
    }
}

impl ControlUnit {
    pub fn load_program(&mut self, program: &[u8]) {
        program.iter().enumerate().for_each(|(i, word)| {
            self.program_memory.write_u8(i as Address, *word);
        })
    }

    pub fn load_data_section(&mut self, data_section: HashMap<Address, (u64, WordSize)>) {
        data_section
            .iter()
            .for_each(|(address, (value, word_size))| match word_size {
                WordSize::Byte => self.data_path.data_memory.write_u8(*address, *value as u8),
                WordSize::Long => self.data_path.data_memory.write_u64(*address, *value),
            })
    }

    pub fn load_interrupt_vectors(&mut self, interrupt_vectors: [Address; 8]) {
        interrupt_vectors
            .iter()
            .enumerate()
            .for_each(|(i, address)| {
                self.vector_table.insert(i as u8, *address);
            });
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
            execution_state: ExecutionState::Start,
            interrupt_flag: true,
            vector_table: HashMap::new(),
            tick: 0,
            log: String::from(""),
        }
    }
}
