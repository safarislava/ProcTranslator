use crate::isa::WordSize;
use crate::machine::alu::{Alu, AluOperator};
use crate::machine::data_memory::{DataMemory, VectorWord};
use crate::machine::io::IO;
use crate::machine::nzcv::Nzcv;
use crate::machine::vector_alu::{VectorAlu, VectorAluOperator};

pub type DataRegisterSelector = u8;

pub type AddressRegisterSelector = u8;

pub enum WriteSelector {
    Scalar,
    Vector,
}

pub enum WriteDataSelector {
    Memory,
    Alu,
}

pub enum DataSelector {
    DataRegister,
    AddressRegister,
    Memory,
    External,
}

pub enum OffsetSelector {
    DataRegister,
    External,
}

pub enum AluInputSelector {
    Data,
    Offset,
}

pub enum PreModeSelector {
    None,
    Decrement,
}

pub enum PostModeSelector {
    None,
    Increment,
}

pub enum ExternalSelector {
    ControlUnit,
    IO,
}

pub enum VectorModeSelector {
    Alu,
    Decoder,
}

pub enum BranchSelector {
    Beq,
    Bne,
    Bgt,
    Bge,
    Blt,
    Ble,
    Bcs,
    Bcc,
    Bvs,
    Bvc,
}

pub struct DataPath {
    right_data_register: u64,
    right_offset: u64,
    left_alu_input: u64,
    right_alu_input: u64,
    alu: Alu,
    pub alu_output: u64,

    data_registers_mux: u64,
    pub data_registers: [u64; 8],

    address_registers_mux: u64,
    pub address_registers: [u64; 8],

    pub external_selector: ExternalSelector,

    pub pre_mode_selector: PreModeSelector,
    pub post_mode_selector: PostModeSelector,

    pub data_memory: DataMemory,
    pub memory_output: VectorWord,
    pub data_address: u64,
    pub memory_output_mux: u64,
    pub write_data: VectorWord,
    pub write_data_mux: VectorWord,

    pub io: IO,
    pub control_unit_output: u64,

    vector_alu: VectorAlu,
    pub left_vector_input: VectorWord,
    pub right_vector_input_register: VectorWord,
    pub vector_alu_output: VectorWord,
    pub vector_output_mux: VectorWord,
    pub vector_output_register: VectorWord,
}

impl DataPath {
    pub fn read_data_register(&mut self, selector: DataRegisterSelector, word_size: &WordSize) {
        self.data_registers_mux = match word_size {
            WordSize::Byte => self.data_registers[selector as usize] & 0xFF,
            WordSize::Long => self.data_registers[selector as usize],
        }
    }

    pub fn latch_data_register(
        &mut self,
        register_selector: DataRegisterSelector,
        word_size_selector: &WordSize,
    ) {
        self.data_registers[register_selector as usize] = match word_size_selector {
            WordSize::Byte => self.alu_output & 0xFF,
            WordSize::Long => self.alu_output,
        };
    }

    pub fn read_address_register(
        &mut self,
        selector: AddressRegisterSelector,
        word_size: &WordSize,
    ) {
        self.address_registers_mux = match word_size {
            WordSize::Byte => self.address_registers[selector as usize] & 0xFF,
            WordSize::Long => self.address_registers[selector as usize],
        };
    }

    pub fn latch_address_register(
        &mut self,
        register_selector: AddressRegisterSelector,
        word_size: &WordSize,
    ) {
        let decrement = match self.pre_mode_selector {
            PreModeSelector::None => 0,
            PreModeSelector::Decrement => 1,
        };
        let increment = match self.post_mode_selector {
            PostModeSelector::None => 0,
            PostModeSelector::Increment => 1,
        };

        let input = self.alu_output + increment - decrement;
        self.address_registers[register_selector as usize] = match word_size {
            WordSize::Byte => input & 0xFF,
            WordSize::Long => input,
        };
    }

    pub fn read_data_memory(&mut self) {
        self.memory_output = self.data_memory.read(self.data_address / 4);
    }

    pub fn write_data_memory(&mut self, selector: WriteSelector) {
        self.data_memory.write(
            self.data_address / 4,
            match selector {
                WriteSelector::Scalar => self.write_data,
                WriteSelector::Vector => self.vector_output_mux,
            },
        )
    }

    pub fn update_write_data_mux(&mut self, selector: WriteDataSelector) {
        self.write_data_mux = match selector {
            WriteDataSelector::Memory => self.memory_output,
            WriteDataSelector::Alu => [self.alu_output; 4],
        }
    }

    pub fn latch_write_data(&mut self) {
        self.write_data = self.write_data_mux;
    }

    pub fn latch_write_data_part(&mut self, word_size: &WordSize) {
        let low_address = (self.data_address % 4) as usize;
        let mask = match word_size {
            WordSize::Byte => 0xff,
            WordSize::Long => 0xffffffffffffffff,
        };
        self.write_data[low_address] =
            (self.write_data[low_address] & !mask) | (self.write_data_mux[low_address] & mask);
    }

    fn update_data(&mut self, selector: DataSelector) -> u64 {
        match selector {
            DataSelector::DataRegister => self.data_registers_mux,
            DataSelector::AddressRegister => self.address_registers_mux,
            DataSelector::Memory => self.memory_output_mux,
            DataSelector::External => match self.external_selector {
                ExternalSelector::ControlUnit => self.control_unit_output,
                ExternalSelector::IO => self.io.output,
            },
        }
    }

