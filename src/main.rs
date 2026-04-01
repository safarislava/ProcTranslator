use proc_translator::logger::setup_logger;
use proc_translator::machine::control_unit::ControlUnit;
use proc_translator::translator::asm_translator::translate;
use proc_translator::translator::common::{Address, ResBox, compile_to_hir, dump_to_file};
use proc_translator::translator::lir::compile_lir;
use std::collections::HashMap;
use std::fs;

fn main() -> ResBox<()> {
    setup_logger();

    let name = "calc";
    let content = fs::read_to_string(format!("examples/correct/{name}.java"))?;

    let control_flow_graph = compile_to_hir(&content)?;
    dump_to_file(format!("output/{name}.dot"), control_flow_graph.to_dot())?;

    let (text_section, data_section) = compile_lir(control_flow_graph);
    let program = translate(text_section);

    machine(&program, data_section);
    Ok(())
}

fn machine(program: &[u8], data_section: HashMap<Address, u64>) {
    let mut control_unit = ControlUnit::default();
    control_unit.load_program(program);
    control_unit.load_data_section(data_section);
    loop {
        if control_unit.step() {
            break;
        }
    }
}

#[allow(dead_code)]
fn create_cfg_schemes() {
    create_cfg_scheme("calc").unwrap();
    create_cfg_scheme("return").unwrap();
    create_cfg_scheme("classes").unwrap();
    create_cfg_scheme("scopes").unwrap();
}

fn create_cfg_scheme(name: &str) -> ResBox<()> {
    let content = fs::read_to_string(format!("examples/correct/{name}.java"))?;
    let cfg = compile_to_hir(&content)?;
    dump_to_file(format!("output/{name}.dot"), cfg.to_dot())?;
    Ok(())
}
