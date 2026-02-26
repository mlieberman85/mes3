pub mod cnrom;
pub mod gxrom;
pub mod mmc1;
pub mod mmc3;
pub mod nrom;
pub mod uxrom;

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::cartridge::{INesHeader, RomLoadError};

/// Nametable mirroring mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    FourScreen,
    SingleScreenLo,
    SingleScreenHi,
}

/// Trait for cartridge mappers.
pub trait Mapper: Send {
    /// Read from PRG-ROM address space ($8000-$FFFF).
    fn read_prg(&self, prg_rom: &[u8], addr: u16) -> u8;
    /// Write to PRG-ROM address space (bank switching registers).
    fn write_prg(&mut self, addr: u16, val: u8);
    /// Read from CHR address space ($0000-$1FFF).
    fn read_chr(&self, chr_data: &[u8], addr: u16) -> u8;
    /// Write to CHR address space (CHR-RAM only).
    fn write_chr(&self, chr_data: &mut Vec<u8>, addr: u16, val: u8);
    /// Get current mirroring mode.
    fn mirroring(&self) -> Mirroring;
    /// Check for pending IRQ (mapper 4 only).
    fn irq_pending(&self) -> bool {
        false
    }
    /// Notify mapper of scanline (for scanline counters).
    fn notify_scanline(&mut self) {}
    /// Save mapper state.
    fn save_state(&self) -> Vec<u8>;
    /// Load mapper state.
    fn load_state(&mut self, data: &[u8]);
}

/// Factory function: create a mapper by number.
pub fn create_mapper(
    number: u8,
    header: &INesHeader,
    prg_rom: &[u8],
    _chr_rom: &[u8],
) -> Result<Box<dyn Mapper>, RomLoadError> {
    match number {
        0 => Ok(Box::new(nrom::Nrom::new(header, prg_rom))),
        1 => Ok(Box::new(mmc1::Mmc1::new(header, prg_rom))),
        2 => Ok(Box::new(uxrom::UxRom::new(header, prg_rom))),
        3 => Ok(Box::new(cnrom::Cnrom::new(header))),
        4 => Ok(Box::new(mmc3::Mmc3::new(header, prg_rom))),
        66 => Ok(Box::new(gxrom::GxRom::new(header))),
        _ => Err(RomLoadError::UnsupportedMapper(number)),
    }
}
