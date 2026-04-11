use crate::translator::common::Address;

pub type VectorWord = [u64; 4];

pub struct DataMemory {
    pub data: Vec<VectorWord>,
}

impl DataMemory {
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![[0; 4]; size],
        }
    }

    pub fn read(&self, address: Address) -> VectorWord {
        assert!(
            address < self.data.len() as u64,
            "Address {} is out of bounds",
            address
        );
        self.data[address as usize]
    }

    pub fn write(&mut self, address: Address, value: VectorWord) {
        assert!(
            address < self.data.len() as u64,
            "Address {} is out of bounds",
            address
        );
        self.data[address as usize] = value;
    }
}
