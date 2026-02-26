use nes_core::bus::Bus;
use nes_core::cpu::{Cpu, CpuFlags};

/// Helper: set up a CPU + Bus, write code at the given address, set PC there.
fn setup_cpu(code: &[u8], start: u16) -> (Cpu, Bus) {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.pc = start;
    for (i, &byte) in code.iter().enumerate() {
        bus.ram[(start as usize + i) & 0x07FF] = byte;
    }
    (cpu, bus)
}

/// Helper: step the CPU once and return cycles consumed.
fn step(cpu: &mut Cpu, bus: &mut Bus) -> u8 {
    let before = cpu.cycles;
    let cycles = cpu.step(bus);
    assert_eq!(cycles, (cpu.cycles - before) as u8);
    cycles
}

// ---- Loads ----

#[test]
fn test_lda_immediate() {
    let (mut cpu, mut bus) = setup_cpu(&[0xA9, 0x42], 0x0000);
    let cycles = step(&mut cpu, &mut bus);
    assert_eq!(cpu.a, 0x42);
    assert_eq!(cycles, 2);
    assert!(!cpu.status.contains(CpuFlags::ZERO));
    assert!(!cpu.status.contains(CpuFlags::NEGATIVE));
}

#[test]
fn test_lda_zero_flag() {
    let (mut cpu, mut bus) = setup_cpu(&[0xA9, 0x00], 0x0000);
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.a, 0x00);
    assert!(cpu.status.contains(CpuFlags::ZERO));
}

#[test]
fn test_lda_negative_flag() {
    let (mut cpu, mut bus) = setup_cpu(&[0xA9, 0x80], 0x0000);
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.a, 0x80);
    assert!(cpu.status.contains(CpuFlags::NEGATIVE));
}

#[test]
fn test_ldx_immediate() {
    let (mut cpu, mut bus) = setup_cpu(&[0xA2, 0x55], 0x0000);
    let cycles = step(&mut cpu, &mut bus);
    assert_eq!(cpu.x, 0x55);
    assert_eq!(cycles, 2);
}

#[test]
fn test_ldy_immediate() {
    let (mut cpu, mut bus) = setup_cpu(&[0xA0, 0x77], 0x0000);
    let cycles = step(&mut cpu, &mut bus);
    assert_eq!(cpu.y, 0x77);
    assert_eq!(cycles, 2);
}

#[test]
fn test_lda_zero_page() {
    let (mut cpu, mut bus) = setup_cpu(&[0xA5, 0x10], 0x0000);
    bus.ram[0x10] = 0xAB;
    let cycles = step(&mut cpu, &mut bus);
    assert_eq!(cpu.a, 0xAB);
    assert_eq!(cycles, 3);
}

#[test]
fn test_lda_absolute() {
    let (mut cpu, mut bus) = setup_cpu(&[0xAD, 0x50, 0x01], 0x0000);
    bus.ram[0x150] = 0xCD;
    let cycles = step(&mut cpu, &mut bus);
    assert_eq!(cpu.a, 0xCD);
    assert_eq!(cycles, 4);
}

// ---- Stores ----

#[test]
fn test_sta_zero_page() {
    let (mut cpu, mut bus) = setup_cpu(&[0x85, 0x20], 0x0000);
    cpu.a = 0x42;
    let cycles = step(&mut cpu, &mut bus);
    assert_eq!(bus.ram[0x20], 0x42);
    assert_eq!(cycles, 3);
}

#[test]
fn test_stx_zero_page() {
    let (mut cpu, mut bus) = setup_cpu(&[0x86, 0x30], 0x0000);
    cpu.x = 0x99;
    step(&mut cpu, &mut bus);
    assert_eq!(bus.ram[0x30], 0x99);
}

#[test]
fn test_sty_zero_page() {
    let (mut cpu, mut bus) = setup_cpu(&[0x84, 0x40], 0x0000);
    cpu.y = 0xBB;
    step(&mut cpu, &mut bus);
    assert_eq!(bus.ram[0x40], 0xBB);
}

