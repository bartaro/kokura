use serde::{Deserialize, Serialize};

use crate::mbc::{read_ram_bank, read_rom_bank, write_ram_bank};

const FLASH_LEN: usize = 0x100000;
const HIDDEN_FLASH_LEN: usize = 0x100;
const FLASH_SECTOR_LEN: usize = 0x20000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mbc6 {
    pub rom_bank_a: u8,
    pub rom_bank_b: u8,
    pub rom_bank_a_flash: bool,
    pub rom_bank_b_flash: bool,
    pub ram_bank_a: u8,
    pub ram_bank_b: u8,
    pub ram_enabled: bool,
    pub flash_enabled: bool,
    pub flash_write_enabled: bool,
    pub flash_data: Vec<u8>,
    pub hidden_flash: Vec<u8>,
    pub flash_sector0_protected: bool,
    pub flash_id_mode: bool,
    pub flash_hidden_mode: bool,
    pub flash_status_mode: bool,
    pub flash_status: u8,
    pub flash_program_mode: bool,
    pub flash_program_hidden_mode: bool,
    command_history: Vec<(u32, u8)>,
}

impl Default for Mbc6 {
    // Allocate erased 1 MiB flash and 256-byte hidden storage, select
    // ROM windows zero/one, disable RAM/flash access and reset command modes.
    fn default() -> Self {
        Self {
            rom_bank_a: 0,
            rom_bank_b: 1,
            rom_bank_a_flash: false,
            rom_bank_b_flash: false,
            ram_bank_a: 0,
            ram_bank_b: 0,
            ram_enabled: false,
            flash_enabled: false,
            flash_write_enabled: false,
            flash_data: vec![0xFF; FLASH_LEN],
            hidden_flash: vec![0xFF; HIDDEN_FLASH_LEN],
            flash_sector0_protected: false,
            flash_id_mode: false,
            flash_hidden_mode: false,
            flash_status_mode: false,
            flash_status: 0x80,
            flash_program_mode: false,
            flash_program_hidden_mode: false,
            command_history: Vec::new(),
        }
    }
}

impl Mbc6 {
    // Report window A only; window B and ROM-versus-flash selection
    // are not represented by this single diagnostic bank number.
    pub fn current_rom_bank(&self) -> u16 {
        self.rom_bank_a as u16
    }

    // Report RAM window A only; window B has an independent selector.
    pub fn current_ram_bank(&self) -> u16 {
        self.ram_bank_a as u16
    }

