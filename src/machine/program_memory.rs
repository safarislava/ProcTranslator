use crate::translator::common::Address;

pub struct ProgramMemory {
    pub data: Vec<u32>,
}

impl ProgramMemory {
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0; size],
        }
    }

    pub fn read(&self, address: Address) -> u32 {
        assert!(
            address < self.data.len() as u64,
            "Address {} is out of bounds",
            address
        );
        self.data[address as usize]
    }

    pub fn write(&mut self, address: Address, value: u32) {
        assert!(
            address < self.data.len() as u64,
            "Address {} is out of bounds",
            address
        );
        self.data[address as usize] = value;
    }
}
