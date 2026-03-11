pub struct Stack {
    data: Vec<u64>,
}

impl Stack {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn push(&mut self, value: u64) {
        self.data.push(value);
    }

    pub fn pop(&mut self) -> u64 {
        self.data.pop().expect("Stack underflow")
    }
}
