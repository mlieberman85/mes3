/// nestest.nes ROM validation test.
/// This test loads the nestest ROM, runs it in automation mode (PC=$C000),
/// and compares the execution log against a reference.
///
/// The nestest.nes ROM and its reference log must be placed in the
/// test-roms/ directory at the repository root.
///
/// Full implementation in T044.
#[test]
fn test_nestest_rom() {
    let rom_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-roms/nestest.nes");

    if !rom_path.exists() {
        eprintln!("nestest.nes not found at {:?}, skipping", rom_path);
        return;
    }

    let rom_data = std::fs::read(&rom_path).expect("Failed to read nestest.nes");
    let mut nes = nes_core::Nes::new();
    nes.load_rom(&rom_data).expect("Failed to load nestest.nes");

    // Set PC to $C000 for automation mode
    nes.cpu.pc = 0xC000;

    // Run until we hit a loop or reach a known end point
    let max_instructions = 10000;
    for _ in 0..max_instructions {
        nes.cpu.step(&mut nes.bus);
    }

    // Check result codes at $0002 and $0003
    let result_official = nes.bus.ram[0x0002];
    let result_unofficial = nes.bus.ram[0x0003];

    assert_eq!(
        result_official, 0x00,
        "Official opcodes failed with code 0x{:02X}",
        result_official
    );
    assert_eq!(
        result_unofficial, 0x00,
        "Unofficial opcodes failed with code 0x{:02X}",
        result_unofficial
    );
}
