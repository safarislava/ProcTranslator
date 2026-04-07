use crate::translator::expression::Expression;
use crate::translator::hir::ControlFlowGraph;
use crate::translator::{analyzer, ast, hir, parser, simplifier};
use std::error::Error;
use std::fmt::Debug;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub type ResBox<T> = Result<T, Box<dyn Error>>;

pub type Address = u64;

#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub typ: Type,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Void,
    Int,
    Bool,
    Char,
    Array(Box<Type>, u64),
    Class(String),
}

pub type RawExpression = Expression<()>;

pub type TypedExpression = Expression<Type>;

pub fn compile_to_hir(content: &str) -> ResBox<ControlFlowGraph> {
    let syntax_tree = parser::parse_syntax_tree(content)?;
    let ast = ast::build_ast(syntax_tree)?;
    let simple_ast = simplifier::simplify(ast);
    let typed_ast = analyzer::semantic_analyze(simple_ast)?;
    let control_flow_graph = hir::compile_hir(typed_ast);
    Ok(control_flow_graph)
}

pub fn dump_to_file(path: impl AsRef<Path>, value: String) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(value.as_bytes())?;
    Ok(())
}
