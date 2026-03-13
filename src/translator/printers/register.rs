use crate::translator::hir::HirRegister;
use std::fmt;

impl fmt::Display for HirRegister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}", self.0)
    }
}
