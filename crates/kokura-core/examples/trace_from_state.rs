use std::{env, error::Error, fs, path::PathBuf};

use kokura_core::{state::MachineState, Machine};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let rom_path = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: trace_from_state <rom_path> <state_path> [steps]")?;
    let state_path = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: trace_from_state <rom_path> <state_path> [steps]")?;
    let steps = args
        .next()
        .map(|value| value.parse::<u32>())
        .transpose()?
        .unwrap_or(400);

    let rom = fs::read(&rom_path)?;
    let state = MachineState::load_boxed_from_path(&state_path)?;
    let mut machine = Machine::new();
    machine.load_rom(rom)?;
    machine.load_state(&state);

    println!(
        "start bank={} pc={:04X} sp={:04X} af={:02X}{:02X} bc={:02X}{:02X} de={:02X}{:02X} hl={:04X} ime={} ie={:02X} if={:02X} lcdc={:02X} stat={:02X} ly={:02X} bgp={:02X} obp={:02X}/{:02X} ca96={:02X}",
        machine.current_rom_bank(),
        machine.cpu.pc,
        machine.cpu.sp,
        machine.cpu.a,
        machine.cpu.f.0,
        machine.cpu.b,
        machine.cpu.c,
        machine.cpu.d,
        machine.cpu.e,
        machine.cpu.hl(),
        machine.cpu.ime,
        machine.interrupt.ie,
        machine.interrupt.iflag,
        machine.ppu.lcdc,
        machine.ppu.read_stat(),
        machine.ppu.ly,
        machine.ppu.bgp,
        machine.ppu.obp0,
        machine.ppu.obp1,
        machine.peek8(0xCA96),
    );

    for index in 0..steps {
        let bank_before = machine.current_rom_bank();
        let pc_before = machine.cpu.pc;
        let opcode = machine.peek8(pc_before);
        let sp_before = machine.cpu.sp;
        let af_before = ((machine.cpu.a as u16) << 8) | machine.cpu.f.0 as u16;
        let bc_before = ((machine.cpu.b as u16) << 8) | machine.cpu.c as u16;
        let de_before = ((machine.cpu.d as u16) << 8) | machine.cpu.e as u16;
        let hl_before = machine.cpu.hl();
        let ime_before = machine.cpu.ime;
        let ie_before = machine.interrupt.ie;
        let if_before = machine.interrupt.iflag;
        let lcdc_before = machine.ppu.lcdc;
        let stat_before = machine.ppu.read_stat();
        let ly_before = machine.ppu.ly;
        let bgp_before = machine.ppu.bgp;
        let obp0_before = machine.ppu.obp0;
        let obp1_before = machine.ppu.obp1;
        let ca96_before = machine.peek8(0xCA96);

        let step = machine.step_instruction()?;

        let bank_after = machine.current_rom_bank();
        let pc_after = machine.cpu.pc;
        let sp_after = machine.cpu.sp;
        let af_after = ((machine.cpu.a as u16) << 8) | machine.cpu.f.0 as u16;
        let bc_after = ((machine.cpu.b as u16) << 8) | machine.cpu.c as u16;
        let de_after = ((machine.cpu.d as u16) << 8) | machine.cpu.e as u16;
        let hl_after = machine.cpu.hl();
        let ime_after = machine.cpu.ime;
        let ie_after = machine.interrupt.ie;
        let if_after = machine.interrupt.iflag;
        let lcdc_after = machine.ppu.lcdc;
        let stat_after = machine.ppu.read_stat();
        let ly_after = machine.ppu.ly;
        let bgp_after = machine.ppu.bgp;
        let obp0_after = machine.ppu.obp0;
        let obp1_after = machine.ppu.obp1;
        let ca96_after = machine.peek8(0xCA96);

        println!(
            "#{index:04} bank {:02}->{:02} pc {:04X}->{:04X} op={:02X} step_cycles={} sp {:04X}->{:04X} af {:04X}->{:04X} bc {:04X}->{:04X} de {:04X}->{:04X} hl {:04X}->{:04X} ime {}->{} ie {:02X}->{:02X} if {:02X}->{:02X} lcdc {:02X}->{:02X} stat {:02X}->{:02X} ly {:02X}->{:02X} bgp {:02X}->{:02X} obp {:02X}/{:02X}->{:02X}/{:02X} ca96 {:02X}->{:02X}",
            bank_before,
            bank_after,
            pc_before,
            pc_after,
            opcode,
            step.cycles,
            sp_before,
            sp_after,
            af_before,
            af_after,
            bc_before,
            bc_after,
            de_before,
            de_after,
            hl_before,
            hl_after,
            ime_before,
            ime_after,
            ie_before,
            ie_after,
            if_before,
            if_after,
            lcdc_before,
            lcdc_after,
            stat_before,
            stat_after,
            ly_before,
            ly_after,
            bgp_before,
            bgp_after,
            obp0_before,
            obp1_before,
            obp0_after,
            obp1_after,
            ca96_before,
            ca96_after
        );
    }

    Ok(())
}
