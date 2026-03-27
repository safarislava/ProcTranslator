use crate::isa::{Mode, Operand, Operator, WordSize};
use crate::machine::alu::AluOperator;
use crate::machine::data_path::{AddressingModeSelector, DataPath};
use crate::machine::instruction_parser::InstructionParser;
use crate::machine::memory::Memory;
use crate::machine::stack::Stack;
use crate::translator::common::ConstantAddress;
use std::collections::HashMap;

pub enum Order {
    First,
    Second,
}

pub enum PcSelector {
    NextByte,
    NextWord,
    SetByDataPath,
    SetByBuffer,
    SetByStack,
}

pub struct ControlUnit {
    instruction_parser: InstructionParser,
    data_path: DataPath,
    program_memory: Memory,
    stack: Stack,

    read_data: u32,

    data_path_output: i64,

    buffer: u64,
    instruction_decoder_output: u32,

    word_size: WordSize,

    pc: u64,

    debug: u64,
}

impl ControlUnit {
    pub fn set_pc(&mut self, pc: u64) {
        self.pc = pc;
    }

    pub fn latch_pc(&mut self, signal: PcSelector) {
        self.pc = match signal {
            PcSelector::NextByte => self.pc + 1,
            PcSelector::NextWord => self.pc + 4,
            PcSelector::SetByDataPath => self.data_path_output as u64,
            PcSelector::SetByBuffer => self.buffer,
            PcSelector::SetByStack => self.stack.pop(),
        };
    }

    pub fn latch_buffer_n_word(&mut self, n: u8) {
        self.buffer &= 0xffffffff << (32 * n);
        self.buffer |= (self.instruction_decoder_output as u64) << (32 * (1 - n));
    }

    pub fn latch_read_data(&mut self) {
        self.read_data = self.program_memory.read_u32(self.pc);
    }

    pub fn execute_instruction(&mut self) -> bool {
        self.latch_read_data();
        let (operator, word_size) = self.instruction_parser.parse_operator(self.read_data);
        self.debug += 1;
        self.word_size = word_size;

        print!(
            "PC={} : {}.{} ",
            self.pc,
            operator,
            match self.word_size {
                WordSize::Byte => "b",
                WordSize::Long => "w",
            }
        );

        match operator {
            Operator::Hlt => return true,
            Operator::Mov => self.execute_standard_alu_instruction(AluOperator::Trr),
            Operator::Mova => {
                self.latch_pc(PcSelector::NextWord);
                let first = self.parse_register(1);
                let second = self.parse_register(2);
                self.prepare_operand(Order::First, &first);
                self.prepare_operand(Order::Second, &second);
                self.data_path.execute_alu(AluOperator::Trl);
                self.store_by_second_operand(second);
            }
            Operator::Add => self.execute_standard_alu_instruction(AluOperator::Add),
            Operator::Adc => self.execute_standard_alu_instruction(AluOperator::Adc),
            Operator::Sub => self.execute_standard_alu_instruction(AluOperator::Sub),
            Operator::Mul => self.execute_standard_alu_instruction(AluOperator::Mul),
            Operator::Div => self.execute_standard_alu_instruction(AluOperator::Div),
            Operator::Rem => self.execute_standard_alu_instruction(AluOperator::Rem),
            Operator::And => self.execute_standard_alu_instruction(AluOperator::And),
            Operator::Or => self.execute_standard_alu_instruction(AluOperator::Or),
            Operator::Xor => self.execute_standard_alu_instruction(AluOperator::Xor),
            Operator::Not => self.execute_standard_alu_instruction(AluOperator::Not),
            Operator::Lsl => self.execute_standard_alu_instruction(AluOperator::Lsl),
            Operator::Lsr => self.execute_standard_alu_instruction(AluOperator::Lsr),
            Operator::Asl => self.execute_standard_alu_instruction(AluOperator::Asl),
            Operator::Asr => self.execute_standard_alu_instruction(AluOperator::Asr),
            Operator::Jmp => self.execute_jump(),
            Operator::Call => self.execute_call(),
            Operator::Ret => self.latch_pc(PcSelector::SetByStack),
            Operator::Beq => self.execute_branch(self.data_path.transmit_nzcv().zero),
            Operator::Bne => self.execute_branch(!self.data_path.transmit_nzcv().zero),
            Operator::Bgt => {
                let nzcv = self.data_path.transmit_nzcv();
                self.execute_branch(!nzcv.zero && nzcv.negative == nzcv.overflow)
            }
            Operator::Bge => {
                let nzcv = self.data_path.transmit_nzcv();
                self.execute_branch(nzcv.negative == nzcv.overflow)
            }
            Operator::Blt => {
                let nzcv = self.data_path.transmit_nzcv();
                self.execute_branch(nzcv.negative != nzcv.overflow);
            }
            Operator::Ble => {
                let nzcv = self.data_path.transmit_nzcv();
                self.execute_branch(nzcv.zero && nzcv.negative != nzcv.overflow);
            }
            Operator::Bcs => self.execute_branch(self.data_path.transmit_nzcv().carry),
            Operator::Bcc => self.execute_branch(!self.data_path.transmit_nzcv().carry),
            Operator::Bvs => self.execute_branch(self.data_path.transmit_nzcv().overflow),
            Operator::Bvc => self.execute_branch(!self.data_path.transmit_nzcv().overflow),
            Operator::Cmp => {
                self.latch_pc(PcSelector::NextWord);
                let first = self.parse_data_readable(1);
                let second = self.parse_data_readable(2);
                self.prepare_operand(Order::First, &first);
                self.prepare_operand(Order::Second, &second);
                self.data_path.execute_alu(AluOperator::Sub);
            }
        }
        println!();

        for d_register in &self.data_path.d_registers {
            print!("{}\t ", d_register);
        }
        println!();

        for a_register in &self.data_path.a_registers {
            print!("{}\t ", a_register);
        }
        println!();

        false
    }

