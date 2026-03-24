use crate::translator::hir::{ControlFlowGraph, HirTerminator};

impl ControlFlowGraph {
    pub fn to_dot(&self) -> String {
        let mut dot = String::new();
        dot.push_str("digraph CFG {\n");
        dot.push_str("  rankdir=TB;\n");
        dot.push_str("  node [shape=box, fontname=\"Courier\"];\n");
        dot.push_str("  edge [fontname=\"Courier\"];\n\n");

        for block in &self.blocks {
            dot.push_str(&format!("  B{} [label=\"B{}|", block.id, block.id));

            for instr in &block.instructions {
                let instr_str = escape_dot(&instr.to_string());
                dot.push_str(&format!("{}\\l", instr_str));
            }

            if let Some(term) = &block.terminator {
                let term_str = escape_dot(&term.to_string());
                dot.push_str(&format!("{}\\l", term_str));
            }

            dot.push_str("\"];\n");
        }

        for block in &self.blocks {
            if let Some(term) = &block.terminator {
                match term {
                    HirTerminator::Jump(target) => {
                        dot.push_str(&format!("  B{} -> B{};\n", block.id, target));
                    }
                    HirTerminator::Branch {
                        true_block,
                        false_block,
                        ..
                    } => {
                        dot.push_str(&format!(
                            "  B{} -> B{} [label=\"true\"];\n",
                            block.id, true_block
                        ));
                        dot.push_str(&format!(
                            "  B{} -> B{} [label=\"false\"];\n",
                            block.id, false_block
                        ));
                    }
                    HirTerminator::Return(_) => {}
                }
            }
        }

        if !self.blocks.is_empty() {
            dot.push_str(&format!(
                "  B{} [style=filled, fillcolor=lightgreen];\n",
                self.entry_block
            ));
        }

        dot.push_str("}\n");
        dot
    }
}

fn escape_dot(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\l")
        .replace('<', "\\<")
        .replace('>', "\\>")
}
