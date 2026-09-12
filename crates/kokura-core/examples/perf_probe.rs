use std::{env, error::Error, fs, path::PathBuf, time::Instant};

use kokura_core::{types::HardwareMode, Machine};

fn parse_button_mask(name: &str) -> Result<u8, Box<dyn Error>> {
    match name.trim().to_ascii_uppercase().as_str() {
        "RIGHT" => Ok(0x01),
        "LEFT" => Ok(0x02),
        "UP" => Ok(0x04),
        "DOWN" => Ok(0x08),
        "A" => Ok(0x10),
        "B" => Ok(0x20),
        "SELECT" => Ok(0x40),
        "START" => Ok(0x80),
        other => Err(format!("unsupported button in input-seq: {other}").into()),
    }
}

fn parse_input_seq(spec: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut out = Vec::new();
    for chunk in spec.split(';') {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }
        let (name, count_text) = chunk
            .split_once(':')
            .ok_or("input-seq entries must look like NAME:COUNT")?;
        let count: usize = count_text.trim().parse()?;
        let mask = if name.trim().eq_ignore_ascii_case("NONE") {
            0
        } else {
            parse_button_mask(name)?
        };
        out.extend(std::iter::repeat_n(mask, count));
    }
    Ok(out)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let rom = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: cargo run -p kokura-core --example perf_probe -- <rom> [frames] [mode:auto|dmg|cgb] [drain:none|audio] [input-seq]")?;
    let frames = args.next().as_deref().unwrap_or("600").parse::<u64>()?;
    let mode = match args.next().as_deref().unwrap_or("auto") {
        "auto" => None,
        "dmg" => Some(HardwareMode::Dmg),
        "cgb" => Some(HardwareMode::Cgb),
        other => return Err(format!("unsupported mode: {other}").into()),
    };
    let drain_audio = match args.next().as_deref().unwrap_or("none") {
        "none" => false,
        "audio" => true,
        other => return Err(format!("unsupported drain mode: {other}").into()),
    };
    let input_seq = args
        .next()
        .map(|spec| parse_input_seq(&spec))
        .transpose()?
        .unwrap_or_default();

    let mut machine = Machine::new();
    let rom_bytes = fs::read(&rom)?;
    machine.load_rom_with_mode(rom_bytes, mode)?;
    let _ = machine.set_audio_buffer_capacity_frames(65_536);

    let started = Instant::now();
    for frame_idx in 0..frames {
        let _ = machine
            .joypad
            .set_mask(*input_seq.get(frame_idx as usize).unwrap_or(&0));
        machine.run_frame()?;
        if drain_audio {
            let available = machine.audio_frames_available();
            if available > 0 {
                let _ = machine.drain_audio_frames_interleaved_i16(available);
            }
        }
    }
    let elapsed = started.elapsed();
    let fps = frames as f64 / elapsed.as_secs_f64();
    println!(
        "{{\"rom\":\"{}\",\"frames\":{},\"mode\":\"{}\",\"drain_audio\":{},\"ms\":{:.1},\"fps\":{:.2},\"pc\":\"0x{:04X}\",\"rom_bank\":{},\"audio_frames_dropped\":{}}}",
        rom.display(),
        frames,
        match mode {
            None => "auto",
            Some(HardwareMode::Dmg) => "dmg",
            Some(HardwareMode::Cgb) => "cgb",
        },
        if drain_audio { "true" } else { "false" },
        elapsed.as_secs_f64() * 1000.0,
        fps,
        machine.cpu.pc,
        machine.current_rom_bank(),
        machine.audio_frames_dropped()
    );
    Ok(())
}
