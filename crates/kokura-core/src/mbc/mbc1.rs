use serde::{Deserialize, Serialize};

use crate::mbc::{read_ram_bank, read_rom_bank, write_ram_bank};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mbc1 {
    pub rom_bank_low5: u8,
    pub bank_high2: u8,
    pub ram_bank: u8,
    pub ram_enabled: bool,
    pub mode: u8,
}

impl Default for Mbc1 {
    // Select ROM bank one, high bits/RAM bank zero and mode zero, with RAM disabled.
    fn default() -> Self {
        Self {
            rom_bank_low5: 1,
            bank_high2: 0,
            ram_bank: 0,
            ram_enabled: false,
            mode: 0,
        }
    }
}

impl Mbc1 {
    // Keep the lower ROM window at bank zero in mode zero; mode one
    // uses the high two selector bits shifted into the bank number.
    fn lower_rom_bank(&self) -> usize {
        if self.mode == 0 {
            0
        } else {
            ((self.bank_high2 & 0x03) as usize) << 5
        }
    }

    // Combine low-five and high-two ROM selector bits and map combined
    // zero to one. Normal register writes also remap a zero low-five value.
    pub fn current_rom_bank(&self) -> u16 {
        let mut bank = (self.rom_bank_low5 & 0x1F) | ((self.bank_high2 & 0x03) << 5);
        if bank == 0 {
            bank = 1;
        }
        bank as u16
    }

    // Use RAM bank zero in mode zero and the stored two-bit bank in mode one.
    pub fn current_ram_bank(&self) -> u16 {
        if self.mode == 0 {
            0
        } else {
            (self.ram_bank & 0x03) as u16
        }
    }

    // Map the two 16 KiB ROM windows through lower/upper selectors and
    // shared storage-size wrapping; addresses outside ROM return FF.
    pub fn read_rom(&self, rom: &[u8], addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => read_rom_bank(rom, self.lower_rom_bank(), 0x4000, addr as usize),
            0x4000..=0x7FFF => {
                let offset = (addr as usize) - 0x4000;
                read_rom_bank(rom, self.current_rom_bank() as usize, 0x4000, offset)
            }
            _ => 0xFF,
        }
    }

    // Gate RAM reads by the enable latch, then read a wrapped 8 KiB bank
    // using the caller-supplied external-RAM address.
    pub fn read_ram(&self, ram: &[u8], addr: u16) -> u8 {
        if !self.ram_enabled {
            return 0xFF;
        }
        let offset = (addr as usize).saturating_sub(0xA000);
        read_ram_bank(ram, self.current_ram_bank() as usize, 0x2000, offset)
    }

    // Ignore disabled RAM writes; otherwise update the wrapped selected
    // 8 KiB RAM bank if the target byte exists.
    pub fn write_ram(&mut self, ram: &mut [u8], addr: u16, value: u8) {
        if !self.ram_enabled {
            return;
        }
        let offset = (addr as usize).saturating_sub(0xA000);
        write_ram_bank(ram, self.current_ram_bank() as usize, 0x2000, offset, value);
    }

    // Decode RAM enable, low ROM bits, shared high-ROM/RAM bits and banking
    // mode. A zero low-ROM write selects one, independent of high bits.
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram_enabled = (value & 0x0F) == 0x0A,
            0x2000..=0x3FFF => {
                let value = value & 0x1F;
                self.rom_bank_low5 = if value == 0 { 1 } else { value };
            }
            0x4000..=0x5FFF => {
                self.bank_high2 = value & 0x03;
                self.ram_bank = value & 0x03;
            }
            0x6000..=0x7FFF => self.mode = value & 0x01,
            _ => {}
        }
    }
}
