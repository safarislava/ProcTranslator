mod parser;
mod analyzer;
mod ast;
mod expression;
mod common;
mod ir;
mod simplifier;
mod tests;
mod printers;

use crate::common::BoxError;

fn main() -> Result<(), BoxError> {
    compile_file("/Users/safarislava/Documents/Projects/ProcTranslator/examples/correct/scopes.java")?;
    Ok(())
}

fn compile_file(path: &str) -> Result<(), BoxError> {
    let content = std::fs::read_to_string(path)?;

    let syntax_tree = parser::parse_syntax_tree(&content)?;
    let ast = ast::build(syntax_tree)?;
    let simple_ast = simplifier::simplify(ast);
    let typed_ast = analyzer::semantic_analyze(simple_ast)?;

    let cfg = ir::compile(typed_ast);
    cfg.dump_to_file("output/cfg.dot")?;
    Ok(())
}
