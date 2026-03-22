use proc_translator::machine::control_unit::ControlUnit;
use proc_translator::translator::asm_translator::{translate};
use proc_translator::translator::common::{ResBox, compile_to_hir, compile_to_lir, dump_to_file};

fn main() {
    let content = std::fs::read_to_string("examples/correct/calc.java").unwrap();
    let (text_section, data_section) = compile_to_lir(&content).unwrap();
    let program = translate(text_section, data_section);
    machine(24, &program);
    println!("End of program");
}

fn machine(start: u64, program: &[u8]) {
    let mut control_unit = ControlUnit::default();
    control_unit.load_program(program);
    control_unit.set_pc(start);
    loop {
        let stop = control_unit.execute_instruction();
        if stop {
            break;
        }
    }
}

#[allow(dead_code)]
fn create_cfg_schemes() {
    create_cfg_scheme("return").unwrap();
    create_cfg_scheme("classes").unwrap();
    create_cfg_scheme("scopes").unwrap();
}

fn create_cfg_scheme(name: &str) -> ResBox<()> {
    let content = std::fs::read_to_string(format!("examples/correct/{name}.java"))?;
    let cfg = compile_to_hir(&content)?;
    dump_to_file(format!("output/{name}.dot"), cfg.to_dot())?;
    Ok(())
}
