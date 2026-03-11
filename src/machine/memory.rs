use crate::machine::isa::WordSize;

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
}

impl Memory<i64> {
    pub fn write(&mut self, address: usize, value: i64, selector: &WordSize) {
        assert!(
            address > 0 && address < self.data.len(),
            "Address {} is out of bounds",
            address
        );
        match selector {
            WordSize::Byte => {
                self.data[address] = (self.data[address] & 0xFFFFFF00) | (value & 0xFF)
            }
            WordSize::Long => self.data[address] = value,
        }
    }
}

impl Memory<u8> {
    pub fn write(&mut self, address: usize, value: u8) {
        assert!(
            address > 0 && address < self.data.len(),
            "Address {} is out of bounds",
            address
        );
        self.data[address] = value;
    }
}
