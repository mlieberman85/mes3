use alloc::vec::Vec;

use super::{Mapper, Mirroring};
use crate::cartridge::INesHeader;

/// NROM (Mapper 0) — no bank switching.
/// 16KB or 32KB PRG-ROM, 8KB CHR-ROM/RAM.
pub struct Nrom {
    mirroring: Mirroring,
    prg_banks: u8,
}

impl Nrom {
    pub fn new(header: &INesHeader, _prg_rom: &[u8]) -> Self {
        Self {
            mirroring: header.mirroring,
            prg_banks: header.prg_rom_banks,
        }
    }
}

impl Mapper for Nrom {
    fn read_prg(&self, prg_rom: &[u8], addr: u16) -> u8 {
        let addr = (addr - 0x8000) as usize;
        // Mirror 16KB PRG to fill 32KB space
        let addr = if self.prg_banks == 1 {
            addr & 0x3FFF
        } else {
            addr & 0x7FFF
        };
        prg_rom.get(addr).copied().unwrap_or(0)
    }

    fn write_prg(&mut self, _addr: u16, _val: u8) {
        // NROM has no writable registers
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
        Vec::new() // No mutable state
    }

    fn load_state(&mut self, _data: &[u8]) {
        // No mutable state
    }
}
