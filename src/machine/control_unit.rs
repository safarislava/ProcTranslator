use crate::isa::{Mode, Operand, Operator, WordSize};
use crate::machine::alu::AluOperator;
use crate::machine::data_path::{
    AluInputSelector, BranchSelector, DataPath, DataSelector, ExternalSelector, PostModeSelector,
    PreModeSelector, VectorModeSelector, WriteDataSelector, WriteSelector,
};
use crate::machine::instruction_parser::InstructionParser;
use crate::machine::program_memory::ProgramMemory;
use crate::machine::vector_alu::VectorAluOperator;
use crate::translator::common::Address;
use std::collections::HashMap;
use tracing::{debug, info};

pub enum AluInput {
    Left,
    Right,
    None,
}

pub enum PcSelector {
    NextWord,
    ByAddress,
    ByDataPath,
    ByInterrupt,
}

pub enum OutputPcSelector {
    Current,
    Next,
}

pub enum OutputSelector {
    Memory,
    PC,
}

enum ExecutionState {
    Start,
    Execute(u8),
    Done,
    Stop,
}

enum PrepareMode {
    Register,
    Immediate,
}

pub struct InstructionTrace {
    start_tick: u64,
    end_tick: u64,
    pc: Address,
    opcode: String,
    operands: Vec<String>,
    events: Vec<String>,
}

