use crate::translator::hir::HirOperand;
use std::fmt;

impl fmt::Display for HirOperand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HirOperand::Value(reg) => write!(f, "{}", reg),
            HirOperand::Constant(val) => write!(f, "\"{}\"", val),
            HirOperand::Void => write!(f, "void"),
        }
    }
}
