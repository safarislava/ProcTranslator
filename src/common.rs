use crate::expression::Expression;
use std::error::Error;

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

pub type RawAST = AbstractSyntaxTree<RawExpression>;

#[derive(Debug, Clone)]
pub enum AbstractSyntaxNode<E> {
    If {
        condition: E,
    },
    ElseIf {
        condition: E,
    },
    Else,
    While {
        condition: E,
    },
    For {
        initializer: Option<Box<AbstractSyntaxNode<E>>>,
        condition: Option<E>,
        increment: Option<E>,
    },
    Callable {
        result_type: Type,
        name: String,
        arguments: Vec<Var>,
    },
    Class {
        name: String,
    },
    Expression {
        expression: E,
    },
    Declaration {
        typ: Type,
        name: String,
        expression: Option<E>,
    },
    Return {
        value: Option<E>,
    },
    Break,
    Continue,
    Scope,
    File,
}

#[derive(Debug, Clone)]
pub struct AbstractSyntaxTree<E> {
    pub node: AbstractSyntaxNode<E>,
    pub children: Vec<AbstractSyntaxTree<E>>,
}

impl<E> AbstractSyntaxTree<E> {
    pub fn new(node: AbstractSyntaxNode<E>) -> Self {
        Self {
            node,
            children: vec![],
        }
    }
    pub fn with_children(
        node: AbstractSyntaxNode<E>,
        children: Vec<AbstractSyntaxTree<E>>,
    ) -> Self {
        Self { node, children }
    }
}

pub type TypedExpression = Expression<Type>;
pub type TypedAST = AbstractSyntaxTree<TypedExpression>;
