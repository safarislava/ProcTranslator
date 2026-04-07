pub struct IntInputDevice {
    pub data: i64,
    vector: u8,
    pub interrupt: bool,
}

pub struct CharInputDevice {
    pub data: char,
    vector: u8,
    pub interrupt: bool,
}

pub struct IO {
    pub input: i64,
    pub output: i64,

    pub int_input_device: IntInputDevice,
    pub char_input_device: CharInputDevice,
    pub int_output_log: Vec<i64>,
    pub char_output_log: Vec<char>,

    pub interrupt_vector: u8,
}

impl IO {
    pub fn new() -> Self {
        Self {
            input: 0,
            output: 0,
            int_input_device: IntInputDevice {
                data: 0,
                vector: 0,
                interrupt: false,
            },
            char_input_device: CharInputDevice {
                data: '\0',
                vector: 1,
                interrupt: false,
            },
            int_output_log: vec![],
            char_output_log: vec![],
            interrupt_vector: 0,
        }
    }

    pub fn read(&mut self, port: u8) {
        match port {
            0 => {
                self.output = self.int_input_device.data;
                self.int_input_device.interrupt = false;
            }
            1 => {
                self.output = self.int_input_device.vector as i64;
            }
            2 => {
                self.output = self.char_input_device.data as i64;
                self.char_input_device.interrupt = false;
            }
            3 => {
                self.output = self.char_input_device.vector as i64;
            }
            _ => {}
        }
    }

    pub fn write(&mut self, port: u8) {
        match port {
            1 => {
                self.int_input_device.vector = self.input as u8;
            }
            3 => {
                self.char_input_device.vector = self.input as u8;
            }
            4 => {
                self.int_output_log.push(self.input);
            }
            5 => {
                self.char_output_log.push((self.input as u8) as char);
            }
            _ => {}
        }
    }

    pub fn write_internal(&mut self, port: u8, value: i64) {
        match port {
            0 => {
                self.int_input_device.data = value;
            }
            1 => {
                self.int_input_device.vector = value as u8;
            }
            2 => {
                self.char_input_device.data = (value as u8) as char;
            }
            3 => {
                self.char_input_device.vector = value as u8;
            }
            4 => {
                self.int_output_log.push(self.input);
            }
            5 => {
                self.char_output_log.push((self.input as u8) as char);
            }
            _ => {}
        }
    }

    pub fn check_interrupt(&mut self) -> bool {
        self.int_input_device.interrupt || self.char_input_device.interrupt
    }

    pub fn update_interrupt_vector(&mut self) {
        self.interrupt_vector = if self.int_input_device.interrupt {
            self.int_input_device.vector
        } else if self.char_input_device.interrupt {
            self.char_input_device.vector
        } else {
            0
        };
    }
}
