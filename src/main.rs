use proc_translator::logger::setup_logger;
use proc_translator::machine::simulation::{DeviceChoice, InterruptRequest, simulate_machine};
use proc_translator::translator::asm::translate;
use proc_translator::translator::common::{ResBox, compile_to_hir, dump_to_file};
use proc_translator::translator::lir::compile_lir;
use std::fs;

fn main() -> ResBox<()> {
    setup_logger();

    let name = "interrupt";
    let content = fs::read_to_string(format!("examples/correct/{name}.java"))?;

    let control_flow_graph = compile_to_hir(&content)?;
    dump_to_file(format!("output/{name}.dot"), control_flow_graph.to_dot())?;
    let lir_package = compile_lir(control_flow_graph);
    let package = translate(lir_package);
    simulate_machine(
        package,
        vec![
            InterruptRequest {
                tick: 200,
                value: 72,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 1250,
                value: 101,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 1350,
                value: 108,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 1450,
                value: 108,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 1550,
                value: 111,
                device: DeviceChoice::CharInput,
            },
            InterruptRequest {
                tick: 1650,
                value: 0,
                device: DeviceChoice::CharInput,
            },
        ],
    );
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
