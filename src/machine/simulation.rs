use crate::machine::control_unit::ControlUnit;
use crate::translator::asm::ControlUnitPackage;

pub struct InterruptRequest {
    pub tick: u64,
    pub value: i64,
    pub port: u8,
    pub vector_port: u8,
}

pub fn simulate_machine(package: ControlUnitPackage, mut interrupts: Vec<InterruptRequest>) {
    let mut control_unit = ControlUnit::default();
    control_unit.load_program(&package.program);
    control_unit.load_data_section(package.data);
    control_unit.load_interrupt_vectors(package.interrupt_vectors);

    loop {
        if let Some(interrupt) = interrupts.first()
            && interrupt.tick <= control_unit.tick
            && !control_unit.interrupt
        {
            control_unit
                .data_path
                .io
                .write_internal(interrupt.port, interrupt.value);
            let vector = control_unit
                .data_path
                .io
                .read_internal(interrupt.vector_port) as u8;
            control_unit.set_interrupt_vector(vector);

            interrupts.remove(0);
        }
        if control_unit.step() {
            break;
        }
    }
}
