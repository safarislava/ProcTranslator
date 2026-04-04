use proc_translator::logger::setup_logger;
use proc_translator::machine::simulation::{InterruptRequest, simulate_machine};
use proc_translator::translator::asm_translator::translate;
use proc_translator::translator::common::{ResBox, compile_to_hir, dump_to_file};
use proc_translator::translator::lir::compile_lir;
use std::fs;

fn main() -> ResBox<()> {
    setup_logger();

    let name = "interrupt";
    let content = fs::read_to_string(format!("examples/correct/{name}.java"))?;

    let control_flow_graph = compile_to_hir(&content)?;
    dump_to_file(format!("output/{name}.dot"), control_flow_graph.to_dot())?;
    let (text_section, data_section, interrupt_blocks) = compile_lir(control_flow_graph);
    let package = translate(text_section, data_section, interrupt_blocks);

    let interrupts = vec![InterruptRequest {
        tick: 63,
        value: 1,
        port: 0,
        vector_port: 1,
    }];
    simulate_machine(package, interrupts);
    Ok(())
}

#[allow(dead_code)]
fn create_cfg_schemes() {
    create_cfg_scheme("bool").unwrap();
    create_cfg_scheme("calc").unwrap();
    create_cfg_scheme("classes").unwrap();
    create_cfg_scheme("for").unwrap();
    create_cfg_scheme("global").unwrap();
    create_cfg_scheme("prob1").unwrap();
    create_cfg_scheme("return").unwrap();
    create_cfg_scheme("while").unwrap();
    create_cfg_scheme("params").unwrap();
}

fn create_cfg_scheme(name: &str) -> ResBox<()> {
    let content = fs::read_to_string(format!("examples/correct/{name}.java"))?;
    let cfg = compile_to_hir(&content)?;
    dump_to_file(format!("output/{name}.dot"), cfg.to_dot())?;
    Ok(())
}
