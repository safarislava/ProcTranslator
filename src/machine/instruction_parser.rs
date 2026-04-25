use crate::isa::{Mode, Operand, Operator, WordSize};
use std::collections::HashMap;

pub struct InstructionParser {
    operators: HashMap<u8, Operator>,
    modes: HashMap<u8, Mode>,
    word_sizes: HashMap<u8, WordSize>,
}

impl InstructionParser {
    pub fn new() -> Self {
        let operators = HashMap::from([
            (0x00, Operator::Hlt),
            (0x01, Operator::Mov),
            (0x02, Operator::Cmp),
            (0x10, Operator::Add),
            (0x11, Operator::Adc),
            (0x12, Operator::Sub),
            (0x13, Operator::Mul),
            (0x14, Operator::Div),
            (0x15, Operator::Rem),
            (0x16, Operator::And),
            (0x17, Operator::Or),
            (0x18, Operator::Xor),
            (0x19, Operator::Not),
            (0x1A, Operator::Lsl),
            (0x1B, Operator::Lsr),
            (0x1C, Operator::Asl),
            (0x1D, Operator::Asr),
            (0x20, Operator::Jmp),
            (0x21, Operator::Call),
            (0x22, Operator::Ret),
            (0x23, Operator::IntRet),
            (0x30, Operator::Beq),
            (0x31, Operator::Bne),
            (0x32, Operator::Bgt),
            (0x33, Operator::Bge),
            (0x34, Operator::Blt),
            (0x35, Operator::Ble),
            (0x36, Operator::Bcs),
            (0x37, Operator::Bcc),
            (0x38, Operator::Bvs),
            (0x39, Operator::Bvc),
            (0x40, Operator::VAdd),
            (0x42, Operator::VSub),
            (0x43, Operator::VMul),
            (0x44, Operator::VDiv),
            (0x45, Operator::VRem),
            (0x46, Operator::VAnd),
            (0x47, Operator::VOr),
            (0x48, Operator::VXor),
            (0x50, Operator::In),
            (0x51, Operator::Out),
            (0x52, Operator::EI),
            (0x53, Operator::DI),
            (0x60, Operator::VCmpBeq),
            (0x61, Operator::VCmpBne),
            (0x62, Operator::VCmpBgt),
            (0x63, Operator::VCmpBge),
            (0x64, Operator::VCmpBlt),
            (0x65, Operator::VCmpBle),
            (0x66, Operator::VCmpBcs),
            (0x67, Operator::VCmpBcc),
            (0x68, Operator::VCmpBvs),
            (0x69, Operator::VCmpBvc),
        ]);
        let modes = HashMap::from([
            (0x0, Mode::Direct),
            (0x1, Mode::DataRegister),
            (0x2, Mode::AddressRegister),
            (0x3, Mode::Indirect),
            (0x4, Mode::IndirectPostIncrement),
            (0x5, Mode::IndirectPreDecrement),
            (0x6, Mode::IndirectOffset),
            (0x7, Mode::IndirectDirect),
        ]);
        let word_sizes = HashMap::from([(0b0, WordSize::Byte), (0b1, WordSize::Long)]);
        Self {
            operators,
            modes,
            word_sizes,
        }
    }

    pub fn parse_operator(&self, word: u32) -> (Operator, WordSize) {
        let operator_code = ((word >> 25) & 0x7f) as u8;
        let word_size_code = ((word >> 24) & 0x1) as u8;
        (
            self.operators[&operator_code].clone(),
            self.word_sizes[&word_size_code].clone(),
        )
    }

    pub fn parse_operand(&self, word: u8) -> Operand {
        let mode_code = (word >> 5) & 0x7;
        let main_register = (word >> 2) & 0x7;
        let offset = word & 0x3;
        Operand {
            mode: self.modes[&mode_code].clone(),
            main_register,
            offset,
        }
    }
}
