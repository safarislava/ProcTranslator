use crate::common::BoxError;
use crate::expression::{parse_expression, Expression};
use crate::parser::{SyntaxNode, SyntaxTree};

type DeclarationInfo = (String, String, Option<Expression>);

#[derive(Debug, Clone)]
pub struct Var {
    pub name: String,
    pub typ: String,
}

#[derive(Debug, Clone)]
pub enum ASN {
    If { condition: Expression },
    ElseIf { condition: Expression },
    Else,
    While { condition: Expression },
    For { initializer: Option<Initializer>, condition: Option<Expression>, increment: Option<Expression> },
    Function { result_type: String, name: String, arguments: Vec<Var> },
    Class { name: String },
    Expression { expression: Expression },
    Declaration { typ: String, name: String, expression: Option<Expression> },
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
    Declaration { typ: String, name: String, expression: Option<Expression> },
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

fn is_type_keyword(s: &str) -> bool {
    matches!(s, "int" | "string" | "bool" | "float" | "double" | "char" | "void")
}

fn parse_var(arg: &str) -> Result<Var, BoxError> {
    let parts: Vec<&str> = arg.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(format!("Invalid variable declaration: '{arg}'").into());
    }
    Ok(Var { typ: parts[0].to_string(), name: parts[1].to_string(), })
}

fn parse_arguments(args_str: &str) -> Result<Vec<Var>, BoxError> {
    if args_str.trim().is_empty() {
        return Ok(vec![]);
    }
    args_str.split(',').map(|s| parse_var(s.trim())).collect()
}

fn parse_statement_keyword(value: &str) -> Option<ASN> {
    let trimmed = value.trim();

    if let Some(mut rest) = trimmed.strip_prefix("return") {
        rest = rest.trim();
        if rest.is_empty() {
            return Some(ASN::Return { value: None });
        } else {
            return match parse_expression(rest.to_string()) {
                Ok(expr) => Some(ASN::Return { value: Some(expr) }),
                Err(_) => None,
            }
        }
    }

    if trimmed == "break" {
        return Some(ASN::Break);
    }

    if trimmed == "continue" {
        return Some(ASN::Continue);
    }

    None
}

fn parse_declaration(raw_code: String) -> Result<Option<DeclarationInfo>, BoxError> {
    let code = raw_code.trim();
    if code.is_empty() {
        return Ok(None);
    }

    let mut parts = code.splitn(2, char::is_whitespace);
    let first = parts.next().unwrap_or("");

    if !is_type_keyword(first) {
        return Ok(None);
    }

    let typ = first.to_string();
    let rest = parts.next().ok_or("Missing variable name after type")?.trim();
    if rest.is_empty() {
        return Err("Missing variable name after type".into());
    }

    if let Some(eq_pos) = rest.find('=') {
        let name = rest[..eq_pos].trim().to_string();
        if name.is_empty() {
            return Err("Empty variable name".into());
        }
        let value_expr = rest[eq_pos + 1..].trim();
        let value = if value_expr.is_empty() {
            None
        } else {
            Some(parse_expression(value_expr.to_string())?)
        };
        Ok(Some((typ, name, value)))
    } else {
        let name = rest.trim().to_string();
        if name.is_empty() {
            return Err("Empty variable name".into());
        }
        Ok(Some((typ, name, None)))
    }
}

fn build_for_loop(condition: String, body_children: Vec<AST>) -> Result<AST, BoxError> {
    let parts: Vec<&str> = condition.split(';').map(|s| s.trim()).collect();
    if parts.len() != 3 {
        return Err(format!("Invalid for loop format: expected, got: {condition}").into());
    }
    
    let initializer = if parts[0].is_empty() {
        None
    } else if let Some((typ, name, init_value)) = parse_declaration(parts[0].to_string())? {
        Some(Initializer::Declaration { typ, name, expression: init_value })
    } else {
        let expr = parse_expression(parts[0].to_string())?;
        Some(Initializer::Expression { expression: expr })
    };
    
    let condition = if parts[1].is_empty() {
        None
    } else {
        Some(parse_expression(parts[1].to_string())?)
    };
    
    let increment = if parts[2].is_empty() {
        None
    } else {
        Some(parse_expression(parts[2].to_string())?)
    };

    Ok(AST::with_children(ASN::For { initializer, condition, increment }, body_children ))
}

pub fn build(tree: SyntaxTree) -> Result<AST, BoxError> {
    let processed_children: Vec<AST> = tree
        .children
        .into_iter()
        .map(build)
        .collect::<Result<Vec<_>, _>>()?;

    let ast = match tree.node {
        SyntaxNode::If { condition } => AST::with_children(
            ASN::If { condition: parse_expression(condition)? },
            processed_children,
        ),
        SyntaxNode::ElseIf { condition } => AST::with_children(
            ASN::ElseIf { condition: parse_expression(condition)? },
            processed_children,
        ),
        SyntaxNode::Else => AST::with_children(ASN::Else, processed_children),
        SyntaxNode::While { condition } => AST::with_children(
            ASN::While { condition: parse_expression(condition)? },
            processed_children,
        ),
        SyntaxNode::For { condition } => 
            build_for_loop(condition, processed_children)?,
        SyntaxNode::Line { value } => {
            if let Some(asm) = parse_statement_keyword(&value) {
                AST::new(asm)
            }
            else if let Some((typ, name, expr)) = parse_declaration(value.clone())? {
                AST::new(ASN::Declaration { typ, name, expression: expr })
            } else {
                AST::new(ASN::Expression { expression: parse_expression(value)? })
            }
        }
        SyntaxNode::Function { result_type, name, arguments } => {
            let args = parse_arguments(&arguments)?;
            AST::with_children(ASN::Function { result_type, name, arguments: args }, processed_children)
        }
        SyntaxNode::Class { name } => AST::with_children(ASN::Class { name }, processed_children),
        SyntaxNode::Scope => AST::with_children(ASN::Scope, processed_children),
        SyntaxNode::File => AST::with_children(ASN::File, processed_children),
    };

    Ok(ast)
}