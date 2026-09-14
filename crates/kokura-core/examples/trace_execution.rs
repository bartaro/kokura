//! Standalone instruction tracing for a caller-supplied ROM.
//! Watch addresses are hexadecimal; frame and step counts are decimal.
//! Run with --help for positional arguments. Output is English.
use std::{env, error::Error, fs, path::PathBuf};
use kokura_core::Machine;

const USAGE: &str = "usage: trace_execution <rom_path> [start_frame] [steps] [watch_hex,...]";

// Parse the requested trace range, load the supplied ROM, and report bounded steps.
fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let first = args.next().ok_or(USAGE)?;
    if first == "--help" || first == "-h" {
        println!("{USAGE}");
        return Ok(());
    }
    let rom_path = PathBuf::from(first);
    let start_frame = args.next().map(|value| value.parse::<u32>()).transpose()?.unwrap_or(0);
    let steps = args.next().map(|value| value.parse::<u32>()).transpose()?.unwrap_or(32);
    let watches = parse_watches(&args.next().unwrap_or_default())?;
    if args.next().is_some() {
        return Err(USAGE.into());
    }
    let mut machine = Machine::new();
    machine.load_rom(fs::read(rom_path)?)?;
    // Skip a caller-selected number of complete frames before instruction tracing.
    for _ in 0..start_frame {
        machine.run_frame()?;
    }
    trace_steps(&mut machine, steps, &watches)
}

// Parse comma-separated hexadecimal CPU addresses. No ROM-specific watch is built in.
fn parse_watches(text: &str) -> Result<Vec<u16>, Box<dyn Error>> {
    if text.is_empty() {
        return Ok(Vec::new());
    }
    text.split(',')
        .map(|item| {
            let item = item.trim();
            let digits = item.strip_prefix("0x").or_else(|| item.strip_prefix("0X")).unwrap_or(item);
            Ok(u16::from_str_radix(digits, 16)?)
        })
        .collect()
}

// Inspect registers, the current stack and explicitly requested memory locations.
// peek8 avoids device-read side effects; these observations do not advance clocks.
fn format_snapshot(machine: &Machine, watches: &[u16]) -> String {
    let watched = watches.iter()
        .map(|addr| format!("{addr:04X}={:02X}", machine.peek8(*addr)))
        .collect::<Vec<_>>().join(" ");
    let stack = (0..6u16)
        .map(|offset| format!("{:02X}", machine.peek8(machine.cpu.sp.wrapping_add(offset))))
        .collect::<Vec<_>>().join(" ");
    format!(
        "frame={} cycles={} bank={} pc={:04X} sp={:04X} af={:02X}{:02X} bc={:02X}{:02X} de={:02X}{:02X} hl={:04X} ime={} ie={:02X} if={:02X} lcdc={:02X} stat={:02X} ly={:02X} lyc={:02X} bgp={:02X} obp={:02X}/{:02X} watch=[{}] stack=[{}]",
        machine.clocks.frames, machine.clocks.cycles, machine.current_rom_bank(),
        machine.cpu.pc, machine.cpu.sp, machine.cpu.a, machine.cpu.f.0,
        machine.cpu.b, machine.cpu.c, machine.cpu.d, machine.cpu.e, machine.cpu.hl(),
        machine.cpu.ime, machine.interrupt.ie, machine.interrupt.iflag,
        machine.ppu.lcdc, machine.ppu.read_stat(), machine.ppu.ly, machine.ppu.lyc,
        machine.ppu.bgp, machine.ppu.obp0, machine.ppu.obp1, watched, stack,
    )
}

// Trace every requested machine step, including interrupt/HALT handling by the core.
// The displayed opcode is the byte at the pre-step PC, not proof it was executed.
fn trace_steps(machine: &mut Machine, steps: u32, watches: &[u16]) -> Result<(), Box<dyn Error>> {
    println!("start {}", format_snapshot(machine, watches));
    for index in 0..steps {
        let opcode = machine.peek8(machine.cpu.pc);
        let before = format_snapshot(machine, watches);
        let step = machine.step_instruction()?;
        println!("#{index:04} pc_byte={opcode:02X} step_cycles={} before {{{}}} after {{{}}}",
            step.cycles, before, format_snapshot(machine, watches));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Construct original fixture bytes: load A, store it at WRAM base, then NOP.
    fn test_machine() -> Machine {
        let mut rom = vec![0; 0x8000];
        rom[0x0100..0x0106].copy_from_slice(&[0x3E, 0x42, 0xEA, 0x00, 0xC0, 0x00]);
        let mut machine = Machine::new();
        machine.load_rom(rom).unwrap();
        machine
    }

    #[test]
    // Support an empty list, plain hex and both explicit hexadecimal prefixes.
    fn watch_addresses_accept_hex_and_empty() {
        assert!(parse_watches("").unwrap().is_empty());
        assert_eq!(parse_watches("c000, 0xFF00,0XFFFF").unwrap(), vec![0xC000, 0xFF00, 0xFFFF]);
    }

    #[test]
    // Reject overflow, incomplete list entries and non-hexadecimal addresses.
    fn watch_addresses_reject_invalid_values() {
        for value in ["10000", "C000,", "wrong", "0x", "-1"] {
            assert!(parse_watches(value).is_err(), "{value}");
        }
    }

    #[test]
    // Verify that formatting includes the requested value without changing saved state.
    fn snapshot_is_observational() {
        let mut machine = test_machine();
        machine.memory.write8(0xC000, 0xA5);
        let before = machine.save_state().to_bytes().unwrap();
        let text = format_snapshot(&machine, &[0xC000, 0xFF00]);
        assert!(text.contains("C000=A5"));
        assert_eq!(machine.save_state().to_bytes().unwrap(), before);
    }

    #[test]
    // Run the original fixture and prove its WRAM write and exact step endpoint.
    fn trace_runs_requested_steps() {
        let mut machine = test_machine();
        trace_steps(&mut machine, 3, &[0xC000]).unwrap();
        assert_eq!(machine.peek8(0xC000), 0x42);
        assert_eq!(machine.cpu.pc, 0x0106);
    }
}
