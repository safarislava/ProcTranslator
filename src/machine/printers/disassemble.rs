use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Direct,
    DataRegister,
    AddressRegister,
    Indirect,
    IndirectPostIncrement,
    IndirectPreDecrement,
    IndirectOffset,
    IndirectDirect,
}

fn get_mode(code: u8) -> Mode {
    match code {
        0 => Mode::Direct,
        1 => Mode::DataRegister,
        2 => Mode::AddressRegister,
        3 => Mode::Indirect,
        4 => Mode::IndirectPostIncrement,
        5 => Mode::IndirectPreDecrement,
        6 => Mode::IndirectOffset,
        7 => Mode::IndirectDirect,
        _ => Mode::Direct,
    }
}

fn fetch_imm(program: &[u32], i: &mut usize) -> u32 {
    *i += 1;
    if *i < program.len() { program[*i] } else { 0 }
}

fn parse_operand(byte: u8, program: &[u32], i: &mut usize) -> (String, String) {
    let mode_code = (byte >> 5) & 0x7;
    let main_register = (byte >> 2) & 0x7;
    let offset = byte & 0x3;
    let mode = get_mode(mode_code);

    match mode {
        Mode::Direct => {
            let imm = fetch_imm(program, i);
            (format!("#{}", imm as i32), format!("{}", imm as i32))
        }
        Mode::DataRegister => (format!("D{}", main_register), format!("D{}", main_register)),
        Mode::AddressRegister => (format!("A{}", main_register), format!("A{}", main_register)),
        Mode::Indirect => (
            format!("(A{})", main_register),
            format!("MEM[A{}]", main_register),
        ),
        Mode::IndirectPostIncrement => (
            format!("(A{})+", main_register),
            format!("MEM[A{}++]", main_register),
        ),
        Mode::IndirectPreDecrement => (
            format!("-(A{})", main_register),
            format!("MEM[--A{}]", main_register),
        ),
        Mode::IndirectOffset => {
            if offset == 0 {
                let imm = fetch_imm(program, i);
                (
                    format!("(A{}:#{})", main_register, imm as i32),
                    format!("MEM[A{} + {}]", main_register, imm as i32),
                )
            } else {
                let d = 4 + offset;
                (
                    format!("(A{}:D{})", main_register, d),
                    format!("MEM[A{} + D{}]", main_register, d),
                )
            }
        }
        Mode::IndirectDirect => {
            let imm = fetch_imm(program, i);
            (format!("(#{})", imm), format!("MEM[{}]", imm))
        }
    }
}

