use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RomOnly;

impl RomOnly {
    // Label the upper 16 KiB ROM window as bank one; no selector is stored.
    pub fn current_rom_bank(&self) -> u16 {
        1
    }

    // Report the single unbanked RAM window as bank zero.
    pub fn current_ram_bank(&self) -> u16 {
        0
    }

    // Use the CPU address directly as a ROM offset and return FF if absent.
    pub fn read_rom(&self, rom: &[u8], addr: u16) -> u8 {
        rom.get(addr as usize).copied().unwrap_or(0xFF)
    }

    // Read unbanked RAM relative to A000 without an enable latch. Callers
    // supply a cartridge-RAM address; subtraction saturates for lower addresses.
    pub fn read_ram(&self, ram: &[u8], addr: u16) -> u8 {
        let offset = (addr as usize).saturating_sub(0xA000);
        ram.get(offset).copied().unwrap_or(0xFF)
    }

    // Write an existing unbanked RAM byte relative to A000; no enable
    // latch or mirroring is applied, and absent backing is ignored.
    pub fn write_ram(&mut self, ram: &mut [u8], addr: u16, value: u8) {
        let offset = (addr as usize).saturating_sub(0xA000);
        if let Some(byte) = ram.get_mut(offset) {
            *byte = value;
        }
    }

    // Ignore cartridge control writes because this controller has no bank registers.
    pub fn write(&mut self, _addr: u16, _value: u8) {}
}
