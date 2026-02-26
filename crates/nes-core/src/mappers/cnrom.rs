use alloc::vec::Vec;

use super::{Mapper, Mirroring};
use crate::cartridge::INesHeader;

/// CNROM (Mapper 3) — CHR bank switching only.
pub struct Cnrom {
    chr_bank: u8,
    mirroring: Mirroring,
    prg_banks: u8,
}

impl Cnrom {
    pub fn new(header: &INesHeader) -> Self {
        Self {
            chr_bank: 0,
            mirroring: header.mirroring,
            prg_banks: header.prg_rom_banks,
        }
    }
}

impl Mapper for Cnrom {
    fn read_prg(&self, prg_rom: &[u8], addr: u16) -> u8 {
        let addr = (addr - 0x8000) as usize;
        // Mirror 16KB PRG to fill 32KB if only 1 bank
        let addr = if self.prg_banks == 1 {
            addr & 0x3FFF
        } else {
            addr & 0x7FFF
        };
        prg_rom.get(addr).copied().unwrap_or(0)
    }

    fn write_prg(&mut self, _addr: u16, val: u8) {
        self.chr_bank = val & 0x03; // Typically 2-bit bank select
    }

    fn read_chr(&self, chr_data: &[u8], addr: u16) -> u8 {
        let offset = self.chr_bank as usize * 8192 + addr as usize;
        chr_data.get(offset).copied().unwrap_or(0)
    }

    fn write_chr(&self, chr_data: &mut Vec<u8>, addr: u16, val: u8) {
        let offset = self.chr_bank as usize * 8192 + addr as usize;
        if let Some(byte) = chr_data.get_mut(offset) {
            *byte = val;
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }

    fn save_state(&self) -> Vec<u8> {
        alloc::vec![self.chr_bank]
    }

    fn load_state(&mut self, data: &[u8]) {
        if !data.is_empty() {
            self.chr_bank = data[0];
        }
    }
}
