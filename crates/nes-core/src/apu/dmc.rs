/// Delta Modulation Channel (DMC).
#[derive(Clone, Debug)]
pub struct DmcChannel {
    pub enabled: bool,
    pub irq_enabled: bool,
    pub loop_flag: bool,
    pub rate_index: u8,
    pub timer_period: u16,
    pub timer_value: u16,
    pub output_level: u8,
    pub sample_address: u16,
    pub sample_length: u16,
    pub current_address: u16,
    pub bytes_remaining: u16,
    pub sample_buffer: Option<u8>,
    pub shift_register: u8,
    pub bits_remaining: u8,
    pub silence_flag: bool,
    /// IRQ flag set by DMC (read by APU).
    pub irq_flag: bool,
}

impl DmcChannel {
    pub fn new() -> Self {
        Self {
            enabled: false,
            irq_enabled: false,
            loop_flag: false,
            rate_index: 0,
            timer_period: DMC_RATE_TABLE[0],
            timer_value: 0,
            output_level: 0,
            sample_address: 0xC000,
            sample_length: 1,
            current_address: 0xC000,
            bytes_remaining: 0,
            sample_buffer: None,
            shift_register: 0,
            bits_remaining: 8,
            silence_flag: true,
            irq_flag: false,
        }
    }

    /// Get the current output value (0-127).
    pub fn output(&self) -> u8 {
        self.output_level
    }

    /// Tick the DMC timer. Called every CPU cycle.
    /// `read_memory` is used to fetch sample bytes from CPU memory.
    pub fn tick(&mut self, read_memory: &mut dyn FnMut(u16) -> u8) {
        // Try to fill the sample buffer if empty
        self.fill_buffer(read_memory);

        if self.timer_value == 0 {
            self.timer_value = self.timer_period;

            if !self.silence_flag {
                // Output unit: adjust output level based on bit 0 of shift register
                if self.shift_register & 1 != 0 {
                    if self.output_level <= 125 {
                        self.output_level += 2;
                    }
                } else if self.output_level >= 2 {
                    self.output_level -= 2;
                }
            }

            // Shift register clocking
            self.shift_register >>= 1;
            self.bits_remaining = self.bits_remaining.saturating_sub(1);

            if self.bits_remaining == 0 {
                self.bits_remaining = 8;
                if let Some(buffer) = self.sample_buffer.take() {
                    self.silence_flag = false;
                    self.shift_register = buffer;
                } else {
                    self.silence_flag = true;
                }
            }
        } else {
            self.timer_value -= 1;
        }
    }

    /// Attempt to fill the sample buffer from memory if it is empty and there are bytes remaining.
    pub fn fill_buffer(&mut self, read_memory: &mut dyn FnMut(u16) -> u8) {
        if self.sample_buffer.is_none() && self.bytes_remaining > 0 {
            self.sample_buffer = Some(read_memory(self.current_address));
            // Address wraps from $FFFF to $8000
            if self.current_address == 0xFFFF {
                self.current_address = 0x8000;
            } else {
                self.current_address += 1;
            }
            self.bytes_remaining -= 1;

            if self.bytes_remaining == 0 {
                if self.loop_flag {
                    // Restart sample
                    self.current_address = self.sample_address;
                    self.bytes_remaining = self.sample_length;
                } else if self.irq_enabled {
                    self.irq_flag = true;
                }
            }
        }
    }

    /// Handle register writes ($4010-$4013).
    /// `offset` is 0-3 corresponding to the 4 registers.
    pub fn write_register(&mut self, offset: u8, val: u8) {
        match offset {
            0 => {
                // $4010: IRQ enable (bit 7), loop (bit 6), rate index (bits 3-0)
                self.irq_enabled = val & 0x80 != 0;
                self.loop_flag = val & 0x40 != 0;
                self.rate_index = val & 0x0F;
                self.timer_period = DMC_RATE_TABLE[self.rate_index as usize];
                // If IRQ flag is cleared by disabling IRQ
                if !self.irq_enabled {
                    self.irq_flag = false;
                }
            }
            1 => {
                // $4011: Direct load output level (bits 6-0)
                self.output_level = val & 0x7F;
            }
            2 => {
                // $4012: Sample address = $C000 + val * 64
                self.sample_address = 0xC000 + (val as u16) * 64;
            }
            3 => {
                // $4013: Sample length = val * 16 + 1
                self.sample_length = (val as u16) * 16 + 1;
            }
            _ => {}
        }
    }
}

impl Default for DmcChannel {
    fn default() -> Self {
        Self::new()
    }
}

/// NTSC DMC rate table (in CPU cycles).
pub static DMC_RATE_TABLE: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];
