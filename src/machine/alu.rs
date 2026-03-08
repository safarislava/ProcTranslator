use crate::machine::nzcv::NZCV;

pub enum AluOp {
    ADD(u64, u64),
    SUB(u64, u64),
    MUL(u64, u64),
    DIV(u64, u64),
    REM(u64, u64),
    AND(u64, u64),
    OR(u64, u64),
    XOR(u64, u64),
    NOT(u64),
    LSL(u64, u64),
    LSR(u64, u64),
    ASL(u64, u64),
    ASR(u64, u64),
}

pub struct ALU {
    pub nzcv: NZCV,
}

impl ALU {
    pub fn new(nzcv: NZCV) -> Self {
        Self { nzcv }
    }

    pub fn execute_op(&mut self, op: AluOp) -> u64 {
        match op {
            AluOp::ADD(a, b) => {
                let (c, carry) = a.overflowing_add(b);
                let overflow = (!(a ^ b) & (a ^ c)) >> 63 == 1;
                self.set_flags(c, Some(carry), Some(overflow));
                c
            }
            AluOp::SUB(a, b) => {
                let (c, carry) = a.overflowing_sub(b);
                let overflow = ((a ^ b) & (a ^ c)) >> 63 == 1;
                self.set_flags(c, Some(carry), Some(overflow));
                c
            }
            AluOp::MUL(a, b) => {
                let c = a.wrapping_mul(b);
                let carry = (a as u128 * b as u128) > (u64::MAX as u128);
                let full_signed = (a as i64 as i128) * (b as i64 as i128);
                let overflow = full_signed != (c as i64 as i128);
                self.set_flags(c, Some(carry), Some(overflow));
                c
            }
            AluOp::DIV(a, b) => {
                if b == 0 {
                    self.set_flags(0, Some(true), Some(false));
                    return 0;
                }
                let (a_i, b_i) = (a as i64, b as i64);
                if a_i == i64::MIN && b_i == -1 {
                    self.set_flags(a, Some(false), Some(true));
                    a
                } else {
                    let c = (a_i / b_i) as u64;
                    self.set_flags(c, Some(false), Some(false));
                    c
                }
            }
            AluOp::REM(a, b) => {
                if b == 0 {
                    self.set_flags(0, Some(true), Some(false));
                    return 0;
                }
                let (a_i, b_i) = (a as i64, b as i64);
                if a_i == i64::MIN && b_i == -1 {
                    self.set_flags(0, Some(false), Some(true));
                    0
                } else {
                    let c = (a_i % b_i) as u64;
                    self.set_flags(c, Some(false), Some(false));
                    c
                }
            }
            AluOp::AND(a, b) => {
                let c = a & b;
                self.set_flags(c, None, None);
                c
            }
            AluOp::OR(a, b) => {
                let c = a | b;
                self.set_flags(c, None, None);
                c
            }
            AluOp::XOR(a, b) => {
                let c = a ^ b;
                self.set_flags(c, None, None);
                c
            }
            AluOp::NOT(a) => {
                let c = !a;
                self.set_flags(c, None, None);
                c
            }
            AluOp::LSL(a, b) => {
                let shift_count = (b & 63) as u32;
                if shift_count == 0 {
                    self.set_flags(a, None, None);
                    return a;
                }
                let carry = (a >> (64 - shift_count)) & 1 == 1;
                let c = a << shift_count;
                self.set_flags(c, Some(carry), None);
                c
            }
            AluOp::LSR(a, b) => {
                let shift_count = (b & 63) as u32;
                if shift_count == 0 {
                    self.set_flags(a, None, None);
                    return a;
                }
                let carry = (a >> (shift_count - 1)) & 1 == 1;
                let c = a >> shift_count;
                self.set_flags(c, Some(carry), None);
                c
            }
            AluOp::ASL(a, b) => {
                let shift_count = (b & 63) as u32;
                if shift_count == 0 {
                    self.set_flags(a, None, None);
                    return a;
                }
                let c = a << shift_count;
                self.set_flags(c, None, None);
                c
            }
            AluOp::ASR(a, b) => {
                let shift_count = (b & 63) as u32;
                if shift_count == 0 {
                    self.set_flags(a, None, None);
                    return a;
                }
                let c = ((a as i64) >> shift_count) as u64;
                self.set_flags(c, None, None);
                c
            }
        }
    }

    fn set_flags(&mut self, c: u64, carry: Option<bool>, overflow: Option<bool>) {
        self.nzcv.negative = c >> 63 == 1;
        self.nzcv.zero = c == 0;
        if let Some(carry) = carry {
            self.nzcv.carry = carry;
        }
        if let Some(overflow) = overflow {
            self.nzcv.overflow = overflow;
        }
    }
}

impl Default for ALU {
    fn default() -> Self {
        Self::new(NZCV::default())
    }
}
