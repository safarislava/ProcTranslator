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
                ..
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
                    .map(|(a, _)| a.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{} = call B{}({})", destination, block, args)
            }
            HirInstruction::CallPrologue => {
                write!(f, "call prologue")
            }
            HirInstruction::InterruptPrologue => {
                write!(f, "interrupt prologue")
            }
            HirInstruction::LoadParameter {
                destination,
                offset,
                ..
            } => {
                write!(f, "{} = param[{}]", destination, offset)
            }
            HirInstruction::AllocateStack { slot, .. } => {
                write!(f, "alloc {}", slot)
            }
            HirInstruction::StoreStack { slot, value, .. } => {
                write!(f, "{} = {}", slot, value)
            }
            HirInstruction::LoadStack {
                destination, slot, ..
            } => {
                write!(f, "{} = load {}", destination, slot)
            }
            HirInstruction::LoadField {
                destination,
                object,
                offset,
                ..
            } => {
                write!(f, "{} = load field {}[{}]", destination, object, offset)
            }
            HirInstruction::StoreField {
                object,
                offset,
                value,
                ..
            } => {
                write!(f, "{}[{}] = {}", object, offset, value)
            }
            HirInstruction::AllocateObject { destination, size } => {
                write!(f, "{} = new object {}", destination, size)
            }
            HirInstruction::LoadGlobal {
                destination, id, ..
            } => {
                write!(f, "{} = load global {}", destination, id)
            }
            HirInstruction::StoreGlobal { id, value, .. } => {
                write!(f, "{} = store global {}", id, value)
            }
            HirInstruction::Input {
                destination, port, ..
            } => {
                write!(f, "{} = input {}", destination, port)
            }
            HirInstruction::Output { port, value, .. } => {
                write!(f, "output {} = {}", port, value)
            }
            HirInstruction::LoadIndex {
                destination,
                array,
                index,
                ..
            } => {
                write!(f, "{} = {}[{}]", destination, array, index)
            }
            HirInstruction::LoadSlice {
                destination,
                array,
                start,
                ..
            } => {
                write!(f, "{} = {}[{}:]", destination, array, start)
            }
            HirInstruction::StoreIndex {
                array,
                index,
                value,
                ..
            } => {
                write!(f, "{}[{}] = {}", array, index, value)
            }
            HirInstruction::AllocateArray {
                destination, size, ..
            } => {
                write!(f, "{} = new array[{}]", destination, size)
            }
            HirInstruction::StoreSlice {
                target,
                value,
                start,
                ..
            } => {
                write!(f, "{}[{}:] = {}", target, start, value)
            }
            HirInstruction::Not {
                destination,
                operand,
                ..
            } => {
                write!(f, "{} = ~{}", destination, operand)
            }
            HirInstruction::CopyConstantArray { destination, id, .. } => {
                write!(f, "{} = copy array [{}]", destination, id)
            }
        }
    }
}
