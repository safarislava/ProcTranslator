use crate::machine::control_unit::ControlUnit;
use crate::translator::asm::ControlUnitPackage;

#[derive(Clone)]
pub enum DeviceChoice {
    IntInput,
    CharInput,
}

#[derive(Clone)]
pub struct InterruptRequest {
    pub tick: u64,
    pub value: i64,
    pub device: DeviceChoice,
}

pub fn simulate_machine(
    package: ControlUnitPackage,
    mut interrupts: Vec<InterruptRequest>,
) -> (Vec<i64>, Vec<char>) {
    let mut control_unit = ControlUnit::default();
    control_unit.load_program(&package.program);
    control_unit.load_data_section(package.data);
    control_unit.load_interrupt_vectors(package.interrupt_vectors);

    loop {
        if let Some(interrupt) = interrupts.first()
            && interrupt.tick <= control_unit.tick
            && !control_unit.data_path.io.check_interrupt()
        {
            let port = match interrupt.device {
                DeviceChoice::IntInput => 0,
                DeviceChoice::CharInput => 2,
            };
            control_unit
                .data_path
                .io
                .write_internal(port, interrupt.value);

            match interrupt.device {
                DeviceChoice::IntInput => {
                    control_unit.data_path.io.int_input_device.interrupt = true
                }
                DeviceChoice::CharInput => {
                    control_unit.data_path.io.char_input_device.interrupt = true
                }
            };

            control_unit.data_path.io.update_interrupt_vector();

            interrupts.remove(0);
        }
        if control_unit.step() {
            break;
        }
    }

    (
        control_unit.data_path.io.int_output_log,
        control_unit.data_path.io.char_output_log,
    )
}
