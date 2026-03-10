pub struct Memory<T> {
    data: Vec<T>,
}

impl<T: Copy> Memory<T> {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn read(&self, address: u64) -> T {
        assert!(
            address > 0 && address < self.data.len() as u64,
            "Address {} is out of bounds",
            address
        );
        self.data[address as usize]
    }

    pub fn write(&mut self, address: usize, value: T) {
        assert!(
            address > 0 && address < self.data.len(),
            "Address {} is out of bounds",
            address
        );
        self.data[address] = value;
    }
}
