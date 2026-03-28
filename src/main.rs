use proc_translator::machine::control_unit::ControlUnit;
use proc_translator::translator::asm_translator::translate;
use proc_translator::translator::common::{ConstantAddress, ResBox, compile_to_hir, dump_to_file};
use proc_translator::translator::lir::compile_lir;
use std::collections::HashMap;
use std::fs;
use tracing_appender::non_blocking;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, filter::LevelFilter, fmt, prelude::*};

fn main() -> ResBox<()> {
    setup_logger();

    let name = "for";
    let content = fs::read_to_string(format!("examples/correct/{name}.java"))?;

    let (control_flow_graph, classes) = compile_to_hir(&content)?;
    dump_to_file(format!("output/{name}.dot"), control_flow_graph.to_dot())?;

    let (text_section, data_section) = compile_lir(control_flow_graph, classes);
    let program = translate(text_section);

    machine(&program, data_section);
    Ok(())
}

fn machine(program: &[u8], data_section: HashMap<String, ConstantAddress>) {
    let mut control_unit = ControlUnit::default();
    control_unit.load_program(program);
    control_unit.load_constants(data_section);
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
    let content = std::fs::read_to_string(format!("examples/correct/{name}.java"))?;
    let (cfg, _) = compile_to_hir(&content)?;
    dump_to_file(format!("output/{name}.dot"), cfg.to_dot())?;
    Ok(())
}

pub fn setup_logger() {
    let _ = fs::create_dir_all("./logs");

    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let file_name = format!("app_{}.log", timestamp);

    let file_appender = RollingFileAppender::new(Rotation::NEVER, "./logs", file_name);

    let (non_blocking_file, _guard) = non_blocking(file_appender);
    std::mem::forget(_guard);

    let file_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .parse_lossy("");

    let console_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .parse_lossy("");

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_ansi(true)
                .with_target(false)
                .without_time()
                .compact()
                .with_filter(console_filter),
        )
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_target(false)
                .without_time()
                .with_writer(non_blocking_file)
                .with_filter(file_filter),
        )
        .init();
}
