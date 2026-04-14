use proc_translator::io::{
    load_data, load_interrupt_vector, load_interrupts, load_program, write_bin, write_output,
};
use proc_translator::logger::setup_logger;
use proc_translator::machine::simulation::simulate_machine;
use proc_translator::translator::asm::ControlUnitPackage;
use proc_translator::translator::common::{ResBox, compile};
use std::{env, fs};

enum Mode {
    Compile,
    Simulate,
    All,
}

fn main() -> ResBox<()> {
    let args: Vec<String> = env::args().collect();

    let mut name = "cat";
    let mut mode = Mode::All;
    if args.len() > 1 {
        match args[1].as_str() {
            "compile" => {
                mode = Mode::Compile;
                name = args[2].as_str();
            }
            "simulate" => {
                mode = Mode::Simulate;
                name = args[2].as_str();
            }
            "all" => {
                mode = Mode::All;
                name = args[2].as_str();
            }
            _ => panic!("Unknown mode"),
        }
    }

    setup_logger();

    match mode {
        Mode::Compile => {
            let program = fs::read_to_string(format!("examples/{}.java", name))?;
            let package = compile(&program)?;
            write_bin(name, package)?;
        }
        Mode::Simulate => {
            let package = ControlUnitPackage {
                program: load_program(name)?,
                data: load_data(name)?,
                interrupt_vectors: load_interrupt_vector(name)?,
            };
            let interrupts = load_interrupts(name)?;
            let (int_output, char_output) = simulate_machine(package, interrupts);
            write_output(name, int_output, char_output)?;
        }
        Mode::All => {
            let program = fs::read_to_string(format!("examples/{}.java", name))?;
            let package = compile(&program)?;
            let interrupts = load_interrupts(name)?;
            let (int_output, char_output) = simulate_machine(package, interrupts);
            write_output(name, int_output, char_output)?;
        }
    }

    Ok(())
}
