use serde::{Deserialize, Serialize};

use crate::mbc::read_rom_bank;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
// Track whether incoming edges collect a command/data word or emit
// a read word; transaction boundaries reset this state.
enum EepromCommandMode {
    Idle,
    CollectingCommand,
    ReadingWord,
    WritingWord,
    WritingAll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mbc7 {
    pub rom_bank: u8,
    pub ram_enable_1: bool,
    pub ram_enable_2: bool,
    pub accel_latched: bool,
    pub accel_x: u16,
    pub accel_y: u16,
    pub eeprom_io: u8,
    // Store 128 big-endian words separately from the cartridge RAM slice.
    pub eeprom: Vec<u8>,
    pub eeprom_write_enabled: bool,
    eeprom_command_mode: EepromCommandMode,
    eeprom_command_started: bool,
    eeprom_shift_register: u32,
    eeprom_bits_collected: u8,
    eeprom_pending_address: u8,
    eeprom_output_word: u16,
    eeprom_output_bits_remaining: u8,
}

impl Default for Mbc7 {
    // Select ROM one, close both register gates, initialize centered
    // accelerometer values and erased 256-byte EEPROM, and reset serial
    // command state with writes disabled and data-out high.
    fn default() -> Self {
        Self {
            rom_bank: 1,
            ram_enable_1: false,
            ram_enable_2: false,
            accel_latched: false,
            accel_x: 0x8000,
            accel_y: 0x8000,
            eeprom_io: 0x01,
            eeprom: vec![0xFF; 0x100],
            eeprom_write_enabled: false,
            eeprom_command_mode: EepromCommandMode::Idle,
            eeprom_command_started: false,
            eeprom_shift_register: 0,
            eeprom_bits_collected: 0,
            eeprom_pending_address: 0,
            eeprom_output_word: 0,
            eeprom_output_bits_remaining: 0,
        }
    }
}

impl Mbc7 {
    // Require both cartridge register-access enable latches.
    fn ram_enabled(&self) -> bool {
        self.ram_enable_1 && self.ram_enable_2
    }

    // Read the EEPROM serial output bit retained in the I/O latch.
    fn data_out(&self) -> bool {
        self.eeprom_io & 0x01 != 0
    }

    // Change only the serial output bit, preserving stored CS/clock/input bits.
    fn set_data_out(&mut self, high: bool) {
        if high {
            self.eeprom_io |= 0x01;
        } else {
            self.eeprom_io &= !0x01;
        }
    }

    // Report the stored ROM selector, including zero; normal writes mask
    // it to seven bits.
    pub fn current_rom_bank(&self) -> u16 {
        self.rom_bank as u16
    }

    // Return bank zero for the device-register window; EEPROM has its
    // own word addresses rather than cartridge RAM banking.
    pub fn current_ram_bank(&self) -> u16 {
        0
    }

    // Read fixed lower ROM or the selected wrapped 16 KiB upper bank.
    pub fn read_rom(&self, rom: &[u8], addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => rom.get(addr as usize).copied().unwrap_or(0xFF),
            0x4000..=0x7FFF => {
                let offset = (addr as usize) - 0x4000;
                read_rom_bank(rom, self.rom_bank as usize, 0x4000, offset)
            }
            _ => 0xFF,
        }
    }

    // With both gates open, decode address bits 4-7 into accelerometer
    // bytes, constants or EEPROM pins. Supplied external RAM is unused and
    // other address bits mirror the register selection.
    pub fn read_ram(&self, _ram: &[u8], addr: u16) -> u8 {
        if !self.ram_enabled() {
            return 0xFF;
        }
        match (addr >> 4) & 0x0F {
            0x2 => self.accel_x as u8,
            0x3 => (self.accel_x >> 8) as u8,
            0x4 => self.accel_y as u8,
            0x5 => (self.accel_y >> 8) as u8,
            0x6 => 0x00,
            0x7 => 0xFF,
            0x8 => self.eeprom_io & 0xC3,
            _ => 0xFF,
        }
    }

    // With both gates open, process the 55/AA accelerometer reset/latch
    // sequence or clock EEPROM pins. The latched acceleration is the fixed
    // 81D0 value on both axes, not live host-sensor input.
    pub fn write_ram(&mut self, _ram: &mut [u8], addr: u16, value: u8) {
        if !self.ram_enabled() {
            return;
        }
        match (addr >> 4) & 0x0F {
            0x0 => {
                if value == 0x55 {
                    self.accel_latched = false;
                    self.accel_x = 0x8000;
                    self.accel_y = 0x8000;
                }
            }
            0x1 => {
                if value == 0xAA && !self.accel_latched {
                    self.accel_latched = true;
                    self.accel_x = 0x81D0;
                    self.accel_y = 0x81D0;
                }
            }
            0x8 => self.clock_eeprom(value),
            _ => {}
        }
    }

