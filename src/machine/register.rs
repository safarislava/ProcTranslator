pub struct Register {
    value: u64,
}

impl Register {
    pub fn new(value: u64) -> Self {
        Self { value }
    }

    pub fn read(&self) -> u64 {
        self.value
    }

    pub fn write(&mut self, value: u64) {
        self.value = value;
    }
}

impl Default for Register {
    fn default() -> Self {
        Self::new(0)
    }
}
