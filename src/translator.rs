// analyze Tree node, find all new variables and push it on stack

use std::error::Error;
use crate::parser::*;

enum Mnemonic {

}

pub fn translate_to_bin(mut tree: SyntaxTree) -> Result<Vec<u32>, Box<dyn Error>> {
    post_processing(&mut tree)?;
    let mnemonics = translate_to_mnemonic(tree)?;
    Ok(vec![])
}

fn post_processing(tree: &mut SyntaxTree) -> Result<(), Box<dyn Error>> {
    match &tree.node {
        SyntaxNode::For { condition: _ } => for_post_processing(tree)?,
        SyntaxNode::Else | SyntaxNode::ElseIf { condition: _ }  => else_elif_post_processing(tree)?,
        _ => {}
    };

    for child in tree.children.iter_mut() {
        post_processing(child)?;
    }
    Ok(())
}

fn for_post_processing(tree: &mut SyntaxTree) -> Result<(), Box<dyn Error>> {
    let (condition, old_children) = match &tree.node {
        SyntaxNode::For { condition } => (condition.clone(), std::mem::take(&mut tree.children)),
        _ => return Err("Expected a for".into()),
    };

    let parts: Vec<&str> = condition.split(';').map(|s| s.trim()).collect();
    if parts.len() != 3 {
        return Err(format!("Invalid for loop format: {condition}").into());
    }

    let init_part = SyntaxTree::new(SyntaxNode::Line { value: parts[0].to_string() });

    let mut while_body = old_children;
    while_body.push(SyntaxTree::new(SyntaxNode::Line { value: parts[2].to_string() }));

    let mut while_tree = SyntaxTree::new(SyntaxNode::While { condition: parts[1].to_string() });
    while_tree.children = while_body;

    tree.node = SyntaxNode::Scope;
    tree.children = vec![init_part, while_tree];

    Ok(())
}

fn else_elif_post_processing(mut tree: &SyntaxTree) -> Result<(), Box<dyn Error>> {

    Ok(())
}

fn translate_to_mnemonic(tree: SyntaxTree) -> Result<Vec<Mnemonic>, Box<dyn Error>> {
    Ok(vec![])
}