    pub fn extend_buffer(&mut self) {
        let value = (self.buffer & 0xff_ff_ff_ff) as u32;
        if value >> 31 == 0 {
            self.buffer = value as u64;
        } else {
            self.buffer = (value as u64) | (0xff_ff_ff_ff << 32);
        }
    }

    pub fn fill_buffer(&mut self) {
        for i in 0..2 {
            self.latch_read_data();
            self.instruction_decoder_output = self.program_memory.read_u32(self.pc);
            self.latch_pc(PcSelector::NextWord);
            self.latch_buffer_n_word(i);
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
        let offset = (3 - byte) * 8;
        let operand = ((self.read_data & (0xff << offset)) >> offset) as u8;
        self.instruction_parser.parse_operand(operand)
    }

    pub fn parse_data_writable(&mut self, byte: u8) -> Operand {
        let offset = (3 - byte) * 8;
        let operand = ((self.read_data & (0xff << offset)) >> offset) as u8;
        let operand = self.instruction_parser.parse_operand(operand);
        assert!(operand.mode != Mode::Direct);
        operand
    }

    pub fn execute_standard_alu_instruction(&mut self, operator: AluOperator) {
        self.latch_pc(PcSelector::NextWord);
        let first = self.parse_data_readable(1);
        let second = self.parse_data_writable(2);
        self.prepare_operand(Order::Second, &first);
        self.prepare_operand(Order::First, &second);
        self.data_path.execute_alu(operator);
        self.store_by_second_operand(second);
    }

    pub fn execute_jump(&mut self) {
        self.latch_pc(PcSelector::NextByte);
        self.fill_buffer();
        self.latch_pc(PcSelector::SetByBuffer);
    }

    pub fn execute_call(&mut self) {
        self.stack.push(self.pc + 1 + 8);
        self.execute_jump();
    }

    pub fn execute_branch(&mut self, condition: bool) {
        self.latch_pc(PcSelector::NextByte);
        self.fill_buffer();
        if condition {
            self.latch_pc(PcSelector::SetByBuffer);
        }
    }

    pub fn prepare_operand(&mut self, order: Order, operand: &Operand) {
        match operand.mode {
            Mode::Direct => {
                self.latch_read_data();
                self.instruction_decoder_output = self.program_memory.read_u32(self.pc);
                self.latch_pc(PcSelector::NextWord);
                self.latch_buffer_n_word(1);
                self.extend_buffer();

                print!("#{} ", self.buffer as i64);

                self.data_path.control_unit_output = self.buffer as i64;
                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::ControlUnit);
                match order {
                    Order::First => self.data_path.latch_left_alu(),
                    Order::Second => self.data_path.latch_right_alu(),
                }
            }
            Mode::DataRegister => {
                print!("D{} ", operand.main_register);

                self.data_path.read_data_register(operand.main_register);
                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::DataRegister);
                match order {
                    Order::First => self.data_path.latch_left_alu(),
                    Order::Second => self.data_path.latch_right_alu(),
                }
            }
            Mode::AddressRegister => {
                print!("A{} ", operand.main_register);

                self.data_path.read_address_register(operand.main_register);
                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::AddressRegister);
                match order {
                    Order::First => self.data_path.latch_left_alu(),
                    Order::Second => self.data_path.latch_right_alu(),
                }
            }
            Mode::Indirect => {
                self.data_path.read_address_register(operand.main_register);
                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::AddressRegister);
                match order {
                    Order::First => self.data_path.latch_left_alu(),
                    Order::Second => self.data_path.latch_right_alu(),
                }
                self.data_path.execute_alu(AluOperator::Trl);
                self.data_path.latch_data_address();
                self.data_path.read_data_memory();
                self.data_path.latch_read_data();