// ---- Arithmetic ----

#[test]
fn test_adc_no_carry() {
    let (mut cpu, mut bus) = setup_cpu(&[0x69, 0x10], 0x0000);
    cpu.a = 0x20;
    cpu.status.remove(CpuFlags::CARRY);
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.a, 0x30);
    assert!(!cpu.status.contains(CpuFlags::CARRY));
    assert!(!cpu.status.contains(CpuFlags::OVERFLOW));
}

#[test]
fn test_adc_with_carry_out() {
    let (mut cpu, mut bus) = setup_cpu(&[0x69, 0x01], 0x0000);
    cpu.a = 0xFF;
    cpu.status.remove(CpuFlags::CARRY);
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.a, 0x00);
    assert!(cpu.status.contains(CpuFlags::CARRY));
    assert!(cpu.status.contains(CpuFlags::ZERO));
}

#[test]
fn test_adc_overflow() {
    // 0x50 + 0x50 = 0xA0 — two positives giving a negative = overflow
    let (mut cpu, mut bus) = setup_cpu(&[0x69, 0x50], 0x0000);
    cpu.a = 0x50;
    cpu.status.remove(CpuFlags::CARRY);
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.a, 0xA0);
    assert!(cpu.status.contains(CpuFlags::OVERFLOW));
}

#[test]
fn test_sbc_basic() {
    let (mut cpu, mut bus) = setup_cpu(&[0xE9, 0x10], 0x0000);
    cpu.a = 0x50;
    cpu.status.insert(CpuFlags::CARRY); // No borrow
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.a, 0x40);
    assert!(cpu.status.contains(CpuFlags::CARRY)); // No borrow
}

// ---- Branches ----

#[test]
fn test_beq_taken() {
    let (mut cpu, mut bus) = setup_cpu(&[0xF0, 0x05], 0x0000);
    cpu.status.insert(CpuFlags::ZERO);
    let cycles = step(&mut cpu, &mut bus);
    assert_eq!(cpu.pc, 0x07); // PC+2+5
    assert_eq!(cycles, 3); // Branch taken, no page cross
}

#[test]
fn test_beq_not_taken() {
    let (mut cpu, mut bus) = setup_cpu(&[0xF0, 0x05], 0x0000);
    cpu.status.remove(CpuFlags::ZERO);
    let cycles = step(&mut cpu, &mut bus);
    assert_eq!(cpu.pc, 0x02);
    assert_eq!(cycles, 2);
}

#[test]
fn test_bne_taken() {
    let (mut cpu, mut bus) = setup_cpu(&[0xD0, 0x03], 0x0000);
    cpu.status.remove(CpuFlags::ZERO);
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.pc, 0x05);
}

// ---- Stack ----

#[test]
fn test_pha_pla() {
    // PHA then PLA
    let (mut cpu, mut bus) = setup_cpu(&[0x48, 0x68], 0x0000);
    cpu.a = 0x42;
    step(&mut cpu, &mut bus); // PHA
    cpu.a = 0x00;
    step(&mut cpu, &mut bus); // PLA
    assert_eq!(cpu.a, 0x42);
}

#[test]
fn test_php_plp() {
    // PHP then PLP
    let (mut cpu, mut bus) = setup_cpu(&[0x08, 0x28], 0x0000);
    cpu.status = CpuFlags::CARRY | CpuFlags::ZERO | CpuFlags::UNUSED;
    step(&mut cpu, &mut bus); // PHP
    cpu.status = CpuFlags::UNUSED;
    step(&mut cpu, &mut bus); // PLP
    assert!(cpu.status.contains(CpuFlags::CARRY));
    assert!(cpu.status.contains(CpuFlags::ZERO));
}

// ---- Transfers ----

#[test]
fn test_tax() {
    let (mut cpu, mut bus) = setup_cpu(&[0xAA], 0x0000);
    cpu.a = 0x55;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.x, 0x55);
}

