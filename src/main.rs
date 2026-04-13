use proc_translator::logger::setup_logger;
use proc_translator::machine::simulation::simulate_machine;
use proc_translator::translator::asm::translate;
use proc_translator::translator::common::{ResBox, compile_to_hir, dump_to_file};
use proc_translator::translator::lir::compile_lir;
use std::fs;

fn main() -> ResBox<()> {
    setup_logger();
    // create_cfg_schemes();

    let name = "prob1";
    let content = fs::read_to_string(format!("examples/{name}.java"))?;

    let control_flow_graph = compile_to_hir(&content)?;
    let lir_package = compile_lir(control_flow_graph);
    let package = translate(lir_package);
    simulate_machine(package, vec![]);
    Ok(())
}

#[allow(dead_code)]
fn create_cfg_schemes() {
    create_cfg_scheme("array").unwrap();
    create_cfg_scheme("bitwise").unwrap();
    create_cfg_scheme("bool").unwrap();
    create_cfg_scheme("calc").unwrap();
    create_cfg_scheme("cat").unwrap();
    create_cfg_scheme("classes").unwrap();
    create_cfg_scheme("double").unwrap();
    create_cfg_scheme("for").unwrap();
    create_cfg_scheme("global").unwrap();
    create_cfg_scheme("hello_user").unwrap();
    create_cfg_scheme("hello_world").unwrap();
    create_cfg_scheme("params").unwrap();
    create_cfg_scheme("prob1").unwrap();
    create_cfg_scheme("return").unwrap();
    create_cfg_scheme("sort").unwrap();
    create_cfg_scheme("vector").unwrap();
    create_cfg_scheme("vector_test").unwrap();
    create_cfg_scheme("vector_test_simd").unwrap();
    create_cfg_scheme("while").unwrap();
}

fn create_cfg_scheme(name: &str) -> ResBox<()> {
    let content = fs::read_to_string(format!("examples/{name}.java"))?;
    let cfg = compile_to_hir(&content)?;
    dump_to_file(format!("output/{name}.dot"), cfg.to_dot())?;
    Ok(())
}
