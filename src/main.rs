mod analyzer;
mod ast;
mod common;
mod expression;
mod ir;
mod parser;
mod printers;
mod simplifier;
mod tests;

use crate::common::BoxError;
use crate::ir::ControlFlowGraph;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() -> Result<(), BoxError> {
    let name = "classes";
    let content = std::fs::read_to_string(format!("examples/correct/{name}.java"))?;
    let cfg = compile_to_ir(&content)?;
    dump_to_file(format!("output/{name}.dot"), cfg.to_dot())?;
    Ok(())
}

pub fn compile_to_ir(content: &str) -> Result<ControlFlowGraph, BoxError> {
    let syntax_tree = parser::parse_syntax_tree(content)?;
    let ast = ast::build(syntax_tree)?;
    let simple_ast = simplifier::simplify(ast);
    let typed_ast = analyzer::semantic_analyze(simple_ast)?;
    let cfg = ir::compile(typed_ast);
    Ok(cfg)
}

fn dump_to_file(path: impl AsRef<Path>, value: String) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(value.as_bytes())?;
    Ok(())
}
