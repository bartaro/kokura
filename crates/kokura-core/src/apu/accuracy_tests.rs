use super::*;

fn pulse(channel: u8, sweep: u8, frequency: u16) -> Apu {
    let mut apu = Apu::default();
    apu.write(0xFF26, 0x80);
    apu.write(0xFF10, sweep);
    let base = if channel == 1 { 0xFF11 } else { 0xFF16 };
    apu.write(base, 0x80);
    apu.write(base + 1, 0xF0);
    apu.write(base + 2, frequency as u8);
    apu.write(base + 3, 0x80 | (frequency >> 8) as u8);
    apu
}

#[test]
fn retrigger_preserves_pulse_duty_phase_and_live_output() {
    for channel in [1, 2] {
        let mut apu = pulse(channel, 0, 1750);
        let index = usize::from(channel - 1);
        apu.channels[index].duty_position = 5;
        apu.channels[index].force_zero_until_advance = false;
        let previous = apu.channel_digital_output(channel);
        assert_eq!(previous, 15);
        apu.write(if channel == 1 { 0xFF14 } else { 0xFF19 }, 0x86);
        assert_eq!(apu.channels[index].duty_position, 5);
        assert_eq!(apu.channel_digital_output(channel), previous);
    }
}

#[test]
fn first_pulse_trigger_is_suppressed_until_its_first_duty_step() {
    let mut apu = pulse(1, 0, 2047);
    assert_eq!(apu.channel_digital_output(1), 0);
    apu.tick_fast(4);
    assert!(!apu.channels[0].force_zero_until_advance);
    assert_eq!(apu.channels[0].duty_position, 1);
    apu.write(0xFF26, 0);
    apu.write(0xFF26, 0x80);
    apu.write(0xFF11, 0x80);
    apu.write(0xFF12, 0xF0);
    apu.write(0xFF14, 0x80);
    assert_eq!(apu.channel_digital_output(1), 0);
}

#[test]
fn zero_sweep_pace_does_not_change_pitch_after_eight_ticks() {
    let mut apu = pulse(1, 0x01, 512);
    apu.tick_fast(8192 * 32);
    assert_eq!(apu.current_frequency(1), 512);
    assert_eq!(apu.channel_enable & 1, 1);
}

#[test]
fn period_register_writes_do_not_replace_the_sweep_shadow() {
    let mut apu = pulse(1, 0x12, 512);
    apu.write(0xFF13, 0x63);
    apu.write(0xFF14, 0x02);
    assert_eq!(apu.current_frequency(1), 0x263);
    assert_eq!(apu.channels[0].shadow_frequency, 512);
    apu.clock_sweep(&mut Vec::new());
    assert_eq!(apu.current_frequency(1), 640);
}

#[test]
fn sweep_update_keeps_the_running_period_divider_phase() {
    let mut apu = pulse(1, 0x12, 512);
    apu.channels[0].freq_timer = 19;
    apu.clock_sweep(&mut Vec::new());
    assert_eq!(apu.current_frequency(1), 640);
    assert_eq!(apu.channels[0].freq_timer, 19);
}

#[test]
fn zero_shift_sweep_still_checks_overflow_without_writing_pitch() {
    for (frequency, active) in [(500, true), (1200, false)] {
        let mut apu = pulse(1, 0x10, frequency);
        assert!(apu.channels[0].enabled);
        apu.clock_sweep(&mut Vec::new());
        assert_eq!(apu.channels[0].enabled, active);
        assert_eq!(apu.current_frequency(1), frequency);
    }
}

#[test]
fn live_zero_pace_increasing_envelope_supports_zombie_volume_steps() {
    for (channel, envelope, trigger) in [
        (1u8, 0xFF12, 0xFF14),
        (2, 0xFF17, 0xFF19),
        (4, 0xFF21, 0xFF23),
    ] {
        let mut apu = Apu::default();
        apu.write(0xFF26, 0x80);
        apu.write(envelope, 0x58);
        apu.write(trigger, 0x80);
        for writes in 1..=16 {
            apu.write(envelope, 0x08);
            assert_eq!(
                apu.channels[usize::from(channel - 1)].envelope_volume,
                (5 + writes) & 15
            );
            assert!(apu.channels[usize::from(channel - 1)].dac_enabled);
        }
    }
}

