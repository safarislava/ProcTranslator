#[derive(Clone)]
pub struct Nzcv {
    pub negative: bool,
    pub zero: bool,
    pub carry: bool,
    pub overflow: bool,
}

impl Nzcv {
    pub fn new(negative: bool, zero: bool, carry: bool, overflow: bool) -> Self {
        Self {
            negative,
            zero,
            carry,
            overflow,
        }
    }

    pub fn to_byte(&self) -> u64 {
        ((self.negative as u64) << 3)
            | ((self.zero as u64) << 2)
            | ((self.carry as u64) << 1)
            | (self.overflow as u64)
    }

    pub fn restore(&mut self, nzcv: u8) {
        self.negative = ((nzcv >> 3) & 1) == 1;
        self.zero = ((nzcv >> 2) & 1) == 1;
        self.carry = ((nzcv >> 1) & 1) == 1;
        self.overflow = (nzcv & 1) == 1;
    }
}

impl Default for Nzcv {
    fn default() -> Self {
        Self::new(false, true, false, false)
    }
}
