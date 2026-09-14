use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::mbc::read_rom_bank;

// Read whole host-wall-clock seconds, falling back to zero before
// the Unix epoch rather than using emulation cycles.
fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

// Pack a value in the caller-expected 0-99 range as decimal tens/units
// nibbles; the helper does not clamp arbitrary inputs.
fn to_bcd(value: u8) -> u8 {
    ((value / 10) << 4) | (value % 10)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tama5 {
    pub rom_bank: u8,
    pub ram_enabled: bool,
    pub selected_register: u8,
    pub registers: Vec<u8>,
    pub rtc_latched: [u8; 8],
    pub rtc_last_timestamp_secs: u64,
}

impl Default for Tama5 {
    // Select ROM one, disable the register window, allocate zeroed registers
    // and RTC latches, and record the current host second without capturing it.
    fn default() -> Self {
        Self {
            rom_bank: 1,
            ram_enabled: false,
            selected_register: 0,
            registers: vec![0; 0x20],
            rtc_latched: [0; 8],
            rtc_last_timestamp_secs: unix_now_secs(),
        }
    }
}

impl Tama5 {
    // Mask the seven-bit ROM selector and map zero to bank one.
    pub fn current_rom_bank(&self) -> u16 {
        let bank = self.rom_bank & 0x7F;
        if bank == 0 {
            1
        } else {
            bank as u16
        }
    }

    // Report bank zero; this model exposes registers rather than banked RAM.
    pub fn current_ram_bank(&self) -> u16 {
        0
    }

    // Read fixed lower ROM and wrapped selected upper ROM in 16 KiB units.
    pub fn read_rom(&self, rom: &[u8], addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => rom.get(addr as usize).copied().unwrap_or(0xFF),
            0x4000..=0x7FFF => {
                let offset = (addr as usize) - 0x4000;
                read_rom_bank(rom, self.current_rom_bank() as usize, 0x4000, offset)
            }
            _ => 0xFF,
        }
    }

    // When enabled, read the selected RTC latch or register independently
    // of the CPU address and supplied RAM backing; disabled reads return FF.
    pub fn read_ram(&self, _ram: &[u8], _addr: u16) -> u8 {
        if !self.ram_enabled {
            return 0xFF;
        }
        match self.selected_register {
            0x10..=0x17 => self.rtc_latched[(self.selected_register - 0x10) as usize],
            _ => self
                .registers
                .get(self.selected_register as usize)
                .copied()
                .unwrap_or(0xFF),
        }
    }

    // When enabled, update low/high ROM bits, request a host-time latch,
    // write an RTC latch byte or store another register. The supplied external
    // RAM/address are unused; direct latch writes do not set the host clock.
    pub fn write_ram(&mut self, _ram: &mut [u8], _addr: u16, value: u8) {
        if !self.ram_enabled {
            return;
        }

        match self.selected_register {
            0x00 => self.rom_bank = (self.rom_bank & 0x70) | (value & 0x0F),
            0x01 => self.rom_bank = ((value & 0x07) << 4) | (self.rom_bank & 0x0F),
            0x0F => {
                if value & 0x01 != 0 {
                    self.latch_rtc_snapshot();
                }
            }
            0x10..=0x17 => {
                self.rtc_latched[(self.selected_register - 0x10) as usize] = value;
            }
            _ => {
                if let Some(byte) = self.registers.get_mut(self.selected_register as usize) {
                    *byte = value;
                }
            }
        }
    }

    // Decode window enable, direct ROM-bank selection and a five-bit register
    // selector. This path does not itself refresh the RTC latch.
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram_enabled = (value & 0x0F) == 0x0A,
            0x2000..=0x3FFF => {
                let bank = value & 0x7F;
                self.rom_bank = if bank == 0 { 1 } else { bank };
            }
            0x4000..=0x5FFF => self.selected_register = value & 0x1F,
            _ => {}
        }
    }

    // Refresh the RTC latch when the host timestamp changes, including a
    // backward change. A tick in the construction second can leave latches zero.
    pub fn tick(&mut self, _cycles: u32) {
        let now = unix_now_secs();
        if now != self.rtc_last_timestamp_secs {
            self.rtc_last_timestamp_secs = now;
            self.latch_rtc_snapshot();
        }
    }

    // Derive BCD second/minute/hour, epoch-day modulo 100 and weekday from
    // Unix time, then append ROM selector pieces. The day byte is not a
    // calendar day-of-month, and no month/year conversion is performed.
    fn latch_rtc_snapshot(&mut self) {
        let now = unix_now_secs();
        let second = (now % 60) as u8;
        let minute = ((now / 60) % 60) as u8;
        let hour = ((now / 3600) % 24) as u8;
        let day = ((now / 86_400) % 100) as u8;
        let weekday = ((now / 86_400 + 4) % 7) as u8;

        self.rtc_latched[0] = to_bcd(second);
        self.rtc_latched[1] = to_bcd(minute);
        self.rtc_latched[2] = to_bcd(hour);
        self.rtc_latched[3] = to_bcd(day);
        self.rtc_latched[4] = to_bcd(weekday);
        self.rtc_latched[5] = self.rom_bank & 0x0F;
        self.rtc_latched[6] = (self.rom_bank >> 4) & 0x07;
        self.rtc_latched[7] = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // Check low/high selector writes through the enabled register window
    // combine into ROM bank 25 hex.
    fn register_writes_can_rebuild_rom_bank() {
        let mut tama5 = Tama5::default();
        tama5.ram_enabled = true;

        tama5.selected_register = 0x00;
        tama5.write_ram(&mut [], 0xA000, 0x05);
        tama5.selected_register = 0x01;
        tama5.write_ram(&mut [], 0xA000, 0x02);

        assert_eq!(tama5.current_rom_bank(), 0x25);
    }

    #[test]
    // Check that the first RTC read is not FF. A zero-initialized latch also
    // passes, so this test does not prove that host time was freshly captured.
    fn rtc_latch_registers_are_populated() {
        let mut tama5 = Tama5::default();
        tama5.tick(0);
        tama5.ram_enabled = true;
        tama5.selected_register = 0x10;

        assert_ne!(tama5.read_ram(&[], 0xA000), 0xFF);
    }
}
