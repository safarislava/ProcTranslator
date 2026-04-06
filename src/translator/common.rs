use crate::translator::expression::Expression;
use crate::translator::hir::ControlFlowGraph;
use crate::translator::{analyzer, ast, hir, parser, simplifier};
use std::error::Error;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Void,
    Int,
    Bool,
    Char,
    Array(Box<Type>),
    Class(String),
}

pub type RawExpression = Expression<()>;

pub type RawAbstractSyntaxTree = AbstractSyntaxTree<RawExpression>;

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
        arguments: Vec<Variable>,
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
pub type TypedAbstractSyntaxTree = AbstractSyntaxTree<TypedExpression>;

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
