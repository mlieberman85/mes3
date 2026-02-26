use super::LENGTH_TABLE;

/// Triangle wave channel.
#[derive(Clone, Debug)]
pub struct TriangleChannel {
    pub enabled: bool,
    pub length_counter: u8,
    pub linear_counter: u8,
    pub linear_counter_reload: u8,
    pub linear_counter_reload_flag: bool,
    pub control_flag: bool,
    pub timer_period: u16,
    pub timer_value: u16,
    pub sequence_position: u8,
}

impl TriangleChannel {
    pub fn new() -> Self {
        Self {
            enabled: false,
            length_counter: 0,
            linear_counter: 0,
            linear_counter_reload: 0,
            linear_counter_reload_flag: false,
            control_flag: false,
            timer_period: 0,
            timer_value: 0,
            sequence_position: 0,
        }
    }

    /// Get the current output value (0-15).
    pub fn output(&self) -> u8 {
        if !self.enabled || self.length_counter == 0 || self.linear_counter == 0 {
            return 0;
        }
        TRIANGLE_TABLE[self.sequence_position as usize]
    }

    /// Tick the timer. Called every CPU cycle.
    pub fn tick(&mut self) {
        if self.timer_value == 0 {
            self.timer_value = self.timer_period;
            // Only advance sequencer if both counters are non-zero
            if self.length_counter > 0 && self.linear_counter > 0 {
                self.sequence_position = (self.sequence_position + 1) & 31;
            }
        } else {
            self.timer_value -= 1;
        }
    }

    /// Clock the linear counter (quarter frame).
    pub fn tick_linear_counter(&mut self) {
        if self.linear_counter_reload_flag {
            self.linear_counter = self.linear_counter_reload;
        } else if self.linear_counter > 0 {
            self.linear_counter -= 1;
        }
        // If control flag is clear, clear the reload flag
        if !self.control_flag {
            self.linear_counter_reload_flag = false;
        }
    }

    /// Clock the length counter (half frame).
    pub fn tick_length_counter(&mut self) {
        // control_flag doubles as length counter halt
        if !self.control_flag && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    /// Handle register writes ($4008-$400B).
    /// `offset` is 0-3 corresponding to the 4 registers.
    pub fn write_register(&mut self, offset: u8, val: u8) {
        match offset {
            0 => {
                // $4008: Control flag (bit 7), linear counter reload value (bits 6-0)
                self.control_flag = val & 0x80 != 0;
                self.linear_counter_reload = val & 0x7F;
            }
            1 => {
                // $4009: Unused
            }
            2 => {
                // $400A: Timer low 8 bits
                self.timer_period = (self.timer_period & 0xFF00) | val as u16;
            }
            3 => {
                // $400B: Timer high 3 bits + length counter load
                self.timer_period = (self.timer_period & 0x00FF) | (((val & 0x07) as u16) << 8);
                if self.enabled {
                    self.length_counter = LENGTH_TABLE[((val >> 3) & 0x1F) as usize];
                }
                // Set linear counter reload flag
                self.linear_counter_reload_flag = true;
            }
            _ => {}
        }
    }
}

impl Default for TriangleChannel {
    fn default() -> Self {
        Self::new()
    }
}

/// Triangle waveform sequence (32 steps).
static TRIANGLE_TABLE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    13, 14, 15,
];
