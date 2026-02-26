pub mod dmc;
pub mod noise;
pub mod pulse;
pub mod triangle;

use alloc::vec::Vec;

use dmc::DmcChannel;
use noise::NoiseChannel;
use pulse::PulseChannel;
use triangle::TriangleChannel;

/// Length counter lookup table (used when writing register 3 of pulse/triangle/noise).
pub static LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

/// Frame counter cycle thresholds for NTSC (4-step / mode 0).
const FRAME_COUNTER_STEPS_MODE0: [u64; 4] = [7457, 14913, 22371, 29829];

/// Frame counter cycle thresholds for NTSC (5-step / mode 1).
const FRAME_COUNTER_STEPS_MODE1: [u64; 5] = [7457, 14913, 22371, 29829, 37281];

/// NES APU (2A03) state.
#[derive(Clone)]
pub struct Apu {
    pub pulse1: PulseChannel,
    pub pulse2: PulseChannel,
    pub triangle: TriangleChannel,
    pub noise: NoiseChannel,
    pub dmc: DmcChannel,
    /// Frame counter mode (false = 4-step, true = 5-step).
    pub frame_counter_mode: bool,
    /// IRQ inhibit flag.
    pub irq_inhibit: bool,
    /// CPU cycle counter for frame counter timing.
    cycle_count: u64,
    /// Audio sample buffer for current frame.
    sample_buffer: Vec<f32>,
    /// Sample rate (48000 Hz).
    _sample_rate: u32,
    /// Sample accumulator for downsampling.
    sample_accumulator: f64,
    /// CPU cycles per sample.
    cycles_per_sample: f64,
    /// Frame IRQ pending.
    pub frame_irq: bool,
    /// DMC IRQ pending.
    pub dmc_irq: bool,
    /// Status register ($4015) enabled channels.
    pub status: u8,
    /// Even/odd cycle tracking for APU cycle (pulse/noise tick on even).
    even_cycle: bool,
}

impl Apu {
    pub fn new() -> Self {
        let sample_rate = 48000;
        let cpu_clock = 1_789_773.0;
        Self {
            pulse1: PulseChannel::new(false),
            pulse2: PulseChannel::new(true),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            dmc: DmcChannel::new(),
            frame_counter_mode: false,
            irq_inhibit: false,
            cycle_count: 0,
            sample_buffer: Vec::with_capacity(1024),
            _sample_rate: sample_rate,
            sample_accumulator: 0.0,
            cycles_per_sample: cpu_clock / sample_rate as f64,
            frame_irq: false,
            dmc_irq: false,
            status: 0,
            even_cycle: false,
        }
    }

    pub fn reset(&mut self) {
        self.pulse1 = PulseChannel::new(false);
        self.pulse2 = PulseChannel::new(true);
        self.triangle = TriangleChannel::new();
        self.noise = NoiseChannel::new();
        self.dmc = DmcChannel::new();
        self.frame_counter_mode = false;
        self.irq_inhibit = false;
        self.cycle_count = 0;
        self.sample_buffer.clear();
        self.sample_accumulator = 0.0;
        self.frame_irq = false;
        self.dmc_irq = false;
        self.status = 0;
        self.even_cycle = false;
    }

    /// Advance the APU by the given number of CPU cycles.
    /// `read_memory` is used by DMC for sample playback.
    pub fn step(&mut self, cpu_cycles: u64, read_memory: &mut dyn FnMut(u16) -> u8) {
        for _ in 0..cpu_cycles {
            self.cycle_count += 1;
            self.even_cycle = !self.even_cycle;

            // Triangle timer ticks every CPU cycle
            self.triangle.tick();

            // DMC timer ticks every CPU cycle
            self.dmc.tick(read_memory);

            // Pulse and noise timers tick every other CPU cycle (APU cycle)
            if self.even_cycle {
                self.pulse1.tick();
                self.pulse2.tick();
                self.noise.tick();
            }

            // Frame counter
            self.tick_frame_counter();

            // Sync DMC IRQ flag
            self.dmc_irq = self.dmc.irq_flag;

            // Downsample: accumulate and output samples at 48kHz
            self.sample_accumulator += 1.0;
            if self.sample_accumulator >= self.cycles_per_sample {
                self.sample_accumulator -= self.cycles_per_sample;
                let sample = self.mix();
                self.sample_buffer.push(sample);
            }
        }
    }

