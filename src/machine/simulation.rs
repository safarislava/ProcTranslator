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

impl InterruptRequest {
    pub fn new(tick: u64, value: i64, device: DeviceChoice) -> Self {
        Self {
            tick,
            value,
            device,
        }
    }
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
        let ready: Vec<_> = interrupts
            .iter()
            .enumerate()
            .filter(|(_, irq)| irq.tick <= control_unit.tick)
            .map(|(i, _)| i)
            .collect();

        if !ready.is_empty() {
            if control_unit.data_path.io.check_interrupt() {
                for &i in ready.iter().rev() {
                    interrupts.remove(i);
                }
            } else {
                let mut int_ready = Vec::new();
                let mut char_ready = Vec::new();

                for &i in &ready {
                    match interrupts[i].device {
                        DeviceChoice::IntInput => int_ready.push(i),
                        DeviceChoice::CharInput => char_ready.push(i),
                    }
                }

                if !int_ready.is_empty() {
                    int_ready.sort_by_key(|&i| interrupts[i].tick);
                    let chosen = int_ready[0];
                    process_interrupt(&mut control_unit, &interrupts[chosen]);
                    for &i in int_ready.iter().rev() {
                        interrupts.remove(i);
                    }
                } else if !char_ready.is_empty() {
                    char_ready.sort_by_key(|&i| interrupts[i].tick);
                    let chosen = char_ready[0];
                    process_interrupt(&mut control_unit, &interrupts[chosen]);
                    for &i in char_ready.iter().rev() {
                        interrupts.remove(i);
                    }
                }
            }
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

fn process_interrupt(control_unit: &mut ControlUnit, interrupt: &InterruptRequest) {
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
            control_unit.data_path.io.int_input_device.interrupt = true;
        }
        DeviceChoice::CharInput => {
            control_unit.data_path.io.char_input_device.interrupt = true;
        }
    }

    control_unit.data_path.io.update_interrupt_vector();
}
