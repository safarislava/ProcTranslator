use proc_translator::machine::control_unit::ControlUnit;
use proc_translator::translator::common::{ResBox, compile_to_hir, dump_to_file};

fn main() {
    let program = vec![
        0b00000011_00000000_00100000_00000000,
        0x00_00_00_04,
        0b00000011_00000000_00100100_00000000,
        0x00_00_00_05,
        0b00000101_00100000_00100100_00000000,
    ];
    let mut control_unit = ControlUnit::default();
    control_unit.load_program(&program);
    loop {
        let stop = control_unit.execute_instruction();
        if stop {
            break;
        }
    }

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
