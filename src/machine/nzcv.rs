pub struct NZCV {
    pub negative: bool,
    pub zero: bool,
    pub carry: bool,
    pub overflow: bool,
}

impl NZCV {
    pub fn new(negative: bool, zero: bool, carry: bool, overflow: bool) -> Self {
        Self {
            negative,
            zero,
            carry,
            overflow,
        }
    }

    pub fn set_negative(&mut self) {
        self.negative = true;
    }

    pub fn clear_negative(&mut self) {
        self.negative = false;
    }

    pub fn set_zero(&mut self) {
        self.zero = true;
    }

    pub fn clear_zero(&mut self) {
        self.zero = false;
    }

    pub fn set_carry(&mut self) {
        self.carry = true;
    }

    pub fn clear_carry(&mut self) {
        self.carry = false;
    }

    pub fn set_overflow(&mut self) {
        self.overflow = true;
    }

    pub fn clear_overflow(&mut self) {
        self.overflow = false;
    }
}

impl Default for NZCV {
    fn default() -> Self {
        Self::new(false, true, false, false)
    }
}
