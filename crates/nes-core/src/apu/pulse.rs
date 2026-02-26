use super::LENGTH_TABLE;

/// Pulse/square wave channel. Used for pulse1 and pulse2.
#[derive(Clone, Debug)]
pub struct PulseChannel {
    pub enabled: bool,
    pub length_counter: u8,
    pub timer_period: u16,
    pub timer_value: u16,
    pub duty_cycle: u8,
    pub duty_position: u8,
    pub envelope_enabled: bool,
    pub envelope_loop: bool,
    pub envelope_start: bool,
    pub envelope_period: u8,
    pub envelope_divider: u8,
    pub envelope_decay: u8,
    pub constant_volume: bool,
    pub volume: u8,
    pub sweep_enabled: bool,
    pub sweep_period: u8,
    pub sweep_negate: bool,
    pub sweep_shift: u8,
    pub sweep_reload: bool,
    pub sweep_divider: u8,
    /// Whether this is pulse channel 2 (affects sweep negate behavior).
    pub is_channel2: bool,
}

impl PulseChannel {
    pub fn new(is_channel2: bool) -> Self {
        Self {
            enabled: false,
            length_counter: 0,
            timer_period: 0,
            timer_value: 0,
            duty_cycle: 0,
            duty_position: 0,
            envelope_enabled: false,
            envelope_loop: false,
            envelope_start: false,
            envelope_period: 0,
            envelope_divider: 0,
            envelope_decay: 0,
            constant_volume: false,
            volume: 0,
            sweep_enabled: false,
            sweep_period: 0,
            sweep_negate: false,
            sweep_shift: 0,
            sweep_reload: false,
            sweep_divider: 0,
            is_channel2,
        }
    }

    /// Get the current output value (0-15).
    pub fn output(&self) -> u8 {
        if !self.enabled || self.length_counter == 0 || self.timer_period < 8 {
            return 0;
        }
        let duty = DUTY_TABLE[self.duty_cycle as usize][self.duty_position as usize];
        if duty == 0 {
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
            self.duty_position = (self.duty_position + 1) & 7;
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

    /// Compute the target period for the sweep unit.
    pub fn target_period(&self) -> u16 {
        let shift_amount = self.timer_period >> self.sweep_shift;
        if self.sweep_negate {
            if self.is_channel2 {
                // Channel 2: two's complement (subtract shift_amount)
                self.timer_period.wrapping_sub(shift_amount)
            } else {
                // Channel 1: one's complement (subtract shift_amount + 1, i.e. bitwise NOT)
                self.timer_period.wrapping_sub(shift_amount).wrapping_sub(1)
            }
        } else {
            self.timer_period.wrapping_add(shift_amount)
        }
    }

    /// Clock the sweep unit (half frame).
    pub fn tick_sweep(&mut self) {
        let target = self.target_period();

        if self.sweep_divider == 0
            && self.sweep_enabled
            && self.sweep_shift > 0
            && self.timer_period >= 8
            && target <= 0x7FF
        {
            self.timer_period = target;
        }

        if self.sweep_divider == 0 || self.sweep_reload {
            self.sweep_divider = self.sweep_period;
            self.sweep_reload = false;
        } else {
            self.sweep_divider -= 1;
        }
    }

    /// Handle register writes ($4000-$4003 for pulse1, $4004-$4007 for pulse2).
    /// `offset` is 0-3 corresponding to the 4 registers.
    pub fn write_register(&mut self, offset: u8, val: u8) {
        match offset {
            0 => {
                // $4000/$4004: Duty, length counter halt / envelope loop, constant volume, volume/period
                self.duty_cycle = (val >> 6) & 0x03;
                self.envelope_loop = val & 0x20 != 0;
                self.constant_volume = val & 0x10 != 0;
                self.volume = val & 0x0F;
                self.envelope_period = val & 0x0F;
            }
            1 => {
                // $4001/$4005: Sweep enable, period, negate, shift
                self.sweep_enabled = val & 0x80 != 0;
                self.sweep_period = (val >> 4) & 0x07;
                self.sweep_negate = val & 0x08 != 0;
                self.sweep_shift = val & 0x07;
                self.sweep_reload = true;
            }
            2 => {
                // $4002/$4006: Timer low 8 bits
                self.timer_period = (self.timer_period & 0xFF00) | val as u16;
            }
            3 => {
                // $4003/$4007: Timer high 3 bits + length counter load
                self.timer_period = (self.timer_period & 0x00FF) | (((val & 0x07) as u16) << 8);
                if self.enabled {
                    self.length_counter = LENGTH_TABLE[((val >> 3) & 0x1F) as usize];
                }
                // Restart duty position and envelope
                self.duty_position = 0;
                self.envelope_start = true;
            }
            _ => {}
        }
    }
}

/// Duty cycle waveform lookup table.
static DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 12.5%
    [0, 0, 0, 0, 0, 0, 1, 1], // 25%
    [0, 0, 0, 0, 1, 1, 1, 1], // 50%
    [1, 1, 1, 1, 1, 1, 0, 0], // 75% (inverted 25%)
];
