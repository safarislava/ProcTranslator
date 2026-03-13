use crate::isa::WordSize;
use num::Integer;

pub struct Memory<T> {
    data: Vec<T>,
}

impl<T: Copy + Integer> Memory<T> {
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![T::zero(); size],
        }
    }

    pub fn read(&self, address: u64) -> T {
        assert!(
            address < self.data.len() as u64,
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
            address < self.data.len(),
            "Address {} is out of bounds",
            address
        );
        self.data[address] = value;
    }
}
