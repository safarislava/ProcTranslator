use crate::translator::common::Address;

pub struct Memory {
    data: Vec<u8>,
}

impl Memory {
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0; size],
        }
    }

    pub fn read_u32(&self, address: Address) -> u32 {
        assert!(
            address < self.data.len() as u64,
            "Address {} is out of bounds",
            address
        );
        let mut result = 0;
        for i in 0..4 {
            result |= (self.data[address as usize + i] as u32) << ((3 - i) * 8);
        }
        result
    }

    pub fn read_u64(&self, address: Address) -> u64 {
        assert!(
            address < self.data.len() as u64,
            "Address {} is out of bounds",
            address
        );
        let mut result = 0;
        for i in 0..8 {
            result |= (self.data[address as usize + i] as u64) << ((7 - i) * 8);
        }
        result
    }

    pub fn write_u8(&mut self, address: Address, value: u8) {
        assert!(
            address < self.data.len() as u64,
            "Address {} is out of bounds",
            address
        );
        self.data[address as usize] = value;
    }

    #[allow(dead_code)]
    pub fn write_u32(&mut self, address: Address, value: u32) {
        for i in 0..4 {
            self.data[address as usize + i] = ((value >> ((3 - i) * 8)) & 0xff) as u8;
        }
    }

    pub fn write_u64(&mut self, address: Address, value: u64) {
        for i in 0..8 {
            self.data[address as usize + i] = ((value >> ((7 - i) * 8)) & 0xff) as u8;
        }
    }
}
