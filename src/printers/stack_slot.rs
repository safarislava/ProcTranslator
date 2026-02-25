use std::fmt;
use crate::ir::StackSlot;

impl fmt::Display for StackSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "s{}", self.0)
    }
}
