use crate::translator::ir::Operand;
use std::fmt;

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Value(reg) => write!(f, "{}", reg),
            Operand::Constant(val) => write!(f, "\"{}\"", val),
            Operand::Void => write!(f, "void"),
        }
    }
}
