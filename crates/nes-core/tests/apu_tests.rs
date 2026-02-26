use nes_core::apu::noise::NoiseChannel;
use nes_core::apu::pulse::PulseChannel;
use nes_core::apu::triangle::TriangleChannel;
use nes_core::apu::Apu;

/// Dummy memory reader that returns 0 for all addresses.
fn null_reader(_addr: u16) -> u8 {
    0
}

// ──────────────────────────────────────────────
// T045-1: Pulse duty cycle patterns
// ──────────────────────────────────────────────
#[test]
fn test_pulse_duty_cycle() {
    // For each duty mode, verify the output pattern over 8 positions
    let duty_patterns: [[u8; 8]; 4] = [
        [0, 0, 0, 0, 0, 0, 0, 1], // 12.5%
        [0, 0, 0, 0, 0, 0, 1, 1], // 25%
        [0, 0, 0, 0, 1, 1, 1, 1], // 50%
        [1, 1, 1, 1, 1, 1, 0, 0], // 75%
    ];

    for (duty_mode, expected_pattern) in duty_patterns.iter().enumerate() {
        let mut pulse = PulseChannel::new(false);
        pulse.enabled = true;
        pulse.length_counter = 10;
        pulse.timer_period = 100; // >= 8 so output() doesn't mute
        pulse.timer_value = 0;
        pulse.constant_volume = true;
        pulse.volume = 10;
        pulse.duty_cycle = duty_mode as u8;

        let mut outputs = [0u8; 8];
        for i in 0..8 {
            pulse.duty_position = i as u8;
            let out = pulse.output();
            outputs[i] = if out > 0 { 1 } else { 0 };
        }

        assert_eq!(
            outputs, *expected_pattern,
            "Duty mode {} pattern mismatch: got {:?}, expected {:?}",
            duty_mode, outputs, expected_pattern
        );
    }
}

// ──────────────────────────────────────────────
// T045-2: Pulse envelope decay
// ──────────────────────────────────────────────
#[test]
fn test_pulse_envelope() {
    let mut pulse = PulseChannel::new(false);
    pulse.enabled = true;
    pulse.length_counter = 10;
    pulse.timer_period = 100;
    pulse.constant_volume = false;
    pulse.envelope_period = 3;

    // Trigger envelope start
    pulse.envelope_start = true;
    pulse.tick_envelope();

    // After first tick with start flag, decay should be 15
    assert_eq!(pulse.envelope_decay, 15, "Envelope should start at 15");
    assert!(
        !pulse.envelope_start,
        "Envelope start flag should be cleared"
    );

    // Clock the envelope: divider counts down from period (3)
    // Need (period + 1) ticks to decrement decay once: ticks at 3, 2, 1, 0 -> reload + decrement
    pulse.tick_envelope(); // divider: 3 -> 2
    pulse.tick_envelope(); // divider: 2 -> 1
    pulse.tick_envelope(); // divider: 1 -> 0
                           // Now divider == 0, so next tick reloads divider and decrements decay
    pulse.tick_envelope(); // divider reload to 3, decay: 15 -> 14

    assert_eq!(
        pulse.envelope_decay, 14,
        "Envelope decay should decrease to 14"
    );
}

// ──────────────────────────────────────────────
// T045-3: Triangle sequencer advancement
// ──────────────────────────────────────────────
#[test]
fn test_triangle_sequencer() {
    let mut tri = TriangleChannel::new();
    tri.enabled = true;
    tri.length_counter = 10;
    tri.linear_counter = 10;
    tri.timer_period = 0; // Timer reloads to 0, so advances every tick
    tri.timer_value = 0;

    let initial_pos = tri.sequence_position;

    // Tick the triangle - timer at 0 should reload and advance
    tri.tick();
    assert_ne!(
        tri.sequence_position, initial_pos,
        "Sequence position should advance after tick with timer at 0"
    );

    // The expected sequence starts at 15, 14, 13...
    // Position 0 -> output 15, position 1 -> output 14, etc.
    let expected_sequence_start = [15u8, 14, 13, 12, 11, 10, 9, 8];
    tri.sequence_position = 0;
    tri.timer_value = 0;

    for (i, &expected) in expected_sequence_start.iter().enumerate() {
        assert_eq!(
            tri.output(),
            expected,
            "Triangle output at position {} should be {}",
            i,
            expected
        );
        tri.tick();
    }
}

