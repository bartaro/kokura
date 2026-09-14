//! CPU register state and instruction-related modules.
//!
//! Cpu holds registers, interrupt enable state and HALT/STOP state. Machine
//! combines instruction fetch/execution with memory access and timing; this
//! module exposes CPU state and instruction-category helpers.

pub mod decode;
pub mod exec;
pub mod ops_alu;
pub mod ops_jump;
pub mod ops_load;
pub mod ops_misc;
pub mod regs;

use regs::Flags;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cpu {
    pub a: u8,
    pub f: Flags,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
    pub ime: bool,
    #[serde(skip)]
    // Transient EI delay: cloning preserves it, but serde skips it so a
    // deserialized CPU starts with zero delay.
    pub ime_enable_delay: u8,
    pub halted: bool,
    #[serde(skip)]
    // Transient HALT fetch condition: retained by cloning but omitted from
    // serialized states and restored as false by deserialization.
    pub halt_bug: bool,
    pub stopped: bool,
}

impl Cpu {
    // Combine H and L into the 16-bit HL register pair, with H as the high byte.
    pub fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    // Split a 16-bit HL value back into its high and low register bytes.
    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }
}

impl Default for Cpu {
    // Create the base CPU state at cartridge entry 0x0100 with interrupts disabled.
    // Machine initialization applies any additional hardware-mode startup state.
    fn default() -> Self {
        Self {
            a: 0,
            f: Flags::default(),
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0xFFFE,
            pc: 0x0100,
            ime: false,
            ime_enable_delay: 0,
            halted: false,
            halt_bug: false,
            stopped: false,
        }
    }
}
