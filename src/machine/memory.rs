pub struct Memory {
    data: Vec<u64>,
}

impl Memory {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn read(&self, address: usize) -> u64 {
        assert!(
            address > 0 && address < self.data.len(),
            "Address {} is out of bounds",
            address
        );
        self.data[address]
    }

    pub fn write(&mut self, address: usize, value: u64) {
        assert!(
            address > 0 && address < self.data.len(),
            "Address {} is out of bounds",
            address
        );
        self.data[address] = value;
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}