// ──────────────────────────────────────────────
// T045-4: Noise LFSR shifts
// ──────────────────────────────────────────────
#[test]
fn test_noise_lfsr() {
    let mut noise = NoiseChannel::new();
    noise.enabled = true;
    noise.length_counter = 10;
    noise.timer_period = 0; // Immediate reload
    noise.timer_value = 0;
    noise.mode_flag = false;

    let initial_sr = noise.shift_register;
    assert_eq!(initial_sr, 1, "Initial shift register should be 1");

    // Tick the noise channel - LFSR should shift
    noise.tick();
    assert_ne!(
        noise.shift_register, initial_sr,
        "Shift register should change after tick"
    );

    // With initial value 1 and mode_flag=false:
    // bit0 = 1, bit1 = 0, feedback = 1 XOR 0 = 1
    // shift right: 0, set bit14: 0x4000
    assert_eq!(
        noise.shift_register, 0x4000,
        "After first tick, shift register should be 0x4000 (bit 14 set)"
    );
}

// ──────────────────────────────────────────────
// T045-5: Length counter decrement
// ──────────────────────────────────────────────
#[test]
fn test_length_counter_decrement() {
    let mut pulse = PulseChannel::new(false);
    pulse.enabled = true;
    pulse.envelope_loop = false; // envelope_loop doubles as length counter halt

    // Load length counter via register write
    // Register 3, value with index 0 in upper 5 bits -> LENGTH_TABLE[0] = 10
    pulse.write_register(3, 0x00); // length index 0 -> 10
    assert_eq!(
        pulse.length_counter, 10,
        "Length counter should be loaded from table"
    );

    // Clock half frame should decrement
    pulse.tick_length_counter();
    assert_eq!(
        pulse.length_counter, 9,
        "Length counter should decrement to 9"
    );

    // Verify it decrements further
    for i in (0..9).rev() {
        pulse.tick_length_counter();
        assert_eq!(
            pulse.length_counter, i as u8,
            "Length counter should decrement to {}",
            i
        );
    }

    // Should not go below 0
    pulse.tick_length_counter();
    assert_eq!(
        pulse.length_counter, 0,
        "Length counter should not go below 0"
    );
}

// ──────────────────────────────────────────────
// T045-6: Frame counter mode 0 fires IRQ
// ──────────────────────────────────────────────
#[test]
fn test_frame_counter_mode0() {
    let mut apu = Apu::new();

    // Set mode 0 (4-step), IRQ not inhibited
    apu.write_register(0x4017, 0x00); // mode 0, no IRQ inhibit

    assert!(!apu.frame_irq, "Frame IRQ should start clear");

    // Step to just past the 4th step (29829 cycles) - IRQ should fire
    apu.step(29830, &mut null_reader);

    assert!(
        apu.frame_irq,
        "Frame IRQ should be set after step 4 in mode 0"
    );
}

// ──────────────────────────────────────────────
// T045-6b: Frame counter mode 1 does NOT fire IRQ
// ──────────────────────────────────────────────
#[test]
fn test_frame_counter_mode1_no_irq() {
    let mut apu = Apu::new();

    // Set mode 1 (5-step), no IRQ in this mode
    apu.write_register(0x4017, 0x80);

    // Step through an entire 5-step sequence
    apu.step(37282, &mut null_reader);

    assert!(
        !apu.frame_irq,
        "Frame IRQ should NOT be set in mode 1 (5-step)"
    );
}

// ──────────────────────────────────────────────
// T045-7: Non-linear mixer returns non-zero output
// ──────────────────────────────────────────────
#[test]
fn test_mix_nonzero() {
    let mut apu = Apu::new();

    // Enable pulse 1 and set it to produce output
    apu.write_register(0x4015, 0x01); // Enable pulse 1
    apu.write_register(0x4000, 0x30); // Duty 0, constant volume, volume = 0 (but we override)

    // Directly configure pulse1 for audible output
    apu.pulse1.enabled = true;
    apu.pulse1.length_counter = 10;
    apu.pulse1.timer_period = 100;
    apu.pulse1.constant_volume = true;
    apu.pulse1.volume = 15;
    apu.pulse1.duty_cycle = 2; // 50% duty
    apu.pulse1.duty_position = 4; // position where duty table = 1

    let mix = apu.mix();
    assert!(
        mix > 0.0,
        "Mix should return non-zero when pulse1 is active, got {}",
        mix
    );

    // Also test with triangle
    apu.triangle.enabled = true;
    apu.triangle.length_counter = 10;
    apu.triangle.linear_counter = 10;
    apu.triangle.sequence_position = 0; // outputs 15

    let mix2 = apu.mix();
    assert!(
        mix2 > mix,
        "Mix should increase with triangle added, got {}",
        mix2
    );
}

