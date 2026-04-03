use crate::machine::control_unit::ControlUnit;
use crate::translator::common::Address;
use std::collections::HashMap;

pub fn simulate_machine(
    program: &[u8],
    data_section: HashMap<Address, u64>,
    interrupt_blocks: [Address; 8],
) {
    let mut control_unit = ControlUnit::default();
    control_unit.load_program(program);
    control_unit.load_data_section(data_section);
    control_unit.load_interrupt_vectors(interrupt_blocks);
    loop {
        if control_unit.step() {
            break;
        }
    }
}
