use crate::ppu::{Ppu, PpuCtrl, PpuStatus};

/// PPU register read/write handlers for $2000-$2007.
impl Ppu {
    /// Write to $2000 (PPUCTRL).
    pub fn write_ctrl(&mut self, val: u8) {
        let was_nmi = self.ctrl.contains(PpuCtrl::NMI_ENABLE);
        self.ctrl = PpuCtrl::from_bits_truncate(val);
        // Update nametable select in t
        self.t = (self.t & 0xF3FF) | ((val as u16 & 0x03) << 10);
        // If NMI just enabled and we're in VBlank, trigger NMI
        if !was_nmi
            && self.ctrl.contains(PpuCtrl::NMI_ENABLE)
            && self.status.contains(PpuStatus::VBLANK)
        {
            self.nmi_pending = true;
        }
    }

    /// Write to $2001 (PPUMASK).
    pub fn write_mask(&mut self, val: u8) {
        self.mask = super::PpuMask::from_bits_truncate(val);
    }

    /// Read $2002 (PPUSTATUS).
    pub fn read_status(&mut self) -> u8 {
        let val = self.status.bits();
        self.status.remove(PpuStatus::VBLANK);
        self.write_latch = false;
        val
    }

    /// Write to $2003 (OAMADDR).
    pub fn write_oam_addr(&mut self, val: u8) {
        self.oam_addr = val;
    }

    /// Read $2004 (OAMDATA).
    pub fn read_oam_data(&self) -> u8 {
        self.oam[self.oam_addr as usize]
    }

    /// Write to $2004 (OAMDATA).
    pub fn write_oam_data(&mut self, val: u8) {
        self.oam[self.oam_addr as usize] = val;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    /// Write to $2005 (PPUSCROLL).
    pub fn write_scroll(&mut self, val: u8) {
        if !self.write_latch {
            // First write: X scroll
            self.fine_x = val & 0x07;
            self.t = (self.t & 0xFFE0) | ((val as u16) >> 3);
        } else {
            // Second write: Y scroll
            self.t = (self.t & 0x8C1F) | ((val as u16 & 0x07) << 12) | ((val as u16 & 0xF8) << 2);
        }
        self.write_latch = !self.write_latch;
    }

    /// Write to $2006 (PPUADDR).
    pub fn write_addr(&mut self, val: u8) {
        if !self.write_latch {
            // First write: high byte
            self.t = (self.t & 0x00FF) | ((val as u16 & 0x3F) << 8);
        } else {
            // Second write: low byte
            self.t = (self.t & 0xFF00) | val as u16;
            self.v = self.t;
        }
        self.write_latch = !self.write_latch;
    }

    /// Read $2007 (PPUDATA). Requires a CHR read function for pattern table access.
    pub fn read_data(&mut self, chr_read: &dyn Fn(u16) -> u8) -> u8 {
        let addr = self.v & 0x3FFF;
        let val = if addr >= 0x3F00 {
            // Palette reads are immediate; buffer gets underlying nametable data
            self.data_buffer = self.ppu_bus_read(addr - 0x1000, chr_read);
            self.read_palette(addr)
        } else {
            let buffered = self.data_buffer;
            self.data_buffer = self.ppu_bus_read(addr, chr_read);
            buffered
        };
        self.v = self
            .v
            .wrapping_add(if self.ctrl.contains(PpuCtrl::VRAM_INC) {
                32
            } else {
                1
            });
        val
    }

    /// Write to $2007 (PPUDATA). Requires a CHR write function for pattern table access.
    pub fn write_data(&mut self, val: u8, chr_write: &mut dyn FnMut(u16, u8)) {
        let addr = self.v & 0x3FFF;
        self.ppu_bus_write(addr, val, chr_write);
        self.v = self
            .v
            .wrapping_add(if self.ctrl.contains(PpuCtrl::VRAM_INC) {
                32
            } else {
                1
            });
    }
}