// ──────────────────────────────────────────────
// T045-8: Status register read/write
// ──────────────────────────────────────────────
#[test]
fn test_status_register() {
    let mut apu = Apu::new();

    // Enable all channels
    apu.write_register(0x4015, 0x1F);
    assert!(apu.pulse1.enabled);
    assert!(apu.pulse2.enabled);
    assert!(apu.triangle.enabled);
    assert!(apu.noise.enabled);
    assert!(apu.dmc.enabled);

    // Disable pulse 1
    apu.write_register(0x4015, 0x1E);
    assert!(!apu.pulse1.enabled);
    assert_eq!(
        apu.pulse1.length_counter, 0,
        "Disabled channel length counter should be 0"
    );
}

// ──────────────────────────────────────────────
// T045-9: Status read clears frame IRQ
// ──────────────────────────────────────────────
#[test]
fn test_status_read_clears_frame_irq() {
    let mut apu = Apu::new();
    apu.frame_irq = true;

    let status = apu.read_status();
    assert!(status & 0x40 != 0, "Status should report frame IRQ");
    assert!(!apu.frame_irq, "Reading status should clear frame IRQ");
}

// ──────────────────────────────────────────────
// T045-10: Sweep unit target period
// ──────────────────────────────────────────────
#[test]
fn test_sweep_target_period() {
    // Channel 1 (one's complement negate)
    let mut p1 = PulseChannel::new(false);
    p1.timer_period = 0x100;
    p1.sweep_shift = 1;
    p1.sweep_negate = true;

    // Channel 1 negate: period - (period >> shift) - 1
    // 0x100 - 0x80 - 1 = 0x7F
    let target1 = p1.target_period();
    assert_eq!(
        target1, 0x7F,
        "Channel 1 one's complement negate: expected 0x7F, got 0x{:X}",
        target1
    );

    // Channel 2 (two's complement negate)
    let mut p2 = PulseChannel::new(true);
    p2.timer_period = 0x100;
    p2.sweep_shift = 1;
    p2.sweep_negate = true;

    // Channel 2 negate: period - (period >> shift)
    // 0x100 - 0x80 = 0x80
    let target2 = p2.target_period();
    assert_eq!(
        target2, 0x80,
        "Channel 2 two's complement negate: expected 0x80, got 0x{:X}",
        target2
    );
}

// ──────────────────────────────────────────────
// T045-11: Triangle linear counter
// ──────────────────────────────────────────────
#[test]
fn test_triangle_linear_counter() {
    let mut tri = TriangleChannel::new();
    tri.enabled = true;

    // Write register 0: control flag clear, reload value = 5
    tri.write_register(0, 0x05); // control_flag = false, reload = 5
                                 // Write register 3: sets reload flag and loads length counter
    tri.write_register(3, 0x08); // length counter load, sets linear counter reload flag

    assert!(tri.linear_counter_reload_flag, "Reload flag should be set");

    // Clock linear counter - should load from reload value
    tri.tick_linear_counter();
    assert_eq!(
        tri.linear_counter, 5,
        "Linear counter should load reload value"
    );
    // With control flag clear, reload flag should be cleared
    assert!(
        !tri.linear_counter_reload_flag,
        "Reload flag should be cleared when control_flag is false"
    );

    // Further clocks should decrement
    tri.tick_linear_counter();
    assert_eq!(tri.linear_counter, 4);
    tri.tick_linear_counter();
    assert_eq!(tri.linear_counter, 3);
}

// ──────────────────────────────────────────────
// T045-12: Length counter halt
// ──────────────────────────────────────────────
#[test]
fn test_length_counter_halt() {
    let mut pulse = PulseChannel::new(false);
    pulse.enabled = true;
    pulse.envelope_loop = true; // This also serves as length counter halt
    pulse.length_counter = 10;

    // Clock half frame - should NOT decrement because halt is set
    pulse.tick_length_counter();
    assert_eq!(
        pulse.length_counter, 10,
        "Length counter should not decrement when halted"
    );
}
