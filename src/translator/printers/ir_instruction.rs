use crate::translator::hir::HirInstruction;
use std::fmt;

impl fmt::Display for HirInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
            HirInstruction::AllocateStack { slot } => {
                write!(f, "alloc {}", slot)
            }
            HirInstruction::StoreStack { slot, value } => {
                write!(f, "{} = {}", slot, value)
            }
            HirInstruction::LoadStack { destination, slot } => {
                write!(f, "{} = load {}", destination, slot)
            }
            HirInstruction::LoadField {
                destination,
                object,
                offset,
            } => {
                write!(f, "{} = load field {}[{}]", destination, object, offset)
            }
            HirInstruction::StoreField {
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
            HirInstruction::LoadGlobal { destination, id } => {
                write!(f, "{} = load global {}", destination, id)
            }
            HirInstruction::StoreGlobal { id, value } => {
                write!(f, "{} = store global {}", id, value)
            }
        }
    }
}