    /// Handle frame counter timing and generate quarter/half frame clocks.
    fn tick_frame_counter(&mut self) {
        if !self.frame_counter_mode {
            // 4-step mode (mode 0)
            match self.cycle_count {
                c if c == FRAME_COUNTER_STEPS_MODE0[0] => {
                    // Step 1: quarter frame
                    self.quarter_frame();
                }
                c if c == FRAME_COUNTER_STEPS_MODE0[1] => {
                    // Step 2: quarter frame + half frame
                    self.quarter_frame();
                    self.half_frame();
                }
                c if c == FRAME_COUNTER_STEPS_MODE0[2] => {
                    // Step 3: quarter frame
                    self.quarter_frame();
                }
                c if c == FRAME_COUNTER_STEPS_MODE0[3] => {
                    // Step 4: quarter frame + half frame + IRQ
                    self.quarter_frame();
                    self.half_frame();
                    if !self.irq_inhibit {
                        self.frame_irq = true;
                    }
                    self.cycle_count = 0;
                }
                _ => {}
            }
        } else {
            // 5-step mode (mode 1)
            match self.cycle_count {
                c if c == FRAME_COUNTER_STEPS_MODE1[0] => {
                    // Step 1: quarter frame
                    self.quarter_frame();
                }
                c if c == FRAME_COUNTER_STEPS_MODE1[1] => {
                    // Step 2: quarter frame + half frame
                    self.quarter_frame();
                    self.half_frame();
                }
                c if c == FRAME_COUNTER_STEPS_MODE1[2] => {
                    // Step 3: quarter frame
                    self.quarter_frame();
                }
                c if c == FRAME_COUNTER_STEPS_MODE1[3] => {
                    // Step 4: nothing (no IRQ in 5-step mode)
                }
                c if c == FRAME_COUNTER_STEPS_MODE1[4] => {
                    // Step 5: quarter frame + half frame
                    self.quarter_frame();
                    self.half_frame();
                    self.cycle_count = 0;
                }
                _ => {}
            }
        }
    }

    /// Quarter frame: clock envelopes and triangle linear counter.
    fn quarter_frame(&mut self) {
        self.pulse1.tick_envelope();
        self.pulse2.tick_envelope();
        self.noise.tick_envelope();
        self.triangle.tick_linear_counter();
    }

    /// Half frame: clock length counters and sweep units.
    fn half_frame(&mut self) {
        self.pulse1.tick_length_counter();
        self.pulse2.tick_length_counter();
        self.triangle.tick_length_counter();
        self.noise.tick_length_counter();
        self.pulse1.tick_sweep();
        self.pulse2.tick_sweep();
    }

    /// Write to APU register ($4000-$4017).
    pub fn write_register(&mut self, addr: u16, val: u8) {
        match addr {
            // Pulse 1: $4000-$4003
            0x4000..=0x4003 => {
                self.pulse1.write_register((addr - 0x4000) as u8, val);
            }
            // Pulse 2: $4004-$4007
            0x4004..=0x4007 => {
                self.pulse2.write_register((addr - 0x4004) as u8, val);
            }
            // Triangle: $4008-$400B
            0x4008..=0x400B => {
                self.triangle.write_register((addr - 0x4008) as u8, val);
            }
            // Noise: $400C-$400F
            0x400C..=0x400F => {
                self.noise.write_register((addr - 0x400C) as u8, val);
            }
            // DMC: $4010-$4013
            0x4010..=0x4013 => {
                self.dmc.write_register((addr - 0x4010) as u8, val);
            }
            // Status: $4015
            0x4015 => {
                self.status = val;
                self.pulse1.enabled = val & 0x01 != 0;
                self.pulse2.enabled = val & 0x02 != 0;
                self.triangle.enabled = val & 0x04 != 0;
                self.noise.enabled = val & 0x08 != 0;
                self.dmc.enabled = val & 0x10 != 0;

                // If channel disabled, zero its length counter
                if !self.pulse1.enabled {
                    self.pulse1.length_counter = 0;
                }
                if !self.pulse2.enabled {
                    self.pulse2.length_counter = 0;
                }
                if !self.triangle.enabled {
                    self.triangle.length_counter = 0;
                }
                if !self.noise.enabled {
                    self.noise.length_counter = 0;
                }

                // DMC: if disabled, set bytes_remaining to 0.
                // If enabled and bytes_remaining is 0, restart sample.
                if !self.dmc.enabled {
                    self.dmc.bytes_remaining = 0;
                } else if self.dmc.bytes_remaining == 0 {
                    self.dmc.current_address = self.dmc.sample_address;
                    self.dmc.bytes_remaining = self.dmc.sample_length;
                }

                // Clear DMC IRQ flag
                self.dmc.irq_flag = false;
                self.dmc_irq = false;
            }
            // Frame counter: $4017
            0x4017 => {
                self.frame_counter_mode = val & 0x80 != 0;
                self.irq_inhibit = val & 0x40 != 0;
                if self.irq_inhibit {
                    self.frame_irq = false;
                }
                // Reset frame counter
                self.cycle_count = 0;
                // In 5-step mode, immediately clock quarter and half frame
                if self.frame_counter_mode {
                    self.quarter_frame();
                    self.half_frame();
                }
            }
            _ => {}
        }
    }

