use proc_translator::machine::alu::{ALU, AluOp};
#[test]
fn test_add() {
    let mut alu = ALU::default();
    let result = alu.execute_op(AluOp::ADD(10, 20));
    assert_eq!(result, 30);
    assert!(!alu.nzcv.negative);
    assert!(!alu.nzcv.zero);
    assert!(!alu.nzcv.carry);
    assert!(!alu.nzcv.overflow);
}

#[test]
fn test_add_min_value() {
    let mut alu = ALU::default();
    let min_value = 1u64 << 63;
    let result = alu.execute_op(AluOp::ADD(min_value, min_value));
    assert_eq!(result, 0);
    assert!(!alu.nzcv.negative);
    assert!(alu.nzcv.zero);
    assert!(alu.nzcv.carry);
    assert!(alu.nzcv.overflow);
}

#[test]
fn test_add_carry() {
    let mut alu = ALU::default();
    let result = alu.execute_op(AluOp::ADD(u64::MAX, 1));
    assert_eq!(result, 0);
    assert!(!alu.nzcv.negative);
    assert!(alu.nzcv.zero);
    assert!(alu.nzcv.carry);
    assert!(!alu.nzcv.overflow);
}

#[test]
fn test_add_overflow() {
    let mut alu = ALU::default();
    let a = i64::MAX as u64;
    let result = alu.execute_op(AluOp::ADD(a, 1));
    assert_eq!(result >> 63, 1);
    assert!(alu.nzcv.negative);
    assert!(!alu.nzcv.zero);
    assert!(alu.nzcv.overflow);
    assert!(!alu.nzcv.carry);
}

#[test]
fn test_sub() {
    let mut alu = ALU::default();
    let result = alu.execute_op(AluOp::SUB(20, 10));
    assert_eq!(result, 10);
    assert!(!alu.nzcv.negative);
    assert!(!alu.nzcv.zero);
    assert!(!alu.nzcv.carry);
    assert!(!alu.nzcv.overflow);
}

#[test]
fn test_sub_carry() {
    let mut alu = ALU::default();
    let result = alu.execute_op(AluOp::SUB(0, 1));
    assert_eq!(result, u64::MAX);
    assert!(alu.nzcv.negative);
    assert!(!alu.nzcv.zero);
    assert!(alu.nzcv.carry);
    assert!(!alu.nzcv.overflow);
}

#[test]
fn test_sub_overflow_to_negative() {
    let mut alu = ALU::default();
    let min_val = 1u64 << 63;
    let result = alu.execute_op(AluOp::SUB(min_val, 1));
    assert_eq!(result, i64::MAX as u64);
    assert!(!alu.nzcv.negative);
    assert!(alu.nzcv.overflow);
    assert!(!alu.nzcv.carry);
}

#[test]
fn test_sub_zero() {
    let mut alu = ALU::default();
    let result = alu.execute_op(AluOp::SUB(0, 0));
    assert_eq!(result, 0);
    assert!(!alu.nzcv.negative);
    assert!(alu.nzcv.zero);
    assert!(!alu.nzcv.carry);
    assert!(!alu.nzcv.overflow);
}

#[test]
fn test_logical_ops() {
    let mut alu = ALU::default();

    let result = alu.execute_op(AluOp::AND(0b1010, 0b1100));
    assert_eq!(result, 0b1000);

    let result = alu.execute_op(AluOp::XOR(0xFF, 0xFF));
    assert_eq!(result, 0);

    let result = alu.execute_op(AluOp::NOT(0));
    assert_eq!(result, u64::MAX);
}

#[test]
fn test_lsl_carry() {
    let mut alu = ALU::default();
    let value = 1u64 << 63;
    let result = alu.execute_op(AluOp::LSL(value, 1));
    assert_eq!(result, 0);
    assert!(alu.nzcv.zero);
    assert!(alu.nzcv.carry);
}

#[test]
fn test_lsl() {
    let mut alu = ALU::default();
    let result = alu.execute_op(AluOp::LSL(1, 63));
    assert_eq!(result, 1u64 << 63);
    assert!(alu.nzcv.negative);
    assert!(!alu.nzcv.carry);

    let result = alu.execute_op(AluOp::LSL(0x1234, 64));
    assert_eq!(result, 0x1234);
}

#[test]
fn test_lsr_carry() {
    let mut alu = ALU::default();
    let result = alu.execute_op(AluOp::LSR(1, 1));
    assert_eq!(result, 0);
    assert!(alu.nzcv.zero);
    assert!(alu.nzcv.carry);
}

#[test]
fn test_lsr() {
    let mut alu = ALU::default();
    let result = alu.execute_op(AluOp::LSR(1u64 << 63, 63));
    assert_eq!(result, 1);
    assert!(!alu.nzcv.negative);
    assert!(!alu.nzcv.carry);
}

#[test]
fn test_asr_sign_extension() {
    let mut alu = ALU::default();
    let value = 1u64 << 63;
    let result = alu.execute_op(AluOp::ASR(value, 2));
    assert_eq!(result, 0xE000000000000000);
    assert!(alu.nzcv.negative);
}

#[test]
fn test_asr_positive() {
    let mut alu = ALU::default();
    let result = alu.execute_op(AluOp::ASR(0x4000000000000000, 1));
    assert_eq!(result, 0x2000000000000000);
    assert!(!alu.nzcv.negative);
}

#[test]
fn test_div_by_zero() {
    let mut alu = ALU::default();
    let _ = alu.execute_op(AluOp::DIV(10, 0));
    assert!(alu.nzcv.carry);
}

#[test]
fn test_div_overflow() {
    let mut alu = ALU::default();
    let min_value = 1u64 << 63;
    let _ = alu.execute_op(AluOp::DIV(min_value, u64::MAX));
    assert!(alu.nzcv.overflow);
}

#[test]
fn test_mul() {
    let mut alu = ALU::default();
    let result = alu.execute_op(AluOp::MUL(10, 5));
    assert_eq!(result, 50);
    assert!(!alu.nzcv.negative);
    assert!(!alu.nzcv.zero);
    assert!(!alu.nzcv.carry);
    assert!(!alu.nzcv.overflow);
}

#[test]
fn test_mul_carry() {
    let mut alu = ALU::default();
    let _ = alu.execute_op(AluOp::MUL(u64::MAX, 2));
    assert!(alu.nzcv.negative);
    assert!(!alu.nzcv.zero);
    assert!(alu.nzcv.carry);
    assert!(!alu.nzcv.overflow);
}

#[test]
fn test_mul_negative_result() {
    let mut alu = ALU::default();
    let result = alu.execute_op(AluOp::MUL(5, u64::MAX));
    assert_eq!(result as i64, -5);
    assert!(alu.nzcv.negative);
    assert!(!alu.nzcv.zero);
    assert!(alu.nzcv.carry);
    assert!(!alu.nzcv.overflow);
}

#[test]
fn test_rem() {
    let mut alu = ALU::default();
    let result = alu.execute_op(AluOp::REM(10, 3));
    assert_eq!(result, 1);
    assert!(!alu.nzcv.negative);
    assert!(!alu.nzcv.zero);
    assert!(!alu.nzcv.carry);
    assert!(!alu.nzcv.overflow);
}

#[test]
fn test_rem_by_zero() {
    let mut alu = ALU::default();
    let _ = alu.execute_op(AluOp::REM(10, 0));
    assert!(alu.nzcv.carry);
}
