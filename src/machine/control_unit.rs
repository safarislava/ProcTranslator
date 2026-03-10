use crate::machine::alu::AluOperator;
use crate::machine::data_path::{AddressingModeSelector, DataPath};
use crate::machine::isa::{InstructionParser, Mode, Operand, Operator};
use crate::machine::memory::Memory;

pub enum Order {
    First,
    Second,
}

pub enum PcSelector {
    Next,
    SetByDataPath,
    SetByBuffer,
}


pub struct ControlUnit {
    instruction_parser: InstructionParser,
    data_path: DataPath,
    program_memory: Memory<u8>,

    memory_output: u8,
    read_data: u8,

    data_path_output: i64,

    buffer: u64,
    decoder_output: u8,

    pc: u64,
}

impl ControlUnit {
    pub fn latch_pc(&mut self, signal: PcSelector) {
        self.pc = match signal {
            PcSelector::Next => self.pc + 1,
            PcSelector::SetByDataPath => self.data_path_output as u64,
            PcSelector::SetByBuffer => self.buffer,
        };
    }

    pub fn latch_buffer_n_byte(&mut self, n: u8) {
        self.buffer = (self.decoder_output as u64) << (8 * n);
    }
    pub fn read_program_memory(&mut self) {
        self.memory_output = self.program_memory.read(self.pc);
    }

    pub fn latch_read_data(&mut self) {
        self.read_data = self.memory_output;
    }

    pub fn execute_instruction(&mut self) {
        let operator = self.instruction_parser.parse_operator(self.read_data);
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
            Operator::REM => self.execute_standard_alu_instruction(AluOperator::Rem),
            Operator::AND => self.execute_standard_alu_instruction(AluOperator::And),
            Operator::OR => self.execute_standard_alu_instruction(AluOperator::Or),
            Operator::XOR => self.execute_standard_alu_instruction(AluOperator::Xor),
            Operator::NOT => self.execute_standard_alu_instruction(AluOperator::Not),
            Operator::LSL => self.execute_standard_alu_instruction(AluOperator::Lsl),
            Operator::LSR => self.execute_standard_alu_instruction(AluOperator::Lsr),
            Operator::ASL => self.execute_standard_alu_instruction(AluOperator::Asl),
            Operator::ASR => self.execute_standard_alu_instruction(AluOperator::Asr),
            Operator::JMP => {}
            Operator::CALL => {}
            Operator::FUNC => {}
            Operator::RET => {}
            Operator::LINK => {}
            Operator::UNLK => {}
            Operator::BEQ => self.execute_branch(self.data_path.transmit_nzcv().zero),
            Operator::BNE => self.execute_branch(!self.data_path.transmit_nzcv().zero),
            Operator::BGT => {
                let nzcv = self.data_path.transmit_nzcv();
                self.execute_branch(!nzcv.zero && nzcv.negative == nzcv.overflow)
            }
            Operator::BGE => {
                let nzcv = self.data_path.transmit_nzcv();
                self.execute_branch(nzcv.negative == nzcv.overflow)
            }
            Operator::BLT => {
                let nzcv = self.data_path.transmit_nzcv();
                self.execute_branch(nzcv.negative != nzcv.overflow);
            }
            Operator::BLE => {
                let nzcv = self.data_path.transmit_nzcv();
                self.execute_branch(nzcv.zero && nzcv.negative != nzcv.overflow);
            }
            Operator::BCS => self.execute_branch(self.data_path.transmit_nzcv().carry),
            Operator::BCC => self.execute_branch(!self.data_path.transmit_nzcv().carry),
            Operator::BVS => self.execute_branch(self.data_path.transmit_nzcv().overflow),
            Operator::BVC => self.execute_branch(!self.data_path.transmit_nzcv().overflow),
            Operator::CMP => {
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
            self.decoder_output = self.program_memory.read(self.pc);
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

    pub fn execute_branch(&mut self, condition: bool) {
        if condition {
            self.fill_buffer();
            self.latch_pc(PcSelector::SetByBuffer)
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

                self.data_path.control_unit_output = 1;
                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::ControlUnit);
                self.data_path.latch_right_alu();
                self.data_path.execute_alu(AluOperator::Add);
                self.data_path.latch_address_register(operand.main_register);

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

                self.data_path.control_unit_output = 1;
                self.data_path
                    .update_alu_input_mux(AddressingModeSelector::ControlUnit);
                self.data_path.latch_right_alu();
                self.data_path.execute_alu(AluOperator::Sub);
                self.data_path.latch_address_register(operand.main_register);
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
                self.data_path.latch_data_register(operand.main_register);
            }
            Mode::AddressRegister => {
                self.data_path.latch_address_register(operand.main_register);
            }
            Mode::Indirect
            | Mode::IndirectPostIncrement
            | Mode::IndirectPreDecrement
            | Mode::IndirectOffset => {
                self.data_path.latch_write_data();
                self.data_path.write_data_memory();
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
            memory_output: 0,
            read_data: 0,
            pc: 0,
            data_path_output: 0,
            buffer: 0,
            decoder_output: 0,
        }
    }
}