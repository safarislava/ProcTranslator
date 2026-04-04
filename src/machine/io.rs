use std::collections::HashMap;

pub struct IO {
    pub input: i64,
    pub output: i64,
    ports: HashMap<u8, i64>,
}

impl IO {
    pub fn new() -> Self {
        Self {
            input: 0,
            output: 0,
            ports: HashMap::from([(0, 0), (1, 0)]),
        }
    }

    pub fn read(&mut self, port: u8) {
        if let Some(device_value) = self.ports.get_mut(&port) {
            self.output = *device_value;
        } else {
            panic!("Port {} doesn't exist", port)
        }
    }

    pub fn write(&mut self, port: u8) {
        if let Some(device_value) = self.ports.get_mut(&port) {
            *device_value = self.input;
        } else {
            panic!("Port {} doesn't exist", port)
        }
    }

    pub fn read_internal(&mut self, port: u8) -> i64 {
        if let Some(device_value) = self.ports.get_mut(&port) {
            *device_value
        } else {
            panic!("Port {} doesn't exist", port)
        }
    }

    pub fn write_internal(&mut self, port: u8, value: i64) {
        if let Some(device_value) = self.ports.get_mut(&port) {
            *device_value = value;
        } else {
            panic!("Port {} doesn't exist", port)
        }
    }
}
