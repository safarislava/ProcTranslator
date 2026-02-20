use crate::ast::{Initializer, ASN, AST};
use crate::common::BoxError;
use crate::expression::Expression;

fn simplify(ast: AST) -> Result<AST, BoxError> {
    let mut simplified_children = Vec::new();
    let mut iter = ast.children.into_iter().peekable();

    while let Some(child) = iter.next() {
        match &child.node {
            ASN::If { .. } => {
                let combined_if = build_if_tree(child, &mut iter)?;
                simplified_children.push(simplify(combined_if)?);
            }
            _ => {
                simplified_children.push(simplify(child)?);
            }
        }
    }

    match ast.node {
        ASN::For { initializer, condition, increment } => {
            let mut scope_children = Vec::new();

            if let Some(init) = initializer {
                let init_node = match init {
                    Initializer::Declaration { typ, name, expression } =>
                        ASN::Declaration { typ, name, expression },
                    Initializer::Expression { expression } =>
                        ASN::Expression { expression },
                };
                scope_children.push(AST::new(init_node));
            }

            let mut while_body = simplified_children;
            if let Some(inc) = increment {
                while_body.push(AST::new(ASN::Expression { expression: inc }));
            }

            let condition = condition.unwrap_or_else(|| Expression::Literal("true".into()));
            let while_node = AST::with_children(ASN::While { condition }, while_body);
            scope_children.push(while_node);

            Ok(AST::with_children(ASN::Scope, scope_children))
        }
        other_node => Ok(AST::with_children(other_node, simplified_children)),
    }
}

fn build_if_tree(mut current_node: AST, iter: &mut std::iter::Peekable<impl Iterator<Item = AST>>) -> Result<AST, BoxError> {
    if let Some(next_node) = iter.peek() {
        match &next_node.node {
            ASN::ElseIf { .. } => {
                let next_ast = iter.next().unwrap();
                let nested_if_node = build_if_tree(next_ast, iter)?;

                let else_node = AST::with_children(ASN::Else, vec![nested_if_node]);
                current_node.children.push(else_node);
            }
            ASN::Else => {
                let else_node = iter.next().unwrap();
                current_node.children.push(else_node);
            }
            _ => {}
        }
    }

    if let ASN::ElseIf { condition } = current_node.node {
        current_node.node = ASN::If { condition };
    }
    Ok(current_node)
}

#[derive(Debug, Clone)]
pub enum Operand {
    Variable(String),
    Temporary(usize),
    Constant(String),
}

#[derive(Debug, Clone)]
pub enum IrInstruction {
    Assign { dest: Operand, src: Operand },
    BinaryOp { dest: Operand, left: Operand, op: String, right: Operand },
    Label(String),
    Jump(String),
    JumpIfFalse { cond: Operand, target: String },
    Call { name: String, args: Vec<Operand>, dest: Option<Operand> },
    Return(Option<Operand>),
}

pub struct IrContext {
    temp_counter: usize,
    label_counter: usize,
}

impl IrContext {
    pub fn new() -> Self {
        IrContext { temp_counter: 0, label_counter: 0 }
    }
    fn linearize(&mut self, ast: AST) -> Result<Vec<IrInstruction>, BoxError> {
        Ok(vec![])
    }

    fn emit_expression(&self, expression: Expression) -> Result<(Operand, Vec<IrInstruction>), BoxError> {
        // x = (a + b) * c  =>
        // %t1 = a + b
        // %t2 = %t1 * c
        // returns (%t2, [instructions])
        Err("Not implemented".into())
    }
}

pub fn compile(ast: AST) -> Result<Vec<IrInstruction>, BoxError> {
    let simple_ast = simplify(ast)?;
    let mut ir_ctx = IrContext::new();
    ir_ctx.linearize(simple_ast)
}
