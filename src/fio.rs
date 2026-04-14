use crate::machine::simulation::{DeviceChoice, InterruptRequest};
use crate::translator::asm::ControlUnitPackage;
use crate::translator::common::ResBox;
use std::fs;

pub fn load_program(name: &str) -> ResBox<Vec<u32>> {
    let program = fs::read_to_string(format!("bin/{}.program", name))?;
    Ok(program
        .split("\n")
        .map(|v| v.parse::<u32>().unwrap())
        .collect())
}

pub fn load_data(name: &str) -> ResBox<Vec<u64>> {
    let data = fs::read_to_string(format!("bin/{}.data", name))?;
    Ok(data
        .split("\n")
        .map(|v| v.parse::<u64>().unwrap())
        .collect())
}

pub fn load_interrupt_vector(name: &str) -> ResBox<[u64; 8]> {
    let interrupt_vectors = fs::read_to_string(format!("bin/{}.vector", name))?;
    Ok(*interrupt_vectors
        .split("\n")
        .map(|v| v.parse::<u64>().unwrap())
        .collect::<Vec<u64>>()
        .as_array()
        .unwrap())
}

pub fn load_interrupts(name: &str) -> ResBox<Vec<InterruptRequest>> {
    let path = format!("examples/{}.interrupt", name);

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Ok(vec![]),
    };

    let mut interrupts = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 3 {
            return Err(format!(
                "Ошибка в файле {}.interrupt: строка {} должна содержать 3 значения (tick value device)",
                name, line_num + 1
            ).into());
        }

        let tick: u64 = parts[0]
            .parse()
            .map_err(|_| format!("Неверный tick в строке {}: {}", line_num + 1, parts[0]))?;

        let device = match parts[2] {
            "IntInput" => DeviceChoice::IntInput,
            "CharInput" => DeviceChoice::CharInput,
            other => {
                return Err(
                    format!("Неверный device в строке {}: '{}'.", line_num + 1, other).into(),
                );
            }
        };

        let value = match parts[1].parse() {
            Ok(v) => v,
            Err(_) => parts[1].chars().next().unwrap() as u8 as i64,
        };

        interrupts.push(InterruptRequest {
            tick,
            value,
            device,
        });
    }

    Ok(interrupts)
}

pub fn write_bin(name: &str, package: ControlUnitPackage) -> ResBox<()> {
    fs::write(
        format!("bin/{}.program", name),
        package
            .program
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    )?;
    fs::write(
        format!("bin/{}.data", name),
        package
            .data
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    )?;
    fs::write(
        format!("bin/{}.vector", name),
        package
            .interrupt_vectors
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    )?;

    Ok(())
}

pub fn write_output(name: &str, int_output: Vec<i64>, char_output: Vec<char>) -> ResBox<()> {
    let string_output = char_output.iter().collect::<String>();
    let output = format!("{:?}\n{}", int_output, string_output);

    fs::write(format!("output/{}.txt", name), output)?;
    Ok(())
}