#[test]
fn powered_off_mixer_and_envelope_registers_ignore_writes() {
    for cgb in [false, true] {
        let mut apu = Apu::default();
        apu.set_cgb_mode(cgb);
        apu.write(0xFF12, 0xF3);
        apu.write(0xFF24, 0x77);
        apu.write(0xFF25, 0xFF);
        apu.write(0xFF14, 0x80);
        apu.write(0xFF30, 0xAB);
        apu.write(0xFF26, 0x80);
        assert_eq!(apu.read(0xFF12), 0);
        assert_eq!(apu.read(0xFF24), 0);
        assert_eq!(apu.read(0xFF25), 0);
        assert_eq!(apu.read(0xFF30), 0xAB);
        assert_eq!(apu.channel_enable, 0);
    }
}

#[test]
fn only_dmg_accepts_length_writes_while_powered_off() {
    for cgb in [false, true] {
        let mut apu = Apu::default();
        apu.set_cgb_mode(cgb);
        for (register, value, index) in [
            (0xFF11, 0xFF, 0),
            (0xFF16, 0xFF, 1),
            (0xFF1B, 0xFF, 2),
            (0xFF20, 0xFF, 3),
        ] {
            apu.write(register, value);
            assert_eq!(apu.channels[index].length_counter, if cgb { 0 } else { 1 });
        }
        assert_eq!(apu.read(0xFF11), 0x3F);
    }
}

#[test]
fn apu_registers_apply_hardware_read_masks() {
    let masks = [
        0x80, 0x3F, 0, 0xFF, 0xBF, 0xFF, 0x3F, 0, 0xFF, 0xBF, 0x7F, 0xFF, 0x9F, 0xFF, 0xBF, 0xFF,
        0xFF, 0, 0, 0xBF, 0, 0,
    ];
    let mut apu = Apu::default();
    apu.write(0xFF26, 0x80);
    for (index, mask) in masks.into_iter().enumerate() {
        let address = 0xFF10 + index as u16;
        apu.write(address, 0);
        assert_eq!(apu.read(address), mask, "register {address:04X}");
    }
}

#[test]
fn high_pass_coefficient_accounts_for_all_clocks_in_a_pcm_sample() {
    for (cgb, per_clock) in [(false, 0.999_958_f64), (true, 0.998_943_f64)] {
        let mut apu = Apu::default();
        apu.set_cgb_mode(cgb);
        assert!((apu.hpf_charge_factor() - per_clock.powi(64)).abs() < 1e-12);
    }
}

#[test]
fn high_pass_rejects_dc_more_quickly_on_cgb_than_dmg() {
    let mut tail = Vec::new();
    for cgb in [false, true] {
        let mut apu = Apu::default();
        apu.set_cgb_mode(cgb);
        apu.write(0xFF26, 0x80);
        apu.channels[0].dac_enabled = true;
        let mut sample = Vec::new();
        for _ in 0..120 {
            apu.pcm_accum_left = 32 * i32::from(APU_PCM_SAMPLE_CYCLES);
            apu.pcm_accum_right = apu.pcm_accum_left;
            apu.pcm_accum_cycles = APU_PCM_SAMPLE_CYCLES;
            apu.enqueue_pcm_frame(&mut 0);
            sample = apu.drain_interleaved_i16(1);
        }
        tail.push(sample[0].abs());
    }
    assert!(tail[0] > 1000 && tail[0] < 1800);
    assert!(tail[1] < 2);
}

#[test]
fn filtered_pulse_tone_remains_audible_and_stereo_balanced() {
    let mut apu = pulse(1, 0, 1750);
    apu.set_cgb_mode(true);
    apu.write(0xFF24, 0x77);
    apu.write(0xFF25, 0x11);
    apu.tick_fast(4_194_304 / 8);
    let pcm = apu.drain_interleaved_i16(usize::MAX);
    let frames: Vec<_> = pcm.chunks_exact(2).skip(1024).collect();
    assert!(frames.iter().all(|frame| frame[0] == frame[1]));
    let rms = (frames
        .iter()
        .map(|frame| f64::from(frame[0]).powi(2))
        .sum::<f64>()
        / frames.len() as f64)
        .sqrt();
    assert!(rms > 1000. && rms < 15000., "pulse RMS {rms}");
}

#[test]
fn traced_and_fast_audio_paths_produce_identical_pcm() {
    let mut traced = pulse(1, 0x12, 512);
    traced.write(0xFF24, 0x77);
    traced.write(0xFF25, 0x11);
    let mut fast = traced.clone();
    traced.tick(123_456);
    fast.tick_fast(123_456);
    assert_eq!(
        traced.drain_interleaved_i16(usize::MAX),
        fast.drain_interleaved_i16(usize::MAX)
    );
}
