use proc_translator::machine::control_unit::ControlUnit;

fn main() {
    let program: [u8; 25] = [
        (0x1 << 1) + 1, // MOV
        0,              // Direct
        1 << 5,         // D0
        4,              // 4
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        (0x1 << 1) + 1,      // MOV
        0,                   // Direct
        (1 << 5) + (1 << 2), // D1
        5,                   // 5
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        (0x10 << 1) + 1,     // ADD
        1 << 5,              // D0
        (1 << 5) + (1 << 2), // D1
    ];
    let mut control_unit = ControlUnit::default();
    control_unit.load_program(&program);
    loop {
        let stop = control_unit.execute_instruction();
        if stop {
            break;
        }
    }
    println!("Stop")
}

// fn create_cfg_scheme() -> ResBox<()> {
//     let name = "return";
//     let content = std::fs::read_to_string(format!("examples/correct/{name}.java"))?;
//     let cfg = compile_to_ir(&content)?;
//     dump_to_file(format!("output/{name}.dot"), cfg.to_dot())?;
//     Ok(())
// }
