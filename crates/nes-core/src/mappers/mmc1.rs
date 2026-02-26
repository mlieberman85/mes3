use alloc::vec::Vec;

use super::{Mapper, Mirroring};
use crate::cartridge::INesHeader;

/// MMC1 (Mapper 1) — serial shift register, PRG/CHR bank switching.
pub struct Mmc1 {
    shift_register: u8,
    shift_count: u8,
    control: u8,
    chr_bank0: u8,
    chr_bank1: u8,
    prg_bank: u8,
    prg_banks: u8,
    mirroring: Mirroring,
}

impl Mmc1 {
    pub fn new(header: &INesHeader, _prg_rom: &[u8]) -> Self {
        Self {
            shift_register: 0,
            shift_count: 0,
            control: 0x0C, // Default: fix last bank, switch 8KB CHR
            chr_bank0: 0,
            chr_bank1: 0,
            prg_bank: 0,
            prg_banks: header.prg_rom_banks,
            mirroring: header.mirroring,
        }
    }

    fn update_mirroring(&mut self) {
        self.mirroring = match self.control & 0x03 {
            0 => Mirroring::SingleScreenLo,
            1 => Mirroring::SingleScreenHi,
            2 => Mirroring::Vertical,
            3 => Mirroring::Horizontal,
            _ => unreachable!(),
        };
    }
}

impl Mapper for Mmc1 {
    fn read_prg(&self, prg_rom: &[u8], addr: u16) -> u8 {
        let prg_mode = (self.control >> 2) & 0x03;
        let bank_size = 16384;
        let offset = match prg_mode {
            0 | 1 => {
                // 32KB mode
                let bank = (self.prg_bank & 0xFE) as usize;
                bank * bank_size + (addr as usize - 0x8000)
            }
            2 => {
                // Fix first bank, switch second
                if addr < 0xC000 {
                    addr as usize - 0x8000
                } else {
                    self.prg_bank as usize * bank_size + (addr as usize - 0xC000)
                }
            }
            3 => {
                // Switch first bank, fix last
                if addr < 0xC000 {
                    self.prg_bank as usize * bank_size + (addr as usize - 0x8000)
                } else {
                    (self.prg_banks as usize - 1) * bank_size + (addr as usize - 0xC000)
                }
            }
            _ => unreachable!(),
        };
        prg_rom.get(offset).copied().unwrap_or(0)
    }

    fn write_prg(&mut self, addr: u16, val: u8) {
        if val & 0x80 != 0 {
            // Reset shift register
            self.shift_register = 0;
            self.shift_count = 0;
            self.control |= 0x0C;
            return;
        }

        self.shift_register |= (val & 0x01) << self.shift_count;
        self.shift_count += 1;

        if self.shift_count == 5 {
            let value = self.shift_register;
            match (addr >> 13) & 0x03 {
                0 => {
                    self.control = value;
                    self.update_mirroring();
                }
                1 => {
                    self.chr_bank0 = value;
                }
                2 => {
                    self.chr_bank1 = value;
                }
                3 => {
                    self.prg_bank = value & 0x0F;
                }
                _ => {}
            }
            self.shift_register = 0;
            self.shift_count = 0;
        }
    }

    fn read_chr(&self, chr_data: &[u8], addr: u16) -> u8 {
        let chr_mode = (self.control >> 4) & 0x01;
        let offset = if chr_mode == 0 {
            // 8KB mode
            let bank = (self.chr_bank0 & 0xFE) as usize;
            bank * 4096 + addr as usize
        } else {
            // 4KB mode
            if addr < 0x1000 {
                self.chr_bank0 as usize * 4096 + addr as usize
            } else {
                self.chr_bank1 as usize * 4096 + (addr as usize - 0x1000)
            }
        };
        chr_data.get(offset).copied().unwrap_or(0)
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
        alloc::vec![
            self.shift_register,
            self.shift_count,
            self.control,
            self.chr_bank0,
            self.chr_bank1,
            self.prg_bank,
        ]
    }

    fn load_state(&mut self, data: &[u8]) {
        if data.len() >= 6 {
            self.shift_register = data[0];
            self.shift_count = data[1];
            self.control = data[2];
            self.chr_bank0 = data[3];
            self.chr_bank1 = data[4];
            self.prg_bank = data[5];
            self.update_mirroring();
        }
    }
}
