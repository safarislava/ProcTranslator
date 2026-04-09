use crate::translator::ast::{RawAbstractSyntaxNode, RawAbstractSyntaxTree};
use crate::translator::common::RawExpression;
use crate::translator::expression::{Expression, ExpressionBinaryOperator};

fn simplify_ast(ast: RawAbstractSyntaxTree) -> RawAbstractSyntaxTree {
    let RawAbstractSyntaxTree { node, children } = ast;
    let mut simplified_children = Vec::new();
    let mut iterator = children.into_iter().peekable();

    while let Some(mut child) = iterator.next() {
        if let RawAbstractSyntaxNode::If { .. } = child.node {
            child = build_if_tree(child, &mut iterator);
        }
        simplified_children.push(simplify_ast(child));
    }

    match node {
        RawAbstractSyntaxNode::For {
            initializer,
            condition,
            increment,
        } => {
            let mut scope_children = Vec::new();

            if let Some(init_box) = initializer {
                let init_ast = RawAbstractSyntaxTree::new(*init_box);
                scope_children.push(simplify_ast(init_ast));
            }

            let mut while_body = simplified_children;
            if let Some(increment) = increment {
                while_body.push(RawAbstractSyntaxTree::new(
                    RawAbstractSyntaxNode::Expression {
                        expression: increment,
                    },
                ));
            }

            let condition = condition.unwrap_or_else(|| RawExpression::Literal {
                typ: (),
                value: "true".into(),
            });

            let while_node = RawAbstractSyntaxTree::with_children(
                RawAbstractSyntaxNode::While { condition },
                while_body,
            );
            scope_children.push(while_node);

            RawAbstractSyntaxTree::with_children(RawAbstractSyntaxNode::Scope, scope_children)
        }
        other_node => RawAbstractSyntaxTree::with_children(other_node, simplified_children),
    }
}

fn build_if_tree(
    mut current_node: RawAbstractSyntaxTree,
    iterator: &mut std::iter::Peekable<impl Iterator<Item = RawAbstractSyntaxTree>>,
) -> RawAbstractSyntaxTree {
    if let RawAbstractSyntaxNode::ElseIf { condition } = current_node.node {
        current_node.node = RawAbstractSyntaxNode::If { condition };
    }

    match iterator.peek().map(|typed_ast| &typed_ast.node) {
        Some(RawAbstractSyntaxNode::ElseIf { .. }) => {
            let next_node = iterator.next().unwrap();
            let nested_if = build_if_tree(next_node, iterator);
            current_node
                .children
                .push(RawAbstractSyntaxTree::with_children(
                    RawAbstractSyntaxNode::Else,
                    vec![nested_if],
                ));
        }
        Some(RawAbstractSyntaxNode::Else) => {
            let else_node = iterator.next().unwrap();
            current_node.children.push(else_node);
        }
        _ => {}
    }

    current_node
}

fn simplify_expression(expression: RawExpression) -> RawExpression {
    match expression.clone() {
        RawExpression::BinaryOperator {
            typ,
            left,
            operator,
            right,
        } => match operator {
            ExpressionBinaryOperator::AssignAdd
            | ExpressionBinaryOperator::AssignSub
            | ExpressionBinaryOperator::AssignMul
            | ExpressionBinaryOperator::AssignDiv
            | ExpressionBinaryOperator::AssignAnd
            | ExpressionBinaryOperator::AssignOr
            | ExpressionBinaryOperator::AssignXor => {
                let operator = match operator {
                    ExpressionBinaryOperator::AssignAdd => ExpressionBinaryOperator::Add,
                    ExpressionBinaryOperator::AssignSub => ExpressionBinaryOperator::Sub,
                    ExpressionBinaryOperator::AssignMul => ExpressionBinaryOperator::Multiply,
                    ExpressionBinaryOperator::AssignDiv => ExpressionBinaryOperator::Divide,
                    ExpressionBinaryOperator::AssignAnd => ExpressionBinaryOperator::BitwiseAnd,
                    ExpressionBinaryOperator::AssignOr => ExpressionBinaryOperator::BitwiseOr,
                    ExpressionBinaryOperator::AssignXor => ExpressionBinaryOperator::BitwiseXor,
                    _ => unreachable!(),
                };

                let name = match *left {
                    Expression::Variable { typ: _, name } => name,
                    _ => unreachable!(),
                };

                RawExpression::Assign {
                    typ,
                    name: name.clone(),
                    value: Box::new(Expression::BinaryOperator {
                        typ: (),
                        left: Box::new(Expression::Variable { typ: (), name }),
                        operator,
                        right,
                    }),
                }
            }
            _ => expression,
        },
        _ => expression,
    }
}

fn simplify_expressions(ast: RawAbstractSyntaxTree) -> RawAbstractSyntaxTree {
    let RawAbstractSyntaxTree { node, children } = ast;
    let mut simplified_children = Vec::new();

    for child in children {
        simplified_children.push(simplify_expressions(child));
    }

    match node {
        RawAbstractSyntaxNode::Expression { expression } => RawAbstractSyntaxTree::with_children(
            RawAbstractSyntaxNode::Expression {
                expression: simplify_expression(expression),
            },
            simplified_children,
        ),
        _ => RawAbstractSyntaxTree::with_children(node, simplified_children),
    }
}

pub fn simplify(ast: RawAbstractSyntaxTree) -> RawAbstractSyntaxTree {
    let ast = simplify_ast(ast);
    simplify_expressions(ast)
}
