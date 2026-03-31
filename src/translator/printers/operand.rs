use crate::translator::hir::HirOperand;
use std::fmt;

impl fmt::Display for HirOperand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HirOperand::Value(register) => write!(f, "{}", register),
            HirOperand::Link(register) => write!(f, "link {}", register),
            HirOperand::Constant(value) => write!(f, "\"{}\"", value),
            HirOperand::Void => write!(f, "void"),
        }
    }
}
