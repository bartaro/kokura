use serde::{Deserialize, Serialize};

use crate::mbc::Mbc1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuC1 {
    pub inner: Mbc1,
    pub ir_mode: bool,
    pub ir_output: bool,
}

impl Default for HuC1 {
    // Initialize MBC1 banking with IR mode/output disabled.
    fn default() -> Self {
        Self {
            inner: Mbc1::default(),
            ir_mode: false,
            ir_output: false,
        }
    }
}

impl HuC1 {
    // Expose the ROM selector from the embedded MBC1 model.
    pub fn current_rom_bank(&self) -> u16 {
        self.inner.current_rom_bank()
    }

    // Expose the RAM selector from the embedded MBC1 model.
    pub fn current_ram_bank(&self) -> u16 {
        self.inner.current_ram_bank()
    }

    // Use the embedded MBC1 ROM mapping in both RAM and IR modes.
    pub fn read_rom(&self, rom: &[u8], addr: u16) -> u8 {
        self.inner.read_rom(rom, addr)
    }

    // In IR mode return C0 plus the stored output bit; otherwise delegate
    // to MBC1 RAM. This register echo does not sample an external IR receiver.
    pub fn read_ram(&self, ram: &[u8], addr: u16) -> u8 {
        if self.ir_mode {
            0xC0 | u8::from(self.ir_output)
        } else {
            self.inner.read_ram(ram, addr)
        }
    }

    // In IR mode latch output bit zero; otherwise perform the MBC1 RAM write.
    pub fn write_ram(&mut self, ram: &mut [u8], addr: u16, value: u8) {
        if self.ir_mode {
            self.ir_output = value & 0x01 != 0;
        } else {
            self.inner.write_ram(ram, addr, value);
        }
    }

    // Select IR for a low-nibble value of E in 0000-1FFF. Every other value
    // selects and enables RAM in this model; other controls delegate to MBC1.
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                let mode = value & 0x0F;
                self.ir_mode = mode == 0x0E;
                self.inner.ram_enabled = !self.ir_mode;
            }
            _ => self.inner.write(addr, value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // Check RAM access after a zero mode write, which selects RAM rather
    // than using the ordinary MBC1 RAM-disable interpretation.
    fn ram_mode_is_not_disabled_by_non_ir_values() {
        let mut huc1 = HuC1::default();
        let mut ram = vec![0; 0x2000];

        huc1.write(0x0000, 0x00);
        huc1.write_ram(&mut ram, 0xA000, 0x42);

        assert_eq!(huc1.read_ram(&ram, 0xA000), 0x42);
    }

    #[test]
    // Check the IR output-bit echo after selecting IR mode and writing one.
    // This does not exercise communication with another device.
    fn ir_mode_exposes_ir_register() {
        let mut huc1 = HuC1::default();
        let mut ram = vec![0; 0x2000];

        huc1.write(0x0000, 0x0E);
        huc1.write_ram(&mut ram, 0xA000, 0x01);

        assert_eq!(huc1.read_ram(&ram, 0xA000), 0xC1);
    }
}
