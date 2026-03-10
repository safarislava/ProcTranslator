use std::collections::HashMap;

#[derive(Clone)]
pub struct Operand {
    pub mode: Mode,
    pub main_register: u8,
    pub offset_register: u8,
}

#[derive(Clone)]
pub enum Operator {
    Mov,
    Mova,
    Add,
    Adc,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Xor,
    Not,
    Lsl,
    Lsr,
    Asl,
    Asr,
    Jmp,
    Call,
    Func,
    Ret,
    Link,
    Unlk,
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
    Cmp,
}

#[derive(Clone, Eq, PartialEq)]
pub enum Mode {
    Direct,
    DataRegister,
    AddressRegister,
    Indirect,
    IndirectPostIncrement,
    IndirectPreDecrement,
    IndirectOffset,
}

pub struct InstructionParser {
    operators: HashMap<u8, Operator>,
    modes: HashMap<u8, Mode>,
}

impl InstructionParser {
    pub fn new() -> Self {
        let operators = HashMap::from([
            (0x01, Operator::Mov),
            (0x02, Operator::Mova),
            (0x10, Operator::Add),
            (0x11, Operator::Adc),
            (0x12, Operator::Sub),
            (0x13, Operator::Mul),
            (0x14, Operator::Div),
            (0x15, Operator::Rem),
            (0x20, Operator::And),
            (0x21, Operator::Or),
            (0x22, Operator::Xor),
            (0x23, Operator::Not),
            (0x30, Operator::Lsl),
            (0x31, Operator::Lsr),
            (0x32, Operator::Asl),
            (0x33, Operator::Asr),
            (0x40, Operator::Jmp),
            (0x41, Operator::Call),
            (0x42, Operator::Func),
            (0x43, Operator::Ret),
            (0x44, Operator::Link),
            (0x45, Operator::Unlk),
            (0x50, Operator::Beq),
            (0x51, Operator::Bne),
            (0x52, Operator::Bgt),
            (0x53, Operator::Bge),
            (0x54, Operator::Blt),
            (0x55, Operator::Ble),
            (0x56, Operator::Bcs),
            (0x57, Operator::Bcc),
            (0x58, Operator::Bvs),
            (0x59, Operator::Bvc),
            (0x60, Operator::Cmp),
        ]);
        let modes = HashMap::from([
            (0x0, Mode::Direct),
            (0x1, Mode::DataRegister),
            (0x2, Mode::AddressRegister),
            (0x3, Mode::Indirect),
            (0x4, Mode::IndirectPostIncrement),
            (0x5, Mode::IndirectPreDecrement),
            (0x6, Mode::IndirectOffset),
        ]);
        Self { operators, modes }
    }

    pub fn parse_operator(&self, word: u8) -> Operator {
        self.operators[&word].clone()
    }

    pub fn parse_operand(&self, word: u8) -> Operand {
        let mode_code = (word & 0b11100000) >> 5;
        let main_register = (word & 0b11100) >> 3;
        let offset_register = word & 0b11;
        Operand {
            mode: self.modes[&mode_code].clone(),
            main_register,
            offset_register,
        }
    }
}
