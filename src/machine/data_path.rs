use crate::isa::WordSize;
use crate::machine::alu::{Alu, AluOperator};
use crate::machine::io::IO;
use crate::machine::memory::Memory;
use crate::machine::nzcv::Nzcv;

pub type DataRegisterReadSelector = u8;

pub type DataRegisterWriteSelector = u8;

pub type AddressRegisterReadSelector = u8;

pub type AddressRegisterWriteSelector = u8;

pub enum DataSelector {
    DataRegister,
    AddressRegister,
    ReadData,
    External,
}

pub enum BufferSelector {
    DataRegister,
    External,
}

pub enum AluInputSelector {
    Data,
    Buffer,
}

pub enum PreModeSelector {
    None,
    DecrementByte,
    DecrementWord,
}

pub enum PostModeSelector {
    None,
    IncrementByte,
    IncrementWord,
}

#[allow(dead_code)]
pub enum ExternalSelector {
    ControlUnit,
    IO,
}

pub struct DataPath {
    pub data_memory: Memory,

    left_alu_input: i64,
    right_alu_input: i64,
    alu: Alu,
    alu_output: i64,

    data_registers_mux: i64,
    pub data_registers: Vec<i64>,

    address_registers_mux: i64,
    pub address_registers: Vec<i64>,

    left_data: i64,
    left_buffer: i64,
    right_data: i64,
    right_buffer: i64,

    pub external_selector: ExternalSelector,

    pub pre_mode_selector: PreModeSelector,
    pub post_mode_selector: PostModeSelector,

    pub memory_output: i64,
    pub data_address: u64,
    pub read_data: i64,
    pub write_data: i64,

    pub io: IO,
    pub control_unit_output: i64,
}

impl DataPath {
    pub fn read_data_register(&mut self, selector: DataRegisterReadSelector) {
        self.data_registers_mux = self.data_registers[selector as usize];
    }

    pub fn latch_data_register(
        &mut self,
        register_selector: DataRegisterWriteSelector,
        word_size_selector: &WordSize,
    ) {
        match word_size_selector {
            WordSize::Byte => {
                self.data_registers[register_selector as usize] = self.alu_output & 0xFF
            }
            WordSize::Long => self.data_registers[register_selector as usize] = self.alu_output,
        }
    }

    pub fn read_address_register(&mut self, selector: AddressRegisterReadSelector) {
        self.address_registers_mux = self.address_registers[selector as usize];
    }

    pub fn latch_address_register(
        &mut self,
        register_selector: AddressRegisterWriteSelector,
        word_size_selector: &WordSize,
    ) {
        let decrement = match self.pre_mode_selector {
            PreModeSelector::None => 0,
            PreModeSelector::DecrementByte => 1,
            PreModeSelector::DecrementWord => 8,
        };
        let increment = match self.post_mode_selector {
            PostModeSelector::None => 0,
            PostModeSelector::IncrementByte => 1,
            PostModeSelector::IncrementWord => 8,
        };

        let input = self.alu_output + increment - decrement;
        match word_size_selector {
            WordSize::Byte => self.address_registers[register_selector as usize] = input & 0xFF,
            WordSize::Long => self.address_registers[register_selector as usize] = input,
        }
    }

    pub fn read_data_memory(&mut self) {
        self.memory_output = self.data_memory.read_u64(self.data_address) as i64;
    }

    pub fn write_data_memory(&mut self, selector: &WordSize) {
        match selector {
            WordSize::Byte => {
                self.data_memory
                    .write_u8(self.data_address, (self.write_data & 0xff) as u8);
            }
            WordSize::Long => {
                self.data_memory
                    .write_u64(self.data_address, self.write_data as u64);
            }
        }
    }

    fn update_data(&mut self, selector: DataSelector) -> i64 {
        match selector {
            DataSelector::DataRegister => self.data_registers_mux,
            DataSelector::AddressRegister => self.address_registers_mux,
            DataSelector::ReadData => self.read_data,
            DataSelector::External => match self.external_selector {
                ExternalSelector::ControlUnit => self.control_unit_output,
                ExternalSelector::IO => self.io.output,
            },
        }
    }

    fn update_buffer(&mut self, selector: BufferSelector) -> i64 {
        match selector {
            BufferSelector::DataRegister => self.data_registers_mux,
            BufferSelector::External => match self.external_selector {
                ExternalSelector::ControlUnit => self.control_unit_output,
                ExternalSelector::IO => self.io.output,
            },
        }
    }

    pub fn update_left_data(&mut self, selector: DataSelector) {
        self.left_data = self.update_data(selector);
    }

    pub fn update_left_buffer(&mut self, selector: BufferSelector) {
        self.left_buffer = self.update_buffer(selector);
    }

    pub fn update_left_alu_input(&mut self, selector: AluInputSelector) {
        self.left_alu_input = match selector {
            AluInputSelector::Data => self.left_data,
            AluInputSelector::Buffer => self.left_buffer,
        }
    }

    pub fn update_right_data(&mut self, selector: DataSelector) {
        self.right_data = self.update_data(selector);
    }

    pub fn update_right_buffer(&mut self, selector: BufferSelector) {
        self.right_buffer = self.update_buffer(selector);
    }

    pub fn update_right_alu_input(&mut self, selector: AluInputSelector) {
        self.right_alu_input = match selector {
            AluInputSelector::Data => self.right_data,
            AluInputSelector::Buffer => self.right_buffer,
        }
    }

    pub fn execute_alu(&mut self, operator: AluOperator) {
        self.alu_output = self.alu.execute_operator(
            operator,
            self.left_alu_input as u64,
            self.right_alu_input as u64,
        ) as i64;
    }

    pub fn latch_data_address(&mut self) {
        self.data_address = match self.pre_mode_selector {
            PreModeSelector::None => self.alu_output as u64,
            PreModeSelector::DecrementByte => (self.alu_output - 1) as u64,
            PreModeSelector::DecrementWord => (self.alu_output - 8) as u64,
        };
    }

    pub fn latch_read_data(&mut self) {
        self.read_data = self.memory_output;
    }

    pub fn latch_write_data(&mut self) {
        self.write_data = self.alu_output;
    }

    pub fn transmit_nzcv(&self) -> &Nzcv {
        &self.alu.nzcv
    }

    pub fn restore_nzcv(&mut self) {
        self.alu.nzcv.restore((self.alu_output & 0xff) as u8);
    }

    pub fn read_io(&mut self, port: u8) {
        self.io.read(port);
    }

    pub fn write_io(&mut self, port: u8) {
        self.io.input = self.alu_output;
        self.io.write(port);
    }
}

impl Default for DataPath {
    fn default() -> Self {
        Self {
            data_memory: Memory::new(100000),
            left_alu_input: 0,
            right_alu_input: 0,
            alu: Alu::default(),
            alu_output: 0,
            data_registers_mux: 0,
            data_registers: vec![0; 8],
            address_registers_mux: 0,
            address_registers: vec![0; 8],
            left_data: 0,
            left_buffer: 0,
            right_data: 0,
            right_buffer: 0,
            external_selector: ExternalSelector::ControlUnit,
            pre_mode_selector: PreModeSelector::None,
            post_mode_selector: PostModeSelector::None,
            memory_output: 0,
            data_address: 0,
            read_data: 0,
            write_data: 0,
            io: IO::new(),
            control_unit_output: 0,
        }
    }
}
