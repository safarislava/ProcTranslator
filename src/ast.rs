use crate::common::BoxError;
use crate::expression::{parse_expression, Expression};
use crate::parser::{SyntaxNode, SyntaxTree};

type DeclarationInfo = (String, String, Option<Expression>);

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

#[derive(Debug, Clone)]
pub enum ASN {
    If { condition: Expression },
    ElseIf { condition: Expression },
    Else,
    While { condition: Expression },
    For { initializer: Option<Initializer>, condition: Option<Expression>, increment: Option<Expression> },
    Callable { result_type: Type, name: String, arguments: Vec<Var> },
    Class { name: String },
    Expression { expression: Expression },
    Declaration { typ: Type, name: String, expression: Option<Expression> },
    Return { value: Option<Expression> },
    Break,
    Continue,
    Scope,
    File,
}

#[derive(Debug, Clone)]
pub struct AST {
    pub node: ASN,
    pub children: Vec<AST>,
}

#[derive(Debug, Clone)]
pub enum Initializer {
    Declaration { typ: Type, name: String, expression: Option<Expression> },
    Expression { expression: Expression },
}

impl AST {
    pub fn new(node: ASN) -> Self {
        Self { node, children: vec![] }
    }
    pub fn with_children(node: ASN, children: Vec<AST>) -> Self {
        Self { node, children }
    }
}

fn parse_type(s: &str) -> Type {
    match s.trim() {
        "void" => Type::Void,
        "int" => Type::Int,
        "float" => Type::Float,
        "str" => Type::Str,
        "bool" => Type::Bool,
        other => Type::Class(other.to_string()),
    }
}

fn parse_var(arg: &str) -> Result<Var, BoxError> {
    let parts: Vec<&str> = arg.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(format!("Invalid variable declaration: '{arg}'").into());
    }
    Ok(Var { typ: parse_type(parts[0]), name: parts[1].to_string(), })
}

fn parse_arguments(args_str: &str) -> Result<Vec<Var>, BoxError> {
    let trimmed = args_str.trim();
    if trimmed.is_empty() {
        return Ok(vec![]);
    }
    trimmed.split(',').map(|s| parse_var(s.trim())).collect()
}

fn parse_statement_keyword(value: &str) -> Result<Option<ASN>, BoxError> {
    let trimmed = value.trim().trim_end_matches(';');

    if trimmed == "return" {
        return Ok(Some(ASN::Return { value: None }));
    }

    if let Some(stripped) = trimmed.strip_prefix("return ") {
        let expr = parse_expression(stripped.trim())?;
        return Ok(Some(ASN::Return { value: Some(expr) }));
    }

    if trimmed == "break" { return Ok(Some(ASN::Break)); }
    if trimmed == "continue" { return Ok(Some(ASN::Continue)); }

    Ok(None)
}
fn parse_declaration(raw_code: &str) -> Result<Option<DeclarationInfo>, BoxError> {
    let code = raw_code.trim();
    if code.is_empty() {
        return Ok(None);
    }

    let parts: Vec<&str> = code.splitn(2, char::is_whitespace).collect();
    if parts.len() < 2 {
        return Ok(None);
    }

    let first = parts[0];
    let rest = parts[1].trim();

    let is_primitive = matches!(first, "int" | "float" | "str" | "bool" | "void");
    let is_class = first.chars().next().is_some_and(|c| c.is_uppercase());

    if !is_primitive && !is_class {
        return Ok(None);
    }

    if let Some(eq_pos) = rest.find('=') {
        let name = rest[..eq_pos].trim().to_string();
        let value_expr = rest[eq_pos + 1..].trim();

        if name.is_empty() { return Ok(None); }

        let value = if value_expr.is_empty() {
            None
        } else {
            Some(parse_expression(value_expr)?)
        };
        Ok(Some((first.to_string(), name, value)))
    } else {
        let name = rest.trim().to_string();
        if name.contains('(') || name.is_empty() {
            return Ok(None);
        }
        Ok(Some((first.to_string(), name, None)))
    }
}

fn build_for_loop(condition: String, body_children: Vec<AST>) -> Result<AST, BoxError> {
    let parts: Vec<&str> = condition.split(';').map(|s| s.trim()).collect();
    if parts.len() != 3 {
        return Err(format!("Invalid for loop format: {}", condition).into());
    }
    
    let initializer = if parts[0].is_empty() {
        None
    } else if let Ok(Some((typ_str, name, init_value))) = parse_declaration(parts[0]) {
        let typ = parse_type(&typ_str);
        Some(Initializer::Declaration { typ, name, expression: init_value })
    } else {
        let expr = parse_expression(parts[0])?;
        Some(Initializer::Expression { expression: expr })
    };
    
    let condition = if parts[1].is_empty() {
        None
    } else {
        Some(parse_expression(parts[1])?)
    };
    
    let increment = if parts[2].is_empty() {
        None
    } else {
        Some(parse_expression(parts[2])?)
    };

    Ok(AST::with_children(ASN::For { initializer, condition, increment }, body_children ))
}

pub fn build(tree: SyntaxTree) -> Result<AST, BoxError> {
    let processed_children: Vec<AST> = tree.children
        .into_iter().map(build).collect::<Result<Vec<_>, _>>()?;

    let ast = match tree.node {
        SyntaxNode::If { condition } => AST::with_children(
            ASN::If { condition: parse_expression(&condition)? },
            processed_children,
        ),
        SyntaxNode::ElseIf { condition } => AST::with_children(
            ASN::ElseIf { condition: parse_expression(&condition)? },
            processed_children,
        ),
        SyntaxNode::Else => AST::with_children(ASN::Else, processed_children),
        SyntaxNode::While { condition } => AST::with_children(
            ASN::While { condition: parse_expression(&condition)? },
            processed_children,
        ),
        SyntaxNode::For { condition } =>
            build_for_loop(condition, processed_children)?,
        SyntaxNode::Line { value } => {
            if let Some(asn) = parse_statement_keyword(&value)? {
                AST::new(asn)
            } else if let Ok(Some((typ_str, name, expr))) = parse_declaration(&value) {
                let typ = parse_type(&typ_str);
                AST::new(ASN::Declaration { typ, name, expression: expr })
            } else {
                AST::new(ASN::Expression { expression: parse_expression(&value)? })
            }
        }
        SyntaxNode::Function { result_type, name, arguments } => {
            let args = parse_arguments(&arguments)?;
            let result_type = parse_type(&result_type);
            AST::with_children(ASN::Callable { result_type, name, arguments: args }, processed_children)
        }
        SyntaxNode::Class { name } => AST::with_children(ASN::Class { name }, processed_children),
        SyntaxNode::Scope => AST::with_children(ASN::Scope, processed_children),
        SyntaxNode::File => AST::with_children(ASN::File, processed_children),
    };

    Ok(ast)
}