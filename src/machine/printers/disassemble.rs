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
        _ => panic!("Unknown mode code: {}", code),
    }
}

fn operand_to_string(byte: u8) -> (String, bool) {
    let mode_code = (byte >> 5) & 0x7;
    let main_register = (byte >> 2) & 0x7;
    let offset = byte & 0x3;
    let mode = get_mode(mode_code);

    match mode {
        Mode::Direct => ("#".to_string(), true),
        Mode::DataRegister => (format!("D{}", main_register), false),
        Mode::AddressRegister => (format!("A{}", main_register), false),
        Mode::Indirect => (format!("(A{})", main_register), false),
        Mode::IndirectPostIncrement => (format!("(A{})+", main_register), false),
        Mode::IndirectPreDecrement => (format!("-(A{})", main_register), false),
        Mode::IndirectOffset => {
            if offset == 0 {
                (format!("(A{}:#)", main_register), true)
            } else {
                (format!("(A{}:D{})", main_register, 4 + offset), false)
            }
        }
        Mode::IndirectDirect => ("(#)".to_string(), true),
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
        (0x49, "VEND"),
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
        let ir = program[i];
        let operator_code = ((ir >> 25) & 0x7f) as u8;
        let word_size = ((ir >> 24) & 1) as u8;
        let word_size = if word_size == 0 { ".b" } else { ".w" };

        let operator = *operators.get(&operator_code).unwrap_or(&"???");

        let mut line = format!("0x{:04X}: 0x{:08X}   {}{} ", i, ir, operator, word_size);

        match operator_code {
            0x00 => {}
            0x20 | 0x21 | 0x30..=0x39 => {
                if i + 1 < program.len() {
                    let high = ir & 0x00ffffff;
                    let low = program[i + 1];
                    let address = ((high as u64) << 32) | (low as u64);
                    line += &format!("0x{:016X}", address);
                    i += 1;
                }
            }
            0x22 | 0x23 | 0x52 | 0x53 | 0x49 => {}
            0x50 | 0x51 => {
                let port = ((ir >> 16) & 0xff) as u8;
                let operand_byte = ((ir >> 8) & 0xff) as u8;
                let (mut operand, needs_extra) = operand_to_string(operand_byte);

                inline_immediate(program, &mut i, &mut operand, needs_extra);

                line += &format!("#{}, {}", port, operand);
            }
            _ => {
                let byte1 = ((ir >> 16) & 0xff) as u8;
                let byte2 = ((ir >> 8) & 0xff) as u8;

                let (mut source, source_needs) = operand_to_string(byte1);
                let (mut destination, destination_needs) = operand_to_string(byte2);

                inline_immediate(program, &mut i, &mut source, source_needs);
                inline_immediate(program, &mut i, &mut destination, destination_needs);

                line += &format!("{}, {}", destination, source);
            }
        }

        lines.push(line);
        i += 1;
    }

    lines
}

fn inline_immediate(program: &[u32], i: &mut usize, source: &mut String, needs: bool) {
    if needs && *i + 1 < program.len() {
        let immediate = program[*i + 1];
        *source = source.replace("#", &format!("#0x{:08X}", immediate));
        *source = source.replace("(#)", &format!("(#0x{:08X})", immediate));
        *i += 1;
    }
}
