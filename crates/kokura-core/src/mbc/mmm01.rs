use serde::{Deserialize, Serialize};

use crate::mbc::Mbc1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mmm01 {
    pub mapped: bool,
    pub inner: Mbc1,
}

impl Default for Mmm01 {
    // Start with mapped false and the default embedded MBC1 state.
    fn default() -> Self {
        Self {
            mapped: false,
            inner: Mbc1::default(),
        }
    }
}

impl Mmm01 {
    // Report the embedded MBC1 selector; mapped does not alter this result.
    pub fn current_rom_bank(&self) -> u16 {
        self.inner.current_rom_bank()
    }

    // Report the embedded MBC1 RAM selector.
    pub fn current_ram_bank(&self) -> u16 {
        self.inner.current_ram_bank()
    }

    // Delegate ROM reads directly to MBC1. This wrapper does not implement
    // a separate startup mapping or mapping-lock address transformation.
    pub fn read_rom(&self, rom: &[u8], addr: u16) -> u8 {
        self.inner.read_rom(rom, addr)
    }

    // Delegate RAM reads to the embedded MBC1 bank and enable logic.
    pub fn read_ram(&self, ram: &[u8], addr: u16) -> u8 {
        self.inner.read_ram(ram, addr)
    }

    // Delegate RAM writes to the embedded MBC1 implementation.
    pub fn write_ram(&mut self, ram: &mut [u8], addr: u16, value: u8) {
        self.inner.write_ram(ram, addr, value);
    }

    // Latch mapped true on any ROM-window control write, then forward it
    // to MBC1. The mapped flag is observational and does not gate later mapping.
    pub fn write(&mut self, addr: u16, value: u8) {
        if (0x0000..=0x7FFF).contains(&addr) {
            self.mapped = true;
        }
        self.inner.write(addr, value);
    }
}
