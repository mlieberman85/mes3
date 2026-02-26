pub mod registers;

use alloc::vec;
use alloc::vec::Vec;
use bitflags::bitflags;

use crate::mappers::Mirroring;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PpuCtrl: u8 {
        const NAMETABLE_LO  = 0b0000_0001;
        const NAMETABLE_HI  = 0b0000_0010;
        const VRAM_INC      = 0b0000_0100;
        const SPRITE_TABLE  = 0b0000_1000;
        const BG_TABLE      = 0b0001_0000;
        const SPRITE_SIZE   = 0b0010_0000;
        const MASTER_SLAVE  = 0b0100_0000;
        const NMI_ENABLE    = 0b1000_0000;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PpuMask: u8 {
        const GREYSCALE        = 0b0000_0001;
        const SHOW_BG_LEFT     = 0b0000_0010;
        const SHOW_SPRITES_LEFT = 0b0000_0100;
        const SHOW_BG          = 0b0000_1000;
        const SHOW_SPRITES     = 0b0001_0000;
        const EMPHASIZE_RED    = 0b0010_0000;
        const EMPHASIZE_GREEN  = 0b0100_0000;
        const EMPHASIZE_BLUE   = 0b1000_0000;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PpuStatus: u8 {
        const SPRITE_OVERFLOW = 0b0010_0000;
        const SPRITE_ZERO_HIT = 0b0100_0000;
        const VBLANK          = 0b1000_0000;
    }
}

/// NES PPU (2C02) state.
#[derive(Clone)]
pub struct Ppu {
    pub vram: [u8; 2048],
    pub oam: [u8; 256],
    pub palette: [u8; 32],
    pub ctrl: PpuCtrl,
    pub mask: PpuMask,
    pub status: PpuStatus,
    pub oam_addr: u8,
    /// Current VRAM address (15-bit, "loopy v").
    pub v: u16,
    /// Temporary VRAM address (15-bit, "loopy t").
    pub t: u16,
    /// Fine X scroll (3-bit).
    pub fine_x: u8,
    /// First/second write toggle.
    pub write_latch: bool,
    /// PPU data read buffer.
    pub data_buffer: u8,
    /// Current scanline (0-261).
    pub scanline: u16,
    /// Current dot/cycle within scanline (0-340).
    pub dot: u16,
    pub frame_count: u64,
    /// RGBA pixel output (256x240x4 = 245760 bytes), heap-allocated to avoid
    /// blowing the WASM stack.
    pub frame_buffer: Vec<u8>,
    /// NMI pending flag.
    nmi_pending: bool,
    /// Whether we're on an odd frame (for odd-frame dot skip).
    odd_frame: bool,
    /// Cached mirroring mode (updated each step from cartridge).
    mirroring: Mirroring,
    /// Number of visible scanlines rendered this step (for mapper IRQ clocking).
    scanline_irq_ticks: u16,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            vram: [0; 2048],
            oam: [0; 256],
            palette: [0; 32],
            ctrl: PpuCtrl::empty(),
            mask: PpuMask::empty(),
            status: PpuStatus::empty(),
            oam_addr: 0,
            v: 0,
            t: 0,
            fine_x: 0,
            write_latch: false,
            data_buffer: 0,
            scanline: 0,
            dot: 0,
            frame_count: 0,
            frame_buffer: vec![0; 245_760],
            nmi_pending: false,
            odd_frame: false,
            mirroring: Mirroring::Horizontal,
            scanline_irq_ticks: 0,
        }
    }

    pub fn reset(&mut self) {
        self.ctrl = PpuCtrl::empty();
        self.mask = PpuMask::empty();
        self.status = PpuStatus::empty();
        self.oam_addr = 0;
        self.v = 0;
        self.t = 0;
        self.fine_x = 0;
        self.write_latch = false;
        self.data_buffer = 0;
        self.scanline = 0;
        self.dot = 0;
        self.nmi_pending = false;
        self.odd_frame = false;
        self.scanline_irq_ticks = 0;
    }

    /// Advance the PPU by the given number of PPU cycles.
    pub fn step(&mut self, ppu_cycles: u64, mirroring: Mirroring, chr_read: &dyn Fn(u16) -> u8) {
        self.mirroring = mirroring;
        for _ in 0..ppu_cycles {
            self.tick(chr_read);
        }
    }

    fn tick(&mut self, chr_read: &dyn Fn(u16) -> u8) {
        // Advance position first (hardware advances then acts on the new position)
        self.dot += 1;
        if self.dot > 340 {
            self.dot = 0;
            self.scanline += 1;
            if self.scanline > 261 {
                self.scanline = 0;
                self.frame_count += 1;
                self.odd_frame = !self.odd_frame;
            }
        }

        let rendering = self.rendering_enabled();

        // --- Visible scanlines (0-239) ---
        if self.scanline < 240 && rendering {
            // Render the entire scanline at dot 1 (scanline-based approach)
            if self.dot == 1 {
                self.render_scanline(chr_read);
                self.scanline_irq_ticks += 1;
            }

            // Dot 256: increment fine Y scroll
            if self.dot == 256 {
                self.increment_y();
            }

            // Dot 257: copy horizontal bits from t to v
            if self.dot == 257 {
                self.v = (self.v & !0x041F) | (self.t & 0x041F);
            }
        }

        // --- VBlank set (scanline 241, dot 1) ---
        if self.scanline == 241 && self.dot == 1 {
            self.status.insert(PpuStatus::VBLANK);
            if self.ctrl.contains(PpuCtrl::NMI_ENABLE) {
                self.nmi_pending = true;
            }
        }

        // --- Pre-render scanline (261) ---
        if self.scanline == 261 {
            if self.dot == 1 {
                self.status.remove(PpuStatus::VBLANK);
                self.status.remove(PpuStatus::SPRITE_ZERO_HIT);
                self.status.remove(PpuStatus::SPRITE_OVERFLOW);
            }

            if rendering {
                // Dot 257: copy horizontal bits from t to v
                if self.dot == 257 {
                    self.v = (self.v & !0x041F) | (self.t & 0x041F);
                }

                // Dots 280-304: copy vertical bits from t to v
                if (280..=304).contains(&self.dot) {
                    self.v = (self.v & 0x041F) | (self.t & !0x041F);
                }

                // Odd-frame skip: skip last dot on odd frames
                if self.dot == 339 && self.odd_frame {
                    self.dot = 0;
                    self.scanline = 0;
                    self.frame_count += 1;
                    self.odd_frame = !self.odd_frame;
                }
            }
        }
    }

    // --- Rendering ---

    fn render_scanline(&mut self, chr_read: &dyn Fn(u16) -> u8) {
        let scanline = self.scanline;
        if scanline >= 240 {
            return;
        }

        let show_bg = self.mask.contains(PpuMask::SHOW_BG);
        let show_sprites = self.mask.contains(PpuMask::SHOW_SPRITES);

        if !show_bg && !show_sprites {
            let color_idx = (self.palette[0] & 0x3F) as usize;
            let rgba = SYSTEM_PALETTE[color_idx];
            let row_offset = scanline as usize * 256 * 4;
            for x in 0..256 {
                let offset = row_offset + x * 4;
                self.frame_buffer[offset] = rgba[0];
                self.frame_buffer[offset + 1] = rgba[1];
                self.frame_buffer[offset + 2] = rgba[2];
                self.frame_buffer[offset + 3] = rgba[3];
            }
            return;
        }

        // Background pixels: palette index (0 = transparent/universal BG)
        let mut bg_pixels = [0u8; 256];
        if show_bg {
            self.render_bg_scanline(chr_read, &mut bg_pixels);
        }

        // Sprite pixels, priorities, and sprite-zero indicator
        let mut sp_pixels = [0u8; 256];
        let mut sp_priorities = [0u8; 256]; // 0 = in front of BG, 1 = behind BG
        let mut sp_zero = [false; 256];
        if show_sprites {
            self.render_sprite_scanline(
                scanline,
                chr_read,
                &mut sp_pixels,
                &mut sp_priorities,
                &mut sp_zero,
            );
        }

        // Compose final output
        let row_offset = scanline as usize * 256 * 4;
        for x in 0..256usize {
            let bg_pixel = bg_pixels[x];
            let sp_pixel = sp_pixels[x];

            let show_bg_here = show_bg && (x >= 8 || self.mask.contains(PpuMask::SHOW_BG_LEFT));
            let show_sp_here =
                show_sprites && (x >= 8 || self.mask.contains(PpuMask::SHOW_SPRITES_LEFT));

            let bg_opaque = show_bg_here && (bg_pixel & 0x03) != 0;
            let sp_opaque = show_sp_here && (sp_pixel & 0x03) != 0;

            // Sprite-zero hit: both BG and sprite 0 opaque, x < 255
            if bg_opaque && sp_opaque && sp_zero[x] && x < 255 {
                self.status.insert(PpuStatus::SPRITE_ZERO_HIT);
            }

            // Priority multiplexer
            let palette_idx = if sp_opaque && (!bg_opaque || sp_priorities[x] == 0) {
                sp_pixel
            } else if bg_opaque {
                bg_pixel
            } else {
                0
            };

            let color_idx = (self.palette[palette_idx as usize] & 0x3F) as usize;
            let rgba = SYSTEM_PALETTE[color_idx];
            let offset = row_offset + x * 4;
            self.frame_buffer[offset] = rgba[0];
            self.frame_buffer[offset + 1] = rgba[1];
            self.frame_buffer[offset + 2] = rgba[2];
            self.frame_buffer[offset + 3] = rgba[3];
        }
    }

    fn render_bg_scanline(&self, chr_read: &dyn Fn(u16) -> u8, pixels: &mut [u8; 256]) {
        let bg_table: u16 = if self.ctrl.contains(PpuCtrl::BG_TABLE) {
            0x1000
        } else {
            0
        };
        let fine_x = self.fine_x;
        let fine_y = (self.v >> 12) & 0x07;
        let mut tile_v = self.v;
        let mut screen_x: usize = 0;
        let start_bit = fine_x;

        // Render up to 33 tiles to cover 256 pixels + fine_x offset
        for tile_num in 0..34u16 {
            // Tile address from nametable
            let nt_addr = 0x2000 | (tile_v & 0x0FFF);
            let tile_index = self.ppu_bus_read(nt_addr, chr_read) as u16;

            // Attribute table
            let coarse_x = tile_v & 0x001F;
            let coarse_y = (tile_v >> 5) & 0x001F;
            let nt_select = (tile_v >> 10) & 0x03;
            let at_addr = 0x23C0 | (nt_select << 10) | ((coarse_y >> 2) << 3) | (coarse_x >> 2);
            let at_byte = self.ppu_bus_read(at_addr, chr_read);
            let at_shift = ((coarse_y & 0x02) << 1) | (coarse_x & 0x02);
            let palette_num = (at_byte >> at_shift) & 0x03;

            // Pattern table bytes
            let pattern_addr = bg_table + tile_index * 16 + fine_y;
            let lo = chr_read(pattern_addr);
            let hi = chr_read(pattern_addr + 8);

            // Pixel extraction
            let first_bit = if tile_num == 0 { start_bit } else { 0 };
            for bit_pos in first_bit..8 {
                if screen_x >= 256 {
                    break;
                }
                let bit = 7 - bit_pos;
                let color_lo = (lo >> bit) & 1;
                let color_hi = (hi >> bit) & 1;
                let color = color_lo | (color_hi << 1);

                pixels[screen_x] = if color == 0 {
                    0
                } else {
                    (palette_num << 2) | color
                };
                screen_x += 1;
            }

            if screen_x >= 256 {
                break;
            }

            // Increment coarse X with nametable wrapping
            if tile_v & 0x001F == 31 {
                tile_v &= !0x001F;
                tile_v ^= 0x0400; // Switch horizontal nametable
            } else {
                tile_v += 1;
            }
        }
    }

    fn render_sprite_scanline(
        &mut self,
        scanline: u16,
        chr_read: &dyn Fn(u16) -> u8,
        pixels: &mut [u8; 256],
        priorities: &mut [u8; 256],
        zero_bits: &mut [bool; 256],
    ) {
        let sprite_height: u16 = if self.ctrl.contains(PpuCtrl::SPRITE_SIZE) {
            16
        } else {
            8
        };
        let sprite_table: u16 = if self.ctrl.contains(PpuCtrl::SPRITE_TABLE) {
            0x1000
        } else {
            0
        };

        // Find sprites on this scanline (max 8, with overflow detection)
        let mut sprites_on_line = [(0u8, 0u8, 0u8, 0u8, false); 8];
        let mut sprite_count = 0u8;

        for i in 0..64usize {
            let y = self.oam[i * 4] as u16;
            let tile = self.oam[i * 4 + 1];
            let attr = self.oam[i * 4 + 2];
            let x = self.oam[i * 4 + 3];

            // Sprites display at Y+1. Check if this scanline is within the sprite.
            if y >= 0xEF {
                continue;
            }
            let row = scanline.wrapping_sub(y + 1);
            if row >= sprite_height {
                continue;
            }

            if sprite_count < 8 {
                sprites_on_line[sprite_count as usize] = (y as u8, tile, attr, x, i == 0);
                sprite_count += 1;
            } else {
                self.status.insert(PpuStatus::SPRITE_OVERFLOW);
                break;
            }
        }

        // Render sprites in reverse order so lower-indexed sprites are drawn last (higher priority)
        for s in (0..sprite_count).rev() {
            let (y, tile, attr, x, is_zero) = sprites_on_line[s as usize];
            let flip_h = attr & 0x40 != 0;
            let flip_v = attr & 0x80 != 0;
            let priority = (attr >> 5) & 1; // 0 = in front, 1 = behind BG
            let palette_base = ((attr & 0x03) + 4) << 2; // Sprite palettes 4-7

            let row = scanline.wrapping_sub(y as u16 + 1);

            let pattern_addr = if sprite_height == 16 {
                // 8x16 mode: tile bit 0 selects pattern table
                let table = (tile as u16 & 1) * 0x1000;
                let tile_num = (tile & 0xFE) as u16;
                let effective_row = if flip_v { 15 - row } else { row };
                let actual_tile = if effective_row >= 8 {
                    tile_num + 1
                } else {
                    tile_num
                };
                table + actual_tile * 16 + (effective_row & 7)
            } else {
                // 8x8 mode
                let effective_row = if flip_v { 7 - row } else { row };
                sprite_table + tile as u16 * 16 + effective_row
            };

            let lo = chr_read(pattern_addr);
            let hi = chr_read(pattern_addr + 8);

            for bit in 0..8u16 {
                let px = x as u16 + bit;
                if px >= 256 {
                    continue;
                }

                let bit_pos = if flip_h { bit as u8 } else { 7 - bit as u8 };
                let color_lo = (lo >> bit_pos) & 1;
                let color_hi = (hi >> bit_pos) & 1;
                let color = color_lo | (color_hi << 1);

                if color == 0 {
                    continue;
                }

                // Lower-indexed sprites overwrite higher-indexed (we iterate in reverse)
                pixels[px as usize] = palette_base | color;
                priorities[px as usize] = priority;

                if is_zero {
                    zero_bits[px as usize] = true;
                }
            }
        }
    }

    // --- Scrolling helpers ---

    /// Increment fine Y, wrapping to coarse Y and nametable as needed.
    fn increment_y(&mut self) {
        if (self.v & 0x7000) != 0x7000 {
            // Fine Y < 7
            self.v += 0x1000;
        } else {
            // Fine Y wraps to 0
            self.v &= !0x7000;
            let mut coarse_y = (self.v & 0x03E0) >> 5;
            if coarse_y == 29 {
                coarse_y = 0;
                self.v ^= 0x0800; // Switch vertical nametable
            } else if coarse_y == 31 {
                coarse_y = 0; // Wrap without nametable switch
            } else {
                coarse_y += 1;
            }
            self.v = (self.v & !0x03E0) | (coarse_y << 5);
        }
    }

    // --- PPU bus read (nametable + CHR) ---

    fn ppu_bus_read(&self, addr: u16, chr_read: &dyn Fn(u16) -> u8) -> u8 {
        let addr = addr & 0x3FFF;
        match addr {
            0x0000..=0x1FFF => chr_read(addr),
            0x2000..=0x3EFF => {
                let idx = self.mirror_nametable_addr(addr);
                self.vram[idx]
            }
            0x3F00..=0x3FFF => self.read_palette(addr),
            _ => 0,
        }
    }

    /// Write to the PPU address space (for $2007 writes).
    pub fn ppu_bus_write(&mut self, addr: u16, val: u8, chr_write: &mut dyn FnMut(u16, u8)) {
        let addr = addr & 0x3FFF;
        match addr {
            0x0000..=0x1FFF => chr_write(addr, val),
            0x2000..=0x3EFF => {
                let idx = self.mirror_nametable_addr(addr);
                self.vram[idx] = val;
            }
            0x3F00..=0x3FFF => self.write_palette(addr, val),
            _ => {}
        }
    }

    fn mirror_nametable_addr(&self, addr: u16) -> usize {
        let addr = (addr - 0x2000) & 0x0FFF;
        let table = (addr / 0x0400) as usize;
        let offset = (addr % 0x0400) as usize;

        let mapped_table = match self.mirroring {
            Mirroring::Horizontal => match table {
                0 | 1 => 0,
                _ => 1,
            },
            Mirroring::Vertical => match table {
                0 | 2 => 0,
                _ => 1,
            },
            Mirroring::SingleScreenLo => 0,
            Mirroring::SingleScreenHi => 1,
            Mirroring::FourScreen => table.min(1), // 2KB VRAM limits to 2 tables
        };

        mapped_table * 0x0400 + offset
    }

    // --- Public interface ---

    /// Take and clear the NMI pending flag.
    pub fn take_nmi(&mut self) -> bool {
        let pending = self.nmi_pending;
        self.nmi_pending = false;
        pending
    }

    /// Take and clear the scanline IRQ tick count (for mapper clocking).
    pub fn take_scanline_irq_ticks(&mut self) -> u16 {
        let ticks = self.scanline_irq_ticks;
        self.scanline_irq_ticks = 0;
        ticks
    }

    /// Check if rendering is enabled.
    pub fn rendering_enabled(&self) -> bool {
        self.mask.contains(PpuMask::SHOW_BG) || self.mask.contains(PpuMask::SHOW_SPRITES)
    }

    /// Set the mirroring mode (called by bus when mapper changes it).
    pub fn set_mirroring(&mut self, mirroring: Mirroring) {
        self.mirroring = mirroring;
    }

    // Palette helpers

    fn read_palette(&self, addr: u16) -> u8 {
        let idx = (addr & 0x1F) as usize;
        let idx = if idx >= 16 && idx.is_multiple_of(4) {
            idx - 16
        } else {
            idx
        };
        self.palette[idx]
    }

    fn write_palette(&mut self, addr: u16, val: u8) {
        let idx = (addr & 0x1F) as usize;
        let idx = if idx >= 16 && idx.is_multiple_of(4) {
            idx - 16
        } else {
            idx
        };
        self.palette[idx] = val;
    }

    /// Save PPU state to a byte buffer.
    pub fn save_state(&self, buf: &mut alloc::vec::Vec<u8>) {
        buf.extend_from_slice(&self.vram);
        buf.extend_from_slice(&self.oam);
        buf.extend_from_slice(&self.palette);
        buf.push(self.ctrl.bits());
        buf.push(self.mask.bits());
        buf.push(self.status.bits());
        buf.push(self.oam_addr);
        buf.extend_from_slice(&self.v.to_le_bytes());
        buf.extend_from_slice(&self.t.to_le_bytes());
        buf.push(self.fine_x);
        buf.push(self.write_latch as u8);
        buf.push(self.data_buffer);
        buf.extend_from_slice(&self.scanline.to_le_bytes());
        buf.extend_from_slice(&self.dot.to_le_bytes());
        buf.extend_from_slice(&self.frame_count.to_le_bytes());
        buf.push(self.odd_frame as u8);
    }

    /// Load PPU state from a byte buffer.
    pub fn load_state(&mut self, data: &[u8], cursor: &mut usize) -> bool {
        let needed = 2048 + 256 + 32 + 1 + 1 + 1 + 1 + 2 + 2 + 1 + 1 + 1 + 2 + 2 + 8 + 1;
        if *cursor + needed > data.len() {
            return false;
        }
        self.vram.copy_from_slice(&data[*cursor..*cursor + 2048]);
        *cursor += 2048;
        self.oam.copy_from_slice(&data[*cursor..*cursor + 256]);
        *cursor += 256;
        self.palette.copy_from_slice(&data[*cursor..*cursor + 32]);
        *cursor += 32;
        self.ctrl = PpuCtrl::from_bits_truncate(data[*cursor]);
        *cursor += 1;
        self.mask = PpuMask::from_bits_truncate(data[*cursor]);
        *cursor += 1;
        self.status = PpuStatus::from_bits_truncate(data[*cursor]);
        *cursor += 1;
        self.oam_addr = data[*cursor];
        *cursor += 1;
        self.v = u16::from_le_bytes([data[*cursor], data[*cursor + 1]]);
        *cursor += 2;
        self.t = u16::from_le_bytes([data[*cursor], data[*cursor + 1]]);
        *cursor += 2;
        self.fine_x = data[*cursor];
        *cursor += 1;
        self.write_latch = data[*cursor] != 0;
        *cursor += 1;
        self.data_buffer = data[*cursor];
        *cursor += 1;
        self.scanline = u16::from_le_bytes([data[*cursor], data[*cursor + 1]]);
        *cursor += 2;
        self.dot = u16::from_le_bytes([data[*cursor], data[*cursor + 1]]);
        *cursor += 2;
        self.frame_count = u64::from_le_bytes(data[*cursor..*cursor + 8].try_into().unwrap());
        *cursor += 8;
        self.odd_frame = data[*cursor] != 0;
        *cursor += 1;
        true
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

/// The NES system palette (64 colors, RGBA).
/// Standard NES palette — each entry is [R, G, B, A].
pub static SYSTEM_PALETTE: [[u8; 4]; 64] = [
    [0x62, 0x62, 0x62, 0xFF],
    [0x00, 0x2D, 0x69, 0xFF],
    [0x00, 0x10, 0x90, 0xFF],
    [0x24, 0x00, 0x8E, 0xFF],
    [0x48, 0x00, 0x6E, 0xFF],
    [0x5C, 0x00, 0x30, 0xFF],
    [0x58, 0x00, 0x00, 0xFF],
    [0x42, 0x14, 0x00, 0xFF],
    [0x24, 0x2C, 0x00, 0xFF],
    [0x06, 0x3C, 0x00, 0xFF],
    [0x00, 0x40, 0x00, 0xFF],
    [0x00, 0x38, 0x20, 0xFF],
    [0x00, 0x2C, 0x5C, 0xFF],
    [0x00, 0x00, 0x00, 0xFF],
    [0x00, 0x00, 0x00, 0xFF],
    [0x00, 0x00, 0x00, 0xFF],
    [0xAB, 0xAB, 0xAB, 0xFF],
    [0x00, 0x5B, 0xB7, 0xFF],
    [0x17, 0x3B, 0xE8, 0xFF],
    [0x54, 0x18, 0xED, 0xFF],
    [0x87, 0x0E, 0xCB, 0xFF],
    [0xA1, 0x0D, 0x7B, 0xFF],
    [0x9E, 0x18, 0x1A, 0xFF],
    [0x82, 0x34, 0x00, 0xFF],
    [0x55, 0x52, 0x00, 0xFF],
    [0x28, 0x68, 0x00, 0xFF],
    [0x08, 0x72, 0x00, 0xFF],
    [0x00, 0x6C, 0x32, 0xFF],
    [0x00, 0x5E, 0x84, 0xFF],
    [0x00, 0x00, 0x00, 0xFF],
    [0x00, 0x00, 0x00, 0xFF],
    [0x00, 0x00, 0x00, 0xFF],
    [0xFF, 0xFF, 0xFF, 0xFF],
    [0x2B, 0xAB, 0xFF, 0xFF],
    [0x57, 0x87, 0xFF, 0xFF],
    [0x8E, 0x63, 0xFF, 0xFF],
    [0xBF, 0x56, 0xFF, 0xFF],
    [0xE0, 0x56, 0xC8, 0xFF],
    [0xDE, 0x60, 0x62, 0xFF],
    [0xCC, 0x7E, 0x12, 0xFF],
    [0xA3, 0x9C, 0x00, 0xFF],
    [0x72, 0xB4, 0x00, 0xFF],
    [0x48, 0xC0, 0x14, 0xFF],
    [0x32, 0xBE, 0x64, 0xFF],
    [0x30, 0xB4, 0xB8, 0xFF],
    [0x3C, 0x3C, 0x3C, 0xFF],
    [0x00, 0x00, 0x00, 0xFF],
    [0x00, 0x00, 0x00, 0xFF],
    [0xFF, 0xFF, 0xFF, 0xFF],
    [0xAB, 0xDB, 0xFF, 0xFF],
    [0xBB, 0xC9, 0xFF, 0xFF],
    [0xD1, 0xBC, 0xFF, 0xFF],
    [0xE6, 0xB6, 0xFF, 0xFF],
    [0xF2, 0xB6, 0xE6, 0xFF],
    [0xF2, 0xBC, 0xBC, 0xFF],
    [0xE8, 0xCA, 0x9E, 0xFF],
    [0xD8, 0xDA, 0x92, 0xFF],
    [0xC2, 0xE4, 0x92, 0xFF],
    [0xAE, 0xEA, 0x9C, 0xFF],
    [0xA4, 0xEA, 0xBC, 0xFF],
    [0xA4, 0xE4, 0xDE, 0xFF],
    [0xA8, 0xA8, 0xA8, 0xFF],
    [0x00, 0x00, 0x00, 0xFF],
    [0x00, 0x00, 0x00, 0xFF],
];