pub fn disassemble(program: &[u32]) -> Vec<String> {
    let mut lines = Vec::new();

    let mut i = 0usize;

    let operators: HashMap<u8, &'static str> = [
        (0x00, "HLT"),
        (0x01, "MOV"),
        (0x02, "CMP"),
        (0x10, "ADD"),
        (0x11, "ADC"),
        (0x12, "SUB"),
        (0x13, "MUL"),
        (0x14, "DIV"),
        (0x15, "REM"),
        (0x16, "AND"),
        (0x17, "OR"),
        (0x18, "XOR"),
        (0x19, "NOT"),
        (0x1A, "LSL"),
        (0x1B, "LSR"),
        (0x1C, "ASL"),
        (0x1D, "ASR"),
        (0x20, "JMP"),
        (0x21, "CALL"),
        (0x22, "RET"),
        (0x23, "INTRET"),
        (0x30, "BEQ"),
        (0x31, "BNE"),
        (0x32, "BGT"),
        (0x33, "BGE"),
        (0x34, "BLT"),
        (0x35, "BLE"),
        (0x36, "BCS"),
        (0x37, "BCC"),
        (0x38, "BVS"),
        (0x39, "BVC"),
        (0x40, "VADD"),
        (0x42, "VSUB"),
        (0x43, "VMUL"),
        (0x44, "VDIV"),
        (0x45, "VREM"),
        (0x46, "VAND"),
        (0x47, "VOR"),
        (0x48, "VXOR"),
        (0x50, "IN"),
        (0x51, "OUT"),
        (0x52, "EI"),
        (0x53, "DI"),
        (0x60, "VCMPBEQ"),
        (0x61, "VCMPBNE"),
        (0x62, "VCMPBGT"),
        (0x63, "VCMPBGE"),
        (0x64, "VCMPBLT"),
        (0x65, "VCMPBLE"),
        (0x66, "VCMPBCS"),
        (0x67, "VCMPBCC"),
        (0x68, "VCMPBVS"),
        (0x69, "VCMPBVC"),
    ]
    .iter()
    .cloned()
    .collect();

    while i < program.len() {
        let start_i = i;
        let ir = program[i];
        let operator_code = ((ir >> 25) & 0x7f) as u8;
        let is_8_bytes = ((ir >> 24) & 1) == 1;
        let word_size = if is_8_bytes { ".l" } else { ".b" };

        let operator = *operators.get(&operator_code).unwrap_or(&"???");
        let has_size = matches!(operator_code, 0x01..=0x02 | 0x10..=0x1D);

        let mut args_m = Vec::new();
        let mut args_d = Vec::new();

        match operator_code {
            0x00 | 0x22 | 0x23 | 0x52 | 0x53 => {}
            0x20 | 0x21 | 0x30..=0x39 => {
                let high = (ir & 0x00ffffff) as u64;
                i += 1;
                let low = if i < program.len() {
                    program[i] as u64
                } else {
                    0
                };
                let address = (high << 32) | low;

                let label = format!("0x{:X}", address);
                args_m.push(label.clone());
                args_d.push(label);
            }
            0x01 | 0x02 | 0x19 => {
                let b1 = ((ir >> 16) & 0xff) as u8;
                let b2 = ((ir >> 8) & 0xff) as u8;
                let (m1, d1) = parse_operand(b1, program, &mut i);
                let (m2, d2) = parse_operand(b2, program, &mut i);
                args_m.push(m1);
                args_m.push(m2);
                args_d.push(d1);
                args_d.push(d2);
            }
            0x50 | 0x51 => {
                let port_byte = ((ir >> 16) & 0xff) as u8;
                let operand_byte = ((ir >> 8) & 0xff) as u8;

                // Порт передаётся напрямую, а не как режим операнда
                let port_m = format!("#{}", port_byte);
                let port_d = format!("{}", port_byte);

                // А вот данные/источник парсим обычным способом
                let (op_m, op_d) = parse_operand(operand_byte, program, &mut i);

                args_m.push(port_m);
                args_m.push(op_m);

                args_d.push(port_d);
                args_d.push(op_d);
            }
            0x10..=0x18 | 0x1A..=0x1D | 0x40..=0x48 | 0x60..=0x69 => {
                let b1 = ((ir >> 16) & 0xff) as u8;
                let b2 = ((ir >> 8) & 0xff) as u8;
                let b3 = (ir & 0xff) as u8;
                let (m1, d1) = parse_operand(b1, program, &mut i);
                let (m2, d2) = parse_operand(b2, program, &mut i);
                let (m3, d3) = parse_operand(b3, program, &mut i);
                args_m.push(m1);
                args_m.push(m2);
                args_m.push(m3);
                args_d.push(d1);
                args_d.push(d2);
                args_d.push(d3);
            }
            _ => {}
        }

        let mnem_str = if args_m.is_empty() {
            operator.to_string()
        } else {
            let base = if has_size {
                format!("{}{}", operator, word_size)
            } else {
                operator.to_string()
            };
            format!("{} {}", base, args_m.join(", "))
        };

        let desc_str = match operator_code {
            0x00 => "Остановка выполнения процессора".to_string(),
            0x01 => format!("{} <- {}", args_d[1], args_d[0]),
            0x02 => format!("NZCV <- {} - {}", args_d[0], args_d[1]),
            0x10 => format!("{} <- {} + {}", args_d[2], args_d[0], args_d[1]),
            0x11 => format!("{} <- {} + {} + C", args_d[2], args_d[0], args_d[1]),
            0x12 => format!("{} <- {} - {}", args_d[2], args_d[0], args_d[1]),
            0x13 => format!("{} <- {} * {}", args_d[2], args_d[0], args_d[1]),
            0x14 => format!("{} <- {} / {}", args_d[2], args_d[0], args_d[1]),
            0x15 => format!("{} <- {} % {}", args_d[2], args_d[0], args_d[1]),
            0x16 => format!("{} <- {} & {}", args_d[2], args_d[0], args_d[1]),
            0x17 => format!("{} <- {} | {}", args_d[2], args_d[0], args_d[1]),
            0x18 => format!("{} <- {} ^ {}", args_d[2], args_d[0], args_d[1]),
            0x19 => format!("{} <- !{}", args_d[1], args_d[0]),
            0x1A | 0x1C => format!("{} <- {} << {}", args_d[2], args_d[0], args_d[1]),
            0x1B | 0x1D => format!("{} <- {} >> {}", args_d[2], args_d[0], args_d[1]),
            0x20 => format!("PC <- {}", args_d[0]),
            0x21 => format!("MEM[--A7] <- PC + 1; PC <- {}", args_d[0]),
            0x22 => "PC <- MEM[A7++]".to_string(),
            0x23 => "PC <- MEM[A7++]; NZCV <- MEM[A7++]; INT <- 0".to_string(),
            0x30 => format!("if Z == 1 then PC <- {}", args_d[0]),
            0x31 => format!("if Z == 0 then PC <- {}", args_d[0]),
            0x32 => format!("if Z == 0 and N == O then PC <- {}", args_d[0]),
            0x33 => format!("if N == 0 then PC <- {}", args_d[0]),
            0x34 => format!("if N == 1 then PC <- {}", args_d[0]),
            0x35 => format!("if Z == 1 or N == 0 then PC <- {}", args_d[0]),
            0x36 => format!("if C == 1 then PC <- {}", args_d[0]),
            0x37 => format!("if C == 0 then PC <- {}", args_d[0]),
            0x38 => format!("if V == 1 then PC <- {}", args_d[0]),
            0x39 => format!("if V == 0 then PC <- {}", args_d[0]),
            0x40 => format!(
                "for i in 0..4 do MEM[{} + i] <- MEM[{} + i] + MEM[{} + i]",
                args_d[2], args_d[0], args_d[1]
            ),
            0x42 => format!(
                "for i in 0..4 do MEM[{} + i] <- MEM[{} + i] - MEM[{} + i]",
                args_d[2], args_d[0], args_d[1]
            ),
            0x43 => format!(
                "for i in 0..4 do MEM[{} + i] <- MEM[{} + i] * MEM[{} + i]",
                args_d[2], args_d[0], args_d[1]
            ),
            0x44 => format!(
                "for i in 0..4 do MEM[{} + i] <- MEM[{} + i] / MEM[{} + i]",
                args_d[2], args_d[0], args_d[1]
            ),
            0x45 => format!(
                "for i in 0..4 do MEM[{} + i] <- MEM[{} + i] % MEM[{} + i]",
                args_d[2], args_d[0], args_d[1]
            ),
            0x46 => format!(
                "for i in 0..4 do MEM[{} + i] <- MEM[{} + i] & MEM[{} + i]",
                args_d[2], args_d[0], args_d[1]
            ),
            0x47 => format!(
                "for i in 0..4 do MEM[{} + i] <- MEM[{} + i] | MEM[{} + i]",
                args_d[2], args_d[0], args_d[1]
            ),
            0x48 => format!(
                "for i in 0..4 do MEM[{} + i] <- MEM[{} + i] ^ MEM[{} + i]",
                args_d[2], args_d[0], args_d[1]
            ),
            0x50 => format!("{} <- IO[{}]", args_d[1], args_d[0]),
            0x51 => format!("IO[{}] <- {}", args_d[0], args_d[1]),
            0x52 => "IF <- 1; Включает прерывания".to_string(),
            0x53 => "IF <- 0; Выключает прерывания".to_string(),
            0x60 => format!(
                "for i in 0..4 do MEM[{} + i] <- MEM[{} + i] == MEM[{} + i] ? TRUE_MASK : 0x0",
                args_d[2], args_d[0], args_d[1]
            ),
            0x61 => format!(
                "for i in 0..4 do MEM[{} + i] <- MEM[{} + i] != MEM[{} + i] ? TRUE_MASK : 0x0",
                args_d[2], args_d[0], args_d[1]
            ),
            0x62 => format!(
                "for i in 0..4 do MEM[{} + i] <- MEM[{} + i] > MEM[{} + i] ? TRUE_MASK : 0x0",
                args_d[2], args_d[0], args_d[1]
            ),
            0x63 => format!(
                "for i in 0..4 do MEM[{} + i] <- MEM[{} + i] >= MEM[{} + i] ? TRUE_MASK : 0x0",
                args_d[2], args_d[0], args_d[1]
            ),
            0x64 => format!(
                "for i in 0..4 do MEM[{} + i] <- MEM[{} + i] < MEM[{} + i] ? TRUE_MASK : 0x0",
                args_d[2], args_d[0], args_d[1]
            ),
            0x65 => format!(
                "for i in 0..4 do MEM[{} + i] <- MEM[{} + i] <= MEM[{} + i] ? TRUE_MASK : 0x0",
                args_d[2], args_d[0], args_d[1]
            ),
            0x66 => format!(
                "for i in 0..4 do MEM[{} + i] <- C == 1 ? TRUE_MASK : 0x0",
                args_d[2]
            ),
            0x67 => format!(
                "for i in 0..4 do MEM[{} + i] <- C == 0 ? TRUE_MASK : 0x0",
                args_d[2]
            ),
            0x68 => format!(
                "for i in 0..4 do MEM[{} + i] <- V == 1 ? TRUE_MASK : 0x0",
                args_d[2]
            ),
            0x69 => format!(
                "for i in 0..4 do MEM[{} + i] <- V == 0 ? TRUE_MASK : 0x0",
                args_d[2]
            ),
            _ => "Неизвестная инструкция".to_string(),
        };

        let mut hex_words = Vec::new();
        for j in start_i..=i {
            if j < program.len() {
                hex_words.push(format!("{:08X}", program[j]));
            }
        }
        let hex_str = hex_words.join(" ");

        lines.push(format!(
            "0x{:04X} | {:<35} | {:<35} | {}",
            start_i, hex_str, mnem_str, desc_str
        ));

        i += 1;
    }

    lines
}