#[test]
fn test_tay() {
    let (mut cpu, mut bus) = setup_cpu(&[0xA8], 0x0000);
    cpu.a = 0x77;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.y, 0x77);
}

// ---- Shifts ----

#[test]
fn test_asl_accumulator() {
    let (mut cpu, mut bus) = setup_cpu(&[0x0A], 0x0000);
    cpu.a = 0x81;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.a, 0x02);
    assert!(cpu.status.contains(CpuFlags::CARRY));
}

#[test]
fn test_lsr_accumulator() {
    let (mut cpu, mut bus) = setup_cpu(&[0x4A], 0x0000);
    cpu.a = 0x03;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.a, 0x01);
    assert!(cpu.status.contains(CpuFlags::CARRY));
}

// ---- Compare ----

#[test]
fn test_cmp_equal() {
    let (mut cpu, mut bus) = setup_cpu(&[0xC9, 0x42], 0x0000);
    cpu.a = 0x42;
    step(&mut cpu, &mut bus);
    assert!(cpu.status.contains(CpuFlags::ZERO));
    assert!(cpu.status.contains(CpuFlags::CARRY));
}

#[test]
fn test_cmp_less() {
    let (mut cpu, mut bus) = setup_cpu(&[0xC9, 0x50], 0x0000);
    cpu.a = 0x10;
    step(&mut cpu, &mut bus);
    assert!(!cpu.status.contains(CpuFlags::ZERO));
    assert!(!cpu.status.contains(CpuFlags::CARRY));
    assert!(cpu.status.contains(CpuFlags::NEGATIVE));
}

// ---- Increments/Decrements ----

#[test]
fn test_inx() {
    let (mut cpu, mut bus) = setup_cpu(&[0xE8], 0x0000);
    cpu.x = 0x05;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.x, 0x06);
}

#[test]
fn test_dex() {
    let (mut cpu, mut bus) = setup_cpu(&[0xCA], 0x0000);
    cpu.x = 0x05;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.x, 0x04);
}

#[test]
fn test_inc_zero_page() {
    let (mut cpu, mut bus) = setup_cpu(&[0xE6, 0x10], 0x0000);
    bus.ram[0x10] = 0xFF;
    step(&mut cpu, &mut bus);
    assert_eq!(bus.ram[0x10], 0x00);
    assert!(cpu.status.contains(CpuFlags::ZERO));
}

// ---- Logic ----

#[test]
fn test_and_immediate() {
    let (mut cpu, mut bus) = setup_cpu(&[0x29, 0x0F], 0x0000);
    cpu.a = 0xFF;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.a, 0x0F);
}

#[test]
fn test_ora_immediate() {
    let (mut cpu, mut bus) = setup_cpu(&[0x09, 0xF0], 0x0000);
    cpu.a = 0x0F;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.a, 0xFF);
}

#[test]
fn test_eor_immediate() {
    let (mut cpu, mut bus) = setup_cpu(&[0x49, 0xFF], 0x0000);
    cpu.a = 0xAA;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.a, 0x55);
}

// ---- JMP / JSR / RTS ----

#[test]
fn test_jmp_absolute() {
    let (mut cpu, mut bus) = setup_cpu(&[0x4C, 0x00, 0x02], 0x0000);
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.pc, 0x0200);
}

#[test]
fn test_jsr_rts() {
    // JSR $0010 at $0000, RTS at $0010
    let (mut cpu, mut bus) = setup_cpu(&[0x20, 0x10, 0x00], 0x0000);
    bus.ram[0x10] = 0x60; // RTS
    step(&mut cpu, &mut bus); // JSR
    assert_eq!(cpu.pc, 0x0010);
    step(&mut cpu, &mut bus); // RTS
    assert_eq!(cpu.pc, 0x0003);
}

// ---- Flags ----

#[test]
fn test_sec_clc() {
    let (mut cpu, mut bus) = setup_cpu(&[0x38, 0x18], 0x0000);
    step(&mut cpu, &mut bus); // SEC
    assert!(cpu.status.contains(CpuFlags::CARRY));
    step(&mut cpu, &mut bus); // CLC
    assert!(!cpu.status.contains(CpuFlags::CARRY));
}

