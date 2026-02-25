use crate::common::{AbstractSyntaxNode, BoxError, RawAST, RawExpression, Type, Var};
use crate::expression::parse_expression;
use crate::parser::{SyntaxNode, SyntaxTree};

type DeclarationInfo = (String, String, Option<RawExpression>);

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
    Ok(Var {
        typ: parse_type(parts[0]),
        name: parts[1].to_string(),
    })
}

fn parse_arguments(args_str: &str) -> Result<Vec<Var>, BoxError> {
    let trimmed = args_str.trim();
    if trimmed.is_empty() {
        return Ok(vec![]);
    }
    trimmed.split(',').map(|s| parse_var(s.trim())).collect()
}

fn parse_statement_keyword(
    value: &str,
) -> Result<Option<AbstractSyntaxNode<RawExpression>>, BoxError> {
    let trimmed = value.trim().trim_end_matches(';');

    if trimmed == "return" {
        return Ok(Some(AbstractSyntaxNode::Return { value: None }));
    }

    if let Some(stripped) = trimmed.strip_prefix("return ") {
        let expr = parse_expression(stripped.trim())?;
        return Ok(Some(AbstractSyntaxNode::Return { value: Some(expr) }));
    }
    if trimmed == "break" {
        return Ok(Some(AbstractSyntaxNode::Break));
    }
    if trimmed == "continue" {
        return Ok(Some(AbstractSyntaxNode::Continue));
    }
    Ok(None)
}

fn parse_declaration(raw_code: &str) -> Result<Option<DeclarationInfo>, BoxError> {
    let code = raw_code.trim();
    if code.is_empty() {
        return Ok(None);
    }

    let parts: Vec<&str> = code.splitn(2, ' ').collect();
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
        if name.is_empty() {
            return Ok(None);
        }

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

fn build_for_loop(condition: String, body_children: Vec<RawAST>) -> Result<RawAST, BoxError> {
    let parts: Vec<&str> = condition.split(';').map(|s| s.trim()).collect();
    if parts.len() != 3 {
        return Err(format!("Invalid for loop format: {}", condition).into());
    }

    let initializer = if parts[0].is_empty() {
        None
    } else if let Ok(Some((typ_str, name, init_value))) = parse_declaration(parts[0]) {
        let typ = parse_type(&typ_str);
        Some(Box::new(AbstractSyntaxNode::Declaration {
            typ,
            name,
            expression: init_value,
        }))
    } else {
        let expr = parse_expression(parts[0])?;
        Some(Box::new(AbstractSyntaxNode::Expression {
            expression: expr,
        }))
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

    Ok(RawAST::with_children(
        AbstractSyntaxNode::For {
            initializer,
            condition,
            increment,
        },
        body_children,
    ))
}

pub fn build(tree: SyntaxTree) -> Result<RawAST, BoxError> {
    let processed_children: Vec<RawAST> = tree
        .children
        .into_iter()
        .map(build)
        .collect::<Result<Vec<_>, _>>()?;
    let ast = match tree.node {
        SyntaxNode::If { condition } => RawAST::with_children(
            AbstractSyntaxNode::If {
                condition: parse_expression(&condition)?,
            },
            processed_children,
        ),
        SyntaxNode::ElseIf { condition } => RawAST::with_children(
            AbstractSyntaxNode::ElseIf {
                condition: parse_expression(&condition)?,
            },
            processed_children,
        ),
        SyntaxNode::Else => RawAST::with_children(AbstractSyntaxNode::Else, processed_children),
        SyntaxNode::While { condition } => RawAST::with_children(
            AbstractSyntaxNode::While {
                condition: parse_expression(&condition)?,
            },
            processed_children,
        ),
        SyntaxNode::For { condition } => build_for_loop(condition, processed_children)?,
        SyntaxNode::Line { value } => {
            if let Some(asn) = parse_statement_keyword(&value)? {
                RawAST::new(asn)
            } else if let Ok(Some((typ_str, name, expr))) = parse_declaration(&value) {
                let typ = parse_type(&typ_str);
                RawAST::new(AbstractSyntaxNode::Declaration {
                    typ,
                    name,
                    expression: expr,
                })
            } else {
                RawAST::new(AbstractSyntaxNode::Expression {
                    expression: parse_expression(&value)?,
                })
            }
        }
        SyntaxNode::Function {
            result_type,
            name,
            arguments,
        } => {
            let args = parse_arguments(&arguments)?;
            let result_type = parse_type(&result_type);
            RawAST::with_children(
                AbstractSyntaxNode::Callable {
                    result_type,
                    name,
                    arguments: args,
                },
                processed_children,
            )
        }
        SyntaxNode::Class { name } => {
            RawAST::with_children(AbstractSyntaxNode::Class { name }, processed_children)
        }
        SyntaxNode::Scope => RawAST::with_children(AbstractSyntaxNode::Scope, processed_children),
        SyntaxNode::File => RawAST::with_children(AbstractSyntaxNode::File, processed_children),
    };
    Ok(ast)
}
