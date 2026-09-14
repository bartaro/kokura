use serde::{Deserialize, Serialize};

use crate::types::SerialTraceEvent;

const SERIAL_INTERNAL_BIT_CYCLES: u16 = 512;

#[derive(Debug, Clone, Default)]
// Return an internal-clock completion request and optional trace records.
pub struct SerialTickResult {
    pub interrupt_requested: bool,
    pub trace: Vec<SerialTraceEvent>,
}

#[derive(Debug, Clone, Default)]
// Report whole-byte peer exchange completion and each endpoint
// interrupt/trace result separately.
pub struct SerialLinkExchangeResult {
    pub completed: bool,
    pub self_interrupt_requested: bool,
    pub peer_interrupt_requested: bool,
    pub self_trace: Vec<SerialTraceEvent>,
    pub peer_trace: Vec<SerialTraceEvent>,
}

#[derive(Debug, Clone, Default)]
// Report a completed external byte and its outgoing SB value.
// When completed is false, the default outgoing byte is not a transfer result.
pub struct SerialExternalClockResult {
    pub completed: bool,
    pub outgoing: u8,
    pub interrupt_requested: bool,
    pub trace: Vec<SerialTraceEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Serial {
    pub sb: u8,
    pub sc: u8,
    cycle_accum: u16,
    bits_remaining: u8,
    transfer_active: bool,
}

impl Serial {
    // Expose the stored control byte with bits 1-6 forced high. Although
    // write_sc stores bit one, this accessor does not reveal its stored value.
    pub fn read_sc(&self) -> u8 {
        self.sc | 0x7E
    }

    // Replace the serial data/shift register, including during an active transfer.
    pub fn write_sb(&mut self, value: u8) {
        self.sb = value;
    }

    // Keep control bits 7, 1 and 0. A set start bit restarts an eight-bit
    // transfer and emits its start record; a clear start bit cancels transfer
    // progress. Clock rate in tick remains fixed regardless of bit one.
    pub fn write_sc(&mut self, value: u8) -> Vec<SerialTraceEvent> {
        self.sc = value & 0x83;
        let mut trace = Vec::new();
        if self.sc & 0x80 != 0 {
            self.transfer_active = true;
            self.bits_remaining = 8;
            self.cycle_accum = 0;
            trace.push(SerialTraceEvent::TransferStart {
                sb: self.sb,
                sc: self.read_sc(),
                internal_clock: self.internal_clock(),
            });
        } else {
            self.transfer_active = false;
            self.bits_remaining = 0;
            self.cycle_accum = 0;
        }
        trace
    }

    // Advance an armed internal-clock transfer at 512 cycles per bit,
    // shifting in ones for the disconnected input and requesting IRQ after
    // eight bits. The supplied cycle count is narrowed to u16 before accumulation.
    pub fn tick(&mut self, cycles: u32) -> SerialTickResult {
        let mut out = SerialTickResult::default();
        if !self.transfer_active || !self.internal_clock() {
            return out;
        }
        self.cycle_accum = self.cycle_accum.saturating_add(cycles as u16);
        while self.cycle_accum >= SERIAL_INTERNAL_BIT_CYCLES && self.bits_remaining > 0 {
            self.cycle_accum -= SERIAL_INTERNAL_BIT_CYCLES;
            self.sb = (self.sb << 1) | 1;
            self.bits_remaining -= 1;
            if self.bits_remaining == 0 {
                self.transfer_active = false;
                self.sc &= !0x80;
                out.interrupt_requested = true;
                out.trace.push(SerialTraceEvent::TransferComplete {
                    sb: self.sb,
                    sc: self.read_sc(),
                    internal_clock: true,
                });
            }
        }
        out
    }

    // Apply the same fixed-rate disconnected shift without trace allocation,
    // returning whether the byte completed. Cycle input is narrowed to u16
    // and the accumulated remainder saturates rather than wrapping.
    pub fn tick_fast(&mut self, cycles: u32) -> bool {
        if !self.transfer_active || !self.internal_clock() {
            return false;
        }
        self.cycle_accum = self.cycle_accum.saturating_add(cycles as u16);
        while self.cycle_accum >= SERIAL_INTERNAL_BIT_CYCLES && self.bits_remaining > 0 {
            self.cycle_accum -= SERIAL_INTERNAL_BIT_CYCLES;
            self.sb = (self.sb << 1) | 1;
            self.bits_remaining -= 1;
            if self.bits_remaining == 0 {
                self.transfer_active = false;
                self.sc &= !0x80;
                return true;
            }
        }
        false
    }

    // Return the internal transfer-active latch.
    pub fn transfer_active(&self) -> bool {
        self.transfer_active
    }

    // Report whether SC bit zero selects the internal clock source.
    pub fn internal_clock(&self) -> bool {
        self.sc & 0x01 != 0
    }

    /// Clocks one complete byte from an external serial device.
    ///
    /// The transfer only completes when the Game Boy has armed an external-clock
    /// transfer (`SC.7 = 1`, `SC.0 = 0`). The returned `outgoing` byte is the value
    /// that was present in SB before the external device supplied `incoming`.
    // Complete an armed external-clock transfer as a whole-byte operation:
    // return the previous SB value, install incoming data, clear progress and
    // report completion/IRQ without modeling individual external clock edges.
    pub fn clock_external_byte(&mut self, incoming: u8) -> SerialExternalClockResult {
        let mut out = SerialExternalClockResult::default();
        if !self.transfer_active || self.internal_clock() {
            return out;
        }

        out.outgoing = self.sb;
        self.sb = incoming;
        self.sc &= !0x80;
        self.cycle_accum = 0;
        self.bits_remaining = 0;
        self.transfer_active = false;

        out.completed = true;
        out.interrupt_requested = true;
        out.trace.push(SerialTraceEvent::TransferComplete {
            sb: self.sb,
            sc: self.read_sc(),
            internal_clock: false,
        });
        out
    }

    // When both endpoints are armed with opposite clock sources, swap their
    // current SB bytes immediately and complete both transfers with IRQ/events.
    // This helper does not wait for eight timed link-clock edges.
    pub fn exchange_with_peer(&mut self, peer: &mut Serial) -> SerialLinkExchangeResult {
        let mut out = SerialLinkExchangeResult::default();
        if !self.transfer_active || !peer.transfer_active {
            return out;
        }
        if self.internal_clock() == peer.internal_clock() {
            return out;
        }

        let self_out = self.sb;
        let peer_out = peer.sb;

        self.sb = peer_out;
        self.sc &= !0x80;
        self.cycle_accum = 0;
        self.bits_remaining = 0;
        self.transfer_active = false;

        peer.sb = self_out;
        peer.sc &= !0x80;
        peer.cycle_accum = 0;
        peer.bits_remaining = 0;
        peer.transfer_active = false;

        out.completed = true;
        out.self_interrupt_requested = true;
        out.peer_interrupt_requested = true;
        out.self_trace.push(SerialTraceEvent::TransferComplete {
            sb: self.sb,
            sc: self.read_sc(),
            internal_clock: self.internal_clock(),
        });
        out.peer_trace.push(SerialTraceEvent::TransferComplete {
            sb: peer.sb,
            sc: peer.read_sc(),
            internal_clock: peer.internal_clock(),
        });
        out
    }
}

impl Default for Serial {
    // Initialize zero data/control and no active transfer or accumulated cycles.
    fn default() -> Self {
        Self {
            sb: 0,
            sc: 0,
            cycle_accum: 0,
            bits_remaining: 0,
            transfer_active: false,
        }
    }
}
