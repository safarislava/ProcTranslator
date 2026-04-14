use crate::translator::asm::{ControlUnitPackage, translate};
use crate::translator::expression::Expression;
use crate::translator::{analyzer, ast, hir, lir, parser, simplifier};
use std::error::Error;
use std::fmt::Debug;

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

pub fn compile(content: &str) -> ResBox<ControlUnitPackage> {
    let syntax_tree = parser::parse_syntax_tree(content)?;
    let ast = ast::build_ast(syntax_tree)?;
    let simple_ast = simplifier::simplify(ast);
    let typed_ast = analyzer::semantic_analyze(simple_ast)?;
    let control_flow_graph = hir::compile_hir(typed_ast);
    let lir_package = lir::compile_lir(control_flow_graph);
    let package = translate(lir_package);
    Ok(package)
}
