use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::mappers::{self, Mapper, Mirroring};

/// Errors that can occur when loading a ROM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RomLoadError {
    /// Not a valid iNES file (bad magic bytes or truncated).
    InvalidFormat,
    /// Mapper number is not supported.
    UnsupportedMapper(u8),
    /// ROM is PAL region (not supported).
    PalNotSupported,
}

/// Parsed iNES header.
#[derive(Debug, Clone)]
pub struct INesHeader {
    pub prg_rom_banks: u8,
    pub chr_rom_banks: u8,
    pub mapper_number: u8,
    pub mirroring: Mirroring,
    pub has_battery_ram: bool,
    pub has_trainer: bool,
}

/// A loaded NES cartridge.
pub struct Cartridge {
    pub header: INesHeader,
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    /// Whether CHR data is RAM (chr_rom_banks == 0 in header).
    pub chr_is_ram: bool,
    pub sram: [u8; 8192],
    pub mapper: Box<dyn Mapper>,
}

impl Cartridge {
    /// Parse an iNES ROM from raw bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, RomLoadError> {
        // Validate header magic
        if data.len() < 16 || &data[0..4] != b"NES\x1A" {
            return Err(RomLoadError::InvalidFormat);
        }

        let prg_rom_banks = data[4];
        let chr_rom_banks = data[5];
        let flags6 = data[6];
        let flags7 = data[7];

        // Parse mapper number (upper nybbles of flags6 and flags7)
        let mapper_number = (flags6 >> 4) | (flags7 & 0xF0);

        // Parse mirroring
        let four_screen = flags6 & 0x08 != 0;
        let mirroring = if four_screen {
            Mirroring::FourScreen
        } else if flags6 & 0x01 != 0 {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };

        let has_battery_ram = flags6 & 0x02 != 0;
        let has_trainer = flags6 & 0x04 != 0;

        // Check for PAL flag (flags9 bit 0 or flags10 bits)
        if data.len() > 9 && data[9] & 0x01 != 0 {
            return Err(RomLoadError::PalNotSupported);
        }

        let header = INesHeader {
            prg_rom_banks,
            chr_rom_banks,
            mapper_number,
            mirroring,
            has_battery_ram,
            has_trainer,
        };

        // Calculate offsets
        let trainer_size = if has_trainer { 512 } else { 0 };
        let prg_size = prg_rom_banks as usize * 16384;
        let chr_size = chr_rom_banks as usize * 8192;

        let prg_start = 16 + trainer_size;
        let prg_end = prg_start + prg_size;
        let chr_start = prg_end;
        let chr_end = chr_start + chr_size;

        // Validate data length
        if data.len() < chr_end {
            return Err(RomLoadError::InvalidFormat);
        }

        let prg_rom = data[prg_start..prg_end].to_vec();

        let chr_is_ram = chr_rom_banks == 0;
        let chr_rom = if chr_is_ram {
            // CHR-RAM: allocate 8KB
            alloc::vec![0u8; 8192]
        } else {
            data[chr_start..chr_end].to_vec()
        };

        // Create mapper
        let mapper = mappers::create_mapper(mapper_number, &header, &prg_rom, &chr_rom)?;

        Ok(Cartridge {
            header,
            prg_rom,
            chr_rom,
            chr_is_ram,
            sram: [0; 8192],
            mapper,
        })
    }

    /// Read from PRG address space.
    pub fn read_prg(&self, addr: u16) -> u8 {
        if (0x6000..0x8000).contains(&addr) {
            // SRAM
            self.sram[(addr - 0x6000) as usize]
        } else {
            self.mapper.read_prg(&self.prg_rom, addr)
        }
    }

    /// Write to PRG address space.
    pub fn write_prg(&mut self, addr: u16, val: u8) {
        if (0x6000..0x8000).contains(&addr) {
            // SRAM
            self.sram[(addr - 0x6000) as usize] = val;
        } else {
            self.mapper.write_prg(addr, val);
        }
    }

    /// Read from CHR address space.
    pub fn read_chr(&self, addr: u16) -> u8 {
        self.mapper.read_chr(&self.chr_rom, addr)
    }

    /// Write to CHR address space (CHR-RAM only).
    pub fn write_chr(&mut self, addr: u16, val: u8) {
        if self.chr_is_ram {
            self.mapper.write_chr(&mut self.chr_rom, addr, val);
        }
    }

    /// Get the current nametable mirroring mode.
    pub fn mirroring(&self) -> Mirroring {
        self.mapper.mirroring()
    }

    /// Check if the mapper has an IRQ pending.
    pub fn irq_pending(&self) -> bool {
        self.mapper.irq_pending()
    }

    /// Notify the mapper of a scanline (for MMC3 IRQ counter).
    pub fn notify_scanline(&mut self) {
        self.mapper.notify_scanline();
    }
}