    // Decode the first enable key, seven-bit ROM selection and the second
    // enable key, which requires the exact byte 40.
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram_enable_1 = (value & 0x0F) == 0x0A,
            0x2000..=0x3FFF => self.rom_bank = value & 0x7F,
            0x4000..=0x5FFF => self.ram_enable_2 = value == 0x40,
            _ => {}
        }
    }

    // Do no cycle-based work; EEPROM changes follow pin writes and complete
    // without a modeled busy delay.
    pub fn tick(&mut self, _cycles: u32) {}

    // Detect chip-select transitions and selected rising clock edges,
    // then store input pins while preserving the output produced by processing.
    // A simultaneous CS rise and clock rise starts and clocks the transaction.
    fn clock_eeprom(&mut self, value: u8) {
        let next_io = value & 0xC2;
        let old_cs = self.eeprom_io & 0x80 != 0;
        let old_clk = self.eeprom_io & 0x40 != 0;
        let new_cs = next_io & 0x80 != 0;
        let new_clk = next_io & 0x40 != 0;
        let di = next_io & 0x02 != 0;

        if !old_cs && new_cs {
            self.begin_transaction();
        }

        if new_cs && !old_clk && new_clk {
            self.on_rising_clock(di);
        }

        if old_cs && !new_cs {
            self.end_transaction();
        }

        self.eeprom_io = next_io | u8::from(self.data_out());
    }

    // Reset command/shift progress and raise data-out while retaining
    // EEPROM contents and the write-enable latch.
    fn begin_transaction(&mut self) {
        self.eeprom_command_started = false;
        self.eeprom_command_mode = EepromCommandMode::Idle;
        self.eeprom_shift_register = 0;
        self.eeprom_bits_collected = 0;
        self.eeprom_output_bits_remaining = 0;
        self.set_data_out(true);
    }

    // Discard partial command/data progress on CS falling and raise
    // data-out, retaining completed writes and write-enable state.
    fn end_transaction(&mut self) {
        self.eeprom_command_started = false;
        self.eeprom_command_mode = EepromCommandMode::Idle;
        self.eeprom_shift_register = 0;
        self.eeprom_bits_collected = 0;
        self.eeprom_output_bits_remaining = 0;
        self.set_data_out(true);
    }

    // Wait for a start bit, collect ten command bits, then shift one word
    // out or collect sixteen data bits for write/write-all. Reads stop after
    // one word; writes honor the enable latch and finish immediately.
    fn on_rising_clock(&mut self, di: bool) {
        match self.eeprom_command_mode {
            EepromCommandMode::Idle => {
                if di {
                    self.eeprom_command_started = true;
                    self.eeprom_command_mode = EepromCommandMode::CollectingCommand;
                    self.eeprom_shift_register = 0;
                    self.eeprom_bits_collected = 0;
                }
            }
            EepromCommandMode::CollectingCommand => {
                self.push_bit(di);
                if self.eeprom_bits_collected == 10 {
                    self.decode_command();
                }
            }
            EepromCommandMode::ReadingWord => {
                if self.eeprom_output_bits_remaining > 0 {
                    let bit = (self.eeprom_output_word & 0x8000) != 0;
                    self.set_data_out(bit);
                    self.eeprom_output_word <<= 1;
                    self.eeprom_output_bits_remaining -= 1;
                } else {
                    self.set_data_out(true);
                }
            }
            EepromCommandMode::WritingWord => {
                self.push_bit(di);
                if self.eeprom_bits_collected == 16 {
                    if self.eeprom_write_enabled {
                        self.write_word(
                            self.eeprom_pending_address,
                            self.eeprom_shift_register as u16,
                        );
                    }
                    self.eeprom_command_mode = EepromCommandMode::Idle;
                    self.set_data_out(true);
                }
            }
            EepromCommandMode::WritingAll => {
                self.push_bit(di);
                if self.eeprom_bits_collected == 16 {
                    if self.eeprom_write_enabled {
                        let value = self.eeprom_shift_register as u16;
                        for address in 0..0x80u8 {
                            self.write_word(address, value);
                        }
                    }
                    self.eeprom_command_mode = EepromCommandMode::Idle;
                    self.set_data_out(true);
                }
            }
        }
    }

    // Shift the next input bit into the command/data accumulator and
    // saturate the collected-bit counter.
    fn push_bit(&mut self, high: bool) {
        self.eeprom_shift_register = (self.eeprom_shift_register << 1) | u32::from(high);
        self.eeprom_bits_collected = self.eeprom_bits_collected.saturating_add(1);
    }

    // Decode read/write/erase or global write-enable/erase-all/write-all
    // commands, clearing input progress first. Word addresses use seven bits.
    // Enable/disable commands retain CollectingCommand until CS resets it.
    fn decode_command(&mut self) {
        let bits = self.eeprom_shift_register as u16 & 0x03FF;
        let opcode = (bits >> 8) & 0x03;
        self.eeprom_shift_register = 0;
        self.eeprom_bits_collected = 0;

        match opcode {
            0b10 => {
                self.eeprom_pending_address = (bits & 0x7F) as u8;
                self.eeprom_output_word = self.read_word(self.eeprom_pending_address);
                self.eeprom_output_bits_remaining = 16;
                self.eeprom_command_mode = EepromCommandMode::ReadingWord;
            }
            0b01 => {
                self.eeprom_pending_address = (bits & 0x7F) as u8;
                self.eeprom_command_mode = EepromCommandMode::WritingWord;
            }
            0b11 => {
                self.eeprom_pending_address = (bits & 0x7F) as u8;
                if self.eeprom_write_enabled {
                    self.write_word(self.eeprom_pending_address, 0xFFFF);
                }
                self.eeprom_command_mode = EepromCommandMode::Idle;
                self.set_data_out(true);
            }
            0b00 => {
                match (bits >> 6) & 0x0F {
                    0b0000 => self.eeprom_write_enabled = false,
                    0b0001 => self.eeprom_command_mode = EepromCommandMode::WritingAll,
                    0b0010 => {
                        if self.eeprom_write_enabled {
                            self.eeprom.fill(0xFF);
                        }
                        self.eeprom_command_mode = EepromCommandMode::Idle;
                    }
                    0b0011 => self.eeprom_write_enabled = true,
                    _ => {}
                }
                self.set_data_out(true);
            }
            _ => {}
        }
    }

    // Combine two big-endian EEPROM bytes; missing bytes read as FF.
    fn read_word(&self, address: u8) -> u16 {
        let index = usize::from(address) * 2;
        let hi = self.eeprom.get(index).copied().unwrap_or(0xFF);
        let lo = self.eeprom.get(index + 1).copied().unwrap_or(0xFF);
        u16::from_be_bytes([hi, lo])
    }

    // Store a big-endian word into existing backing bytes. Enable checks
    // are the caller's responsibility; this helper only bounds-checks storage.
    fn write_word(&mut self, address: u8, value: u16) {
        let index = usize::from(address) * 2;
        let [hi, lo] = value.to_be_bytes();
        if let Some(slot) = self.eeprom.get_mut(index) {
            *slot = hi;
        }
        if let Some(slot) = self.eeprom.get_mut(index + 1) {
            *slot = lo;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Keep CS asserted while writing a low then high clock with one input bit.
    fn pulse(mbc7: &mut Mbc7, di: bool) {
        let low = 0x80 | if di { 0x02 } else { 0x00 };
        let high = low | 0x40;
        mbc7.write_ram(&mut [], 0xA080, low);
        mbc7.write_ram(&mut [], 0xA080, high);
    }

    // Drop then assert CS to start a fresh component-test transaction.
    fn begin(mbc7: &mut Mbc7) {
        mbc7.write_ram(&mut [], 0xA080, 0x00);
        mbc7.write_ram(&mut [], 0xA080, 0x80);
    }

    // Drop CS to end a component-test transaction and reset partial progress.
    fn end(mbc7: &mut Mbc7) {
        mbc7.write_ram(&mut [], 0xA080, 0x00);
    }

    // Start a transaction, send the leading one bit, then transmit the
    // provided command bits without imposing a command-length check here.
    fn send_start_and_command(mbc7: &mut Mbc7, command_bits: &[u8]) {
        begin(mbc7);
        pulse(mbc7, true);
        for &bit in command_bits {
            pulse(mbc7, bit != 0);
        }
    }

    #[test]
    // Enable writes, transmit word 1234 to address one, then read sixteen
    // output bits back. This covers one normal word path, not all commands,
    // busy timing, sensor behavior or persistence across machine reloads.
    fn eeprom_write_and_read_round_trip() {
        let mut mbc7 = Mbc7::default();
        mbc7.ram_enable_1 = true;
        mbc7.ram_enable_2 = true;

        send_start_and_command(&mut mbc7, &[0, 0, 1, 1, 0, 0, 0, 0, 0, 0]);
        end(&mut mbc7);

        send_start_and_command(&mut mbc7, &[0, 1, 0, 0, 0, 0, 0, 0, 0, 1]);
        for bit in [0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1, 1, 0, 1, 0, 0] {
            pulse(&mut mbc7, bit != 0);
        }
        end(&mut mbc7);

        send_start_and_command(&mut mbc7, &[1, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let mut read_back = 0u16;
        for _ in 0..16 {
            pulse(&mut mbc7, false);
            read_back = (read_back << 1) | u16::from(mbc7.read_ram(&[], 0xA080) & 0x01);
        }
        end(&mut mbc7);

        assert_eq!(read_back, 0x1234);
    }
}
