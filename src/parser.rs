use std::error::Error;
use regex::Regex;

pub enum SyntaxNode {
    If { condition: String },
    ElseIf { condition: String },
    Else,
    While { condition: String },
    For { condition: String },
    Function { result_type: String, name: String, arguments: String },
    Class { name: String },
    Line { value: String },
    Scope,
    File,
    Empty
}

pub struct SyntaxTree {
    node: SyntaxNode,
    children: Vec<SyntaxTree>,
}

impl SyntaxTree {
    pub fn new(node: SyntaxNode) -> Self {
        SyntaxTree { node, children: Vec::new() }
    }
}

pub fn parse_syntax_tree(raw_code: &str) -> Result<SyntaxTree, Box<dyn Error>> {
    let if_pattern = Regex::new(r"^if\s*\((.+)\)")?;
    let else_if_pattern = Regex::new(r"^else\s+if\s*\((.+)\)")?;
    let else_pattern = Regex::new(r"^else")?;
    let while_pattern = Regex::new(r"^while\s*\((.+)\)")?;
    let for_pattern = Regex::new(r"^for\s*\((.+)\)")?;
    let function_pattern = Regex::new(r"^(\w+)\s+(\w+)\s*\(([^)]*)\)")?;
    let class_pattern = Regex::new(r"^class\s+(\w+)")?;

    let sentences = parse_sentences(raw_code)?;
    let mut tree_stack: Vec<SyntaxTree> = vec![ SyntaxTree::new(SyntaxNode::File) ];

    for sentence in sentences.iter() {
        if sentence == "{" {
            if let Some(top) = tree_stack.last() {
                match &top.node {
                    SyntaxNode::If { .. } |
                    SyntaxNode::ElseIf { .. } |
                    SyntaxNode::Else |
                    SyntaxNode::While { .. } |
                    SyntaxNode::For { .. } |
                    SyntaxNode::Function { .. } |
                    SyntaxNode::Class { .. } => {
                        continue;
                    }
                    _ => {
                        tree_stack.push(SyntaxTree::new(SyntaxNode::Scope));
                    }
                }
            }
        }
        else if sentence == "}" {
            if tree_stack.len() < 2 {
                return Err("Unmatched closing bracket".into());
            }
            let current_tree = tree_stack.pop().unwrap();
            tree_stack.last_mut().unwrap().children.push(current_tree);
        }
        else if let Some(captures) = if_pattern.captures(sentence) {
            let condition = captures.get(1).unwrap().as_str().to_string();
            tree_stack.push(SyntaxTree::new(SyntaxNode::If { condition }));
        }
        else if let Some(captures) = else_if_pattern.captures(sentence) {
            let condition = captures.get(1).unwrap().as_str().to_string();
            tree_stack.push(SyntaxTree::new(SyntaxNode::ElseIf { condition }));
        }
        else if let Some(captures) = else_pattern.captures(sentence) {
            tree_stack.push(SyntaxTree::new(SyntaxNode::Else));
        }
        else if let Some(captures) = while_pattern.captures(sentence) {
            let condition = captures.get(1).unwrap().as_str().to_string();
            tree_stack.push(SyntaxTree::new(SyntaxNode::While { condition }));
        }
        else if let Some(captures) = for_pattern.captures(sentence) {
            let condition = captures.get(1).unwrap().as_str().to_string();
            tree_stack.push(SyntaxTree::new(SyntaxNode::For { condition }));
        }
        else if let Some(captures) = function_pattern.captures(sentence) {
            let result_type = captures.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
            let name = captures.get(2).map(|m| m.as_str()).unwrap_or("").to_string();
            let arguments = captures.get(3).map(|m| m.as_str()).unwrap_or("").to_string();
            tree_stack.push(SyntaxTree::new(SyntaxNode::Function { result_type, name, arguments }));
        }
        else if let Some(captures) = class_pattern.captures(sentence) {
            let name = captures.get(1).unwrap().as_str().to_string();
            tree_stack.push(SyntaxTree::new(SyntaxNode::Class { name }));
        }
        else if sentence.ends_with(';') {
            let value = sentence.trim_end_matches(';').trim().to_string();
            tree_stack.last_mut().unwrap()
                .children.push(SyntaxTree::new(SyntaxNode::Line { value }));
        }
        else if !sentence.trim().is_empty() {
            return Err(From::from("Empty sentence"));
        }
        else {
            return Err(From::from("Unknown syntax"));
        }
    }
    tree_stack.pop().ok_or_else(|| "Syntax tree is empty".into())
}

fn parse_sentences(raw_code: &str) -> Result<Vec<String>, Box<dyn Error>> {
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
                    return Err(From::from("Unmatched closing bracket"));
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

pub fn find_main_fn(tree: &SyntaxTree) -> Result<&SyntaxTree, Box<dyn Error>> {
    if let SyntaxNode::Function { result_type, name, arguments} = &tree.node {
        if name == "Main" {
            return Ok(tree)
        }
    }
    for child in &tree.children {
        match find_main_fn(child) {
            Ok(tree) => return Ok(tree),
            Err(_) => continue,
        }
    }
    Err(From::from("Not found"))
}