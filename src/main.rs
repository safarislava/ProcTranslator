mod parser;
mod translator;

use std::fs;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let file_path = "/Users/safarislava/Documents/Projects/ProcessorModel/src/examples/code.java";
    let content = fs::read_to_string(file_path)?;

    let syntax_tree = parser::parse_syntax_tree(&content)?;
    let bin = translator::translate_to_bin(syntax_tree);

    Ok(())
}
