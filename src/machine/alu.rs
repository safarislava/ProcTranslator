use crate::machine::nzcv::Nzcv;

#[derive(Clone)]
pub enum AluOperator {
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
    Trl,
    Trr,
}

pub struct Alu {
    pub nzcv: Nzcv,
}

impl Alu {
    pub fn new(nzcv: Nzcv) -> Self {
        Self { nzcv }
    }

    pub fn execute_operator(&mut self, operator: AluOperator, a: u64, b: u64) -> u64 {
        let (result, carry, overflow) = match operator {
            AluOperator::Add => {
                let (result, carry) = a.overflowing_add(b);
                let overflow = (!(a ^ b) & (a ^ result)) >> 63 == 1;
                (result, Some(carry), Some(overflow))
            }
            AluOperator::Adc => {
                let old_carry = self.nzcv.carry as u64;
                let full_result = (a as u128) + (b as u128) + (old_carry as u128);
                let result = full_result as u64;
                let carry = full_result > u64::MAX as u128;
                let overflow = (!(a ^ b) & (a ^ result)) >> 63 == 1;
                (result, Some(carry), Some(overflow))
            }
            AluOperator::Sub => {
                let (result, carry) = a.overflowing_sub(b);
                let overflow = ((a ^ b) & (a ^ result)) >> 63 == 1;
                (result, Some(carry), Some(overflow))
            }
            AluOperator::Mul => {
                let result = a.wrapping_mul(b);
                let carry = (a as u128 * b as u128) > (u64::MAX as u128);
                let full_result = (a as i64 as i128) * (b as i64 as i128);
                let overflow = full_result != (result as i64 as i128);
                (result, Some(carry), Some(overflow))
            }
            AluOperator::Div => {
                if b == 0 {
                    (0u64, Some(true), Some(false))
                } else {
                    let (a_i, b_i) = (a as i64, b as i64);
                    if a_i == i64::MIN && b_i == -1 {
                        (a, Some(false), Some(true))
                    } else {
                        let result = (a_i / b_i) as u64;
                        (result, Some(false), Some(false))
                    }
                }
            }
            AluOperator::Rem => {
                if b == 0 {
                    (0u64, Some(true), Some(false))
                } else {
                    let (a_i, b_i) = (a as i64, b as i64);
                    if a_i == i64::MIN && b_i == -1 {
                        (0, Some(false), Some(true))
                    } else {
                        let result = (a_i % b_i) as u64;
                        (result, Some(false), Some(false))
                    }
                }
            }
            AluOperator::And => (a & b, None, None),
            AluOperator::Or => (a | b, None, None),
            AluOperator::Xor => (a ^ b, None, None),
            AluOperator::Not => (!a, None, None),
            AluOperator::Lsl => {
                let shift_count = (a & 63) as u32;
                if shift_count == 0 {
                    (b, None, None)
                } else {
                    let carry = (b >> (64 - shift_count)) & 1 == 1;
                    let result = b << shift_count;
                    (result, Some(carry), None)
                }
            }
            AluOperator::Lsr => {
                let shift_count = (a & 63) as u32;
                if shift_count == 0 {
                    (b, None, None)
                } else {
                    let carry = (b >> (shift_count - 1)) & 1 == 1;
                    let result = b >> shift_count;
                    (result, Some(carry), None)
                }
            }
            AluOperator::Asl => {
                let shift_count = (a & 63) as u32;
                (b << shift_count, None, None)
            }
            AluOperator::Asr => {
                let shift_count = (a & 63) as u32;
                let result = ((b as i64) >> shift_count) as u64;
                (result, None, None)
            }
            AluOperator::Trl => (a, None, None),
            AluOperator::Trr => (b, None, None),
        };
        self.set_flags(result, carry, overflow);
        result
    }

    fn set_flags(&mut self, result: u64, carry: Option<bool>, overflow: Option<bool>) {
        self.nzcv.negative = result >> 63 == 1;
        self.nzcv.zero = result == 0;
        if let Some(carry) = carry {
            self.nzcv.carry = carry;
        }
        if let Some(overflow) = overflow {
            self.nzcv.overflow = overflow;
        }
    }
}

impl Default for Alu {
    fn default() -> Self {
        Self::new(Nzcv::default())
    }
}
