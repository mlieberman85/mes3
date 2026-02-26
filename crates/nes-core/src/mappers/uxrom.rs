use alloc::vec::Vec;

use super::{Mapper, Mirroring};
use crate::cartridge::INesHeader;

/// UxROM (Mapper 2) — PRG bank switching only.
pub struct UxRom {
    bank_select: u8,
    prg_banks: u8,
    mirroring: Mirroring,
}

impl UxRom {
    pub fn new(header: &INesHeader, _prg_rom: &[u8]) -> Self {
        Self {
            bank_select: 0,
            prg_banks: header.prg_rom_banks,
            mirroring: header.mirroring,
        }
    }
}

impl Mapper for UxRom {
    fn read_prg(&self, prg_rom: &[u8], addr: u16) -> u8 {
        let bank_size = 16384;
        let offset = if addr < 0xC000 {
            // Switchable bank
            self.bank_select as usize * bank_size + (addr as usize - 0x8000)
        } else {
            // Fixed last bank
            (self.prg_banks as usize - 1) * bank_size + (addr as usize - 0xC000)
        };
        prg_rom.get(offset).copied().unwrap_or(0)
    }

    fn write_prg(&mut self, _addr: u16, val: u8) {
        self.bank_select = val;
    }

    fn read_chr(&self, chr_data: &[u8], addr: u16) -> u8 {
        chr_data.get(addr as usize).copied().unwrap_or(0)
    }

    fn write_chr(&self, chr_data: &mut Vec<u8>, addr: u16, val: u8) {
        if let Some(byte) = chr_data.get_mut(addr as usize) {
            *byte = val;
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }

    fn save_state(&self) -> Vec<u8> {
        alloc::vec![self.bank_select]
    }

    fn load_state(&mut self, data: &[u8]) {
        if !data.is_empty() {
            self.bank_select = data[0];
        }
    }
}
