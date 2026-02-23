mod parser;
mod analyzer;
mod ast;
mod expression;
mod common;
mod ir;

use std::fs;
use crate::common::BoxError;

fn main() -> Result<(), BoxError> {
    let file_path = "/Users/safarislava/Documents/Projects/ProcTranslator/src/examples/scopes.java";
    let content = fs::read_to_string(file_path)?;

    let syntax_tree = parser::parse_syntax_tree(&content)?;
    let ast = ast::build(syntax_tree)?;
    let typed_ast = analyzer::semantic_analyze(ast)?;

    let cfg = ir::compile(typed_ast);

    Ok(())
}
