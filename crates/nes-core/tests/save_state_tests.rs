use nes_core::cpu::CpuFlags;
use nes_core::Nes;

/// Helper to create a Nes with some known state (no ROM loaded).
fn make_nes_with_state() -> Nes {
    let mut nes = Nes::new();
    // Set CPU registers to recognizable values
    nes.cpu.a = 0x42;
    nes.cpu.x = 0x13;
    nes.cpu.y = 0x37;
    nes.cpu.sp = 0xFD;
    nes.cpu.pc = 0xC000;
    nes.cpu.status = CpuFlags::CARRY | CpuFlags::ZERO | CpuFlags::UNUSED;
    nes.cpu.cycles = 12345;

    // Write recognizable data to RAM
    nes.bus.ram[0] = 0xAA;
    nes.bus.ram[1] = 0xBB;
    nes.bus.ram[2047] = 0xCC;

    // Write to PPU state
    nes.bus.ppu.v = 0x2100;
    nes.bus.ppu.t = 0x1234;
    nes.bus.ppu.fine_x = 5;
    nes.bus.ppu.write_latch = true;
    nes.bus.ppu.oam[0] = 0x55;
    nes.bus.ppu.oam[255] = 0x66;
    nes.bus.ppu.palette[0] = 0x30;
    nes.bus.ppu.palette[1] = 0x16;
    nes.bus.ppu.vram[0] = 0x77;
    nes.bus.ppu.vram[2047] = 0x88;

    nes
}

#[test]
fn test_cpu_save_load_roundtrip() {
    let nes = make_nes_with_state();

    let state = nes.save_state();
    assert!(!state.is_empty());

    let mut nes2 = Nes::new();
    assert!(nes2.load_state(&state));

    assert_eq!(nes2.cpu.a, 0x42);
    assert_eq!(nes2.cpu.x, 0x13);
    assert_eq!(nes2.cpu.y, 0x37);
    assert_eq!(nes2.cpu.sp, 0xFD);
    assert_eq!(nes2.cpu.pc, 0xC000);
    assert!(nes2.cpu.status.contains(CpuFlags::CARRY));
    assert!(nes2.cpu.status.contains(CpuFlags::ZERO));
    assert_eq!(nes2.cpu.cycles, 12345);
}

#[test]
fn test_ram_save_load_roundtrip() {
    let nes = make_nes_with_state();

    let state = nes.save_state();
    let mut nes2 = Nes::new();
    assert!(nes2.load_state(&state));

    assert_eq!(nes2.bus.ram[0], 0xAA);
    assert_eq!(nes2.bus.ram[1], 0xBB);
    assert_eq!(nes2.bus.ram[2047], 0xCC);
}

#[test]
fn test_ppu_save_load_roundtrip() {
    let nes = make_nes_with_state();

    let state = nes.save_state();
    let mut nes2 = Nes::new();
    assert!(nes2.load_state(&state));

    assert_eq!(nes2.bus.ppu.v, 0x2100);
    assert_eq!(nes2.bus.ppu.t, 0x1234);
    assert_eq!(nes2.bus.ppu.fine_x, 5);
    assert!(nes2.bus.ppu.write_latch);
    assert_eq!(nes2.bus.ppu.oam[0], 0x55);
    assert_eq!(nes2.bus.ppu.oam[255], 0x66);
    assert_eq!(nes2.bus.ppu.palette[0], 0x30);
    assert_eq!(nes2.bus.ppu.palette[1], 0x16);
    assert_eq!(nes2.bus.ppu.vram[0], 0x77);
    assert_eq!(nes2.bus.ppu.vram[2047], 0x88);
}

#[test]
fn test_save_load_produces_identical_state() {
    let nes = make_nes_with_state();

    // Save twice, should be identical
    let state1 = nes.save_state();
    let state2 = nes.save_state();
    assert_eq!(state1, state2);

    // Load and save again
    let mut nes2 = Nes::new();
    assert!(nes2.load_state(&state1));
    let state3 = nes2.save_state();
    assert_eq!(state1, state3);
}

#[test]
fn test_load_state_rejects_truncated_data() {
    let nes = make_nes_with_state();
    let state = nes.save_state();

    // Truncate the data
    let truncated = &state[..state.len() / 2];
    let mut nes2 = Nes::new();
    assert!(!nes2.load_state(truncated));
}

#[test]
fn test_load_state_rejects_empty_data() {
    let mut nes = Nes::new();
    assert!(!nes.load_state(&[]));
}