                print!(
                    "(A{})={} ",
                    operand.main_register, self.data_path.memory_output
                );

                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::ReadData);
                match order {
                    Order::First => self.data_path.latch_left_alu(),
                    Order::Second => self.data_path.latch_right_alu(),
                }
            }
            Mode::IndirectPostIncrement => {
                match order {
                    Order::First => self.data_path.execute_alu(AluOperator::Trr),
                    Order::Second => self.data_path.execute_alu(AluOperator::Trl),
                }
                self.data_path.latch_buffer();

                self.data_path.read_address_register(operand.main_register);
                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::AddressRegister);
                self.data_path.latch_left_alu();
                self.data_path.execute_alu(AluOperator::Trl);

                self.data_path.latch_data_address();
                self.data_path.read_data_memory();
                self.data_path.latch_read_data();

                print!(
                    "(A{})+={} ",
                    operand.main_register, self.data_path.memory_output
                );

                self.data_path.control_unit_output = match self.word_size {
                    WordSize::Byte => 1,
                    WordSize::Long => 8,
                };
                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::ControlUnit);
                self.data_path.latch_right_alu();
                self.data_path.execute_alu(AluOperator::Add);
                self.data_path
                    .latch_address_register(operand.main_register, &self.word_size);

                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::ReadData);
                match order {
                    Order::First => self.data_path.latch_left_alu(),
                    Order::Second => self.data_path.latch_right_alu(),
                }
                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::Buffer);
                match order {
                    Order::First => self.data_path.latch_right_alu(),
                    Order::Second => self.data_path.latch_left_alu(),
                }
            }
            Mode::IndirectPreDecrement => {
                match order {
                    Order::First => self.data_path.execute_alu(AluOperator::Trr),
                    Order::Second => self.data_path.execute_alu(AluOperator::Trl),
                }
                self.data_path.latch_buffer();

                self.data_path.read_address_register(operand.main_register);
                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::AddressRegister);
                self.data_path.latch_left_alu();

                self.data_path.control_unit_output = match self.word_size {
                    WordSize::Byte => 1,
                    WordSize::Long => 8,
                };
                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::ControlUnit);
                self.data_path.latch_right_alu();
                self.data_path.execute_alu(AluOperator::Sub);
                self.data_path
                    .latch_address_register(operand.main_register, &self.word_size);

                self.data_path.latch_data_address();
                self.data_path.read_data_memory();
                self.data_path.latch_read_data();

                print!(
                    "-(A{})={} ",
                    operand.main_register, self.data_path.memory_output
                );

                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::ReadData);
                match order {
                    Order::First => self.data_path.latch_left_alu(),
                    Order::Second => self.data_path.latch_right_alu(),
                }
                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::Buffer);
                match order {
                    Order::First => self.data_path.latch_right_alu(),
                    Order::Second => self.data_path.latch_left_alu(),
                }
            }
            Mode::IndirectOffset => {
                match order {
                    Order::First => self.data_path.execute_alu(AluOperator::Trr),
                    Order::Second => self.data_path.execute_alu(AluOperator::Trl),
                }
                self.data_path.latch_buffer();

                self.data_path.read_address_register(operand.main_register);
                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::AddressRegister);
                self.data_path.latch_left_alu();

                self.data_path.read_data_register(operand.offset_register);
                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::DataRegister);
                self.data_path.latch_right_alu();
                self.data_path.execute_alu(AluOperator::Add);

                self.data_path.latch_data_address();
                self.data_path.read_data_memory();
                self.data_path.latch_read_data();

                print!(
                    "(A{}+D{})={} ",
                    operand.main_register, operand.offset_register, self.data_path.memory_output
                );

                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::ReadData);
                match order {
                    Order::First => self.data_path.latch_left_alu(),
                    Order::Second => self.data_path.latch_right_alu(),
                }
                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::Buffer);
                match order {
                    Order::First => self.data_path.latch_right_alu(),
                    Order::Second => self.data_path.latch_left_alu(),
                }
            }
            Mode::IndirectDirect => {
                self.fill_buffer();

                self.data_path.control_unit_output = self.buffer as i64;
                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::ControlUnit);
                match order {
                    Order::First => {
                        self.data_path.latch_left_alu();
                        self.data_path.execute_alu(AluOperator::Trl);
                    }
                    Order::Second => {
                        self.data_path.latch_right_alu();
                        self.data_path.execute_alu(AluOperator::Trr);
                    }
                }
                self.data_path.latch_data_address();
                self.data_path.read_data_memory();
                self.data_path.latch_read_data();

                print!("(#{})={} ", self.buffer, self.data_path.memory_output);

                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::ReadData);
                match order {
                    Order::First => self.data_path.latch_left_alu(),
                    Order::Second => self.data_path.latch_right_alu(),
                }
            }
        }
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
                print!(
                    "\nStore: address = {}, value = {}",
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
            buffer: 0,
            instruction_decoder_output: 0,
            word_size: WordSize::Long,
            debug: 0,
        }
    }
}
