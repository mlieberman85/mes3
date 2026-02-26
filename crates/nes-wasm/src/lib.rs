use wasm_bindgen::prelude::*;

use nes_core::cartridge::RomLoadError;
use nes_core::Nes;

/// WASM-exposed NES emulator wrapper.
#[wasm_bindgen]
pub struct Emulator {
    nes: Nes,
}

/// Result of loading a ROM.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomLoadResult {
    Ok = 0,
    InvalidFormat = 1,
    UnsupportedMapper = 2,
    PalNotSupported = 3,
}

/// NES controller button identifiers.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    A = 0,
    B = 1,
    Select = 2,
    Start = 3,
    Up = 4,
    Down = 5,
    Left = 6,
    Right = 7,
}

/// Diagnostic info snapshot.
#[wasm_bindgen]
pub struct EmulatorInfo {
    version: String,
    #[wasm_bindgen(readonly)]
    pub rom_loaded: bool,
    #[wasm_bindgen(readonly)]
    pub mapper_number: u8,
    #[wasm_bindgen(readonly)]
    pub cpu_cycles: u64,
    #[wasm_bindgen(readonly)]
    pub frame_count: u64,
    #[wasm_bindgen(readonly)]
    pub fps: f64,
}

#[wasm_bindgen]
impl EmulatorInfo {
    #[wasm_bindgen(getter)]
    pub fn version(&self) -> String {
        self.version.clone()
    }
}

#[wasm_bindgen]
impl Emulator {
    /// Create a new emulator instance.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { nes: Nes::new() }
    }

    /// Load a ROM from raw bytes.
    pub fn load_rom(&mut self, data: &[u8]) -> RomLoadResult {
        match self.nes.load_rom(data) {
            Ok(()) => RomLoadResult::Ok,
            Err(RomLoadError::InvalidFormat) => RomLoadResult::InvalidFormat,
            Err(RomLoadError::UnsupportedMapper(_)) => RomLoadResult::UnsupportedMapper,
            Err(RomLoadError::PalNotSupported) => RomLoadResult::PalNotSupported,
        }
    }

    /// Reset CPU/PPU/APU to power-on state.
    pub fn reset(&mut self) {
        self.nes.reset();
    }

    /// Run one full NTSC frame. Returns false if no ROM loaded.
    pub fn run_frame(&mut self) -> bool {
        self.nes.run_frame()
    }

    /// Pointer to the RGBA frame buffer in WASM memory.
    pub fn frame_buffer_ptr(&self) -> *const u8 {
        self.nes.frame_buffer().as_ptr()
    }

    /// Consume audio samples generated this frame.
    pub fn audio_buffer(&mut self) -> Vec<f32> {
        self.nes.audio_buffer()
    }

    /// Set a controller button pressed/released.
    pub fn set_button_state(&mut self, button: Button, pressed: bool) {
        self.nes.set_button_state(button as u8, pressed);
    }

    /// Serialize complete emulator state.
    pub fn save_state(&self) -> Vec<u8> {
        self.nes.save_state()
    }

    /// Restore state from binary. Returns false on invalid data.
    pub fn load_state(&mut self, data: &[u8]) -> bool {
        self.nes.load_state(data)
    }

    /// Return current diagnostic snapshot.
    pub fn get_info(&self) -> EmulatorInfo {
        EmulatorInfo {
            version: String::from(env!("CARGO_PKG_VERSION")),
            rom_loaded: self.nes.bus.rom_loaded(),
            mapper_number: 0, // TODO: expose from cartridge
            cpu_cycles: self.nes.cpu_cycles(),
            frame_count: self.nes.frame_count(),
            fps: 0.0, // Measured by frontend
        }
    }
}

impl Default for Emulator {
    fn default() -> Self {
        Self::new()
    }
}
