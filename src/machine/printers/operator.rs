use crate::isa::Operator;
use std::fmt::Display;

impl Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = match self {
            Operator::Hlt => "HLT",
            Operator::Mov => "MOV",
            Operator::Mova => "MOVA",
            Operator::Add => "ADD",
            Operator::Adc => "ADC",
            Operator::Sub => "SUB",
            Operator::Mul => "MUL",
            Operator::Div => "DIV",
            Operator::Rem => "REM",
            Operator::And => "AND",
            Operator::Or => "OR",
            Operator::Xor => "XOR",
            Operator::Not => "NOT",
            Operator::Lsl => "LSL",
            Operator::Lsr => "LSR",
            Operator::Asl => "ASL",
            Operator::Asr => "ASR",
            Operator::Jmp => "JMP",
            Operator::Call => "CALL",
            Operator::Ret => "RET",
            Operator::Beq => "BEQ",
            Operator::Bne => "BNE",
            Operator::Bgt => "BGT",
            Operator::Bge => "BGE",
            Operator::Blt => "BLT",
            Operator::Ble => "BLE",
            Operator::Bcs => "BCS",
            Operator::Bcc => "BCC",
            Operator::Bvs => "BVS",
            Operator::Bvc => "BVC",
            Operator::Cmp => "CMP",
        };
        f.write_str(code)
    }
}
