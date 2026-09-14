use serde::{Deserialize, Serialize};

const VRAM_BANK_SIZE: usize = 0x2000;
const WRAM_BANK_SIZE: usize = 0x1000;
const BG_PALETTE_RAM_LEN: usize = 0x40;
const OBJ_PALETTE_RAM_LEN: usize = 0x40;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
// Describe an attempted palette write, including blocked accesses and
// the auto-increment setting observed before the index advances.
pub struct PaletteAccessResult {
    pub index: u8,
    pub value: u8,
    pub blocked: bool,
    pub auto_increment: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Bank backing plus a flat address image. Machine bus code supplies
// cartridge/device behavior and timing restrictions above this storage layer.
pub struct Memory {
    data: Vec<u8>,
    vram_banks: Vec<u8>,
    wram_banks: Vec<u8>,
    vbk: u8,
    svbk: u8,
    bg_palette_index: u8,
    obj_palette_index: u8,
    bg_palette_ram: Vec<u8>,
    obj_palette_ram: Vec<u8>,
}

impl Memory {
    /// Creates the memory image seen after KOKURA's boot-ROM bypass.
    ///
    /// Real DMG/CGB HRAM is not guaranteed to contain zero after power-on.
    /// Keep the pattern deterministic for reproducible debugging, but make it
    /// non-zero so ROMs cannot accidentally rely on KOKURA's old zero-filled
    /// HRAM behavior.
    // Seed the default image with a reproducible HRAM pattern over FF80-FFFE.
    // This models the boot bypass convention rather than a hardware RAM dump.
    pub fn power_on() -> Self {
        let mut out = Self::default();
        let mut state = 0x6Du8;
        for addr in 0xFF80usize..=0xFFFE {
            state = state.wrapping_mul(17).wrapping_add(0x3D);
            out.data[addr] = state ^ addr as u8;
        }
        out
    }

    // Route VRAM, fixed/switched WRAM and its echo to bank backing; read other
    // addresses from the flat image. Device side effects belong to the machine bus.
    pub fn read8(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0x9FFF => self.read_vram8(addr),
            0xC000..=0xCFFF => self.read_wram8(addr),
            0xD000..=0xDFFF => self.read_wram8(addr),
            0xE000..=0xFDFF => self.read_wram8(addr - 0x2000),
            _ => self.data[addr as usize],
        }
    }

    // Update bank backing and shadows for VRAM/WRAM/echo writes; otherwise
    // write the flat image without dispatching device-register behavior.
    pub fn write8(&mut self, addr: u16, value: u8) {
        match addr {
            0x8000..=0x9FFF => self.write_vram8(addr, value),
            0xC000..=0xCFFF => self.write_wram8(addr, value),
            0xD000..=0xDFFF => self.write_wram8(addr, value),
            0xE000..=0xFDFF => self.write_wram8(addr - 0x2000, value),
            _ => self.data[addr as usize] = value,
        }
    }

    // Borrow the currently selected 8 KiB VRAM bank.
    pub fn vram(&self) -> &[u8] {
        self.vram_bank(self.vbk)
    }

    // Borrow selected VRAM backing for direct writes; this mutable slice
    // does not synchronize the separate flat-image shadow.
    pub fn vram_mut(&mut self) -> &mut [u8] {
        let bank = self.vbk;
        self.vram_bank_mut(bank)
    }

    // Mask the selector to one bit and borrow that complete VRAM bank.
    pub fn vram_bank(&self, bank: u8) -> &[u8] {
        let bank = usize::from(bank & 0x01);
        let start = bank * VRAM_BANK_SIZE;
        &self.vram_banks[start..start + VRAM_BANK_SIZE]
    }

    // Borrow one masked VRAM bank directly, bypassing flat-shadow updates.
    pub fn vram_bank_mut(&mut self, bank: u8) -> &mut [u8] {
        let bank = usize::from(bank & 0x01);
        let start = bank * VRAM_BANK_SIZE;
        &mut self.vram_banks[start..start + VRAM_BANK_SIZE]
    }

    // Return the effective low-bit VRAM bank selector.
    pub fn active_vram_bank(&self) -> u8 {
        self.vbk & 0x01
    }

    // Latch the low selector bit, update the register shadow and refresh
    // the flat VRAM window before returning the selected bank.
    pub fn set_vbk(&mut self, value: u8) -> u8 {
        self.vbk = value & 0x01;
        self.data[0xFF4F] = 0xFE | self.vbk;
        self.refresh_vram_window();
        self.vbk
    }

    // Expose the VRAM selector with unused register bits set.
    pub fn vbk_register(&self) -> u8 {
        0xFE | self.vbk
    }

    // Borrow the 160-byte sprite-attribute region from the flat image.
    pub fn oam(&self) -> &[u8] {
        &self.data[0xFE00..0xFEA0]
    }

    // Borrow sprite-attribute storage directly; bus access restrictions
    // are not enforced by this slice accessor.
    pub fn oam_mut(&mut self) -> &mut [u8] {
        &mut self.data[0xFE00..0xFEA0]
    }

