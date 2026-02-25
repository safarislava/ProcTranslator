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

fn main() -> Result<(), BoxError> {
    compile_and_make_dump(
        "/Users/safarislava/Documents/Projects/ProcTranslator/examples/correct/scopes.java",
    )?;
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

fn compile_and_make_dump(path: &str) -> Result<(), BoxError> {
    let content = std::fs::read_to_string(path)?;
    let cfg = compile_to_ir(&content)?;
    cfg.dump_to_file("output/cfg.dot")?;
    Ok(())
}
