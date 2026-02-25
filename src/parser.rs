use crate::common::BoxError;
use regex::Regex;

#[derive(Debug)]
pub enum SyntaxNode {
    If {
        condition: String,
    },
    ElseIf {
        condition: String,
    },
    Else,
    While {
        condition: String,
    },
    For {
        condition: String,
    },
    Function {
        result_type: String,
        name: String,
        arguments: String,
    },
    Class {
        name: String,
    },
    Line {
        value: String,
    },
    Scope,
    File,
}

#[derive(Debug)]
pub struct SyntaxTree {
    pub node: SyntaxNode,
    pub children: Vec<SyntaxTree>,
}

impl SyntaxTree {
    pub fn new(node: SyntaxNode) -> Self {
        SyntaxTree {
            node,
            children: Vec::new(),
        }
    }
}

pub fn parse_syntax_tree(raw_code: &str) -> Result<SyntaxTree, BoxError> {
    let if_pattern = Regex::new(r"^if\s*\((.+)\)")?;
    let else_if_pattern = Regex::new(r"^else\s+if\s*\((.+)\)")?;
    let else_pattern = Regex::new(r"^else")?;
    let while_pattern = Regex::new(r"^while\s*\((.+)\)")?;
    let for_pattern = Regex::new(r"^for\s*\((.+)\)")?;
    let function_pattern = Regex::new(r"^(\w+)\s+(\w+)\s*\(([^)]*)\)")?;
    let class_pattern = Regex::new(r"^class\s+(\w+)")?;

    let sentences = parse_sentences(raw_code)?;
    let mut tree_stack: Vec<SyntaxTree> = vec![SyntaxTree::new(SyntaxNode::File)];
    let mut expect_body = false;

    for sentence in sentences.iter() {
        if sentence == "{" {
            if expect_body {
                expect_body = false;
            } else {
                tree_stack.push(SyntaxTree::new(SyntaxNode::Scope));
            }
        } else if sentence == "}" {
            if tree_stack.len() < 2 {
                return Err("Unmatched closing bracket".into());
            }
            let current_tree = tree_stack.pop().unwrap();
            tree_stack.last_mut().unwrap().children.push(current_tree);
        } else if let Some(captures) = if_pattern.captures(sentence) {
            let condition = captures.get(1).unwrap().as_str().to_string();
            tree_stack.push(SyntaxTree::new(SyntaxNode::If { condition }));
            expect_body = true;
        } else if let Some(captures) = else_if_pattern.captures(sentence) {
            let condition = captures.get(1).unwrap().as_str().to_string();
            tree_stack.push(SyntaxTree::new(SyntaxNode::ElseIf { condition }));
            expect_body = true;
        } else if else_pattern.is_match(sentence) {
            tree_stack.push(SyntaxTree::new(SyntaxNode::Else));
            expect_body = true;
        } else if let Some(captures) = while_pattern.captures(sentence) {
            let condition = captures.get(1).unwrap().as_str().to_string();
            tree_stack.push(SyntaxTree::new(SyntaxNode::While { condition }));
            expect_body = true;
        } else if let Some(captures) = for_pattern.captures(sentence) {
            let condition = captures.get(1).unwrap().as_str().to_string();
            tree_stack.push(SyntaxTree::new(SyntaxNode::For { condition }));
            expect_body = true;
        } else if let Some(captures) = function_pattern.captures(sentence) {
            let result_type = captures
                .get(1)
                .map(|m| m.as_str())
                .unwrap_or("void")
                .to_string();
            let name = captures
                .get(2)
                .map(|m| m.as_str())
                .unwrap_or("")
                .to_string();
            let arguments = captures
                .get(3)
                .map(|m| m.as_str())
                .unwrap_or("")
                .to_string();
            tree_stack.push(SyntaxTree::new(SyntaxNode::Function {
                result_type,
                name,
                arguments,
            }));
            expect_body = true;
        } else if let Some(captures) = class_pattern.captures(sentence) {
            let name = captures.get(1).unwrap().as_str().to_string();
            tree_stack.push(SyntaxTree::new(SyntaxNode::Class { name }));
            expect_body = true;
        } else if sentence.ends_with(';') {
            let value = sentence.trim_end_matches(';').trim().to_string();
            tree_stack
                .last_mut()
                .unwrap()
                .children
                .push(SyntaxTree::new(SyntaxNode::Line { value }));
        } else {
            return Err("Wrong syntax".into());
        }
    }

    if tree_stack.len() != 1 {
        return Err("Unclosed brackets".into());
    }

    tree_stack
        .pop()
        .ok_or_else(|| "Syntax tree is empty".into())
}

fn parse_sentences(raw_code: &str) -> Result<Vec<String>, BoxError> {
    let mut sentences = vec![];
    let mut token = String::new();
    let mut depth = 0;

    for c in raw_code.chars() {
        match c {
            '(' => {
                depth += 1;
                token.push(c);
            }
            ')' => {
                if depth == 0 {
                    return Err("Unmatched closing bracket".into());
                }
                depth -= 1;
                token.push(c);
            }
            '{' | '}' => {
                if !token.trim().is_empty() {
                    sentences.push(token.trim().to_string());
                }
                sentences.push(c.to_string());
                token = String::new();
            }
            ';' => {
                if token.trim().is_empty() {
                    token = String::new();
                    continue;
                }
                token.push(c);
                if depth == 0 {
                    sentences.push(token.trim().to_string());
                    token = String::new();
                }
            }
            _ => {
                token.push(c);
            }
        }
    }

    if !token.trim().is_empty() {
        sentences.push(token.trim().to_string());
    }

    Ok(sentences)
}
