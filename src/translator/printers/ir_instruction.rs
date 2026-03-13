use crate::translator::hir::HirInstruction;
use std::fmt;

impl fmt::Display for HirInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HirInstruction::LoadConst { destination, value } => {
                write!(f, "{} = const {}", destination, value)
            }
            HirInstruction::BinaryOperator {
                destination,
                left,
                operator,
                right,
            } => {
                write!(f, "{} = {} {:?} {}", destination, left, operator, right)
            }
            HirInstruction::Call {
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
            HirInstruction::CallPrologue => {
                write!(f, "call prologue")
            }
            HirInstruction::LoadParameter { destination, index } => {
                write!(f, "{} = param[{}]", destination, index)
            }
            HirInstruction::StackAllocate { slot } => {
                write!(f, "alloc {}", slot)
            }
            HirInstruction::StackStore { slot, value } => {
                write!(f, "{} = {}", slot, value)
            }
            HirInstruction::StackLoad { destination, slot } => {
                write!(f, "{} = load {}", destination, slot)
            }
            HirInstruction::GetField {
                destination,
                object,
                offset,
            } => {
                write!(f, "{} = getfield {}[{}]", destination, object, offset)
            }
            HirInstruction::PutField {
                object,
                offset,
                value,
            } => {
                write!(f, "{}[{}] = {}", object, offset, value)
            }
            HirInstruction::AllocateObject {
                destination,
                class_name,
            } => {
                write!(f, "{} = new {}", destination, class_name)
            }
        }
    }
}
