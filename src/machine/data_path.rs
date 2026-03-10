use crate::machine::alu::{ALU, AluOperator};
use crate::machine::memory::Memory;
use crate::machine::nzcv::NZCV;

pub type DataRegisterReadSelector = u8;

pub type DataRegisterWriteSelector = u8;

pub type AddressRegisterReadSelector = u8;

pub type AddressRegisterWriteSelector = u8;

pub enum AddressingModeSelector {
    DataRegister,
    AddressRegister,
    ReadData,
    ControlUnit,
    Buffer,
}

pub struct DataPath {
    data_memory: Memory<i64>,

    alu: ALU,
    alu_output: i64,

    alu_input_mux: i64,
    buffer: i64,
    left_alu: i64,
    right_alu: i64,

    d_registers_mux: i64,
    d_registers: Vec<i64>,

    a_registers_mux: i64,
    a_registers: Vec<i64>,

    memory_output: i64,
    data_address: u64,
    read_data: i64,
    write_data: i64,

    pub control_unit_output: i64,
}

impl DataPath {
    pub fn read_data_register(&mut self, selector: DataRegisterReadSelector) {
        self.d_registers_mux = self.d_registers[selector as usize];
    }

    pub fn latch_data_register(&mut self, selector: DataRegisterWriteSelector) {
        self.d_registers[selector as usize] = self.alu_output;
    }

    pub fn read_address_register(&mut self, selector: AddressRegisterReadSelector) {
        self.a_registers_mux = self.a_registers[selector as usize];
    }

    pub fn latch_address_register(&mut self, selector: AddressRegisterWriteSelector) {
        self.a_registers[selector as usize] = self.alu_output;
    }

    pub fn read_data_memory(&mut self) {
        self.memory_output = self.data_memory.read(self.data_address)
    }

    pub fn write_data_memory(&mut self) {
        self.data_memory
            .write(self.data_address as usize, self.write_data)
    }

    pub fn update_alu_input_mux(&mut self, selector: AddressingModeSelector) {
        self.alu_input_mux = match selector {
            AddressingModeSelector::DataRegister => self.d_registers_mux,
            AddressingModeSelector::AddressRegister => self.a_registers_mux,
            AddressingModeSelector::ReadData => self.read_data,
            AddressingModeSelector::ControlUnit => self.control_unit_output,
            AddressingModeSelector::Buffer => self.buffer,
        };
    }

    pub fn execute_alu(&mut self, operator: AluOperator) {
        self.alu
            .execute_operator(operator, self.left_alu as u64, self.right_alu as u64);
    }

    pub fn latch_buffer(&mut self) {
        self.buffer = self.alu_input_mux;
    }

    pub fn latch_left_alu(&mut self) {
        self.left_alu = self.alu_input_mux;
    }

    pub fn latch_right_alu(&mut self) {
        self.right_alu = self.alu_input_mux;
    }

    pub fn latch_data_address(&mut self) {
        self.data_address = self.alu_output as u64;
    }

    pub fn latch_read_data(&mut self) {
        self.read_data = self.memory_output;
    }

    pub fn latch_write_data(&mut self) {
        self.write_data = self.alu_output;
    }

    pub fn transmit_nzcv(&self) -> &NZCV {
        &self.alu.nzcv
    }
}

impl Default for DataPath {
    fn default() -> Self {
        Self {
            data_memory: Memory::new(),
            alu: ALU::default(),
            alu_output: 0,
            alu_input_mux: 0,
            buffer: 0,
            left_alu: 0,
            right_alu: 0,
            d_registers_mux: 0,
            d_registers: vec![],
            a_registers_mux: 0,
            a_registers: vec![],
            memory_output: 0,
            data_address: 0,
            read_data: 0,
            write_data: 0,
            control_unit_output: 0,
        }
    }
}
