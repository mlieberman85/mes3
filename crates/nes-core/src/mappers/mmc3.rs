use alloc::vec::Vec;

use super::{Mapper, Mirroring};
use crate::cartridge::INesHeader;

/// MMC3 (Mapper 4) — PRG/CHR bank switching with scanline counter and IRQ.
pub struct Mmc3 {
    bank_select: u8,
    bank_registers: [u8; 8],
    prg_mode: bool,
    chr_inversion: bool,
    mirroring: Mirroring,
    irq_counter: u8,
    irq_latch: u8,
    irq_enabled: bool,
    irq_reload: bool,
    irq_pending: bool,
    prg_banks: u8,
}

impl Mmc3 {
    pub fn new(header: &INesHeader, _prg_rom: &[u8]) -> Self {
        Self {
            bank_select: 0,
            bank_registers: [0; 8],
            prg_mode: false,
            chr_inversion: false,
            mirroring: header.mirroring,
            irq_counter: 0,
            irq_latch: 0,
            irq_enabled: false,
            irq_reload: false,
            irq_pending: false,
            prg_banks: header.prg_rom_banks,
        }
    }

    fn prg_bank_offset(&self, bank: usize) -> usize {
        let total_banks = self.prg_banks as usize * 2; // 8KB banks
        (bank % total_banks) * 8192
    }
}

impl Mapper for Mmc3 {
    fn read_prg(&self, prg_rom: &[u8], addr: u16) -> u8 {
        let total_8k_banks = self.prg_banks as usize * 2;
        let last_bank = total_8k_banks - 1;
        let second_last = total_8k_banks.saturating_sub(2);

        let (bank, offset) = match addr {
            0x8000..=0x9FFF => {
                let bank = if self.prg_mode {
                    second_last
                } else {
                    self.bank_registers[6] as usize
                };
                (bank, (addr - 0x8000) as usize)
            }
            0xA000..=0xBFFF => (self.bank_registers[7] as usize, (addr - 0xA000) as usize),
            0xC000..=0xDFFF => {
                let bank = if self.prg_mode {
                    self.bank_registers[6] as usize
                } else {
                    second_last
                };
                (bank, (addr - 0xC000) as usize)
            }
            0xE000..=0xFFFF => (last_bank, (addr - 0xE000) as usize),
            _ => return 0,
        };

        let abs_offset = self.prg_bank_offset(bank) + offset;
        prg_rom.get(abs_offset).copied().unwrap_or(0)
    }

    fn write_prg(&mut self, addr: u16, val: u8) {
        let even = addr & 0x01 == 0;
        match addr {
            0x8000..=0x9FFF => {
                if even {
                    // Bank select
                    self.bank_select = val & 0x07;
                    self.prg_mode = val & 0x40 != 0;
                    self.chr_inversion = val & 0x80 != 0;
                } else {
                    // Bank data
                    self.bank_registers[self.bank_select as usize] = val;
                }
            }
            0xA000..=0xBFFF => {
                if even {
                    // Mirroring
                    self.mirroring = if val & 0x01 != 0 {
                        Mirroring::Horizontal
                    } else {
                        Mirroring::Vertical
                    };
                }
                // Odd: PRG-RAM protect (not implemented)
            }
            0xC000..=0xDFFF => {
                if even {
                    self.irq_latch = val;
                } else {
                    self.irq_counter = 0;
                    self.irq_reload = true;
                }
            }
            0xE000..=0xFFFF => {
                if even {
                    self.irq_enabled = false;
                    self.irq_pending = false;
                } else {
                    self.irq_enabled = true;
                }
            }
            _ => {}
        }
    }

    fn read_chr(&self, chr_data: &[u8], addr: u16) -> u8 {
        let bank_1k = match addr {
            0x0000..=0x03FF => {
                if self.chr_inversion {
                    self.bank_registers[2]
                } else {
                    self.bank_registers[0] & 0xFE
                }
            }
            0x0400..=0x07FF => {
                if self.chr_inversion {
                    self.bank_registers[3]
                } else {
                    self.bank_registers[0] | 0x01
                }
            }
            0x0800..=0x0BFF => {
                if self.chr_inversion {
                    self.bank_registers[4]
                } else {
                    self.bank_registers[1] & 0xFE
                }
            }
            0x0C00..=0x0FFF => {
                if self.chr_inversion {
                    self.bank_registers[5]
                } else {
                    self.bank_registers[1] | 0x01
                }
            }
            0x1000..=0x13FF => {
                if self.chr_inversion {
                    self.bank_registers[0] & 0xFE
                } else {
                    self.bank_registers[2]
                }
            }
            0x1400..=0x17FF => {
                if self.chr_inversion {
                    self.bank_registers[0] | 0x01
                } else {
                    self.bank_registers[3]
                }
            }
            0x1800..=0x1BFF => {
                if self.chr_inversion {
                    self.bank_registers[1] & 0xFE
                } else {
                    self.bank_registers[4]
                }
            }
            0x1C00..=0x1FFF => {
                if self.chr_inversion {
                    self.bank_registers[1] | 0x01
                } else {
                    self.bank_registers[5]
                }
            }
            _ => return 0,
        };
        let offset = bank_1k as usize * 1024 + (addr & 0x03FF) as usize;
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

    fn irq_pending(&self) -> bool {
        self.irq_pending
    }

    fn notify_scanline(&mut self) {
        if self.irq_counter == 0 || self.irq_reload {
            self.irq_counter = self.irq_latch;
            self.irq_reload = false;
        } else {
            self.irq_counter -= 1;
        }

        if self.irq_counter == 0 && self.irq_enabled {
            self.irq_pending = true;
        }
    }

    fn save_state(&self) -> Vec<u8> {
        let mut state = Vec::with_capacity(17);
        state.push(self.bank_select);
        state.extend_from_slice(&self.bank_registers);
        state.push(self.prg_mode as u8);
        state.push(self.chr_inversion as u8);
        state.push(self.irq_counter);
        state.push(self.irq_latch);
        state.push(self.irq_enabled as u8);
        state.push(self.irq_reload as u8);
        state.push(self.irq_pending as u8);
        // Mirroring: 0 = Horizontal, 1 = Vertical
        state.push(match self.mirroring {
            Mirroring::Vertical => 1,
            _ => 0,
        });
        state
    }

    fn load_state(&mut self, data: &[u8]) {
        if data.len() >= 15 {
            self.bank_select = data[0];
            self.bank_registers.copy_from_slice(&data[1..9]);
            self.prg_mode = data[9] != 0;
            self.chr_inversion = data[10] != 0;
            self.irq_counter = data[11];
            self.irq_latch = data[12];
            self.irq_enabled = data[13] != 0;
            self.irq_reload = data[14] != 0;
            if data.len() > 15 {
                self.irq_pending = data[15] != 0;
            }
            // Restore mirroring (added after irq_pending)
            if data.len() > 16 {
                self.mirroring = if data[16] != 0 {
                    Mirroring::Vertical
                } else {
                    Mirroring::Horizontal
                };
            }
        }
    }
}
