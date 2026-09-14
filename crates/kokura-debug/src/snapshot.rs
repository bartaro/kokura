use kokura_core::Machine;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CpuSnapshot {
    pub pc: u16,
    pub sp: u16,
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub frames: u64,
    pub cycles: u64,
    pub current_rom_bank: u16,
    pub current_ram_bank: u16,
    pub ime: bool,
    pub ie: u8,
    pub iflag: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct VideoSnapshot {
    pub lcdc: u8,
    pub stat: u8,
    pub ly: u8,
    pub scx: u8,
    pub scy: u8,
    pub wx: u8,
    pub wy: u8,
    pub bgp: u8,
    pub frame_hash: u32,
    pub vram_hash: u32,
    pub oam_hash: u32,
    pub bg_hash: u32,
    pub window_hash: u32,
    pub sprite_hash: u32,
    pub screen_changed: bool,
    pub bg_enabled: bool,
    pub lcd_enabled: bool,
    pub window_enabled: bool,
    pub sprite_enabled: bool,
    pub visible_sprite_count: u32,
    pub nonzero_oam_entries: u32,
    pub ppu_mode: String,
    pub stat_coincidence: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimerSnapshot {
    pub div: u16,
    pub tima: u8,
    pub tma: u8,
    pub tac: u8,
}

// Compute a wrapping multiply-then-XOR byte hash with the FNV offset
// basis. Unlike state-file ROM checksums, this starts with a nonzero seed.
pub fn hash_bytes(bytes: &[u8]) -> u32 {
    bytes.iter().fold(2166136261u32, |acc, &b| {
        acc.wrapping_mul(16777619) ^ b as u32
    })
}

// Hash the bytes exposed by the machine framebuffer accessor.
pub fn compute_frame_hash(machine: &Machine) -> u32 {
    hash_bytes(machine.framebuffer())
}
// Hash only the currently selected 8 KiB VRAM bank, not both CGB banks.
pub fn compute_vram_hash(machine: &Machine) -> u32 {
    hash_bytes(machine.memory.vram())
}
// Hash the complete 160-byte sprite-attribute region.
pub fn compute_oam_hash(machine: &Machine) -> u32 {
    hash_bytes(machine.memory.oam())
}
// Hash selected-bank tile bytes plus SCX, SCY, BGP and LCDC.
// This background-change hint excludes tile maps and is not a rendered image.
pub fn compute_bg_hash(machine: &Machine) -> u32 {
    let v = machine.memory.vram();
    let mut data = Vec::with_capacity(0x1820);
    data.extend_from_slice(&v[..0x1800]);
    data.extend_from_slice(&[
        machine.ppu.scx,
        machine.ppu.scy,
        machine.ppu.bgp,
        machine.ppu.lcdc,
    ]);
    hash_bytes(&data)
}
// Hash the selected window tile map and WX/WY/LCDC in the current VRAM
// bank; tile graphics and palette data are not included in this hint.
pub fn compute_window_hash(machine: &Machine) -> u32 {
    let v = machine.memory.vram();
    let base = if machine.ppu.window_map_base() == 0x1C00 {
        0x1C00
    } else {
        0x1800
    };
    let end = (base + 0x400).min(v.len());
    let mut data = Vec::with_capacity(0x410);
    data.extend_from_slice(&v[base..end]);
    data.extend_from_slice(&[machine.ppu.wx, machine.ppu.wy, machine.ppu.lcdc]);
    hash_bytes(&data)
}
// Hash OAM plus OBP0/OBP1/LCDC; sprite tile graphics are not included.
pub fn compute_sprite_hash(machine: &Machine) -> u32 {
    let mut data = Vec::with_capacity(0xB0);
    data.extend_from_slice(machine.memory.oam());
    data.extend_from_slice(&[machine.ppu.obp0, machine.ppu.obp1, machine.ppu.lcdc]);
    hash_bytes(&data)
}

impl From<&Machine> for CpuSnapshot {
    // Capture registers, reconstructed register pairs, clocks, bank selectors
    // and interrupt state without advancing emulation.
    fn from(machine: &Machine) -> Self {
        Self {
            pc: machine.cpu.pc,
            sp: machine.cpu.sp,
            a: machine.cpu.a,
            f: machine.cpu.f.0,
            b: machine.cpu.b,
            c: machine.cpu.c,
            d: machine.cpu.d,
            e: machine.cpu.e,
            h: machine.cpu.h,
            l: machine.cpu.l,
            bc: ((machine.cpu.b as u16) << 8) | machine.cpu.c as u16,
            de: ((machine.cpu.d as u16) << 8) | machine.cpu.e as u16,
            hl: machine.cpu.hl(),
            frames: machine.clocks.frames,
            cycles: machine.clocks.cycles,
            current_rom_bank: machine.current_rom_bank(),
            current_ram_bank: machine.current_ram_bank(),
            ime: machine.cpu.ime,
            ie: machine.interrupt.ie,
            iflag: machine.interrupt.iflag,
        }
    }
}

impl VideoSnapshot {
    // Collect video registers, partial-data hashes and sprite estimates.
    // A missing previous hash counts as changed; otherwise only frame-hash
    // inequality sets screen_changed, rather than a full pixel comparison.
    pub fn from_machine(machine: &Machine, previous_hash: Option<u32>) -> Self {
        let frame_hash = compute_frame_hash(machine);
        let screen_changed = previous_hash.map(|prev| prev != frame_hash).unwrap_or(true);
        let oam = machine.memory.oam();
        Self {
            lcdc: machine.ppu.lcdc,
            stat: machine.ppu.stat,
            ly: machine.ppu.ly,
            scx: machine.ppu.scx,
            scy: machine.ppu.scy,
            wx: machine.ppu.wx,
            wy: machine.ppu.wy,
            bgp: machine.ppu.bgp,
            frame_hash,
            vram_hash: compute_vram_hash(machine),
            oam_hash: compute_oam_hash(machine),
            bg_hash: compute_bg_hash(machine),
            window_hash: compute_window_hash(machine),
            sprite_hash: compute_sprite_hash(machine),
            screen_changed,
            bg_enabled: machine.ppu.bg_enabled(),
            lcd_enabled: machine.ppu.lcd_enabled(),
            window_enabled: machine.ppu.window_enabled(),
            sprite_enabled: machine.ppu.sprite_enabled(),
            visible_sprite_count: machine.ppu.visible_sprite_count_estimate(oam),
            nonzero_oam_entries: machine.ppu.nonzero_oam_entries(oam),
            ppu_mode: format!("{:?}", machine.ppu.current_mode()),
            stat_coincidence: machine.ppu.stat_coincidence(),
        }
    }
}

impl From<&Machine> for TimerSnapshot {
    // Copy raw divider/counter/modulo/control state without ticking the timer.
    fn from(machine: &Machine) -> Self {
        Self {
            div: machine.timer.div,
            tima: machine.timer.tima,
            tma: machine.timer.tma,
            tac: machine.timer.tac,
        }
    }
}
