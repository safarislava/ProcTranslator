use crate::isa::{Mode, Operand, Operator, WordSize};
use crate::translator::hir::{BlockId, ControlFlowGraph, HirBasicBlock};
use std::collections::HashMap;

pub struct AsmTranslator {
    operators: HashMap<Operator, u8>,
    modes: HashMap<Mode, u8>,
    word_sizes: HashMap<WordSize, u8>,
}

impl AsmTranslator {
    pub fn new(
        operators: HashMap<Operator, u8>,
        modes: HashMap<Mode, u8>,
        word_sizes: HashMap<WordSize, u8>,
    ) -> Self {
        Self {
            operators,
            modes,
            word_sizes,
        }
    }

    pub fn translate(&mut self, control_flow_graph: ControlFlowGraph) {
        let mut program_instructions: Vec<u8> = vec![];
        for block in control_flow_graph.blocks {
            let mut block_instructions = self.translate_block(block.clone());
            program_instructions.append(&mut block_instructions);
        }
    }

    pub fn translate_block(&mut self, block: HirBasicBlock) -> Vec<u8> {
        todo!()
    }
}

impl Default for AsmTranslator {
    fn default() -> Self {
        let operators = HashMap::from([
            (Operator::Hlt, 0x00),
            (Operator::Mov, 0x01),
            (Operator::Mova, 0x02),
            (Operator::Add, 0x03),
            (Operator::Adc, 0x11),
            (Operator::Sub, 0x12),
            (Operator::Mul, 0x13),
            (Operator::Div, 0x14),
            (Operator::Rem, 0x15),
            (Operator::And, 0x20),
            (Operator::Or, 0x21),
            (Operator::Xor, 0x22),
            (Operator::Not, 0x23),
            (Operator::Lsl, 0x24),
            (Operator::Lsr, 0x31),
            (Operator::Asl, 0x32),
            (Operator::Asr, 0x33),
            (Operator::Jmp, 0x40),
            (Operator::Call, 0x41),
            (Operator::Ret, 0x42),
            (Operator::Beq, 0x50),
            (Operator::Bne, 0x51),
            (Operator::Bgt, 0x52),
            (Operator::Bge, 0x53),
            (Operator::Blt, 0x54),
            (Operator::Ble, 0x55),
            (Operator::Bcs, 0x56),
            (Operator::Bcc, 0x57),
            (Operator::Bvs, 0x58),
            (Operator::Bvc, 0x59),
            (Operator::Cmp, 0x60),
        ]);
        let modes = HashMap::from([
            (Mode::Direct, 0x0),
            (Mode::DataRegister, 0x1),
            (Mode::AddressRegister, 0x2),
            (Mode::Indirect, 0x3),
            (Mode::IndirectPostIncrement, 0x4),
            (Mode::IndirectPreDecrement, 0x5),
            (Mode::IndirectOffset, 0x6),
        ]);
        let word_sizes = HashMap::from([(WordSize::Byte, 0b0), (WordSize::Long, 0b1)]);
        Self::new(operators, modes, word_sizes)
    }
}
