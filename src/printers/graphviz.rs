use std::fmt;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use crate::ir::{IrInstruction, Operand, Register, StackSlot, Terminator, CFG};


impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}", self.0)
    }
}

impl fmt::Display for StackSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "s{}", self.0)
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Value(reg) => write!(f, "{}", reg),
            Operand::Constant(val) => write!(f, "\"{}\"", val),
            Operand::Void => write!(f, "void"),
        }
    }
}

impl fmt::Display for IrInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrInstruction::LoadConst { dest, value } => {
                write!(f, "{} = const {}", dest, value)
            }
            IrInstruction::BinaryOp { dest, left, op, right } => {
                write!(f, "{} = {} {:?} {}", dest, left, op, right)
            }
            IrInstruction::Call { dest, block, arguments } => {
                let args = arguments.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ");
                write!(f, "{} = call B{}({})", dest, block, args)
            }
            IrInstruction::LoadParam { dest, index } => {
                write!(f, "{} = param[{}]", dest, index)
            }
            IrInstruction::StackAlloc { slot } => {
                write!(f, "alloc {}", slot)
            }
            IrInstruction::StackStore { slot, value } => {
                write!(f, "{} = {}", slot, value)
            }
            IrInstruction::StackLoad { dest, slot } => {
                write!(f, "{} = load {}", dest, slot)
            }
            IrInstruction::GetField { dest, object, offset } => {
                write!(f, "{} = getfield {}[{}]", dest, object, offset)
            }
            IrInstruction::PutField { object, offset, value } => {
                write!(f, "{}[{}] = {}", object, offset, value)
            }
            IrInstruction::AllocObject { dest, class_name } => {
                write!(f, "{} = new {}", dest, class_name)
            }
        }
    }
}

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

impl CFG {
    pub fn to_dot(&self) -> String {
        let mut dot = String::new();
        dot.push_str("digraph CFG {\n");
        dot.push_str("  rankdir=TB;\n");
        dot.push_str("  node [shape=box, fontname=\"Courier\"];\n");
        dot.push_str("  edge [fontname=\"Courier\"];\n\n");

        for block in &self.blocks {
            dot.push_str(&format!("  B{} [label=\"{{B{}|", block.id, block.id));

            for instr in &block.instructions {
                let instr_str = escape_dot(&instr.to_string());
                dot.push_str(&format!("{}\\l", instr_str));
            }

            if let Some(term) = &block.terminator {
                let term_str = escape_dot(&term.to_string());
                dot.push_str(&format!("{}\\l", term_str));
            }

            dot.push_str("}\"];\n");
        }
        
        for block in &self.blocks {
            if let Some(term) = &block.terminator {
                match term {
                    Terminator::Jump(target) => {
                        dot.push_str(&format!("  B{} -> B{};\n", block.id, target));
                    }
                    Terminator::Branch { true_block, false_block, .. } => {
                        dot.push_str(&format!("  B{} -> B{} [label=\"true\"];\n", block.id, true_block));
                        dot.push_str(&format!("  B{} -> B{} [label=\"false\"];\n", block.id, false_block));
                    }
                    Terminator::Return(_) => {}
                }
            }
        }

        if self.blocks.len() > 0 {
            dot.push_str(&format!("  B{} [style=filled, fillcolor=lightgreen];\n", self.entry_block));
        }

        dot.push_str("}\n");
        dot
    }

    pub fn dump_to_file(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let dot_content = self.to_dot();
        let mut file = File::create(path)?;
        file.write_all(dot_content.as_bytes())?;
        Ok(())
    }
}

fn escape_dot(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\l")
        .replace('<', "\\<")
        .replace('>', "\\>")
}
