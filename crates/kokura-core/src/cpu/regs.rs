use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
// Store the raw flag byte; named accessors operate on its upper nibble.
// Direct construction/deserialization can retain lower bits.
pub struct Flags(pub u8);

impl Flags {
    const Z: u8 = 0x80;
    const N: u8 = 0x40;
    const H: u8 = 0x20;
    const C: u8 = 0x10;

    // Read the zero-result flag from bit seven.
    pub fn z(self) -> bool {
        self.0 & Self::Z != 0
    }
    // Read the subtract-operation flag from bit six.
    pub fn n(self) -> bool {
        self.0 & Self::N != 0
    }
    // Read the half-carry flag from bit five.
    pub fn h(self) -> bool {
        self.0 & Self::H != 0
    }
    // Read the carry flag from bit four.
    pub fn c(self) -> bool {
        self.0 & Self::C != 0
    }

    // Set or clear only the zero-result flag.
    pub fn set_z(&mut self, value: bool) {
        self.set(Self::Z, value);
    }
    // Set or clear only the subtract-operation flag.
    pub fn set_n(&mut self, value: bool) {
        self.set(Self::N, value);
    }
    // Set or clear only the half-carry flag.
    pub fn set_h(&mut self, value: bool) {
        self.set(Self::H, value);
    }
    // Set or clear only the carry flag.
    pub fn set_c(&mut self, value: bool) {
        self.set(Self::C, value);
    }

    // Change the requested bits while preserving all others. This helper
    // does not sanitize the unused low nibble of the public raw flag byte.
    fn set(&mut self, mask: u8, value: bool) {
        if value {
            self.0 |= mask;
        } else {
            self.0 &= !mask;
        }
    }
}
