#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub enum WordSize {
    Byte,
    Long,
}

#[derive(Clone)]
pub struct Operand {
    pub mode: Mode,
    pub main_register: u8,
    pub offset: u8,
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub enum Operator {
    Hlt,
    Mov,
    Cmp,
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
    Ret,
    IntRet,
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
    VAdd,
    VSub,
    VMul,
    VDiv,
    VRem,
    VAnd,
    VOr,
    VXor,
    VEnd,
    In,
    Out,
    EI,
    DI,
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub enum Mode {
    Direct,
    DataRegister,
    AddressRegister,
    Indirect,
    IndirectPostIncrement,
    IndirectPreDecrement,
    IndirectOffset,
    IndirectDirect,
}
