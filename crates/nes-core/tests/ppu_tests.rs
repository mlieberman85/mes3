use nes_core::mappers::Mirroring;
use nes_core::ppu::{Ppu, PpuCtrl, PpuStatus};

fn dummy_chr(_addr: u16) -> u8 {
    0
}

#[test]
fn test_vblank_sets_at_scanline_241() {
    let mut ppu = Ppu::new();

    // Advance to scanline 241, dot 1
    // Total dots to reach (241, 1) = 241 * 341 + 1 = 82_182
    let target_dots: u64 = 241 * 341 + 1;
    ppu.step(target_dots, Mirroring::Horizontal, &dummy_chr);

    assert!(ppu.status.contains(PpuStatus::VBLANK));
}

#[test]
fn test_vblank_clears_at_prerender() {
    let mut ppu = Ppu::new();

    // Set VBlank manually
    ppu.status.insert(PpuStatus::VBLANK);
    ppu.status.insert(PpuStatus::SPRITE_ZERO_HIT);

    // Advance to scanline 261, dot 1
    let target_dots: u64 = 261 * 341 + 1;
    ppu.step(target_dots, Mirroring::Horizontal, &dummy_chr);

    assert!(!ppu.status.contains(PpuStatus::VBLANK));
    assert!(!ppu.status.contains(PpuStatus::SPRITE_ZERO_HIT));
}

#[test]
fn test_nmi_triggers_when_enabled() {
    let mut ppu = Ppu::new();

    ppu.ctrl = PpuCtrl::NMI_ENABLE;

    // Advance past VBlank set point
    let target_dots: u64 = 241 * 341 + 2;
    ppu.step(target_dots, Mirroring::Horizontal, &dummy_chr);

    assert!(ppu.take_nmi());
}

#[test]
fn test_nmi_does_not_trigger_when_disabled() {
    let mut ppu = Ppu::new();

    ppu.ctrl = PpuCtrl::empty(); // NMI disabled

    let target_dots: u64 = 241 * 341 + 2;
    ppu.step(target_dots, Mirroring::Horizontal, &dummy_chr);

    assert!(!ppu.take_nmi());
}

#[test]
fn test_ppustatus_read_clears_vblank() {
    let mut ppu = Ppu::new();
    ppu.status.insert(PpuStatus::VBLANK);
    ppu.write_latch = true;

    let status = ppu.read_status();
    assert!(status & 0x80 != 0); // VBlank was set
    assert!(!ppu.status.contains(PpuStatus::VBLANK)); // Now cleared
    assert!(!ppu.write_latch); // Latch reset
}

#[test]
fn test_ppuscroll_double_write() {
    let mut ppu = Ppu::new();

    ppu.write_scroll(0b11111_101); // X: coarse=31, fine=5
    assert_eq!(ppu.fine_x, 5);
    assert_eq!(ppu.t & 0x001F, 31);

    ppu.write_scroll(0b11010_011); // Y: coarse=26, fine=3
    assert_eq!((ppu.t >> 12) & 0x07, 3); // fine Y
    assert_eq!((ppu.t >> 5) & 0x1F, 26); // coarse Y (bits 9-5)
}

#[test]
fn test_ppuaddr_double_write() {
    let mut ppu = Ppu::new();

    ppu.write_addr(0x21); // High byte
    ppu.write_addr(0x00); // Low byte

    assert_eq!(ppu.v, 0x2100);
}

#[test]
fn test_oam_data_write() {
    let mut ppu = Ppu::new();
    ppu.write_oam_addr(0x00);

    ppu.write_oam_data(0xAA);
    assert_eq!(ppu.oam[0], 0xAA);
    assert_eq!(ppu.oam_addr, 1); // Auto-increments

    ppu.write_oam_data(0xBB);
    assert_eq!(ppu.oam[1], 0xBB);
}

#[test]
fn test_palette_write_mirror() {
    let mut ppu = Ppu::new();

    // Writing to $3F10 should mirror to $3F00
    ppu.write_addr(0x3F);
    ppu.write_addr(0x10);
    ppu.write_data(0x30, &mut |_, _| {});

    assert_eq!(ppu.palette[0], 0x30);
}

#[test]
fn test_frame_counter_increments() {
    let mut ppu = Ppu::new();

    assert_eq!(ppu.frame_count, 0);

    // One full frame = 262 scanlines * 341 dots = 89342 PPU cycles
    ppu.step(89342, Mirroring::Horizontal, &dummy_chr);

    assert_eq!(ppu.frame_count, 1);
}
