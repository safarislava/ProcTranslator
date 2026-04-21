use proc_translator::io::{
    load_data, load_interrupt_vector, load_interrupts, load_program, write_bin, write_output,
};
use proc_translator::logger::setup_logger;
use proc_translator::machine::simulation::simulate_machine;
use proc_translator::translator::asm::ControlUnitPackage;
use proc_translator::translator::common::{ResBox, compile};
use proc_translator::translator::printers::scheme::create_cfg_schemes;
use std::{env, fs};

fn main() -> ResBox<()> {
    setup_logger();

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "compile" => {
                let name = args[2].as_str();
                let program = fs::read_to_string(format!("examples/{}.java", name))?;
                let package = compile(&program)?;
                write_bin(name, package)?;
            }
            "simulate" => {
                let name = args[2].as_str();
                let package = ControlUnitPackage {
                    program: load_program(name)?,
                    data: load_data(name)?,
                    interrupt_vectors: load_interrupt_vector(name)?,
                };
                let interrupts = load_interrupts(name)?;
                let (int_output, char_output, _) = simulate_machine(package, interrupts);
                write_output(name, int_output, char_output)?;
            }
            "schemes" => {
                create_cfg_schemes();
            }
            "program" => {
                let name = args[2].as_str();
                let program = fs::read_to_string(format!("examples/{}.java", name))?;
                let package = compile(&program)?;
                let interrupts = load_interrupts(name)?;
                let (int_output, char_output, _) = simulate_machine(package, interrupts);
                write_output(name, int_output, char_output)?;
            }
            _ => panic!("Unknown mode"),
        }
    }

    Ok(())
}
