use std::{env, error::Error, fs, path::PathBuf};

use kokura_core::Machine;

fn fmt_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn read_stack_window(machine: &mut Machine, count: usize) -> Vec<u8> {
    (0..count)
        .map(|offset| machine.read8(machine.cpu.sp.wrapping_add(offset as u16)))
        .collect()
}

fn read_watch_window(machine: &mut Machine, start: u16, count: usize) -> Vec<u8> {
    (0..count)
        .map(|offset| machine.read8(start.wrapping_add(offset as u16)))
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let rom_path = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: trace_x_transition <rom_path> [start_frame] [steps]")?;
    let start_frame = args
        .next()
        .map(|value| value.parse::<u32>())
        .transpose()?
        .unwrap_or(262);
    let steps = args
        .next()
        .map(|value| value.parse::<u32>())
        .transpose()?
        .unwrap_or(400);

    let rom = fs::read(&rom_path)?;
    let mut machine = Machine::new();
    machine.load_rom(rom)?;

    for _ in 0..start_frame {
        machine.run_frame()?;
    }

    println!(
        "start frame={} cycles={} pc={:04X} bank={} af={:02X}{:02X} bc={:02X}{:02X} de={:02X}{:02X} hl={:04X} sp={:04X} ime={} ie={:02X} if={:02X} lcdc={:02X} stat={:02X} ly={:02X} lyc={:02X} bgp={:02X} obp0={:02X} obp1={:02X}",
        machine.clocks.frames,
        machine.clocks.cycles,
        machine.cpu.pc,
        machine.cartridge.current_rom_bank(),
        machine.cpu.a,
        machine.cpu.f.0,
        machine.cpu.b,
        machine.cpu.c,
        machine.cpu.d,
        machine.cpu.e,
        machine.cpu.hl(),
        machine.cpu.sp,
        machine.cpu.ime,
        machine.interrupt.ie,
        machine.interrupt.iflag,
        machine.ppu.lcdc,
        machine.ppu.stat,
        machine.ppu.ly,
        machine.ppu.lyc,
        machine.ppu.bgp,
        machine.ppu.obp0,
        machine.ppu.obp1
    );

    for index in 0..steps {
        let bank_before = machine.cartridge.current_rom_bank();
        let pc_before = machine.cpu.pc;
        let opcode = machine.read8(pc_before);
        let sp_before = machine.cpu.sp;
        let a_before = machine.cpu.a;
        let f_before = machine.cpu.f.0;
        let b_before = machine.cpu.b;
        let c_before = machine.cpu.c;
        let d_before = machine.cpu.d;
        let e_before = machine.cpu.e;
        let hl_before = machine.cpu.hl();
        let ime_before = machine.cpu.ime;
        let ie_before = machine.interrupt.ie;
        let if_before = machine.interrupt.iflag;
        let lcdc_before = machine.ppu.lcdc;
        let stat_before = machine.ppu.stat;
        let ly_before = machine.ppu.ly;
        let lyc_before = machine.ppu.lyc;
        let bgp_before = machine.ppu.bgp;
        let obp0_before = machine.ppu.obp0;
        let obp1_before = machine.ppu.obp1;
        let ca96_before = machine.read8(0xCA96);
        let ca9d_before = machine.read8(0xCA9D);
        let d058_before = machine.read8(0xD058);
        let d158_before = machine.read8(0xD158);
        let d15a_before = machine.read8(0xD15A);
        let ffc8_ffcb_before = read_watch_window(&mut machine, 0xFFC8, 4);
        let stack_before = read_stack_window(&mut machine, 6);

        let step = machine.step_instruction()?;

        let bank_after = machine.cartridge.current_rom_bank();
        let pc_after = machine.cpu.pc;
        let sp_after = machine.cpu.sp;
        let a_after = machine.cpu.a;
        let f_after = machine.cpu.f.0;
        let b_after = machine.cpu.b;
        let c_after = machine.cpu.c;
        let d_after = machine.cpu.d;
        let e_after = machine.cpu.e;
        let hl_after = machine.cpu.hl();
        let ime_after = machine.cpu.ime;
        let ie_after = machine.interrupt.ie;
        let if_after = machine.interrupt.iflag;
        let lcdc_after = machine.ppu.lcdc;
        let stat_after = machine.ppu.stat;
        let ly_after = machine.ppu.ly;
        let lyc_after = machine.ppu.lyc;
        let bgp_after = machine.ppu.bgp;
        let obp0_after = machine.ppu.obp0;
        let obp1_after = machine.ppu.obp1;
        let ca96_after = machine.read8(0xCA96);
        let ca9d_after = machine.read8(0xCA9D);
        let d058_after = machine.read8(0xD058);
        let d158_after = machine.read8(0xD158);
        let d15a_after = machine.read8(0xD15A);
        let ffc8_ffcb_after = read_watch_window(&mut machine, 0xFFC8, 4);
        let stack_after = read_stack_window(&mut machine, 6);

        let interesting = bank_before == 7
            || bank_after == 7
            || ie_before != ie_after
            || if_before != if_after
            || lcdc_before != lcdc_after
            || stat_before != stat_after
            || ly_before != ly_after
            || lyc_before != lyc_after
            || bgp_before != bgp_after
            || obp0_before != obp0_after
            || obp1_before != obp1_after
            || pc_before == 0x0040
            || pc_before == 0x0048
            || pc_before == 0x0518
            || pc_before == 0x0F54
            || pc_before == 0x0F8C
            || pc_before == 0x0FB4
            || pc_before == 0x1044
            || pc_before == 0x1047
            || pc_before == 0x24B1
            || pc_before == 0x24EE
            || pc_before == 0x55AA
            || pc_before == 0x55B0
            || pc_before == 0x55B6
            || pc_before == 0x55C3
            || pc_before == 0x1829
            || pc_before == 0x47CC
            || pc_before == 0x48A3
            || pc_before == 0x4348
            || pc_before == 0x4350
            || pc_before == 0x436E;

        if interesting {
            println!(
                "#{index:04} cyc={} frame={} bank {:02}->{:02} pc {:04X}->{:04X} op={:02X} step_cycles={} af {:02X}{:02X}->{:02X}{:02X} bc {:02X}{:02X}->{:02X}{:02X} de {:02X}{:02X}->{:02X}{:02X} hl {:04X}->{:04X} sp {:04X}->{:04X} ime {}->{} ie {:02X}->{:02X} if {:02X}->{:02X} lcdc {:02X}->{:02X} stat {:02X}->{:02X} ly {:02X}->{:02X} lyc {:02X}->{:02X} bgp {:02X}->{:02X} obp {:02X}/{:02X}->{:02X}/{:02X} ca96 {:02X}->{:02X} ca9d {:02X}->{:02X} d058 {:02X}->{:02X} d158 {:02X}->{:02X} d15a {:02X}->{:02X} ffc8..cb [{}] -> [{}] stack [{}] -> [{}]",
                machine.clocks.cycles,
                machine.clocks.frames,
                bank_before,
                bank_after,
                pc_before,
                pc_after,
                opcode,
                step.cycles,
                a_before,
                f_before,
                a_after,
                f_after,
                b_before,
                c_before,
                b_after,
                c_after,
                d_before,
                e_before,
                d_after,
                e_after,
                hl_before,
                hl_after,
                sp_before,
                sp_after,
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
                lyc_before,
                lyc_after,
                bgp_before,
                bgp_after,
                obp0_before,
                obp1_before,
                obp0_after,
                obp1_after,
                ca96_before,
                ca96_after,
                ca9d_before,
                ca9d_after,
                d058_before,
                d058_after,
                d158_before,
                d158_after,
                d15a_before,
                d15a_after,
                fmt_bytes(&ffc8_ffcb_before),
                fmt_bytes(&ffc8_ffcb_after),
                fmt_bytes(&stack_before),
                fmt_bytes(&stack_after)
            );
        }
    }

    Ok(())
}
