use crate::translator::common::{RawExpression, ResBox, Type, Variable};
use crate::translator::expression::parse_expression;
use crate::translator::parser::{SyntaxNode, SyntaxTree};

type DeclarationInfo = (String, String, Option<RawExpression>);

#[derive(Debug, Clone)]
pub enum RawAbstractSyntaxNode {
    If {
        condition: RawExpression,
    },
    ElseIf {
        condition: RawExpression,
    },
    Else,
    While {
        condition: RawExpression,
    },
    For {
        initializer: Option<Box<RawAbstractSyntaxNode>>,
        condition: Option<RawExpression>,
        increment: Option<RawExpression>,
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
        expression: RawExpression,
    },
    Declaration {
        typ: Type,
        name: String,
        expression: Option<RawExpression>,
    },
    Return {
        value: Option<RawExpression>,
    },
    Break,
    Continue,
    Scope,
    File,
}

#[derive(Debug, Clone)]
pub struct RawAbstractSyntaxTree {
    pub node: RawAbstractSyntaxNode,
    pub children: Vec<RawAbstractSyntaxTree>,
}

impl RawAbstractSyntaxTree {
    pub fn new(node: RawAbstractSyntaxNode) -> Self {
        Self {
            node,
            children: vec![],
        }
    }
    pub fn with_children(
        node: RawAbstractSyntaxNode,
        children: Vec<RawAbstractSyntaxTree>,
    ) -> Self {
        Self { node, children }
    }
}

fn parse_type(s: &str) -> Type {
    let s = s.trim();

    if s.ends_with(']')
        && let Some(start) = s.rfind('[')
    {
        let base = &s[..start];
        let size = s[start + 1..s.len() - 1].trim();
        let size = size.parse::<u64>().unwrap();
        return Type::Array(Box::new(parse_type(base)), size);
    }

    match s {
        "void" => Type::Void,
        "int" => Type::Int,
        "char" => Type::Char,
        "bool" => Type::Bool,
        other => Type::Class(other.to_string()),
    }
}

fn parse_variable(s: &str) -> ResBox<Variable> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(format!("Invalid variable declaration: '{s}'").into());
    }
    Ok(Variable {
        typ: parse_type(parts[0]),
        name: parts[1].to_string(),
    })
}

fn parse_arguments(arguments: &str) -> ResBox<Vec<Variable>> {
    let arguments = arguments.trim();
    if arguments.is_empty() {
        return Ok(vec![]);
    }
    arguments
        .split(',')
        .map(|s| parse_variable(s.trim()))
        .collect()
}

fn parse_statement_keyword(value: &str) -> ResBox<Option<RawAbstractSyntaxNode>> {
    let value = value.trim().trim_end_matches(';');

    if value == "return" {
        return Ok(Some(RawAbstractSyntaxNode::Return { value: None }));
    }
    if let Some(value) = value.strip_prefix("return") {
        let expression = parse_expression(value.trim())?;
        return Ok(Some(RawAbstractSyntaxNode::Return {
            value: Some(expression),
        }));
    }
    if value == "break" {
        return Ok(Some(RawAbstractSyntaxNode::Break));
    }
    if value == "continue" {
        return Ok(Some(RawAbstractSyntaxNode::Continue));
    }
    Ok(None)
}

fn parse_declaration(s: &str) -> ResBox<Option<DeclarationInfo>> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }

    let parts: Vec<&str> = s.splitn(2, ' ').collect();
    if parts.len() < 2 {
        return Ok(None);
    }

    let typ = parts[0];
    let rest = parts[1].trim();

    let base_type = match typ.find('[') {
        Some(index) => &typ[..index],
        None => typ,
    };

    let is_primitive = matches!(base_type, "int" | "bool" | "void" | "char");
    let is_class = base_type.chars().next().is_some_and(|c| c.is_uppercase());
    if !is_primitive && !is_class {
        return Ok(None);
    }

    if let Some(equal_position) = rest.find('=') {
        let name = rest[..equal_position].trim().to_string();
        let expression = rest[equal_position + 1..].trim();
        if name.is_empty() {
            return Ok(None);
        }

        let value = if expression.is_empty() {
            None
        } else {
            Some(parse_expression(expression)?)
        };
        Ok(Some((typ.to_string(), name, value)))
    } else {
        let name = rest.trim().to_string();
        if name.contains('(') || name.is_empty() {
            return Ok(None);
        }
        Ok(Some((typ.to_string(), name, None)))
    }
}

