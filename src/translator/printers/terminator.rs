use crate::translator::hir::HirTerminator;
use std::fmt;

impl fmt::Display for HirTerminator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HirTerminator::Jump(target) => write!(f, "jump B{}", target),
            HirTerminator::Branch {
                condition,
                true_block,
                false_block,
            } => {
                write!(
                    f,
                    "branch {} ? B{} : B{}",
                    condition, true_block, false_block
                )
            }
            HirTerminator::Return(val) => match val {
                Some(v) => write!(f, "return {}", v),
                None => write!(f, "return"),
            },
            HirTerminator::IntReturn => {
                write!(f, "int_return")
            }
        }
    }
}
