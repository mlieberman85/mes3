#![no_std]

extern crate alloc;

pub mod apu;
pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod mappers;
pub mod ppu;

use alloc::vec::Vec;
use bus::Bus;
use cpu::Cpu;
use mappers::Mirroring;

/// Top-level NES emulator struct.
pub struct Nes {
    pub cpu: Cpu,
    pub bus: Bus,
    frame_count: u64,
    /// Fractional cycle accumulator for accurate frame timing.
    cycle_accumulator: f64,
}

/// Cycles per NTSC frame (approximate — alternates 29780/29781).
const CYCLES_PER_FRAME: f64 = 29780.67;

impl Nes {
    pub fn new() -> Self {
        Self {
            cpu: Cpu::new(),
            bus: Bus::new(),
            frame_count: 0,
            cycle_accumulator: 0.0,
        }
    }

    /// Load a ROM from raw bytes. Returns Ok(()) on success.
    pub fn load_rom(&mut self, data: &[u8]) -> Result<(), cartridge::RomLoadError> {
        let cart = cartridge::Cartridge::from_bytes(data)?;
        self.bus.load_cartridge(cart);
        self.reset();
        Ok(())
    }

    /// Reset the CPU/PPU/APU to power-on state.
    pub fn reset(&mut self) {
        self.cpu.reset(&mut self.bus);
        self.bus.ppu.reset();
        self.bus.apu.reset();
        self.frame_count = 0;
        self.cycle_accumulator = 0.0;
    }

    /// Run one full NTSC frame using the catch-up model.
    /// Returns false if no ROM is loaded.
    pub fn run_frame(&mut self) -> bool {
        if !self.bus.rom_loaded() {
            return false;
        }

        let target = CYCLES_PER_FRAME + self.cycle_accumulator;
        let target_cycles = target as u64;
        self.cycle_accumulator = target - target_cycles as f64;

        let mut cycles_this_frame: u64 = 0;
        while cycles_this_frame < target_cycles {
            let cpu_cycles = self.cpu.step(&mut self.bus) as u64;

            // Step PPU: temporarily extract cartridge to avoid borrow conflicts
            let ppu_dots = cpu_cycles * 3;
            let mut cart = self.bus.cartridge.take();
            let mirroring = cart
                .as_ref()
                .map(|c| c.mirroring())
                .unwrap_or(Mirroring::Horizontal);
            {
                let chr_read = |addr: u16| -> u8 {
                    cart.as_ref()
                        .map(|c| c.mapper.read_chr(&c.chr_rom, addr))
                        .unwrap_or(0)
                };
                self.bus.ppu.step(ppu_dots, mirroring, &chr_read);
            }
            // Notify mapper of scanlines for IRQ clocking (MMC3)
            let scanline_ticks = self.bus.ppu.take_scanline_irq_ticks();
            if scanline_ticks > 0 {
                if let Some(ref mut c) = cart {
                    for _ in 0..scanline_ticks {
                        c.notify_scanline();
                    }
                }
            }
            self.bus.cartridge = cart;

            // Step APU with memory read closure for DMC sample playback.
            // We borrow the non-APU parts of the bus for DMC reads.
            {
                let ram = &self.bus.ram;
                let cart = &self.bus.cartridge;
                let mut dmc_reader = |addr: u16| -> u8 {
                    match addr {
                        0x0000..=0x1FFF => ram[(addr & 0x07FF) as usize],
                        0x4020..=0xFFFF => cart.as_ref().map(|c| c.read_prg(addr)).unwrap_or(0),
                        _ => 0,
                    }
                };
                self.bus.apu.step(cpu_cycles, &mut dmc_reader);
            }

            // Check for NMI from PPU
            if self.bus.ppu.take_nmi() {
                self.cpu.nmi(&mut self.bus);
            }

            // Check for IRQ from mapper or APU
            if self.bus.irq_pending() && !self.cpu.irq_disabled() {
                self.cpu.irq(&mut self.bus);
            }

            cycles_this_frame += cpu_cycles;
        }

        self.frame_count += 1;
        true
    }

    /// Get the RGBA frame buffer (256x240x4 = 245760 bytes).
    pub fn frame_buffer(&self) -> &[u8] {
        &self.bus.ppu.frame_buffer
    }

    /// Consume audio samples generated this frame.
    pub fn audio_buffer(&mut self) -> Vec<f32> {
        self.bus.apu.take_samples()
    }

    /// Set a controller button state.
    pub fn set_button_state(&mut self, button: u8, pressed: bool) {
        self.bus.controller.set_button(button, pressed);
    }

    /// Get the current frame count.
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get total CPU cycles elapsed.
    pub fn cpu_cycles(&self) -> u64 {
        self.cpu.cycles
    }

    /// Save the complete emulator state to binary.
    pub fn save_state(&self) -> Vec<u8> {
        let mut state = Vec::new();
        self.cpu.save_state(&mut state);
        self.bus.save_state(&mut state);
        state
    }

    /// Load emulator state from binary. Returns false on invalid data.
    pub fn load_state(&mut self, data: &[u8]) -> bool {
        let mut cursor = 0;
        if !self.cpu.load_state(data, &mut cursor) {
            return false;
        }
        if !self.bus.load_state(data, &mut cursor) {
            return false;
        }
        true
    }
}

impl Default for Nes {
    fn default() -> Self {
        Self::new()
    }
}