fn build_for_loop(
    condition: String,
    body_children: Vec<RawAbstractSyntaxTree>,
) -> ResBox<RawAbstractSyntaxTree> {
    let parts: Vec<&str> = condition.split(';').map(|s| s.trim()).collect();
    if parts.len() != 3 {
        return Err(format!("Invalid for loop format: {}", condition).into());
    }

    let initializer = if parts[0].is_empty() {
        None
    } else if let Ok(Some((typ, name, initializer))) = parse_declaration(parts[0]) {
        let typ = parse_type(&typ);
        Some(Box::new(RawAbstractSyntaxNode::Declaration {
            typ,
            name,
            expression: initializer,
        }))
    } else {
        let expression = parse_expression(parts[0])?;
        Some(Box::new(RawAbstractSyntaxNode::Expression { expression }))
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

    Ok(RawAbstractSyntaxTree::with_children(
        RawAbstractSyntaxNode::For {
            initializer,
            condition,
            increment,
        },
        body_children,
    ))
}

pub fn build_ast(tree: SyntaxTree) -> ResBox<RawAbstractSyntaxTree> {
    let mut processed_children: Vec<RawAbstractSyntaxTree> = tree
        .children
        .into_iter()
        .map(build_ast)
        .collect::<Result<Vec<_>, _>>()?;

    let ast = match tree.node {
        SyntaxNode::If { condition } => RawAbstractSyntaxTree::with_children(
            RawAbstractSyntaxNode::If {
                condition: parse_expression(&condition)?,
            },
            processed_children,
        ),
        SyntaxNode::ElseIf { condition } => RawAbstractSyntaxTree::with_children(
            RawAbstractSyntaxNode::ElseIf {
                condition: parse_expression(&condition)?,
            },
            processed_children,
        ),
        SyntaxNode::Else => {
            RawAbstractSyntaxTree::with_children(RawAbstractSyntaxNode::Else, processed_children)
        }
        SyntaxNode::While { condition } => RawAbstractSyntaxTree::with_children(
            RawAbstractSyntaxNode::While {
                condition: parse_expression(&condition)?,
            },
            processed_children,
        ),
        SyntaxNode::For { condition } => build_for_loop(condition, processed_children)?,
        SyntaxNode::Line { value } => {
            if let Some(asn) = parse_statement_keyword(&value)? {
                RawAbstractSyntaxTree::new(asn)
            } else if let Ok(Some((typ, name, expression))) = parse_declaration(&value) {
                let typ = parse_type(&typ);
                RawAbstractSyntaxTree::new(RawAbstractSyntaxNode::Declaration {
                    typ,
                    name,
                    expression,
                })
            } else {
                RawAbstractSyntaxTree::new(RawAbstractSyntaxNode::Expression {
                    expression: parse_expression(&value)?,
                })
            }
        }
        SyntaxNode::Function {
            result_type,
            name,
            arguments,
        } => {
            let arguments = parse_arguments(&arguments)?;
            let result_type = parse_type(&result_type);
            RawAbstractSyntaxTree::with_children(
                RawAbstractSyntaxNode::Callable {
                    result_type,
                    name,
                    arguments,
                },
                processed_children,
            )
        }
        SyntaxNode::Class { name } => RawAbstractSyntaxTree::with_children(
            RawAbstractSyntaxNode::Class { name },
            processed_children,
        ),
        SyntaxNode::Scope => {
            RawAbstractSyntaxTree::with_children(RawAbstractSyntaxNode::Scope, processed_children)
        }
        SyntaxNode::File => {
            let in_function = RawAbstractSyntaxTree::with_children(
                RawAbstractSyntaxNode::Callable {
                    result_type: Type::Int,
                    name: "iin".to_string(),
                    arguments: vec![Variable {
                        name: "port".to_string(),
                        typ: Type::Int,
                    }],
                },
                vec![RawAbstractSyntaxTree::new(RawAbstractSyntaxNode::Return {
                    value: Some(RawExpression::Literal {
                        typ: (),
                        value: "0".to_string(),
                    }),
                })],
            );
            processed_children.push(in_function);

            let in_function = RawAbstractSyntaxTree::with_children(
                RawAbstractSyntaxNode::Callable {
                    result_type: Type::Char,
                    name: "cin".to_string(),
                    arguments: vec![Variable {
                        name: "port".to_string(),
                        typ: Type::Int,
                    }],
                },
                vec![RawAbstractSyntaxTree::new(RawAbstractSyntaxNode::Return {
                    value: Some(RawExpression::Literal {
                        typ: (),
                        value: "'0'".to_string(),
                    }),
                })],
            );
            processed_children.push(in_function);

            let out_function = RawAbstractSyntaxTree::with_children(
                RawAbstractSyntaxNode::Callable {
                    result_type: Type::Void,
                    name: "iout".to_string(),
                    arguments: vec![
                        Variable {
                            name: "port".to_string(),
                            typ: Type::Int,
                        },
                        Variable {
                            name: "value".to_string(),
                            typ: Type::Int,
                        },
                    ],
                },
                vec![RawAbstractSyntaxTree::new(RawAbstractSyntaxNode::Return {
                    value: None,
                })],
            );
            processed_children.push(out_function);

            let out_function = RawAbstractSyntaxTree::with_children(
                RawAbstractSyntaxNode::Callable {
                    result_type: Type::Void,
                    name: "cout".to_string(),
                    arguments: vec![
                        Variable {
                            name: "port".to_string(),
                            typ: Type::Int,
                        },
                        Variable {
                            name: "value".to_string(),
                            typ: Type::Char,
                        },
                    ],
                },
                vec![RawAbstractSyntaxTree::new(RawAbstractSyntaxNode::Return {
                    value: None,
                })],
            );
            processed_children.push(out_function);

            RawAbstractSyntaxTree::with_children(RawAbstractSyntaxNode::File, processed_children)
        }
    };
    Ok(ast)
}
