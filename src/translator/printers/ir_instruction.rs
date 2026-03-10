use crate::translator::ir::IrInstruction;
use std::fmt;

impl fmt::Display for IrInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrInstruction::LoadConst { destination, value } => {
                write!(f, "{} = const {}", destination, value)
            }
            IrInstruction::BinaryOperator {
                destination,
                left,
                operator,
                right,
            } => {
                write!(f, "{} = {} {:?} {}", destination, left, operator, right)
            }
            IrInstruction::Call {
                destination,
                block,
                arguments,
            } => {
                let args = arguments
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{} = call B{}({})", destination, block, args)
            }
            IrInstruction::LoadParameter { destination, index } => {
                write!(f, "{} = param[{}]", destination, index)
            }
            IrInstruction::StackAllocate { slot } => {
                write!(f, "alloc {}", slot)
            }
            IrInstruction::StackStore { slot, value } => {
                write!(f, "{} = {}", slot, value)
            }
            IrInstruction::StackLoad { destination, slot } => {
                write!(f, "{} = load {}", destination, slot)
            }
            IrInstruction::GetField {
                destination,
                object,
                offset,
            } => {
                write!(f, "{} = getfield {}[{}]", destination, object, offset)
            }
            IrInstruction::PutField {
                object,
                offset,
                value,
            } => {
                write!(f, "{}[{}] = {}", object, offset, value)
            }
            IrInstruction::AllocateObject {
                destination,
                class_name,
            } => {
                write!(f, "{} = new {}", destination, class_name)
            }
        }
    }
}
