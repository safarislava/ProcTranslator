use std::collections::HashMap;
use crate::common::BoxError;
use crate::ast::AST;

#[derive(Debug, Clone)]
pub struct Var {
    pub name: String,
    pub typ: String,
}

pub struct VarTable {
    scopes: Vec<HashMap<String, Var>>,
}

pub fn semantic_analyze(ast: &AST) -> Result<(), BoxError> {
    Ok(())
}
