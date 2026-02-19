use std::error::Error;
use crate::parser::SyntaxTree;

enum Mnemonic {

}

pub fn translate_to_bin(tree: SyntaxTree) -> Result<Vec<u32>, Box<dyn Error>> {
    let mnemonics = translate_to_mnemonic(tree)?;
    Ok(vec![])
}

fn translate_to_mnemonic(tree: SyntaxTree) -> Result<Vec<Mnemonic>, Box<dyn Error>> {
    Ok(vec![])
}