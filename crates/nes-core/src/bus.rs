use alloc::vec::Vec;

use crate::apu::Apu;
use crate::cartridge::Cartridge;
use crate::mappers::Mirroring;
use crate::ppu::Ppu;

/// NES controller state (8 buttons).
#[derive(Debug, Clone)]
pub struct Controller {
    /// Current button state bitfield.
    buttons: u8,
    /// Shift register for serial reads.
    shift_register: u8,
    /// Whether strobe is active.
    strobe: bool,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            buttons: 0,
            shift_register: 0,
            strobe: false,
        }
    }

    pub fn set_button(&mut self, button: u8, pressed: bool) {
        if pressed {
            self.buttons |= 1 << button;
        } else {
            self.buttons &= !(1 << button);
        }
    }

    pub fn write_strobe(&mut self, val: u8) {
        self.strobe = val & 0x01 != 0;
        if self.strobe {
            self.shift_register = self.buttons;
        }
    }

    pub fn read(&mut self) -> u8 {
        if self.strobe {
            return self.buttons & 0x01;
        }
        let val = self.shift_register & 0x01;
        self.shift_register >>= 1;
        val
    }
}

impl Default for Controller {
    fn default() -> Self {
        Self::new()
    }
}

/// The NES memory bus — connects CPU to all components.
pub struct Bus {
    /// 2KB internal CPU RAM.
    pub ram: [u8; 2048],
    pub ppu: Ppu,
    pub apu: Apu,
    pub controller: Controller,
    /// Loaded cartridge (None if no ROM loaded).
    pub(crate) cartridge: Option<Cartridge>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            ram: [0; 2048],
            ppu: Ppu::new(),
            apu: Apu::new(),
            controller: Controller::new(),
            cartridge: None,
        }
    }

    pub fn load_cartridge(&mut self, cart: Cartridge) {
        self.cartridge = Some(cart);
    }

    pub fn rom_loaded(&self) -> bool {
        self.cartridge.is_some()
    }

    /// Get the current mirroring mode from the cartridge.
    pub fn mirroring(&self) -> Mirroring {
        self.cartridge
            .as_ref()
            .map(|c| c.mirroring())
            .unwrap_or(Mirroring::Horizontal)
    }

    /// Check if any IRQ is pending (mapper or APU).
    pub fn irq_pending(&self) -> bool {
        let mapper_irq = self
            .cartridge
            .as_ref()
            .map(|c| c.irq_pending())
            .unwrap_or(false);
        mapper_irq || self.apu.irq_pending()
    }

    /// Notify the mapper of scanlines rendered (for MMC3 IRQ counter).
    pub fn notify_scanlines(&mut self, count: u16) {
        if let Some(cart) = &mut self.cartridge {
            for _ in 0..count {
                cart.notify_scanline();
            }
        }
    }

    /// Read a byte from the CPU address space ($0000-$FFFF).
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            // Internal RAM + mirrors
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize],

            // PPU registers + mirrors
            0x2000..=0x3FFF => {
                let reg = addr & 0x0007;
                match reg {
                    0 | 1 | 3 | 5 | 6 => 0,      // Write-only registers
                    2 => self.ppu.status.bits(), // PPUSTATUS (simplified — full impl needs &mut)
                    4 => self.ppu.read_oam_data(),
                    7 => self.ppu.data_buffer, // PPUDATA (simplified)
                    _ => 0,
                }
            }

            // APU and I/O registers
            0x4000..=0x4017 => {
                match addr {
                    0x4015 => 0, // APU status (must use read_mut for side effects)
                    0x4016 => 0, // Controller 1 (handled via mut read)
                    0x4017 => 0, // Controller 2 (not implemented)
                    _ => 0,
                }
            }

            // Cartridge space
            0x4020..=0xFFFF => self
                .cartridge
                .as_ref()
                .map(|c| c.read_prg(addr))
                .unwrap_or(0),

            _ => 0,
        }
    }

    /// Write a byte to the CPU address space.
    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            // Internal RAM + mirrors
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize] = val,

            // PPU registers + mirrors
            0x2000..=0x3FFF => {
                let reg = addr & 0x0007;
                match reg {
                    0 => self.ppu.write_ctrl(val),
                    1 => self.ppu.write_mask(val),
                    3 => self.ppu.write_oam_addr(val),
                    4 => self.ppu.write_oam_data(val),
                    5 => self.ppu.write_scroll(val),
                    6 => self.ppu.write_addr(val),
                    7 => {
                        if let Some(cart) = &mut self.cartridge {
                            let chr_is_ram = cart.chr_is_ram;
                            let chr_rom = &mut cart.chr_rom;
                            let mapper = &mut cart.mapper;
                            self.ppu.write_data(val, &mut |addr, val| {
                                if chr_is_ram && addr < 0x2000 {
                                    mapper.write_chr(chr_rom, addr, val);
                                }
                            });
                        }
                    }
                    _ => {} // $2002 is read-only
                }
            }

            // APU and I/O registers
            0x4000..=0x4017 => {
                match addr {
                    0x4014 => {
                        // OAM DMA
                        let page = (val as u16) << 8;
                        for i in 0..256u16 {
                            let byte = self.read(page + i);
                            self.ppu.oam[self.ppu.oam_addr as usize] = byte;
                            self.ppu.oam_addr = self.ppu.oam_addr.wrapping_add(1);
                        }
                        // DMA takes 513-514 cycles (handled by CPU stall)
                    }
                    0x4016 => self.controller.write_strobe(val),
                    0x4000..=0x4013 | 0x4015 | 0x4017 => {
                        self.apu.write_register(addr, val);
                    }
                    _ => {}
                }
            }

            // Cartridge space
            0x4020..=0xFFFF => {
                if let Some(cart) = &mut self.cartridge {
                    cart.write_prg(addr, val);
                }
            }

            _ => {}
        }
    }

    /// Mutable read for registers with side effects.
    pub fn read_mut(&mut self, addr: u16) -> u8 {
        match addr {
            0x2000..=0x3FFF => {
                let reg = addr & 0x0007;
                match reg {
                    2 => self.ppu.read_status(),
                    4 => self.ppu.read_oam_data(),
                    7 => {
                        if let Some(cart) = &self.cartridge {
                            let chr_rom = &cart.chr_rom;
                            let mapper = &cart.mapper;
                            self.ppu.read_data(&|addr| mapper.read_chr(chr_rom, addr))
                        } else {
                            0
                        }
                    }
                    _ => 0,
                }
            }
            0x4015 => self.apu.read_status(),
            0x4016 => self.controller.read(),
            _ => self.read(addr),
        }
    }

    /// Save bus state (RAM + cartridge state).
    pub fn save_state(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.ram);
        self.ppu.save_state(buf);
        self.apu.save_state(buf);
        if let Some(cart) = &self.cartridge {
            let mapper_state = cart.mapper.save_state();
            buf.extend_from_slice(&(mapper_state.len() as u32).to_le_bytes());
            buf.extend_from_slice(&mapper_state);
            buf.extend_from_slice(&cart.sram);
        }
    }

    /// Load bus state.
    pub fn load_state(&mut self, data: &[u8], cursor: &mut usize) -> bool {
        if *cursor + 2048 > data.len() {
            return false;
        }
        self.ram.copy_from_slice(&data[*cursor..*cursor + 2048]);
        *cursor += 2048;

        if !self.ppu.load_state(data, cursor) {
            return false;
        }
        if !self.apu.load_state(data, cursor) {
            return false;
        }

        if let Some(cart) = &mut self.cartridge {
            if *cursor + 4 > data.len() {
                return false;
            }
            let mapper_len =
                u32::from_le_bytes(data[*cursor..*cursor + 4].try_into().unwrap()) as usize;
            *cursor += 4;
            if *cursor + mapper_len > data.len() {
                return false;
            }
            cart.mapper.load_state(&data[*cursor..*cursor + mapper_len]);
            *cursor += mapper_len;
            if *cursor + 8192 > data.len() {
                return false;
            }
            cart.sram.copy_from_slice(&data[*cursor..*cursor + 8192]);
            *cursor += 8192;
        }

        true
    }
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}
