use std::{env, error::Error, fs, path::PathBuf};

use kokura_core::{types::HardwareMode, Machine};

// Parse decimal addresses or explicitly prefixed hexadecimal addresses.
fn parse_u16(text: &str) -> Result<u16, Box<dyn Error>> {
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Ok(u16::from_str_radix(hex, 16)?)
    } else {
        Ok(text.parse::<u16>()?)
    }
}

// Run a selected frame count and inspect general hardware state and caller
// addresses. The attribute sample starts at the background map origin.
fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let rom = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: cargo run -p kokura-core --example frame_probe -- <rom> [frames] [mode:auto|dmg|cgb] [addr...]")?;
    let frames = args.next().as_deref().unwrap_or("120").parse::<u64>()?;
    let mode = match args.next().as_deref().unwrap_or("auto") {
        "auto" => None,
        "dmg" => Some(HardwareMode::Dmg),
        "cgb" => Some(HardwareMode::Cgb),
        other => return Err(format!("unsupported mode: {other}").into()),
    };
    let addresses: Vec<u16> = args.map(|arg| parse_u16(&arg)).collect::<Result<_, _>>()?;

    let mut machine = Machine::new();
    machine.load_rom_with_mode(fs::read(&rom)?, mode)?;
    let _ = machine.set_audio_buffer_capacity_frames(65_536);
    for _ in 0..frames {
        machine.run_frame()?;
    }

    let reads: Vec<(u16, u8)> = addresses
        .iter()
        .map(|&addr| (addr, machine.peek8(addr)))
        .collect();
    let palette0: Vec<String> = (0..4)
        .map(|color| format!("0x{:04X}", machine.memory.bg_palette_rgb555(0, color)))
        .collect();
    let palette1: Vec<String> = (0..4)
        .map(|color| format!("0x{:04X}", machine.memory.bg_palette_rgb555(1, color)))
        .collect();
    let rgb_sample: Vec<String> = machine
        .framebuffer_rgb555()
        .iter()
        .take(8)
        .map(|value| format!("0x{value:04X}"))
        .collect();
    let bg_map_attr_head: Vec<String> = (0..10usize)
        .map(|offset| {
            format!(
                "0x{:02X}",
                machine.memory.vram_bank(1)[0x1800 + offset]
            )
        })
        .collect();

    println!("rom={}", rom.display());
    println!("frames={frames}");
    println!(
        "mode={}",
        match machine.mode {
            HardwareMode::Dmg => "dmg",
            HardwareMode::Cgb => "cgb",
        }
    );
    println!("rom_supports_cgb={}", machine.cartridge.supports_cgb());
    println!("pc=0x{:04X}", machine.cpu.pc);
    println!("rom_bank={}", machine.current_rom_bank());
    println!("ff4f=0x{:02X}", machine.peek8(0xFF4F));
    println!("ff68=0x{:02X}", machine.peek8(0xFF68));
    println!("ff69=0x{:02X}", machine.peek8(0xFF69));
    println!("bg_palette0_rgb555={}", palette0.join(","));
    println!("bg_palette1_rgb555={}", palette1.join(","));
    println!("bg_map_attr_head={}", bg_map_attr_head.join(","));
    println!("framebuffer_rgb555_head={}", rgb_sample.join(","));
    for (addr, value) in reads {
        println!("read[0x{addr:04X}]=0x{value:02X}");
    }
    Ok(())
}