#[test]
fn test_sei_cli() {
    let (mut cpu, mut bus) = setup_cpu(&[0x78, 0x58], 0x0000);
    cpu.status.remove(CpuFlags::IRQ_DISABLE);
    step(&mut cpu, &mut bus); // SEI
    assert!(cpu.status.contains(CpuFlags::IRQ_DISABLE));
    step(&mut cpu, &mut bus); // CLI
    assert!(!cpu.status.contains(CpuFlags::IRQ_DISABLE));
}

// ---- Undocumented Opcodes ----

#[test]
fn test_lax_zero_page() {
    // LAX $10 (opcode 0xA7)
    let (mut cpu, mut bus) = setup_cpu(&[0xA7, 0x10], 0x0000);
    bus.ram[0x10] = 0x42;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.a, 0x42);
    assert_eq!(cpu.x, 0x42);
}

#[test]
fn test_sax_zero_page() {
    // SAX $20 (opcode 0x87)
    let (mut cpu, mut bus) = setup_cpu(&[0x87, 0x20], 0x0000);
    cpu.a = 0xFF;
    cpu.x = 0x0F;
    step(&mut cpu, &mut bus);
    assert_eq!(bus.ram[0x20], 0x0F); // A & X
}

#[test]
fn test_dcp_zero_page() {
    // DCP $10 (opcode 0xC7) — decrement memory, then compare
    let (mut cpu, mut bus) = setup_cpu(&[0xC7, 0x10], 0x0000);
    bus.ram[0x10] = 0x42;
    cpu.a = 0x41;
    step(&mut cpu, &mut bus);
    assert_eq!(bus.ram[0x10], 0x41);
    assert!(cpu.status.contains(CpuFlags::ZERO));
    assert!(cpu.status.contains(CpuFlags::CARRY));
}

#[test]
fn test_isb_zero_page() {
    // ISB $10 (opcode 0xE7) — increment memory, then SBC
    let (mut cpu, mut bus) = setup_cpu(&[0xE7, 0x10], 0x0000);
    bus.ram[0x10] = 0x09;
    cpu.a = 0x20;
    cpu.status.insert(CpuFlags::CARRY);
    step(&mut cpu, &mut bus);
    assert_eq!(bus.ram[0x10], 0x0A);
    assert_eq!(cpu.a, 0x16); // 0x20 - 0x0A = 0x16
}

#[test]
fn test_nop_undocumented() {
    // Many undocumented NOPs exist (e.g., 0x1A, 0x3A, 0x5A, 0x7A, 0xDA, 0xFA)
    let (mut cpu, mut bus) = setup_cpu(&[0x1A], 0x0000);
    let a_before = cpu.a;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.a, a_before);
    assert_eq!(cpu.pc, 0x0001);
}

// ---- Page Crossing ----

#[test]
fn test_lda_absolute_x_page_cross() {
    // LDA $01F0,X with X=0x20 → crosses page → 5 cycles
    let (mut cpu, mut bus) = setup_cpu(&[0xBD, 0xF0, 0x01], 0x0000);
    cpu.x = 0x20;
    bus.ram[0x0210 & 0x07FF] = 0xAA;
    let cycles = step(&mut cpu, &mut bus);
    assert_eq!(cpu.a, 0xAA);
    assert_eq!(cycles, 5); // 4 + 1 for page cross
}

#[test]
fn test_lda_absolute_x_no_page_cross() {
    // LDA $0100,X with X=0x01 → no page cross → 4 cycles
    let (mut cpu, mut bus) = setup_cpu(&[0xBD, 0x00, 0x01], 0x0000);
    cpu.x = 0x01;
    bus.ram[0x0101 & 0x07FF] = 0xBB;
    let cycles = step(&mut cpu, &mut bus);
    assert_eq!(cpu.a, 0xBB);
    assert_eq!(cycles, 4);
}
