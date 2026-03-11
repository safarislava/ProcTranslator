use crate::machine::alu::AluOperator;
use crate::machine::data_path::{AddressingModeSelector, DataPath};
use crate::machine::isa::{InstructionParser, Mode, Operand, Operator, WordSize};
use crate::machine::memory::Memory;
use crate::machine::stack::Stack;

pub enum Order {
    First,
    Second,
}

pub enum PcSelector {
    Next,
    SetByDataPath,
    SetByBuffer,
    SetByStack,
}

pub struct ControlUnit {
    instruction_parser: InstructionParser,
    data_path: DataPath,
    program_memory: Memory<u8>,
    stack: Stack,

    memory_output: u8,
    read_data: u8,

    data_path_output: i64,

    buffer: u64,
    instruction_decoder_output: u8,

    word_size: WordSize,

    pc: u64,
}

impl ControlUnit {
    pub fn latch_pc(&mut self, signal: PcSelector) {
        self.pc = match signal {
            PcSelector::Next => self.pc + 1,
            PcSelector::SetByDataPath => self.data_path_output as u64,
            PcSelector::SetByBuffer => self.buffer,
            PcSelector::SetByStack => self.stack.pop(),
        };
    }

    pub fn latch_buffer_n_byte(&mut self, n: u8) {
        self.buffer = (self.instruction_decoder_output as u64) << (8 * n);
    }
    pub fn read_program_memory(&mut self) {
        self.memory_output = self.program_memory.read(self.pc);
    }

    pub fn latch_read_data(&mut self) {
        self.read_data = self.memory_output;
    }

    pub fn execute_instruction(&mut self) {
        let (operator, word_size) = self.instruction_parser.parse_operator(self.read_data);
        self.word_size = word_size;
        self.latch_pc(PcSelector::Next);
        match operator {
            Operator::Mov => self.execute_standard_alu_instruction(AluOperator::Trl),
            Operator::Mova => {
                let first = self.parse_register();
                let second = self.parse_register();
                self.prepare_operand(Order::First, &first);
                self.prepare_operand(Order::Second, &second);
                self.data_path.execute_alu(AluOperator::Trl);
                self.save_by_second_operand(second);
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
            Operator::Call => {
                self.stack.push(self.pc);
                self.execute_jump();
            }
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
                let first = self.parse_data_readable();
                let second = self.parse_data_readable();
                self.prepare_operand(Order::First, &first);
                self.prepare_operand(Order::Second, &second);
                self.data_path.execute_alu(AluOperator::Sub);
            }
        }
    }

    pub fn fill_buffer(&mut self) {
        for i in 0..8 {
            self.instruction_decoder_output = self.program_memory.read(self.pc);
            self.latch_pc(PcSelector::Next);
            self.latch_buffer_n_byte(i);
        }
    }

    pub fn parse_register(&mut self) -> Operand {
        let operand = self.instruction_parser.parse_operand(self.read_data);
        self.latch_pc(PcSelector::Next);
        assert!(operand.mode == Mode::AddressRegister || operand.mode == Mode::DataRegister);
        operand
    }

    pub fn parse_data_readable(&mut self) -> Operand {
        let operand = self.instruction_parser.parse_operand(self.read_data);
        self.latch_pc(PcSelector::Next);
        assert!(operand.mode != Mode::AddressRegister);
        operand
    }

    pub fn parse_data_writable(&mut self) -> Operand {
        let operand = self.instruction_parser.parse_operand(self.read_data);
        self.latch_pc(PcSelector::Next);
        assert!(operand.mode != Mode::AddressRegister);
        assert!(operand.mode != Mode::Direct);
        operand
    }

    pub fn execute_standard_alu_instruction(&mut self, operator: AluOperator) {
        let first = self.parse_data_readable();
        let second = self.parse_data_writable();
        self.prepare_operand(Order::First, &first);
        self.prepare_operand(Order::Second, &second);
        self.data_path.execute_alu(operator);
        self.save_by_second_operand(second);
    }

    pub fn execute_jump(&mut self) {
        self.fill_buffer();
        self.latch_pc(PcSelector::SetByBuffer);
    }

    pub fn execute_branch(&mut self, condition: bool) {
        if condition {
            self.execute_jump();
        }
    }

    pub fn prepare_operand(&mut self, order: Order, operand: &Operand) {
        match operand.mode {
            Mode::Direct => {
                self.pc += 1;
                self.data_path.control_unit_output = self.program_memory.read(self.pc) as i64;
                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::ControlUnit);
                match order {
                    Order::First => self.data_path.latch_left_alu(),
                    Order::Second => self.data_path.latch_right_alu(),
                }
            }
            Mode::DataRegister => {
                self.data_path.read_data_register(operand.main_register);
                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::DataRegister);
                match order {
                    Order::First => self.data_path.latch_left_alu(),
                    Order::Second => self.data_path.latch_right_alu(),
                }
            }
            Mode::AddressRegister => {
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
                    Order::Second => self.data_path.execute_alu(AluOperator::Trr),
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
                    Order::Second => self.data_path.execute_alu(AluOperator::Trr),
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
                    Order::Second => self.data_path.execute_alu(AluOperator::Trr),
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
        }
    }

    pub fn save_by_second_operand(&mut self, operand: Operand) {
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
            | Mode::IndirectOffset => {
                self.data_path.latch_write_data();
                self.data_path.write_data_memory(&self.word_size);
            }
            _ => unreachable!(),
        }
    }
}

impl ControlUnit {
    pub fn load_program(&mut self, program: &[u8]) {
        for (i, word) in program.iter().enumerate() {
            self.program_memory.write(i, *word);
        }
    }
}

impl Default for ControlUnit {
    fn default() -> Self {
        Self {
            instruction_parser: InstructionParser::new(),
            data_path: DataPath::default(),
            program_memory: Memory::new(),
            stack: Stack::new(),
            memory_output: 0,
            read_data: 0,
            pc: 0,
            data_path_output: 0,
            buffer: 0,
            instruction_decoder_output: 0,
            word_size: WordSize::Long,
        }
    }
}