    // Read fixed lower ROM and independently selected 8 KiB ROM/flash
    // windows A and B, returning FF outside the cartridge ROM address range.
    pub fn read_rom(&self, rom: &[u8], addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => rom.get(addr as usize).copied().unwrap_or(0xFF),
            0x4000..=0x5FFF => {
                self.read_switchable_region(rom, addr, self.rom_bank_a, self.rom_bank_a_flash)
            }
            0x6000..=0x7FFF => {
                self.read_switchable_region(rom, addr, self.rom_bank_b, self.rom_bank_b_flash)
            }
            _ => 0xFF,
        }
    }

    // Gate access and route A000-AFFF/B000-BFFF through independent
    // wrapped 4 KiB RAM banks; other addresses return FF.
    pub fn read_ram(&self, ram: &[u8], addr: u16) -> u8 {
        if !self.ram_enabled {
            return 0xFF;
        }
        match addr {
            0xA000..=0xAFFF => {
                let offset = (addr as usize) - 0xA000;
                read_ram_bank(ram, self.ram_bank_a as usize, 0x1000, offset)
            }
            0xB000..=0xBFFF => {
                let offset = (addr as usize) - 0xB000;
                read_ram_bank(ram, self.ram_bank_b as usize, 0x1000, offset)
            }
            _ => 0xFF,
        }
    }

    // Write an existing byte in an enabled, independently selected
    // 4 KiB RAM window; ignore disabled or out-of-window accesses.
    pub fn write_ram(&mut self, ram: &mut [u8], addr: u16, value: u8) {
        if !self.ram_enabled {
            return;
        }
        match addr {
            0xA000..=0xAFFF => {
                let offset = (addr as usize) - 0xA000;
                write_ram_bank(ram, self.ram_bank_a as usize, 0x1000, offset, value);
            }
            0xB000..=0xBFFF => {
                let offset = (addr as usize) - 0xB000;
                write_ram_bank(ram, self.ram_bank_b as usize, 0x1000, offset, value);
            }
            _ => {}
        }
    }

    // Give selected flash-window writes priority when flash is enabled;
    // otherwise decode RAM gates, bank selectors and flash controls. Disabling
    // flash exits command modes without erasing its backing storage.
    pub fn write(&mut self, addr: u16, value: u8) {
        if self.flash_enabled && self.flash_region_selected(addr) {
            self.write_flash(addr, value);
            return;
        }

        match addr {
            0x0000..=0x03FF => self.ram_enabled = (value & 0x0F) == 0x0A,
            0x0400..=0x07FF => self.ram_bank_a = value & 0x07,
            0x0800..=0x0BFF => self.ram_bank_b = value & 0x07,
            0x0C00..=0x0FFF => {
                self.flash_enabled = value & 0x01 != 0;
                if !self.flash_enabled {
                    self.exit_flash_modes();
                }
            }
            0x1000..=0x13FF => self.flash_write_enabled = value & 0x01 != 0,
            0x2000..=0x27FF => self.rom_bank_a = value & 0x7F,
            0x2800..=0x2FFF => self.rom_bank_a_flash = value & 0x08 != 0,
            0x3000..=0x37FF => self.rom_bank_b = value & 0x7F,
            0x3800..=0x3FFF => self.rom_bank_b_flash = value & 0x08 != 0,
            _ => {}
        }
    }

    // Do no per-cycle work: flash commands complete synchronously in this model.
    pub fn tick(&mut self, _cycles: u32) {}

    // Resolve the byte offset within window A/B, then select ROM or
    // enabled flash. A selected but disabled flash region reads FF.
    fn read_switchable_region(&self, rom: &[u8], addr: u16, bank: u8, flash_selected: bool) -> u8 {
        let offset = match addr {
            0x4000..=0x5FFF => (addr as usize) - 0x4000,
            0x6000..=0x7FFF => (addr as usize) - 0x6000,
            _ => return 0xFF,
        };

        if flash_selected {
            if !self.flash_enabled {
                return 0xFF;
            }
            return self.read_flash(bank, offset);
        }

        read_rom_bank(rom, bank as usize, 0x2000, offset)
    }

    // Prioritize ID, status and hidden modes before ordinary flash bytes.
    // ID uses fixed values at offsets zero/one; hidden storage mirrors every
    // 256 bytes and normal storage wraps within the allocated flash size.
    fn read_flash(&self, bank: u8, offset: usize) -> u8 {
        if self.flash_id_mode {
            return match offset {
                0 => 0xC2,
                1 => 0x81,
                _ => 0xFF,
            };
        }
        if self.flash_status_mode {
            return self.flash_status_byte();
        }
        if self.flash_hidden_mode {
            return self
                .hidden_flash
                .get(offset & 0xFF)
                .copied()
                .unwrap_or(0xFF);
        }

        let absolute = usize::from(bank) * 0x2000 + offset;
        self.flash_data
            .get(absolute % FLASH_LEN)
            .copied()
            .unwrap_or(0xFF)
    }

    // Check whether the addressed 8 KiB window selects flash, independently
    // of the global flash-enable latch checked by the caller.
    fn flash_region_selected(&self, addr: u16) -> bool {
        matches!(addr, 0x4000..=0x5FFF if self.rom_bank_a_flash)
            || matches!(addr, 0x6000..=0x7FFF if self.rom_bank_b_flash)
    }

    // Handle reset or a pending one-byte program first, then retain up to
    // six absolute-address writes to recognize command sequences. Programming
    // only clears bits; program/erase operations enter status immediately.
    fn write_flash(&mut self, addr: u16, value: u8) {
        // Reset takes priority even while waiting for a program data byte.
        if value == 0xF0 {
            self.exit_flash_modes();
            return;
        }

        let Some(absolute) = self.flash_absolute_address(addr) else {
            return;
        };

        // Hidden-byte programming requires write enable and uses the low
        // eight address bits; the following status transition is unconditional.
        if self.flash_program_hidden_mode {
            if self.flash_write_enabled {
                let index = (absolute as usize) & 0xFF;
                if let Some(slot) = self.hidden_flash.get_mut(index) {
                    *slot &= value;
                }
            }
            self.enter_flash_status();
            return;
        }

        if self.flash_program_mode {
            if self.flash_can_program_absolute(absolute) {
                if let Some(slot) = self.flash_data.get_mut(absolute as usize % FLASH_LEN) {
                    *slot &= value;
                }
            }
            self.enter_flash_status();
            return;
        }

        // Command addresses refer to absolute flash offsets, not the CPU
        // window addresses used to reach those offsets.
        self.command_history.push((absolute, value));
        if self.command_history.len() > 6 {
            self.command_history.remove(0);
        }

        if self.matches_tail(&[
            (Some(0x5555), 0xAA),
            (Some(0x2AAA), 0x55),
            (Some(0x5555), 0x90),
        ]) {
            self.flash_id_mode = true;
            self.flash_hidden_mode = false;
            self.flash_status_mode = false;
            return;
        }
        if self.matches_tail(&[
            (Some(0x5555), 0xAA),
            (Some(0x2AAA), 0x55),
            (Some(0x5555), 0xA0),
        ]) {
            self.flash_program_mode = true;
            self.flash_program_hidden_mode = false;
            self.flash_id_mode = false;
            self.flash_hidden_mode = false;
            self.flash_status_mode = false;
            return;
        }
        if self.matches_tail(&[
            (Some(0x5555), 0xAA),
            (Some(0x2AAA), 0x55),
            (Some(0x5555), 0x60),
            (Some(0x5555), 0xAA),
            (Some(0x2AAA), 0x55),
            (Some(0x5555), 0xE0),
        ]) {
            self.flash_program_hidden_mode = self.flash_write_enabled;
            self.flash_program_mode = false;
            self.flash_hidden_mode = false;
            self.flash_id_mode = false;
            self.flash_status_mode = false;
            return;
        }
        if self.matches_tail(&[
            (Some(0x5555), 0xAA),
            (Some(0x2AAA), 0x55),
            (Some(0x5555), 0x60),
            (Some(0x5555), 0xAA),
            (Some(0x2AAA), 0x55),
            (Some(0x5555), 0x77),
        ]) {
            self.flash_hidden_mode = true;
            self.flash_id_mode = false;
            self.flash_status_mode = false;
            self.flash_program_mode = false;
            self.flash_program_hidden_mode = false;
            return;
        }
        if self.matches_tail(&[
            (Some(0x5555), 0xAA),
            (Some(0x2AAA), 0x55),
            (Some(0x5555), 0x60),
            (Some(0x5555), 0xAA),
            (Some(0x2AAA), 0x55),
            (Some(0x5555), 0x04),
        ]) {
            if self.flash_write_enabled {
                self.hidden_flash.fill(0xFF);
            }
            self.enter_flash_status();
            return;
        }
        if self.matches_tail(&[
            (Some(0x5555), 0xAA),
            (Some(0x2AAA), 0x55),
            (Some(0x5555), 0x60),
            (Some(0x5555), 0xAA),
            (Some(0x2AAA), 0x55),
            (Some(0x5555), 0x40),
        ]) {
            if self.flash_write_enabled {
                self.flash_sector0_protected = false;
            }
            self.enter_flash_status();
            return;
        }
        if self.matches_tail(&[
            (Some(0x5555), 0xAA),
            (Some(0x2AAA), 0x55),
            (Some(0x5555), 0x60),
            (Some(0x5555), 0xAA),
            (Some(0x2AAA), 0x55),
            (Some(0x5555), 0x20),
        ]) {
            if self.flash_write_enabled {
                self.flash_sector0_protected = true;
            }
            self.enter_flash_status();
            return;
        }
        if self.matches_tail(&[
            (Some(0x5555), 0xAA),
            (Some(0x2AAA), 0x55),
            (Some(0x5555), 0x80),
            (Some(0x5555), 0xAA),
            (Some(0x2AAA), 0x55),
            (Some(0x5555), 0x10),
        ]) {
            self.erase_chip();
            self.enter_flash_status();
            return;
        }
        // Sector erase accepts its final 30 command at the target sector
        // address after the five fixed unlock/erase-prefix writes.
        if self.command_history.len() >= 6
            && self.matches_history_slice(
                self.command_history.len() - 6,
                &[
                    (Some(0x5555), 0xAA),
                    (Some(0x2AAA), 0x55),
                    (Some(0x5555), 0x80),
                    (Some(0x5555), 0xAA),
                    (Some(0x2AAA), 0x55),
                ],
            )
        {
            if let Some((absolute, 0x30)) = self.command_history.last().copied() {
                self.erase_sector_containing(absolute);
                self.enter_flash_status();
            }
        }
    }

    // Translate a selected flash-window CPU address into its bank-relative
    // absolute flash address; reject other windows.
    fn flash_absolute_address(&self, addr: u16) -> Option<u32> {
        match addr {
            0x4000..=0x5FFF if self.rom_bank_a_flash => {
                Some(u32::from(self.rom_bank_a) * 0x2000 + u32::from(addr - 0x4000))
            }
            0x6000..=0x7FFF if self.rom_bank_b_flash => {
                Some(u32::from(self.rom_bank_b) * 0x2000 + u32::from(addr - 0x6000))
            }
            _ => None,
        }
    }

    // Compare a command pattern against the most recent history entries,
    // returning false until enough writes have been collected.
    fn matches_tail(&self, pattern: &[(Option<u32>, u8)]) -> bool {
        if self.command_history.len() < pattern.len() {
            return false;
        }
        self.matches_history_slice(self.command_history.len() - pattern.len(), pattern)
    }

    // Match values and optional absolute addresses within a bounded
    // history slice; None in the pattern accepts any address.
    fn matches_history_slice(&self, start: usize, pattern: &[(Option<u32>, u8)]) -> bool {
        if self.command_history.len() < start + pattern.len() {
            return false;
        }
        self.command_history[start..start + pattern.len()]
            .iter()
            .zip(pattern.iter())
            .all(|((absolute, value), (expected_address, expected_value))| {
                expected_address.map_or(true, |address| *absolute == address)
                    && *value == *expected_value
            })
    }

    // Select a 128 KiB sector and fill it with FF. Protection and the
    // write-enable latch gate only sector zero in this implementation; other
    // sectors can be erased while that latch is clear.
    fn erase_sector_containing(&mut self, absolute: u32) {
        let sector = (absolute as usize / FLASH_SECTOR_LEN).min((FLASH_LEN / FLASH_SECTOR_LEN) - 1);
        if sector == 0 && (self.flash_sector0_protected || !self.flash_write_enabled) {
            return;
        }
        let base = sector * FLASH_SECTOR_LEN;
        let end = (base + FLASH_SECTOR_LEN).min(self.flash_data.len());
        self.flash_data[base..end].fill(0xFF);
    }

    // Erase all flash unless sector zero is protected or write enable is
    // clear; in either case preserve sector zero and erase the remaining sectors.
    fn erase_chip(&mut self) {
        if self.flash_sector0_protected || !self.flash_write_enabled {
            let sector1 = FLASH_SECTOR_LEN.min(self.flash_data.len());
            self.flash_data[sector1..].fill(0xFF);
        } else {
            self.flash_data.fill(0xFF);
        }
    }

    // Permit programming outside sector zero regardless of write enable.
    // Sector zero requires both write enable and its protection flag cleared.
    fn flash_can_program_absolute(&self, absolute: u32) -> bool {
        let sector = absolute as usize / FLASH_SECTOR_LEN;
        sector != 0 || (!self.flash_sector0_protected && self.flash_write_enabled)
    }

    // End pending program modes and select an immediately ready status
    // readout. ID/hidden flags are retained, so read-mode priority still applies.
    fn enter_flash_status(&mut self) {
        self.flash_program_mode = false;
        self.flash_program_hidden_mode = false;
        self.flash_status_mode = true;
        self.flash_status = self.flash_status_byte();
    }

    // Report ready plus the sector-zero protection flag; there is no
    // modeled busy interval or asynchronous programming-error status.
    fn flash_status_byte(&self) -> u8 {
        0x80 | if self.flash_sector0_protected {
            0x02
        } else {
            0x00
        }
    }

    // Clear all read/program command modes and history while preserving
    // flash bytes, protection state and bank/access registers.
    fn exit_flash_modes(&mut self) {
        self.flash_id_mode = false;
        self.flash_hidden_mode = false;
        self.flash_status_mode = false;
        self.flash_program_mode = false;
        self.flash_program_hidden_mode = false;
        self.command_history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Arrange enabled flash window A at a chosen bank before issuing one
    // CPU-address write, allowing command addresses to cross selected banks.
    fn write_flash_a(mbc6: &mut Mbc6, bank: u8, addr: u16, value: u8) {
        mbc6.flash_enabled = true;
        mbc6.rom_bank_a_flash = true;
        mbc6.rom_bank_a = bank;
        mbc6.write(addr, value);
    }

    #[test]
    // Check one selected flash bank byte through the ROM read interface.
    fn flash_bank_reads_selected_storage() {
        let mut mbc6 = Mbc6::default();
        mbc6.flash_enabled = true;
        mbc6.rom_bank_a_flash = true;
        mbc6.rom_bank_a = 1;
        mbc6.flash_data[0x2000] = 0x42;

        assert_eq!(mbc6.read_rom(&[], 0x4000), 0x42);
    }

    #[test]
    // Send the identification sequence through banked window A, check its
    // two ID bytes, and confirm F0 restores an erased flash read.
    fn id_mode_reports_jedec_and_exits() {
        let mut mbc6 = Mbc6::default();

        write_flash_a(&mut mbc6, 2, 0x5555, 0xAA);
        write_flash_a(&mut mbc6, 1, 0x4AAA, 0x55);
        write_flash_a(&mut mbc6, 2, 0x5555, 0x90);

        mbc6.rom_bank_a = 0;
        assert_eq!(mbc6.read_rom(&[], 0x4000), 0xC2);
        assert_eq!(mbc6.read_rom(&[], 0x4001), 0x81);

        write_flash_a(&mut mbc6, 0, 0x4000, 0xF0);
        assert_eq!(mbc6.read_rom(&[], 0x4000), 0xFF);
    }

    #[test]
    // Program one byte in writable sector zero and erase it using the
    // sector command sequence. Other sectors and protection paths are not tested.
    fn program_and_sector_erase_work_for_flash() {
        let mut mbc6 = Mbc6::default();
        mbc6.flash_write_enabled = true;

        write_flash_a(&mut mbc6, 2, 0x5555, 0xAA);
        write_flash_a(&mut mbc6, 1, 0x4AAA, 0x55);
        write_flash_a(&mut mbc6, 2, 0x5555, 0xA0);
        write_flash_a(&mut mbc6, 0, 0x4000, 0x12);
        assert_eq!(mbc6.flash_data[0], 0x12);

        write_flash_a(&mut mbc6, 2, 0x5555, 0xAA);
        write_flash_a(&mut mbc6, 1, 0x4AAA, 0x55);
        write_flash_a(&mut mbc6, 2, 0x5555, 0x80);
        write_flash_a(&mut mbc6, 2, 0x5555, 0xAA);
        write_flash_a(&mut mbc6, 1, 0x4AAA, 0x55);
        write_flash_a(&mut mbc6, 0, 0x4000, 0x30);

        assert_eq!(mbc6.flash_data[0], 0xFF);
    }
}
