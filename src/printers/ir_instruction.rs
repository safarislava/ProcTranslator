use std::fmt;
use crate::ir::IrInstruction;

impl fmt::Display for IrInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrInstruction::LoadConst { dest, value } => {
                write!(f, "{} = const {}", dest, value)
            }
            IrInstruction::BinaryOp { dest, left, op, right } => {
                write!(f, "{} = {} {:?} {}", dest, left, op, right)
            }
            IrInstruction::Call { dest, block, arguments } => {
                let args = arguments.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ");
                write!(f, "{} = call B{}({})", dest, block, args)
            }
            IrInstruction::LoadParam { dest, index } => {
                write!(f, "{} = param[{}]", dest, index)
            }
            IrInstruction::StackAlloc { slot } => {
                write!(f, "alloc {}", slot)
            }
            IrInstruction::StackStore { slot, value } => {
                write!(f, "{} = {}", slot, value)
            }
            IrInstruction::StackLoad { dest, slot } => {
                write!(f, "{} = load {}", dest, slot)
            }
            IrInstruction::GetField { dest, object, offset } => {
                write!(f, "{} = getfield {}[{}]", dest, object, offset)
            }
            IrInstruction::PutField { object, offset, value } => {
                write!(f, "{}[{}] = {}", object, offset, value)
            }
            IrInstruction::AllocObject { dest, class_name } => {
                write!(f, "{} = new {}", dest, class_name)
            }
        }
    }
}
