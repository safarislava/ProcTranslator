use crate::translator::hir::HirOperand;
use std::fmt;

impl fmt::Display for HirOperand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HirOperand::Value(register) => write!(f, "{}", register),
            HirOperand::Link(register) => write!(f, "link {}", register),
            HirOperand::Constant(value, _) => write!(f, "\"{}\"", value),
            HirOperand::Variable(slot) => write!(f, "local {}", slot),
            HirOperand::Parameter(offset) => write!(f, "parameter {}", offset),
            HirOperand::GlobalVariable(id) => write!(f, "global {}", id),
            HirOperand::Void => write!(f, "void"),
        }
    }
}