impl InstructionTrace {
    pub fn new(tick: u64, pc: Address, opcode: String) -> Self {
        Self {
            start_tick: tick,
            end_tick: tick,
            pc,
            opcode,
            operands: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn finish(&mut self, tick: u64) {
        self.end_tick = tick;
    }
}

pub struct ControlUnit {
    instruction_parser: InstructionParser,
    pub data_path: DataPath,
    program_memory: ProgramMemory,
    pc: Address,
    read_data: u32,
    ir: u32,
    output_pc_mux: u64,
    word_size: WordSize,
    operator: Operator,
    execution_state: ExecutionState,
    interrupt_flag: bool,
    interrupt_processing: bool,
    vector_table: HashMap<u8, Address>,
    pub tick: u64,
    current_trace: Option<InstructionTrace>,
}

impl ControlUnit {
    fn log_operand(&mut self, op: &str) {
        if let Some(trace) = &mut self.current_trace {
            trace.operands.push(op.to_string());
        }
    }

    fn log_simple_operand(&mut self, operand: &Operand) {
        let op_str = match operand.mode {
            Mode::DataRegister => format!("D{}", operand.main_register),
            Mode::AddressRegister => format!("A{}", operand.main_register),
            _ => return,
        };
        self.log_operand(&op_str);
    }

    fn log_event(&mut self, event: &str) {
        if let Some(trace) = &mut self.current_trace {
            trace.events.push(format!("[T{}] {}", self.tick, event));
        }
    }

    fn latch_pc(&mut self, signal: PcSelector) {
        self.pc = match signal {
            PcSelector::NextWord => self.pc + 1,
            PcSelector::ByAddress => self.assemble_branch_address(),
            PcSelector::ByDataPath => self.data_path.alu_output,
            PcSelector::ByInterrupt => self.vector_table[&self.data_path.io.interrupt_vector],
        };
    }

    fn update_read_data(&mut self) {
        self.read_data = self.program_memory.read(self.pc);
    }

    fn latch_ir(&mut self) {
        self.ir = self.read_data;
    }

    fn update_control_unit_output(&mut self, selector: OutputSelector) {
        match selector {
            OutputSelector::Memory => {
                self.data_path.control_unit_output = self.read_data as i32 as i64 as u64
            }
            OutputSelector::PC => self.data_path.control_unit_output = self.output_pc_mux,
        }
    }

    fn update_output_pc_mux(&mut self, selector: OutputPcSelector) {
        match selector {
            OutputPcSelector::Current => self.output_pc_mux = self.pc,
            OutputPcSelector::Next => self.output_pc_mux = self.pc + 1,
        }
    }

    pub fn step(&mut self) -> bool {
        match self.execution_state {
            ExecutionState::Start => {
                if self.interrupt_flag && self.data_path.io.check_interrupt() {
                    self.interrupt_processing = true;
                    self.execution_state = ExecutionState::Execute(0);
                    return false;
                }
                self.fetch();
                false
            }
            ExecutionState::Execute(step) => {
                self.execute_step(step);
                false
            }
            ExecutionState::Done => {
                if let Some(mut trace) = self.current_trace.take() {
                    trace.finish(self.tick);

                    let ops_str = trace.operands.join(", ");
                    let full_instruction = if ops_str.is_empty() {
                        trace.opcode.clone()
                    } else {
                        format!("{} {}", trace.opcode, ops_str)
                    };

                    info!(
                        target: "trace",
                        "[PC: {:<4}] {:<25} (Ticks: {} -> {})",
                        trace.pc,
                        full_instruction,
                        trace.start_tick,
                        trace.end_tick,
                    );

                    if !trace.events.is_empty() {
                        for event in trace.events {
                            info!(target: "trace", "  └─ {}", event);
                        }
                    }
                }

                debug!(
                    target: "registers",
                    "D: [{:?}] | A: [{:?}]",
                    self.data_path.data_registers.map(|r| r as i64),
                    self.data_path.address_registers.map(|r| r as i64),
                );

                self.execution_state = ExecutionState::Start;
                false
            }
            ExecutionState::Stop => {
                info!(
                    target: "trace",
                    "Result: D0 = {}",
                    self.data_path.data_registers[0] as i64
                );
                true
            }
        }
    }

    fn fetch(&mut self) {
        self.tick += 1;
        self.update_read_data();
        self.latch_ir();

        let (operator, word_size) = self.instruction_parser.parse_operator(self.ir);
        self.word_size = word_size;
        self.operator = operator;

        let instr_str = format!(
            "{}.{}",
            self.operator,
            match self.word_size {
                WordSize::Byte => "b",
                WordSize::Long => "l",
            }
        );
        self.current_trace = Some(InstructionTrace::new(self.tick, self.pc, instr_str));

        self.latch_pc(PcSelector::NextWord);
        self.execution_state = ExecutionState::Execute(0);
    }

    fn execute_step(&mut self, step: u8) {
        self.tick += 1;

        if self.interrupt_processing {
            self.execute_interrupt(step);
            return;
        }

        match self.operator {
            Operator::Hlt => {
                self.execution_state = ExecutionState::Stop;
            }
            Operator::Mov => self.execute_2_operand_operator(step, &AluOperator::Trr),
            Operator::Cmp => self.execute_cmp(step),
            Operator::Add => self.execute_3_operand_operator(step, AluOperator::Add),
            Operator::Adc => self.execute_3_operand_operator(step, AluOperator::Adc),
            Operator::Sub => self.execute_3_operand_operator(step, AluOperator::Sub),
            Operator::Mul => self.execute_3_operand_operator(step, AluOperator::Mul),
            Operator::Div => self.execute_3_operand_operator(step, AluOperator::Div),
            Operator::Rem => self.execute_3_operand_operator(step, AluOperator::Rem),
            Operator::And => self.execute_3_operand_operator(step, AluOperator::And),
            Operator::Or => self.execute_3_operand_operator(step, AluOperator::Or),
            Operator::Xor => self.execute_3_operand_operator(step, AluOperator::Xor),
            Operator::Not => self.execute_2_operand_operator(step, &AluOperator::Not),
            Operator::Lsl => self.execute_3_operand_operator(step, AluOperator::Lsl),
            Operator::Lsr => self.execute_3_operand_operator(step, AluOperator::Lsr),
            Operator::Asl => self.execute_3_operand_operator(step, AluOperator::Asl),
            Operator::Asr => self.execute_3_operand_operator(step, AluOperator::Asr),
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
            Operator::In => self.execute_input(step),
            Operator::Out => self.execute_output(step),
            Operator::EI => self.enable_interrupt(step),
            Operator::DI => self.disable_interrupt(step),
            Operator::VAdd => self.execute_vector(
                step,
                VectorAluOperator::Add,
                VectorModeSelector::Alu,
                BranchSelector::Beq,
            ),
            Operator::VSub => self.execute_vector(
                step,
                VectorAluOperator::Sub,
                VectorModeSelector::Alu,
                BranchSelector::Beq,
            ),
            Operator::VMul => self.execute_vector(
                step,
                VectorAluOperator::Mul,
                VectorModeSelector::Alu,
                BranchSelector::Beq,
            ),
            Operator::VDiv => self.execute_vector(
                step,
                VectorAluOperator::Div,
                VectorModeSelector::Alu,
                BranchSelector::Beq,
            ),
            Operator::VRem => self.execute_vector(
                step,
                VectorAluOperator::Rem,
                VectorModeSelector::Alu,
                BranchSelector::Beq,
            ),
            Operator::VAnd => self.execute_vector(
                step,
                VectorAluOperator::And,
                VectorModeSelector::Alu,
                BranchSelector::Beq,
            ),
            Operator::VOr => self.execute_vector(
                step,
                VectorAluOperator::Or,
                VectorModeSelector::Alu,
                BranchSelector::Beq,
            ),
            Operator::VXor => self.execute_vector(
                step,
                VectorAluOperator::Xor,
                VectorModeSelector::Alu,
                BranchSelector::Beq,
            ),
            Operator::VCmpBeq => self.execute_vector(
                step,
                VectorAluOperator::Sub,
                VectorModeSelector::Decoder,
                BranchSelector::Beq,
            ),
            Operator::VCmpBne => self.execute_vector(
                step,
                VectorAluOperator::Sub,
                VectorModeSelector::Decoder,
                BranchSelector::Bne,
            ),
            Operator::VCmpBgt => self.execute_vector(
                step,
                VectorAluOperator::Sub,
                VectorModeSelector::Decoder,
                BranchSelector::Bgt,
            ),
            Operator::VCmpBge => self.execute_vector(
                step,
                VectorAluOperator::Sub,
                VectorModeSelector::Decoder,
                BranchSelector::Bge,
            ),
            Operator::VCmpBlt => self.execute_vector(
                step,
                VectorAluOperator::Sub,
                VectorModeSelector::Decoder,
                BranchSelector::Blt,
            ),
            Operator::VCmpBle => self.execute_vector(
                step,
                VectorAluOperator::Sub,
                VectorModeSelector::Decoder,
                BranchSelector::Ble,
            ),
            Operator::VCmpBcs => self.execute_vector(
                step,
                VectorAluOperator::Sub,
                VectorModeSelector::Decoder,
                BranchSelector::Bcs,
            ),
            Operator::VCmpBcc => self.execute_vector(
                step,
                VectorAluOperator::Sub,
                VectorModeSelector::Decoder,
                BranchSelector::Bcc,
            ),
            Operator::VCmpBvs => self.execute_vector(
                step,
                VectorAluOperator::Sub,
                VectorModeSelector::Decoder,
                BranchSelector::Bvs,
            ),
            Operator::VCmpBvc => self.execute_vector(
                step,
                VectorAluOperator::Sub,
                VectorModeSelector::Decoder,
                BranchSelector::Bvc,
            ),
        }
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

    fn execute_2_operand_operator(&mut self, step: u8, alu_op: &AluOperator) {
        match step {
            0 => {
                let source = self.parse_data_readable(1);
                let destination = self.parse_data_writable(2);

                if !Self::is_operand_needed_second_step(&source)
                    && !Self::is_operand_needed_second_step(&destination)
                {
                    self.prepare_operand(AluInput::Right, &source, PrepareMode::Immediate);
                    self.log_simple_operand(&destination);
                    self.data_path
                        .update_right_alu_input(AluInputSelector::Immediate);
                    self.data_path
                        .execute_alu(&AluOperator::Trr, &self.word_size);
                    self.store_operand(destination);
                    self.execution_state = ExecutionState::Done;
                } else {
                    self.prepare_operand(AluInput::Right, &source, PrepareMode::Register);
                    self.execution_state = if Self::is_operand_needed_second_step(&source) {
                        ExecutionState::Execute(1)
                    } else {
                        ExecutionState::Execute(2)
                    };
                }
            }
            1 => {
                self.load_indirect_operand(AluInput::Right, PrepareMode::Register);
                self.execution_state = ExecutionState::Execute(2);
            }
            2 => {
                let destination = self.parse_data_writable(2);
                if Self::is_operand_needed_second_step(&destination) {
                    self.prepare_operand(AluInput::None, &destination, PrepareMode::Immediate);
                    self.execution_state = ExecutionState::Execute(3);
                } else {
                    self.log_simple_operand(&destination);
                    self.data_path
                        .update_right_alu_input(AluInputSelector::Register);
                    self.data_path.execute_alu(alu_op, &self.word_size);
                    self.store_operand(destination);
                    self.execution_state = ExecutionState::Done;
                }
            }
            3 => {
                let destination = self.parse_data_writable(2);
                self.load_indirect_operand(AluInput::None, PrepareMode::Immediate);
                self.data_path
                    .update_right_alu_input(AluInputSelector::Register);
                self.data_path
                    .execute_alu(&AluOperator::Trr, &self.word_size);
                self.store_operand(destination);
                self.execution_state = ExecutionState::Execute(4);
            }
            4 => {
                self.store_indirect_operand();
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn execute_cmp(&mut self, step: u8) {
        match step {
            0 => {
                let first = self.parse_data_readable(1);
                self.prepare_operand(AluInput::Right, &first, PrepareMode::Register);
                self.execution_state = if Self::is_operand_needed_second_step(&first) {
                    ExecutionState::Execute(1)
                } else {
                    ExecutionState::Execute(2)
                };
            }
            1 => {
                self.load_indirect_operand(AluInput::Right, PrepareMode::Register);
                self.execution_state = ExecutionState::Execute(2);
            }
            2 => {
                let second = self.parse_data_readable(2);
                self.prepare_operand(AluInput::Left, &second, PrepareMode::Immediate);

                if Self::is_operand_needed_second_step(&second) {
                    self.execution_state = ExecutionState::Execute(3);
                } else {
                    self.data_path
                        .update_left_alu_input(AluInputSelector::Immediate);
                    self.data_path
                        .update_right_alu_input(AluInputSelector::Register);
                    self.data_path
                        .execute_alu(&AluOperator::Sub, &self.word_size);
                    self.execution_state = ExecutionState::Done;
                }
            }
            3 => {
                self.load_indirect_operand(AluInput::Left, PrepareMode::Immediate);
                self.data_path
                    .update_left_alu_input(AluInputSelector::Immediate);
                self.data_path
                    .update_right_alu_input(AluInputSelector::Register);
                self.data_path
                    .execute_alu(&AluOperator::Sub, &self.word_size);
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn execute_3_operand_operator(&mut self, step: u8, alu_op: AluOperator) {
        match step {
            0 => {
                let left = self.parse_data_readable(1);
                let right = self.parse_data_readable(2);
                let destination = self.parse_data_writable(3);

                if !Self::is_operand_needed_second_step(&left)
                    && !Self::is_operand_needed_second_step(&right)
                    && !Self::is_operand_needed_second_step(&destination)
                {
                    self.prepare_operand(AluInput::Left, &left, PrepareMode::Immediate);
                    self.prepare_operand(AluInput::Right, &right, PrepareMode::Immediate);
                    self.log_simple_operand(&destination);
                    self.data_path
                        .update_left_alu_input(AluInputSelector::Immediate);
                    self.data_path
                        .update_right_alu_input(AluInputSelector::Immediate);
                    self.data_path.execute_alu(&alu_op, &self.word_size);
                    self.store_operand(destination);
                    self.execution_state = ExecutionState::Done;
                } else {
                    self.prepare_operand(AluInput::Left, &left, PrepareMode::Register);
                    self.execution_state = if Self::is_operand_needed_second_step(&left) {
                        ExecutionState::Execute(1)
                    } else {
                        ExecutionState::Execute(2)
                    };
                }
            }
            1 => {
                self.load_indirect_operand(AluInput::Left, PrepareMode::Register);
                self.execution_state = ExecutionState::Execute(2);
            }
            2 => {
                let right = self.parse_data_readable(2);
                let destination = self.parse_data_writable(3);

                if !Self::is_operand_needed_second_step(&right)
                    && !Self::is_operand_needed_second_step(&destination)
                {
                    self.prepare_operand(AluInput::Right, &right, PrepareMode::Immediate);
                    self.log_simple_operand(&destination);
                    self.data_path
                        .update_left_alu_input(AluInputSelector::Register);
                    self.data_path
                        .update_right_alu_input(AluInputSelector::Immediate);
                    self.data_path.execute_alu(&alu_op, &self.word_size);
                    self.store_operand(destination);
                    self.execution_state = ExecutionState::Done;
                } else {
                    self.prepare_operand(AluInput::Right, &right, PrepareMode::Register);
                    self.execution_state = if Self::is_operand_needed_second_step(&right) {
                        ExecutionState::Execute(3)
                    } else {
                        ExecutionState::Execute(4)
                    };
                }
            }
            3 => {
                self.load_indirect_operand(AluInput::Right, PrepareMode::Register);
                self.execution_state = ExecutionState::Execute(4);
            }
            4 => {
                let destination = self.parse_data_writable(3);
                if Self::is_operand_needed_second_step(&destination) {
                    self.prepare_operand(AluInput::None, &destination, PrepareMode::Immediate);
                    self.execution_state = ExecutionState::Execute(5);
                } else {
                    self.log_simple_operand(&destination);
                    self.data_path
                        .update_left_alu_input(AluInputSelector::Register);
                    self.data_path
                        .update_right_alu_input(AluInputSelector::Register);
                    self.data_path.execute_alu(&alu_op, &self.word_size);
                    self.store_operand(destination);
                    self.execution_state = ExecutionState::Done;
                }
            }
            5 => {
                let destination = self.parse_data_writable(3);
                self.load_indirect_operand(AluInput::None, PrepareMode::Register);
                self.data_path
                    .update_left_alu_input(AluInputSelector::Register);
                self.data_path
                    .update_right_alu_input(AluInputSelector::Register);
                self.data_path.execute_alu(&alu_op, &self.word_size);
                self.store_operand(destination);
                self.execution_state = ExecutionState::Execute(6);
            }
            6 => {
                self.store_indirect_operand();
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn execute_vector(
        &mut self,
        step: u8,
        operator: VectorAluOperator,
        vector_mode_selector: VectorModeSelector,
        branch_selector: BranchSelector,
    ) {
        match step {
            0 => {
                let right = self.parse_data_readable(1);
                self.prepare_operand(AluInput::Left, &right, PrepareMode::Immediate);
                if Self::is_operand_needed_second_step(&right) {
                    self.execution_state = ExecutionState::Execute(1);
                } else {
                    self.data_path
                        .update_left_alu_input(AluInputSelector::Immediate);
                    self.data_path
                        .execute_alu(&AluOperator::Trl, &WordSize::Long);
                    self.data_path.latch_data_address();
                    self.execution_state = ExecutionState::Execute(2);
                }
            }
            1 => {
                self.load_indirect_operand(AluInput::Left, PrepareMode::Immediate);
                self.data_path
                    .update_left_alu_input(AluInputSelector::Immediate);
                self.data_path
                    .execute_alu(&AluOperator::Trl, &WordSize::Long);
                self.data_path.latch_data_address();
                self.execution_state = ExecutionState::Execute(2);
            }
            2 => {
                self.data_path.read_data_memory();
                self.data_path.latch_right_vector_input_register();
                let left = self.parse_data_readable(2);
                self.prepare_operand(AluInput::Left, &left, PrepareMode::Immediate);
                if Self::is_operand_needed_second_step(&left) {
                    self.execution_state = ExecutionState::Execute(3);
                } else {
                    self.data_path
                        .update_left_alu_input(AluInputSelector::Immediate);
                    self.data_path
                        .execute_alu(&AluOperator::Trl, &WordSize::Long);
                    self.data_path.latch_data_address();
                    self.execution_state = ExecutionState::Execute(4);
                }
            }
            3 => {
                self.load_indirect_operand(AluInput::Left, PrepareMode::Immediate);
                self.data_path
                    .update_left_alu_input(AluInputSelector::Immediate);
                self.data_path
                    .execute_alu(&AluOperator::Trl, &WordSize::Long);
                self.data_path.latch_data_address();
                self.execution_state = ExecutionState::Execute(4);
            }
            4 => {
                self.data_path.read_data_memory();
                self.data_path.update_left_vector_input();
                self.data_path.execute_vector_alu(operator);
                self.data_path
                    .update_vector_alu_output_mux(vector_mode_selector, branch_selector);
                self.data_path.latch_vector_output_register();

                let destination = self.parse_data_readable(3);
                self.prepare_operand(AluInput::Left, &destination, PrepareMode::Immediate);
                if Self::is_operand_needed_second_step(&destination) {
                    self.execution_state = ExecutionState::Execute(5);
                } else {
                    self.data_path
                        .update_left_alu_input(AluInputSelector::Immediate);
                    self.data_path
                        .execute_alu(&AluOperator::Trl, &WordSize::Long);
                    self.data_path.latch_data_address();
                    self.execution_state = ExecutionState::Execute(6);
                }
            }
            5 => {
                self.load_indirect_operand(AluInput::Left, PrepareMode::Immediate);
                self.data_path
                    .update_left_alu_input(AluInputSelector::Immediate);
                self.data_path
                    .execute_alu(&AluOperator::Trl, &WordSize::Long);
                self.data_path.latch_data_address();
                self.execution_state = ExecutionState::Execute(6);
            }
            6 => {
                self.data_path.write_data_memory(WriteSelector::Vector);
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn execute_jump(&mut self, step: u8) {
        match step {
            0 => {
                self.update_read_data();
                self.latch_pc(PcSelector::ByAddress);
                self.log_operand(&format!("{}", self.pc));
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn execute_call(&mut self, step: u8) {
        match step {
            0 => {
                self.update_read_data();
                self.update_output_pc_mux(OutputPcSelector::Next);
                self.update_control_unit_output(OutputSelector::PC);
                self.data_path.external_selector = ExternalSelector::ControlUnit;
                self.data_path.update_right_data_mux(DataSelector::External);
                self.data_path.latch_right_data_register();
                self.latch_pc(PcSelector::ByAddress);
                self.log_operand(&format!("{}", self.pc));
                self.prepare_stack_with_decrement();
                self.execution_state = ExecutionState::Execute(1);
            }
            1 => {
                self.data_path.read_data_memory();
                self.data_path
                    .update_write_data_mux(WriteDataSelector::Memory);
                self.data_path.latch_write_data();
                self.execution_state = ExecutionState::Execute(2);
            }
            2 => {
                self.data_path
                    .update_right_alu_input(AluInputSelector::Register);
                self.data_path
                    .execute_alu(&AluOperator::Trr, &WordSize::Long);
                self.data_path.update_write_data_mux(WriteDataSelector::Alu);
                self.data_path.latch_write_data_part(&WordSize::Long);
                self.execution_state = ExecutionState::Execute(3);
            }
            3 => {
                self.data_path.write_data_memory(WriteSelector::Scalar);
                self.log_event(&format!(
                    "Write: MEM[{}] = {}",
                    self.data_path.data_address, self.data_path.right_data_register
                ));
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn execute_return(&mut self, step: u8) {
        match step {
            0 => {
                self.prepare_stack_with_increment();
                self.execution_state = ExecutionState::Execute(1);
            }
            1 => {
                self.data_path.read_data_memory();
                self.data_path.update_memory_output_mux(&WordSize::Long);
                self.log_event(&format!(
                    "Read: MEM[{}] = {}",
                    self.data_path.data_address, self.data_path.memory_output_mux as i64
                ));
                self.data_path.update_left_data_mux(DataSelector::Memory);
                self.data_path
                    .update_left_alu_input(AluInputSelector::Immediate);
                self.data_path
                    .execute_alu(&AluOperator::Trl, &WordSize::Long);
                self.latch_pc(PcSelector::ByDataPath);
                self.log_event(&format!("Returning to {}", self.pc));
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn execute_interrupt(&mut self, step: u8) {
        match step {
            0 => {
                self.log_event("INTERRUPT received");
                self.interrupt_flag = false;
                self.update_output_pc_mux(OutputPcSelector::Current);
                self.update_control_unit_output(OutputSelector::PC);
                self.latch_pc(PcSelector::ByInterrupt);
                self.log_event(&format!("Interrupt jump to {}", self.pc));
                self.prepare_stack_with_decrement();
                self.execution_state = ExecutionState::Execute(1);
            }
            1 => {
                self.data_path.read_data_memory();
                self.data_path
                    .update_write_data_mux(WriteDataSelector::Memory);
                self.data_path.latch_write_data();
                self.execution_state = ExecutionState::Execute(2);
            }
            2 => {
                self.data_path.set_nzcv_to_alu_output();
                self.data_path.update_write_data_mux(WriteDataSelector::Alu);
                self.data_path.latch_write_data_part(&WordSize::Byte);
                self.execution_state = ExecutionState::Execute(3);
            }
            3 => {
                self.data_path.write_data_memory(WriteSelector::Scalar);
                self.log_event(&format!(
                    "Write: MEM[{}] = {}",
                    self.data_path.data_address, self.data_path.alu_output as i64
                ));
                self.prepare_stack_with_decrement();
                self.execution_state = ExecutionState::Execute(4);
            }
            4 => {
                self.data_path.read_data_memory();
                self.data_path
                    .update_write_data_mux(WriteDataSelector::Memory);
                self.data_path.latch_write_data();
                self.execution_state = ExecutionState::Execute(5);
            }
            5 => {
                self.data_path.external_selector = ExternalSelector::ControlUnit;
                self.data_path.update_left_data_mux(DataSelector::External);
                self.data_path
                    .update_left_alu_input(AluInputSelector::Immediate);
                self.data_path
                    .execute_alu(&AluOperator::Trl, &WordSize::Long);
                self.data_path.update_write_data_mux(WriteDataSelector::Alu);
                self.data_path.latch_write_data_part(&WordSize::Long);
                self.execution_state = ExecutionState::Execute(6);
            }
            6 => {
                self.data_path.write_data_memory(WriteSelector::Scalar);
                self.log_event(&format!(
                    "Write: MEM[{}] = {}",
                    self.data_path.data_address, self.data_path.alu_output as i64
                ));
                self.interrupt_processing = false;
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn execute_interrupt_return(&mut self, step: u8) {
        match step {
            0 => {
                self.prepare_stack_with_increment();
                self.execution_state = ExecutionState::Execute(1);
            }
            1 => {
                self.data_path.read_data_memory();
                self.data_path.update_memory_output_mux(&WordSize::Long);
                self.log_event(&format!(
                    "Read: MEM[{}] = {}",
                    self.data_path.data_address, self.data_path.memory_output_mux as i64
                ));
                self.data_path.update_left_data_mux(DataSelector::Memory);
                self.data_path
                    .update_left_alu_input(AluInputSelector::Immediate);
                self.data_path
                    .execute_alu(&AluOperator::Trl, &WordSize::Long);
                self.latch_pc(PcSelector::ByDataPath);
                self.execution_state = ExecutionState::Execute(2);
            }
            2 => {
                self.prepare_stack_with_increment();
                self.execution_state = ExecutionState::Execute(3);
            }
            3 => {
                self.data_path.read_data_memory();
                self.data_path.update_memory_output_mux(&WordSize::Byte);
                self.log_event(&format!(
                    "Read: MEM[{}] = {}",
                    self.data_path.data_address, self.data_path.memory_output_mux as i64
                ));
                self.data_path.update_left_data_mux(DataSelector::Memory);
                self.data_path
                    .update_left_alu_input(AluInputSelector::Immediate);
                self.data_path
                    .execute_alu(&AluOperator::Trl, &WordSize::Byte);
                self.data_path.restore_nzcv();
                self.log_event("Restored NZCV flags");

                self.interrupt_flag = true;
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn assemble_branch_address(&self) -> u64 {
        let high = ((self.ir & 0x00ffffff) as u64) << 32;
        let low = self.read_data as u64;
        high | low
    }

    fn execute_branch(&mut self, step: u8, condition: bool) {
        match step {
            0 => {
                self.update_read_data();
                self.log_operand(&format!("{}", self.assemble_branch_address()));
                if condition {
                    self.log_event("Branch taken");
                    self.latch_pc(PcSelector::ByAddress);
                } else {
                    self.log_event("Branch skipped");
                    self.latch_pc(PcSelector::NextWord);
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
                self.log_event(&format!(
                    "IN from port #{}: Value = {}",
                    port, self.data_path.io.output as i64
                ));
                self.log_operand(&format!("#{}", port));

                let operand = self.parse_data_writable(2);
                self.prepare_operand(AluInput::Left, &operand, PrepareMode::Immediate);
                if Self::is_operand_needed_second_step(&operand) {
                    self.data_path.update_right_data_mux(DataSelector::External);
                    self.data_path.latch_right_data_register();
                    self.execution_state = ExecutionState::Execute(1);
                } else {
                    self.data_path.update_right_data_mux(DataSelector::External);
                    self.data_path
                        .update_right_alu_input(AluInputSelector::Immediate);
                    self.data_path
                        .execute_alu(&AluOperator::Trr, &self.word_size);
                    self.store_operand(operand);
                    self.execution_state = ExecutionState::Done;
                }
            }
            1 => {
                let operand = self.parse_data_writable(2);
                self.load_indirect_operand(AluInput::Left, PrepareMode::Immediate);
                self.data_path
                    .update_right_alu_input(AluInputSelector::Register);
                self.data_path
                    .execute_alu(&AluOperator::Trr, &self.word_size);
                self.store_operand(operand);
                self.execution_state = ExecutionState::Execute(2);
            }
            2 => {
                self.store_indirect_operand();
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
                self.log_operand(&format!("#{}", port));
                self.prepare_operand(AluInput::Left, &operand, PrepareMode::Immediate);
                if Self::is_operand_needed_second_step(&operand) {
                    self.execution_state = ExecutionState::Execute(1);
                } else {
                    self.data_path
                        .update_left_alu_input(AluInputSelector::Immediate);
                    self.data_path
                        .execute_alu(&AluOperator::Trl, &self.word_size);
                    self.data_path.write_io(port);
                    self.log_event(&format!(
                        "OUT to port #{}: Value = {}",
                        port, self.data_path.alu_output as i64
                    ));
                    self.execution_state = ExecutionState::Done;
                }
            }
            1 => {
                let port = self.parse_port(1);
                self.load_indirect_operand(AluInput::Left, PrepareMode::Immediate);
                self.data_path
                    .update_left_alu_input(AluInputSelector::Immediate);
                self.data_path
                    .execute_alu(&AluOperator::Trl, &self.word_size);
                self.data_path.write_io(port);
                self.log_event(&format!(
                    "OUT to port #{}: Value = {}",
                    port, self.data_path.alu_output as i64
                ));
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn enable_interrupt(&mut self, step: u8) {
        match step {
            0 => {
                self.interrupt_flag = true;
                self.log_event("Interrupts ENABLED");
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn disable_interrupt(&mut self, step: u8) {
        match step {
            0 => {
                self.interrupt_flag = false;
                self.log_event("Interrupts DISABLED");
                self.execution_state = ExecutionState::Done;
            }
            _ => unreachable!(),
        }
    }

    fn prepare_left_alu_input(&mut self, mode: PrepareMode) {
        match mode {
            PrepareMode::Immediate => {}
            PrepareMode::Register => {
                self.data_path.latch_left_data_register();
            }
        }
    }

    fn prepare_right_alu_input(&mut self, mode: PrepareMode) {
        match mode {
            PrepareMode::Immediate => {}
            PrepareMode::Register => {
                self.data_path.latch_right_data_register();
            }
        }
    }

    fn prepare_operand(&mut self, input: AluInput, operand: &Operand, mode: PrepareMode) {
        match operand.mode {
            Mode::Direct => {
                self.update_read_data();
                self.update_control_unit_output(OutputSelector::Memory);
                self.latch_pc(PcSelector::NextWord);
                self.data_path.external_selector = ExternalSelector::ControlUnit;
                match input {
                    AluInput::Left => {
                        self.data_path.update_left_data_mux(DataSelector::External);
                        self.prepare_left_alu_input(mode);
                    }
                    AluInput::Right => {
                        self.data_path.update_right_data_mux(DataSelector::External);
                        self.prepare_right_alu_input(mode);
                    }
                    AluInput::None => {}
                }
                self.log_operand(&format!("#{}", self.read_data as i32));
            }
            Mode::DataRegister => {
                self.data_path.read_data_register(operand.main_register);
                match input {
                    AluInput::Left => {
                        self.data_path
                            .update_left_data_mux(DataSelector::DataRegister);
                        self.prepare_left_alu_input(mode);
                    }
                    AluInput::Right => {
                        self.data_path
                            .update_right_data_mux(DataSelector::DataRegister);
                        self.prepare_right_alu_input(mode);
                    }
                    AluInput::None => {}
                }
                self.log_operand(&format!("D{}", operand.main_register));
            }
            Mode::AddressRegister => {
                self.data_path.read_address_register(operand.main_register);
                match input {
                    AluInput::Left => {
                        self.data_path
                            .update_left_data_mux(DataSelector::AddressRegister);
                        self.prepare_left_alu_input(mode);
                    }
                    AluInput::Right => {
                        self.data_path
                            .update_right_data_mux(DataSelector::AddressRegister);
                        self.prepare_right_alu_input(mode);
                    }
                    AluInput::None => {}
                }
                self.log_operand(&format!("A{}", operand.main_register));
            }
            Mode::Indirect | Mode::IndirectPostIncrement | Mode::IndirectPreDecrement => {
                self.data_path.read_address_register(operand.main_register);
                self.data_path
                    .update_left_data_mux(DataSelector::AddressRegister);
                self.data_path
                    .update_left_alu_input(AluInputSelector::Immediate);
                self.data_path
                    .execute_alu(&AluOperator::Trl, &WordSize::Long);

                match operand.mode {
                    Mode::Indirect => {}
                    Mode::IndirectPreDecrement => {
                        self.data_path.pre_mode_selector = PreModeSelector::Decrement
                    }
                    Mode::IndirectPostIncrement => {
                        self.data_path.post_mode_selector = PostModeSelector::Increment
                    }
                    _ => unreachable!(),
                }
                self.data_path.latch_data_address();
                self.data_path
                    .latch_address_register(operand.main_register, &WordSize::Long);

                match operand.mode {
                    Mode::Indirect => self.log_operand(&format!("(A{})", operand.main_register)),
                    Mode::IndirectPreDecrement => {
                        self.log_operand(&format!("-(A{})", operand.main_register))
                    }
                    Mode::IndirectPostIncrement => {
                        self.log_operand(&format!("(A{})+", operand.main_register))
                    }
                    _ => unreachable!(),
                }
                self.data_path.pre_mode_selector = PreModeSelector::None;
                self.data_path.post_mode_selector = PostModeSelector::None;
            }
            Mode::IndirectOffset => {
                self.data_path.read_address_register(operand.main_register);
                self.data_path
                    .update_left_data_mux(DataSelector::AddressRegister);
                self.data_path
                    .update_left_alu_input(AluInputSelector::Immediate);
                if operand.offset == 0 {
                    self.update_read_data();
                    self.update_control_unit_output(OutputSelector::Memory);
                    self.latch_pc(PcSelector::NextWord);
                    self.data_path.external_selector = ExternalSelector::ControlUnit;
                    self.data_path.update_right_data_mux(DataSelector::External);
                    self.data_path
                        .update_right_alu_input(AluInputSelector::Immediate);
                    self.log_operand(&format!(
                        "(A{}, #{})",
                        operand.main_register, self.read_data as i32
                    ));
                } else {
                    let offset_register = operand.offset + 4;
                    self.data_path.read_data_register(offset_register);
                    self.data_path
                        .update_right_data_mux(DataSelector::DataRegister);
                    self.data_path
                        .update_right_alu_input(AluInputSelector::Immediate);
                    self.log_operand(&format!(
                        "(A{}, D{})",
                        operand.main_register, offset_register
                    ));
                }
                self.data_path
                    .execute_alu(&AluOperator::Add, &WordSize::Long);
                self.data_path.latch_data_address();
            }
            Mode::IndirectDirect => {
                self.update_read_data();
                self.update_control_unit_output(OutputSelector::Memory);
                self.latch_pc(PcSelector::NextWord);
                self.data_path.external_selector = ExternalSelector::ControlUnit;
                self.data_path.update_left_data_mux(DataSelector::External);
                self.data_path
                    .update_left_alu_input(AluInputSelector::Immediate);
                self.data_path
                    .execute_alu(&AluOperator::Trl, &WordSize::Long);
                self.data_path.latch_data_address();
                self.log_operand(&format!("({})", self.read_data));
            }
        }
    }

    fn prepare_stack_with_increment(&mut self) {
        self.data_path.read_address_register(7);
        self.data_path
            .update_left_data_mux(DataSelector::AddressRegister);
        self.data_path
            .update_left_alu_input(AluInputSelector::Immediate);
        self.data_path
            .execute_alu(&AluOperator::Trl, &WordSize::Long);
        self.data_path.post_mode_selector = PostModeSelector::Increment;
        self.data_path.latch_data_address();
        self.data_path.latch_address_register(7, &WordSize::Long);
        self.data_path.post_mode_selector = PostModeSelector::None;
    }

    fn prepare_stack_with_decrement(&mut self) {
        self.data_path.read_address_register(7);
        self.data_path
            .update_left_data_mux(DataSelector::AddressRegister);
        self.data_path
            .update_left_alu_input(AluInputSelector::Immediate);
        self.data_path
            .execute_alu(&AluOperator::Trl, &WordSize::Long);
        self.data_path.pre_mode_selector = PreModeSelector::Decrement;
        self.data_path.latch_data_address();
        self.data_path.latch_address_register(7, &WordSize::Long);
        self.data_path.pre_mode_selector = PreModeSelector::None;
    }

    fn load_indirect_operand(&mut self, input: AluInput, mode: PrepareMode) {
        self.data_path.read_data_memory();
        self.data_path.update_memory_output_mux(&self.word_size);
        self.data_path
            .update_write_data_mux(WriteDataSelector::Memory);
        self.data_path.latch_write_data();
        match input {
            AluInput::Left => {
                self.data_path.update_left_data_mux(DataSelector::Memory);
                self.prepare_left_alu_input(mode);
            }
            AluInput::Right => {
                self.data_path.update_right_data_mux(DataSelector::Memory);
                self.prepare_right_alu_input(mode);
            }
            AluInput::None => {}
        }
        self.log_event(&format!(
            "Read: MEM[{}] = {}",
            self.data_path.data_address, self.data_path.memory_output_mux as i64
        ));
    }

    fn store_operand(&mut self, operand: Operand) {
        match operand.mode {
            Mode::DataRegister => {
                self.data_path
                    .latch_data_register(operand.main_register, &self.word_size);
                self.log_event(&format!(
                    "Write: D{} = {}",
                    operand.main_register, self.data_path.alu_output as i64
                ));
            }
            Mode::AddressRegister => {
                self.data_path
                    .latch_address_register(operand.main_register, &self.word_size);
                self.log_event(&format!(
                    "Write: A{} = {}",
                    operand.main_register, self.data_path.alu_output as i64
                ));
            }
            Mode::Indirect
            | Mode::IndirectPostIncrement
            | Mode::IndirectPreDecrement
            | Mode::IndirectOffset
            | Mode::IndirectDirect => {
                self.data_path.update_write_data_mux(WriteDataSelector::Alu);
                self.data_path.latch_write_data_part(&self.word_size);
            }
            _ => unreachable!(),
        }
    }

    fn store_indirect_operand(&mut self) {
        self.data_path.write_data_memory(WriteSelector::Scalar);
        self.log_event(&format!(
            "Write: MEM[{}] = {}",
            self.data_path.data_address, self.data_path.alu_output as i64
        ));
    }
}

impl ControlUnit {
    pub fn load_program(&mut self, program: &[u32]) {
        program.iter().enumerate().for_each(|(i, word)| {
            self.program_memory.write(i as Address, *word);
        })
    }

    pub fn load_data_section(&mut self, data_section: Vec<u64>) {
        data_section.iter().enumerate().for_each(|(i, value)| {
            self.data_path.data_address = i as u64;
            self.data_path.read_data_memory();
            self.data_path
                .update_write_data_mux(WriteDataSelector::Memory);
            self.data_path.latch_write_data();

            self.data_path.alu_output = *value;
            self.data_path.update_write_data_mux(WriteDataSelector::Alu);
            self.data_path.latch_write_data_part(&WordSize::Long);
            self.data_path.write_data_memory(WriteSelector::Scalar);
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
            program_memory: ProgramMemory::new(100000),
            read_data: 0,
            ir: 0,
            pc: 0,
            word_size: WordSize::Long,
            operator: Operator::Hlt,
            execution_state: ExecutionState::Start,
            interrupt_flag: true,
            interrupt_processing: false,
            vector_table: HashMap::new(),
            tick: 0,
            current_trace: None,
            output_pc_mux: 0,
        }
    }
}
