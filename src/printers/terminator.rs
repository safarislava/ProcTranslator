use std::fmt;
use crate::ir::Terminator;

impl fmt::Display for Terminator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Terminator::Jump(target) => write!(f, "jump B{}", target),
            Terminator::Branch { condition, true_block, false_block } => {
                write!(f, "branch {} ? B{} : B{}", condition, true_block, false_block)
            }
            Terminator::Return(val) => {
                match val {
                    Some(v) => write!(f, "return {}", v),
                    None => write!(f, "return"),
                }
            }
        }
    }
}