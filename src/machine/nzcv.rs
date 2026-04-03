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
}

impl Default for Nzcv {
    fn default() -> Self {
        Self::new(false, true, false, false)
    }
}