    fn update_offset(&mut self, selector: OffsetSelector) -> u64 {
        match selector {
            OffsetSelector::DataRegister => self.data_registers_mux,
            OffsetSelector::External => match self.external_selector {
                ExternalSelector::ControlUnit => self.control_unit_output,
                ExternalSelector::IO => self.io.output,
            },
        }
    }
    pub fn update_left_alu_input(&mut self, selector: DataSelector) {
        self.left_alu_input = self.update_data(selector);
    }

    pub fn latch_right_data_register(&mut self, selector: DataSelector) {
        self.right_data_register = self.update_data(selector);
    }

    pub fn update_right_offset(&mut self, selector: OffsetSelector) {
        self.right_offset = self.update_offset(selector);
    }

    pub fn update_right_alu_input(&mut self, selector: AluInputSelector) {
        self.right_alu_input = match selector {
            AluInputSelector::Data => self.right_data_register,
            AluInputSelector::Offset => self.right_offset,
        }
    }

    pub fn execute_alu(&mut self, operator: AluOperator) {
        self.alu_output =
            self.alu
                .execute_operator(operator, self.left_alu_input, self.right_alu_input);
    }

    pub fn latch_data_address(&mut self) {
        self.data_address = match self.pre_mode_selector {
            PreModeSelector::None => self.alu_output,
            PreModeSelector::Decrement => self.alu_output - 1,
        };
    }

    pub fn update_memory_output_mux(&mut self, word_size: &WordSize) {
        self.memory_output_mux = match word_size {
            WordSize::Byte => self.memory_output[(self.data_address % 4) as usize] & 0xFF,
            WordSize::Long => self.memory_output[(self.data_address % 4) as usize],
        }
    }

    pub fn update_left_vector_input(&mut self) {
        self.left_vector_input = self.memory_output;
    }

    pub fn latch_right_vector_input_register(&mut self) {
        self.right_vector_input_register = self.memory_output;
    }

    pub fn execute_vector_alu(&mut self, operator: VectorAluOperator) {
        self.vector_alu_output = self.vector_alu.execute_operator(
            operator,
            self.left_vector_input,
            self.right_vector_input_register,
        );
    }

    pub fn update_vector_alu_output_mux(
        &mut self,
        mode_selector: VectorModeSelector,
        branch_selector: BranchSelector,
    ) {
        for i in 0..4 {
            match mode_selector {
                VectorModeSelector::Alu => {
                    self.vector_output_mux[i] = self.vector_alu_output[i];
                }
                VectorModeSelector::Decoder => {
                    self.vector_output_mux[i] = 0xffffffffffffffff
                        * match branch_selector {
                            BranchSelector::Beq => self.vector_alu.block[i].nzcv.zero as u64,
                            BranchSelector::Bne => !self.vector_alu.block[i].nzcv.zero as u64,
                            BranchSelector::Bgt => {
                                (!self.vector_alu.block[i].nzcv.zero
                                    && self.vector_alu.block[i].nzcv.negative
                                        == self.vector_alu.block[i].nzcv.overflow)
                                    as u64
                            }
                            BranchSelector::Bge => {
                                (self.vector_alu.block[i].nzcv.negative
                                    == self.vector_alu.block[i].nzcv.overflow)
                                    as u64
                            }
                            BranchSelector::Blt => {
                                (self.vector_alu.block[i].nzcv.negative
                                    != self.vector_alu.block[i].nzcv.overflow)
                                    as u64
                            }
                            BranchSelector::Ble => {
                                (self.vector_alu.block[i].nzcv.zero
                                    || self.vector_alu.block[i].nzcv.negative
                                        != self.vector_alu.block[i].nzcv.overflow)
                                    as u64
                            }
                            BranchSelector::Bcs => self.vector_alu.block[i].nzcv.carry as u64,
                            BranchSelector::Bcc => !self.vector_alu.block[i].nzcv.carry as u64,
                            BranchSelector::Bvs => self.vector_alu.block[i].nzcv.overflow as u64,
                            BranchSelector::Bvc => !self.vector_alu.block[i].nzcv.overflow as u64,
                        }
                }
            }
        }
    }

    pub fn latch_vector_output_register(&mut self) {
        self.vector_output_register = self.vector_output_mux;
    }

    pub fn set_nzcv_to_alu_output(&mut self) {
        self.alu_output = self.alu.nzcv.to_byte();
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
            data_memory: DataMemory::new(25000),
            left_alu_input: 0,
            right_alu_input: 0,
            alu: Alu::default(),
            alu_output: 0,
            data_registers_mux: 0,
            data_registers: [0; 8],
            address_registers_mux: 0,
            address_registers: [0; 8],
            right_data_register: 0,
            right_offset: 0,
            external_selector: ExternalSelector::ControlUnit,
            pre_mode_selector: PreModeSelector::None,
            post_mode_selector: PostModeSelector::None,
            memory_output: [0; 4],
            data_address: 0,
            memory_output_mux: 0,
            write_data: [0; 4],
            write_data_mux: [0; 4],
            io: IO::new(),
            control_unit_output: 0,
            vector_alu: VectorAlu::new(),
            left_vector_input: [0; 4],
            right_vector_input_register: [0; 4],
            vector_alu_output: [0; 4],
            vector_output_mux: [0; 4],
            vector_output_register: [0; 4],
        }
    }
}
