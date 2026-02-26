use super::LENGTH_TABLE;

/// Noise channel -- LFSR-based noise generator.
#[derive(Clone, Debug)]
pub struct NoiseChannel {
    pub enabled: bool,
    pub length_counter: u8,
    pub timer_period: u16,
    pub timer_value: u16,
    pub mode_flag: bool,
    pub shift_register: u16,
    pub envelope_enabled: bool,
    pub envelope_loop: bool,
    pub envelope_start: bool,
    pub envelope_period: u8,
    pub envelope_divider: u8,
    pub envelope_decay: u8,
    pub constant_volume: bool,
    pub volume: u8,
}

impl NoiseChannel {
    pub fn new() -> Self {
        Self {
            enabled: false,
            length_counter: 0,
            timer_period: 0,
            timer_value: 0,
            mode_flag: false,
            shift_register: 1, // Must be non-zero
            envelope_enabled: false,
            envelope_loop: false,
            envelope_start: false,
            envelope_period: 0,
            envelope_divider: 0,
            envelope_decay: 0,
            constant_volume: false,
            volume: 0,
        }
    }

    /// Get the current output value (0-15).
    pub fn output(&self) -> u8 {
        if !self.enabled || self.length_counter == 0 || self.shift_register & 1 != 0 {
            return 0;
        }
        if self.constant_volume {
            self.volume
        } else {
            self.envelope_decay
        }
    }

    /// Tick the timer. Called every other CPU cycle (APU cycle).
    pub fn tick(&mut self) {
        if self.timer_value == 0 {
            self.timer_value = self.timer_period;
            // Clock the LFSR
            let bit0 = self.shift_register & 1;
            let other_bit = if self.mode_flag {
                (self.shift_register >> 6) & 1
            } else {
                (self.shift_register >> 1) & 1
            };
            let feedback = bit0 ^ other_bit;
            self.shift_register >>= 1;
            self.shift_register = (self.shift_register & !(1 << 14)) | (feedback << 14);
        } else {
            self.timer_value -= 1;
        }
    }

    /// Clock the envelope generator (quarter frame).
    pub fn tick_envelope(&mut self) {
        if self.envelope_start {
            self.envelope_start = false;
            self.envelope_decay = 15;
            self.envelope_divider = self.envelope_period;
        } else if self.envelope_divider == 0 {
            self.envelope_divider = self.envelope_period;
            if self.envelope_decay > 0 {
                self.envelope_decay -= 1;
            } else if self.envelope_loop {
                self.envelope_decay = 15;
            }
        } else {
            self.envelope_divider -= 1;
        }
    }

    /// Clock the length counter (half frame).
    pub fn tick_length_counter(&mut self) {
        if !self.envelope_loop && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    /// Handle register writes ($400C-$400F).
    /// `offset` is 0-3 corresponding to the 4 registers.
    pub fn write_register(&mut self, offset: u8, val: u8) {
        match offset {
            0 => {
                // $400C: Length counter halt / envelope loop, constant volume, volume/period
                self.envelope_loop = val & 0x20 != 0;
                self.constant_volume = val & 0x10 != 0;
                self.volume = val & 0x0F;
                self.envelope_period = val & 0x0F;
            }
            1 => {
                // $400D: Unused
            }
            2 => {
                // $400E: Mode flag (bit 7), period index (bits 3-0)
                self.mode_flag = val & 0x80 != 0;
                self.timer_period = NOISE_PERIOD_TABLE[(val & 0x0F) as usize];
            }
            3 => {
                // $400F: Length counter load
                if self.enabled {
                    self.length_counter = LENGTH_TABLE[((val >> 3) & 0x1F) as usize];
                }
                // Restart envelope
                self.envelope_start = true;
            }
            _ => {}
        }
    }
}

impl Default for NoiseChannel {
    fn default() -> Self {
        Self::new()
    }
}

/// NTSC noise timer period lookup table.
pub static NOISE_PERIOD_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];
