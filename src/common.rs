use std::error::Error;
use crate::expression::Expression;

pub type BoxError = Box<dyn Error>;

#[derive(Debug, Clone)]
pub struct Var {
    pub name: String,
    pub typ: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Void,
    Int,
    Float,
    Bool,
    Str,
    Class(String),
}

pub type RawExpression = Expression<()>;
pub type RawAST = AST<RawExpression>;

#[derive(Debug, Clone)]
pub enum ASN<E> {
    If { condition: E },
    ElseIf { condition: E },
    Else,
    While { condition: E },
    For { initializer: Option<Box<ASN<E>>>, condition: Option<E>, increment: Option<E> },
    Callable { result_type: Type, name: String, arguments: Vec<Var> },
    Class { name: String },
    Expression { expression: E },
    Declaration { typ: Type, name: String, expression: Option<E> },
    Return { value: Option<E> },
    Break,
    Continue,
    Scope,
    File,
}

#[derive(Debug, Clone)]
pub struct AST<E> {
    pub node: ASN<E>,
    pub children: Vec<AST<E>>,
}

impl<E> AST<E> {
    pub fn new(node: ASN<E>) -> Self {
        Self { node, children: vec![] }
    }
    pub fn with_children(node: ASN<E>, children: Vec<AST<E>>) -> Self {
        Self { node, children }
    }
}

pub type TypedExpression = Expression<Type>;
pub type TypedAST = AST<TypedExpression>;