    /// Read the APU status register ($4015).
    /// Reading clears the frame IRQ flag.
    pub fn read_status(&mut self) -> u8 {
        let mut result = 0u8;
        if self.pulse1.length_counter > 0 {
            result |= 0x01;
        }
        if self.pulse2.length_counter > 0 {
            result |= 0x02;
        }
        if self.triangle.length_counter > 0 {
            result |= 0x04;
        }
        if self.noise.length_counter > 0 {
            result |= 0x08;
        }
        if self.dmc.bytes_remaining > 0 {
            result |= 0x10;
        }
        if self.frame_irq {
            result |= 0x40;
        }
        if self.dmc_irq {
            result |= 0x80;
        }
        // Reading $4015 clears the frame IRQ flag
        self.frame_irq = false;
        result
    }

    /// Mix all channels into a single output sample using NES non-linear mixing.
    pub fn mix(&self) -> f32 {
        let p1 = self.pulse1.output() as f32;
        let p2 = self.pulse2.output() as f32;
        let t = self.triangle.output() as f32;
        let n = self.noise.output() as f32;
        let d = self.dmc.output() as f32;

        // Pulse mixer (non-linear)
        let pulse_sum = p1 + p2;
        let pulse_out = if pulse_sum > 0.0 {
            95.88 / (8128.0 / pulse_sum + 100.0)
        } else {
            0.0
        };

        // TND mixer (non-linear)
        let tnd_sum = t / 8227.0 + n / 12241.0 + d / 22638.0;
        let tnd_out = if tnd_sum > 0.0 {
            159.79 / (1.0 / tnd_sum + 100.0)
        } else {
            0.0
        };

        pulse_out + tnd_out
    }

    /// Take (consume) the audio samples generated since last call.
    pub fn take_samples(&mut self) -> Vec<f32> {
        let mut samples = Vec::with_capacity(self.sample_buffer.len());
        core::mem::swap(&mut samples, &mut self.sample_buffer);
        samples
    }

    /// Check if any APU IRQ is pending.
    pub fn irq_pending(&self) -> bool {
        self.frame_irq || self.dmc_irq
    }

    /// Save APU state.
    pub fn save_state(&self, buf: &mut Vec<u8>) {
        // Minimal save state -- full implementation in Phase 5
        buf.push(self.status);
        buf.push(self.frame_counter_mode as u8);
        buf.push(self.irq_inhibit as u8);
        buf.extend_from_slice(&self.cycle_count.to_le_bytes());
    }

    /// Load APU state.
    pub fn load_state(&mut self, data: &[u8], cursor: &mut usize) -> bool {
        if *cursor + 11 > data.len() {
            return false;
        }
        self.status = data[*cursor];
        *cursor += 1;
        self.frame_counter_mode = data[*cursor] != 0;
        *cursor += 1;
        self.irq_inhibit = data[*cursor] != 0;
        *cursor += 1;
        self.cycle_count = u64::from_le_bytes(data[*cursor..*cursor + 8].try_into().unwrap());
        *cursor += 8;
        true
    }
}

impl Default for Apu {
    fn default() -> Self {
        Self::new()
    }
}