    // Resolve the three-bit selector, treating register value zero as bank one.
    pub fn active_wram_bank(&self) -> u8 {
        let bank = self.svbk & 0x07;
        if bank == 0 {
            1
        } else {
            bank
        }
    }

    // Keep the raw three-bit register selection, refresh WRAM and echo
    // shadows, and return the effective bank with zero mapped to one.
    pub fn set_svbk(&mut self, value: u8) -> u8 {
        self.svbk = value & 0x07;
        self.data[0xFF70] = 0xF8 | self.svbk;
        self.refresh_wram_window();
        self.active_wram_bank()
    }

    // Return the raw WRAM selector with unused high bits set.
    pub fn svbk_register(&self) -> u8 {
        0xF8 | self.svbk
    }

    // Read the selected background palette byte, or FF when the caller
    // indicates that palette access is blocked; reads do not advance the index.
    pub fn read_bg_palette_data(&self, blocked: bool) -> u8 {
        if blocked {
            0xFF
        } else {
            self.bg_palette_ram[usize::from(self.bg_palette_index & 0x3F)]
        }
    }

    // Write only when allowed and report the attempted index/value.
    // Auto-increment still advances a blocked write, wraps within 64 bytes,
    // and updates the index and selected-data register shadows.
    pub fn write_bg_palette_data(&mut self, value: u8, blocked: bool) -> PaletteAccessResult {
        let index = self.bg_palette_index & 0x3F;
        let auto_increment = self.bg_palette_index & 0x80 != 0;
        if !blocked {
            self.bg_palette_ram[usize::from(index)] = value;
        }
        let result = PaletteAccessResult {
            index,
            value,
            blocked,
            auto_increment,
        };
        if auto_increment {
            self.bg_palette_index = 0x80 | index.wrapping_add(1) & 0x3F;
            self.data[0xFF68] = self.bg_palette_index;
        }
        self.data[0xFF69] = self.read_bg_palette_data(blocked);
        result
    }

    // Latch the six-bit index and auto-increment flag, discarding bit six.
    // Only the index register shadow is refreshed here.
    pub fn set_bg_palette_index(&mut self, value: u8) -> u8 {
        self.bg_palette_index = value & 0xBF;
        self.data[0xFF68] = self.bg_palette_index;
        self.bg_palette_index
    }

    // Return the stored background index and auto-increment flag.
    pub fn bg_palette_index_register(&self) -> u8 {
        self.bg_palette_index
    }

    // Read the selected object palette byte, returning FF for blocked access
    // without advancing the index.
    pub fn read_obj_palette_data(&self, blocked: bool) -> u8 {
        if blocked {
            0xFF
        } else {
            self.obj_palette_ram[usize::from(self.obj_palette_index & 0x3F)]
        }
    }

    // Apply an allowed object palette write and report the attempted access.
    // Auto-increment and register-shadow updates also occur on blocked writes.
    pub fn write_obj_palette_data(&mut self, value: u8, blocked: bool) -> PaletteAccessResult {
        let index = self.obj_palette_index & 0x3F;
        let auto_increment = self.obj_palette_index & 0x80 != 0;
        if !blocked {
            self.obj_palette_ram[usize::from(index)] = value;
        }
        let result = PaletteAccessResult {
            index,
            value,
            blocked,
            auto_increment,
        };
        if auto_increment {
            self.obj_palette_index = 0x80 | index.wrapping_add(1) & 0x3F;
            self.data[0xFF6A] = self.obj_palette_index;
        }
        self.data[0xFF6B] = self.read_obj_palette_data(blocked);
        result
    }

    // Latch the object index and auto-increment flag without refreshing
    // the separate data-register shadow.
    pub fn set_obj_palette_index(&mut self, value: u8) -> u8 {
        self.obj_palette_index = value & 0xBF;
        self.data[0xFF6A] = self.obj_palette_index;
        self.obj_palette_index
    }

    // Return the stored object index and auto-increment flag.
    pub fn obj_palette_index_register(&self) -> u8 {
        self.obj_palette_index
    }

    // Fetch a background color word from the selected palette/color slot.
    pub fn bg_palette_rgb555(&self, palette: u8, color: u8) -> u16 {
        self.palette_rgb555(&self.bg_palette_ram, palette, color)
    }

    // Fetch an object color word from the selected palette/color slot.
    pub fn obj_palette_rgb555(&self, palette: u8, color: u8) -> u16 {
        self.palette_rgb555(&self.obj_palette_ram, palette, color)
    }

    // Mask palette/color selectors and combine two little-endian bytes,
    // substituting zero for missing bytes. Bit 15 is retained in the returned word.
    fn palette_rgb555(&self, ram: &[u8], palette: u8, color: u8) -> u16 {
        let base = usize::from((palette & 0x07) * 8 + (color & 0x03) * 2);
        let lo = ram.get(base).copied().unwrap_or(0) as u16;
        let hi = ram.get(base + 1).copied().unwrap_or(0) as u16;
        (hi << 8) | lo
    }

