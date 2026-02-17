use std::error::Error;
use regex::Regex;

pub struct SyntaxTree {
    value: String,
    children: Vec<SyntaxTree>,
}

pub fn parse_syntax_tree(raw_code: &str) -> Result<SyntaxTree, Box<dyn Error>> {
    let sentences = parse_sentences(raw_code)?;
    let mut tree_stack: Vec<SyntaxTree> = vec![];
    let mut consumed_as_label: Vec<bool> = vec![false; sentences.len()];

    for (i, sentence) in sentences.iter().enumerate() {
        if sentence == "{" {
            let mut new_tree = SyntaxTree { value: String::new(), children: vec![] };
            if i > 0 && !consumed_as_label[i - 1] {
                let prev = &sentences[i - 1];
                if prev != ";" && prev != "{" && prev != "}" {
                    new_tree.value = prev.clone();
                    consumed_as_label[i - 1] = true;
                }
            }
            tree_stack.push(new_tree);
        }
        else if sentence == "}" {
            if tree_stack.is_empty() {
                return Err("Unmatched closing bracket".into());
            }

            let current_tree = tree_stack.pop().unwrap();

            if let Some(parent) = tree_stack.last_mut() {
                parent.children.push(current_tree);
            }
            else {
                return Ok(current_tree);
            }
        }
        else if sentence != ";" {
            let next_is_open_brace = (i + 1 < sentences.len()) && (sentences[i + 1] == "{");

            if !next_is_open_brace {
                if let Some(current_tree) = tree_stack.last_mut() {
                    current_tree.children.push(
                        SyntaxTree { value: sentence.clone(), children: vec![] }
                    );
                }
            }
        }
    }


    if let Some(tree) = tree_stack.pop() {
        Ok(tree)
    }
    else {
        Ok(SyntaxTree { value: String::new(), children: vec![] })
    }
}

fn parse_sentences(raw_code: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let mut sentances = vec![];
    let mut code: String = raw_code.trim().to_string();
    loop {
        match code.find(['{', '}', ';']) {
            Some(sentence_break) => {
                if sentence_break != 0 {
                    sentances.push(code[..sentence_break].trim().to_string());
                }
                sentances.push(code.chars().nth(sentence_break).unwrap().to_string());
                code = code[sentence_break+1..].trim().to_string();
            }
            None => {
                return Ok(sentances);
            }
        }
    }
}

pub fn find_main_fn(tree: &SyntaxTree) -> Result<&SyntaxTree, Box<dyn Error>> {
    let re = Regex::new(r"public static void Main(\.*)")?;
    if re.is_match(&tree.value) {
        return Ok(tree);
    }
    for child in &tree.children {
        match find_main_fn(child) {
            Ok(tree) => return Ok(tree),
            Err(_) => continue,
        }
    }
    Err(From::from("Not found"))
}