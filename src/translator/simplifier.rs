use crate::translator::common::{AbstractSyntaxNode, RawAbstractSyntaxTree, RawExpression};

pub fn simplify(ast: RawAbstractSyntaxTree) -> RawAbstractSyntaxTree {
    let RawAbstractSyntaxTree { node, children } = ast;
    let mut simplified_children = Vec::new();
    let mut iterator = children.into_iter().peekable();

    while let Some(mut child) = iterator.next() {
        if let AbstractSyntaxNode::If { .. } = child.node {
            child = build_if_tree(child, &mut iterator);
        }
        simplified_children.push(simplify(child));
    }

    match node {
        AbstractSyntaxNode::For {
            initializer,
            condition,
            increment,
        } => {
            let mut scope_children = Vec::new();

            if let Some(init_box) = initializer {
                let init_ast = RawAbstractSyntaxTree::new(*init_box);
                scope_children.push(simplify(init_ast));
            }

            let mut while_body = simplified_children;
            if let Some(inc) = increment {
                while_body.push(RawAbstractSyntaxTree::new(AbstractSyntaxNode::Expression {
                    expression: inc,
                }));
            }

            let condition = condition.unwrap_or_else(|| RawExpression::Literal {
                typ: (),
                value: "true".into(),
            });

            let while_node = RawAbstractSyntaxTree::with_children(
                AbstractSyntaxNode::While { condition },
                while_body,
            );
            scope_children.push(while_node);

            RawAbstractSyntaxTree::with_children(AbstractSyntaxNode::Scope, scope_children)
        }
        other_node => RawAbstractSyntaxTree::with_children(other_node, simplified_children),
    }
}

fn build_if_tree(
    mut current_node: RawAbstractSyntaxTree,
    iterator: &mut std::iter::Peekable<impl Iterator<Item = RawAbstractSyntaxTree>>,
) -> RawAbstractSyntaxTree {
    if let AbstractSyntaxNode::ElseIf { condition } = current_node.node {
        current_node.node = AbstractSyntaxNode::If { condition };
    }

    match iterator.peek().map(|typed_ast| &typed_ast.node) {
        Some(AbstractSyntaxNode::ElseIf { .. }) => {
            let next_node = iterator.next().unwrap();
            let nested_if = build_if_tree(next_node, iterator);
            current_node
                .children
                .push(RawAbstractSyntaxTree::with_children(
                    AbstractSyntaxNode::Else,
                    vec![nested_if],
                ));
        }
        Some(AbstractSyntaxNode::Else) => {
            let else_node = iterator.next().unwrap();
            current_node.children.push(else_node);
        }
        _ => {}
    }

    current_node
}
