use crate::machine::alu::{Alu, AluOperator};
use crate::machine::data_memory::VectorWord;

pub enum VectorAluOperator {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Xor,
}

pub struct VectorAlu {
    pub block: [Alu; 4],
}
impl VectorAlu {
    pub fn new() -> Self {
        Self {
            block: [
                Alu::default(),
                Alu::default(),
                Alu::default(),
                Alu::default(),
            ],
        }
    }

    pub fn execute_operator(
        &mut self,
        operator: VectorAluOperator,
        left: VectorWord,
        right: VectorWord,
    ) -> VectorWord {
        let operator = match operator {
            VectorAluOperator::Add => AluOperator::Add,
            VectorAluOperator::Sub => AluOperator::Sub,
            VectorAluOperator::Mul => AluOperator::Mul,
            VectorAluOperator::Div => AluOperator::Div,
            VectorAluOperator::Rem => AluOperator::Rem,
            VectorAluOperator::And => AluOperator::And,
            VectorAluOperator::Or => AluOperator::Or,
            VectorAluOperator::Xor => AluOperator::Xor,
        };

        let mut output = [0; 4];
        for i in 0..4 {
            output[i] = self.block[i].execute_operator(operator.clone(), left[i], right[i]);
        }
        output
    }
}
