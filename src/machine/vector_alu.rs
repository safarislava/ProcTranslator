pub enum VectorAluOperator {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

pub struct VectorAlu {}
impl VectorAlu {
    pub fn execute_operator(&mut self, operator: VectorAluOperator, input: [i64; 8]) -> [i64; 4] {
        let mut output = [0; 4];
        for i in 0..4 {
            match operator {
                VectorAluOperator::Add => output[i] = input[i] + input[i + 4],
                VectorAluOperator::Sub => output[i] = input[i] - input[i + 4],
                VectorAluOperator::Mul => output[i] = input[i] * input[i + 4],
                VectorAluOperator::Div => output[i] = input[i] / input[i + 4],
                VectorAluOperator::Rem => output[i] = input[i] % input[i + 4],
            }
        }
        output
    }
}
