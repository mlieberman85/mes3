use alloc::vec::Vec;

use super::{Mapper, Mirroring};
use crate::cartridge::INesHeader;

/// GxROM (Mapper 66) — 32KB PRG + 8KB CHR bank switching.
///
/// Register ($8000-$FFFF) write:
///   bits 5-4: PRG bank select (32KB)
///   bits 1-0: CHR bank select (8KB)
pub struct GxRom {
    prg_bank: u8,
    chr_bank: u8,
    prg_mask: u8,
    chr_mask: u8,
    mirroring: Mirroring,
}

impl GxRom {
    pub fn new(header: &INesHeader) -> Self {
        let prg_banks = header.prg_rom_banks / 2; // 32KB banks
        let chr_banks = header.chr_rom_banks; // 8KB banks
        Self {
            prg_bank: 0,
            chr_bank: 0,
            prg_mask: prg_banks.saturating_sub(1),
            chr_mask: chr_banks.saturating_sub(1),
            mirroring: header.mirroring,
        }
    }
}

impl Mapper for GxRom {
    fn read_prg(&self, prg_rom: &[u8], addr: u16) -> u8 {
        let bank = (self.prg_bank & self.prg_mask) as usize;
        let offset = bank * 0x8000 + (addr - 0x8000) as usize;
        prg_rom.get(offset).copied().unwrap_or(0)
    }

    fn write_prg(&mut self, _addr: u16, val: u8) {
        self.prg_bank = (val >> 4) & 0x03;
        self.chr_bank = val & 0x03;
    }

    fn read_chr(&self, chr_data: &[u8], addr: u16) -> u8 {
        let bank = (self.chr_bank & self.chr_mask) as usize;
        let offset = bank * 0x2000 + addr as usize;
        chr_data.get(offset).copied().unwrap_or(0)
    }

    fn write_chr(&self, chr_data: &mut Vec<u8>, addr: u16, val: u8) {
        let bank = (self.chr_bank & self.chr_mask) as usize;
        let offset = bank * 0x2000 + addr as usize;
        if let Some(byte) = chr_data.get_mut(offset) {
            *byte = val;
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }

    fn save_state(&self) -> Vec<u8> {
        alloc::vec![self.prg_bank, self.chr_bank]
    }

    fn load_state(&mut self, data: &[u8]) {
        if data.len() >= 2 {
            self.prg_bank = data[0];
            self.chr_bank = data[1];
        }
    }
}