    // Read selected bank backing for a caller-validated 8000-9FFF address.
    fn read_vram8(&self, addr: u16) -> u8 {
        let offset = usize::from(addr - 0x8000);
        self.vram_bank(self.vbk)[offset]
    }

    // Write selected VRAM backing and its flat-image byte together.
    // The caller supplies an address within the VRAM window.
    fn write_vram8(&mut self, addr: u16, value: u8) {
        let offset = usize::from(addr - 0x8000);
        let bank = usize::from(self.vbk);
        self.vram_banks[bank * VRAM_BANK_SIZE + offset] = value;
        self.data[addr as usize] = value;
    }

    // Resolve fixed bank zero or effective switched WRAM, falling back
    // to the flat image for addresses outside the two WRAM windows.
    fn read_wram8(&self, addr: u16) -> u8 {
        match addr {
            0xC000..=0xCFFF => self.wram_banks[usize::from(addr - 0xC000)],
            0xD000..=0xDFFF => {
                let bank = usize::from(self.active_wram_bank());
                let offset = usize::from(addr - 0xD000);
                self.wram_banks[bank * WRAM_BANK_SIZE + offset]
            }
            _ => self.data[addr as usize],
        }
    }

    // Update WRAM backing, its CPU window and the corresponding echo.
    // The switched-bank echo stops at DDFF because FE00 begins OAM.
    fn write_wram8(&mut self, addr: u16, value: u8) {
        match addr {
            0xC000..=0xCFFF => {
                let offset = usize::from(addr - 0xC000);
                self.wram_banks[offset] = value;
                self.data[addr as usize] = value;
                self.data[usize::from(addr + 0x2000)] = value;
            }
            0xD000..=0xDFFF => {
                let bank = usize::from(self.active_wram_bank());
                let offset = usize::from(addr - 0xD000);
                self.wram_banks[bank * WRAM_BANK_SIZE + offset] = value;
                self.data[addr as usize] = value;
                if addr <= 0xDDFF {
                    self.data[usize::from(addr + 0x2000)] = value;
                }
            }
            _ => self.data[addr as usize] = value,
        }
    }

    // Copy the selected VRAM bank into the flat CPU-address shadow.
    fn refresh_vram_window(&mut self) {
        let start = usize::from(self.vbk) * VRAM_BANK_SIZE;
        self.data[0x8000..0xA000].copy_from_slice(&self.vram_banks[start..start + VRAM_BANK_SIZE]);
    }

    // Refresh both WRAM windows, then mirror C000-CFFF and D000-DDFF
    // into the two echo regions without overwriting OAM.
    fn refresh_wram_window(&mut self) {
        self.data[0xC000..0xD000].copy_from_slice(&self.wram_banks[0..WRAM_BANK_SIZE]);
        let bank = usize::from(self.active_wram_bank());
        let start = bank * WRAM_BANK_SIZE;
        self.data[0xD000..0xE000].copy_from_slice(&self.wram_banks[start..start + WRAM_BANK_SIZE]);
        self.data.copy_within(0xC000..0xD000, 0xE000);
        self.data.copy_within(0xD000..0xDE00, 0xF000);
    }
}

impl Default for Memory {
    // Allocate zeroed flat/VRAM/WRAM storage, select VRAM zero and WRAM one,
    // initialize background palettes to FF and object palettes to zero, then
    // refresh bank windows and register shadows. HRAM stays zero in this fixture.
    fn default() -> Self {
        let mut out = Self {
            data: vec![0; 0x10000],
            vram_banks: vec![0; VRAM_BANK_SIZE * 2],
            wram_banks: vec![0; WRAM_BANK_SIZE * 8],
            vbk: 0,
            svbk: 1,
            bg_palette_index: 0,
            obj_palette_index: 0,
            bg_palette_ram: vec![0xFF; BG_PALETTE_RAM_LEN],
            obj_palette_ram: vec![0; OBJ_PALETTE_RAM_LEN],
        };
        out.refresh_vram_window();
        out.refresh_wram_window();
        out.data[0xFF4F] = 0xFE;
        out.data[0xFF68] = 0x00;
        out.data[0xFF69] = 0xFF;
        out.data[0xFF6A] = 0x00;
        out.data[0xFF6B] = 0x00;
        out.data[0xFF70] = 0xF9;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::Memory;

    #[test]
    // Check one nonzero HRAM byte and deterministic values at three addresses
    // across two power-on images; this does not validate a physical RAM pattern.
    fn power_on_hram_is_nonzero_and_reproducible() {
        let first = Memory::power_on();
        let second = Memory::power_on();

        assert_ne!(first.read8(0xFF9F), 0);
        assert_eq!(first.read8(0xFF80), second.read8(0xFF80));
        assert_eq!(first.read8(0xFF9F), second.read8(0xFF9F));
        assert_eq!(first.read8(0xFFFE), second.read8(0xFFFE));
    }

    #[test]
    // Keep the plain default fixture distinct from power-on initialization
    // by checking that its representative HRAM byte remains zero.
    fn plain_default_remains_zero_for_component_fixtures() {
        let memory = Memory::default();

        assert_eq!(memory.read8(0xFF9F), 0);
    }
}
