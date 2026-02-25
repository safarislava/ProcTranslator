use proc_translator::common::{BoxError, compile_to_ir, dump_to_file};

fn main() -> Result<(), BoxError> {
    let name = "return";
    let content = std::fs::read_to_string(format!("examples/correct/{name}.java"))?;
    let cfg = compile_to_ir(&content)?;
    dump_to_file(format!("output/{name}.dot"), cfg.to_dot())?;
    Ok(())
}