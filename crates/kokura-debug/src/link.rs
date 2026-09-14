use std::{cell::RefCell, error::Error, fmt};

use kokura_core::{error::CoreError, Machine};
use serde::Serialize;

use crate::session::DebugSession;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// The selected-peer adapter routes one host/peer pair at a time; Dmg07 uses its
// separate modeled discovery/broadcast protocol with runner index matching port index.
pub enum LinkTopology {
    Pair,
    FourPlayerAdapter {
        host_session: usize,
        active_peer: usize,
    },
    /// Physical Nintendo DMG-07 protocol. Runner session index maps directly to
    /// adapter port/player ID (index 0 is Player 1).
    Dmg07,
}

impl LinkTopology {
    // Return the stable topology label used in serialized run summaries.
    fn label(self) -> &'static str {
        match self {
            Self::Pair => "pair",
            Self::FourPlayerAdapter { .. } => "four_player_adapter",
            Self::Dmg07 => "dmg07",
        }
    }

    // Require two sessions for a cable pair or two through four for either adapter.
    // The selected-peer topology also requires distinct in-range host and peer indices.
    fn validate_session_count(self, session_count: usize) -> Result<(), LinkRunnerError> {
        match self {
            Self::Pair => {
                if session_count == 2 {
                    Ok(())
                } else {
                    Err(LinkRunnerError::InvalidTopology(format!(
                        "pair topology requires exactly 2 sessions, got {}",
                        session_count
                    )))
                }
            }
            Self::FourPlayerAdapter {
                host_session,
                active_peer,
            } => {
                if !(2..=4).contains(&session_count) {
                    return Err(LinkRunnerError::InvalidTopology(format!(
                        "four_player_adapter topology currently expects 2-4 sessions, got {}",
                        session_count
                    )));
                }
                if host_session >= session_count {
                    return Err(LinkRunnerError::InvalidTopology(format!(
                        "four_player_adapter host_session {} is out of range for {} sessions",
                        host_session, session_count
                    )));
                }
                if active_peer >= session_count {
                    return Err(LinkRunnerError::InvalidTopology(format!(
                        "four_player_adapter active_peer {} is out of range for {} sessions",
                        active_peer, session_count
                    )));
                }
                if host_session == active_peer {
                    return Err(LinkRunnerError::InvalidTopology(
                        "four_player_adapter host_session and active_peer must differ".to_string(),
                    ));
                }
                Ok(())
            }
            Self::Dmg07 => {
                if (2..=4).contains(&session_count) {
                    Ok(())
                } else {
                    Err(LinkRunnerError::InvalidTopology(format!(
                        "dmg07 topology requires 2-4 sessions, got {}",
                        session_count
                    )))
                }
            }
        }
    }

    // Validate indices/count first; DMG-07 broadcasts to ports and therefore has no active pair.
    fn active_pair(self, session_count: usize) -> Result<Option<(usize, usize)>, LinkRunnerError> {
        self.validate_session_count(session_count)?;
        Ok(match self {
            Self::Pair => Some((0, 1)),
            Self::FourPlayerAdapter {
                host_session,
                active_peer,
            } => Some((host_session, active_peer)),
            Self::Dmg07 => None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
// Counts describe this call's observed progress. A returned summary may be partial
// after a debug stop and is not proof that every requested frame completed.
pub struct LinkRunSummary {
    pub topology: String,
    pub session_count: usize,
    pub total_steps: u64,
    pub exchange_count: u64,
    pub active_pair: Option<(usize, usize)>,
    pub peer_switch_count: u64,
    pub dynamic_peer_selection_observed: bool,
    pub session_steps: Vec<u64>,
    pub frame_advances: Vec<u64>,
    pub session_exchange_counts: Vec<u64>,
    pub stopped_session: Option<usize>,
    pub halted_on_unsupported_opcode: bool,
}

impl LinkRunSummary {
    // Validate the topology and allocate one zeroed counter per supplied session.
    fn new(topology: LinkTopology, session_count: usize) -> Result<Self, LinkRunnerError> {
        Ok(Self {
            topology: topology.label().to_string(),
            session_count,
            total_steps: 0,
            exchange_count: 0,
            active_pair: topology.active_pair(session_count)?,
            peer_switch_count: 0,
            dynamic_peer_selection_observed: false,
            session_steps: vec![0; session_count],
            frame_advances: vec![0; session_count],
            session_exchange_counts: vec![0; session_count],
            stopped_session: None,
            halted_on_unsupported_opcode: false,
        })
    }

    // Saturating-add compatible run totals, retain the first stop and latest available pair.
    // The caller must combine matching session layouts; extra incoming vector entries are ignored.
    pub fn absorb(&mut self, other: Self) {
        self.total_steps = self.total_steps.saturating_add(other.total_steps);
        self.exchange_count = self.exchange_count.saturating_add(other.exchange_count);
        self.peer_switch_count = self
            .peer_switch_count
            .saturating_add(other.peer_switch_count);
        self.dynamic_peer_selection_observed |= other.dynamic_peer_selection_observed;
        self.active_pair = other.active_pair.or(self.active_pair);
        self.halted_on_unsupported_opcode |= other.halted_on_unsupported_opcode;
        if self.stopped_session.is_none() {
            self.stopped_session = other.stopped_session;
        }
        for (index, value) in other.session_steps.into_iter().enumerate() {
            if let Some(existing) = self.session_steps.get_mut(index) {
                *existing = existing.saturating_add(value);
            }
        }
        for (index, value) in other.frame_advances.into_iter().enumerate() {
            if let Some(existing) = self.frame_advances.get_mut(index) {
                *existing = existing.saturating_add(value);
            }
        }
        for (index, value) in other.session_exchange_counts.into_iter().enumerate() {
            if let Some(existing) = self.session_exchange_counts.get_mut(index) {
                *existing = existing.saturating_add(value);
            }
        }
    }
}

#[derive(Debug)]
pub enum LinkRunnerError {
    Core(CoreError),
    InvalidTopology(String),
}

impl fmt::Display for LinkRunnerError {
    // Display the original core error or the topology-specific explanation without losing its message.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(err) => write!(f, "{err}"),
            Self::InvalidTopology(message) => write!(f, "{message}"),
        }
    }
}

impl Error for LinkRunnerError {
    // Preserve a core failure as an error-chain source; topology failures have no nested error.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core(err) => Some(err),
            Self::InvalidTopology(_) => None,
        }
    }
}

impl From<CoreError> for LinkRunnerError {
    // Wrap machine execution failures so the link runner can propagate them with the question-mark operator.
    fn from(value: CoreError) -> Self {
        Self::Core(value)
    }
}

const DMG07_PORT_COUNT: usize = 4;
const DMG07_PING_HEADER: u8 = 0xFE;
const DMG07_ACK: u8 = 0x88;
const DMG07_START: u8 = 0xAA;
const DMG07_CONFIRM: u8 = 0xCC;
const DMG07_CPU_HZ: u64 = 4_194_304;
const DMG07_PING_BYTE_INTERVAL_US: u64 = 1_548;
const DMG07_PING_PACKET_DELAY_US: u64 = 12_328;
const DMG07_TX_BASE_BYTE_INTERVAL_US: u64 = 1_015;
const DMG07_TX_RATE_NIBBLE_INTERVAL_US: u64 = 106;
const DMG07_TX_MIN_PACKET_US: u64 = 17_000;
const DMG07_TX_PACKET_RATE_US: u64 = 1_000;
const DMG07_TX_PACKET_ADDON_US: u64 = 360;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dmg07Phase {
    Ping { byte_index: usize },
    Confirm { byte_index: usize },
    Transmission { byte_index: usize },
    Restart { byte_index: usize },
}

#[derive(Debug, Clone)]
// Mutable protocol state survives runner calls. SIZE is bounded to 1..4 bytes per port;
// this in-process model does not simulate electrical cable behavior.
struct Dmg07Adapter {
    session_count: Option<usize>,
    phase: Dmg07Phase,
    connected_mask: u8,
    ping_replies: [[u8; 4]; DMG07_PORT_COUNT],
    rate: u8,
    ping_rate: u8,
    packet_size: u8,
    capture_packets: [Vec<u8>; DMG07_PORT_COUNT],
    broadcast_packet: Vec<u8>,
    restart_ff_runs: [u8; DMG07_PORT_COUNT],
    restart_pending: bool,
    next_transfer_cycle: Option<u64>,
}

impl Default for Dmg07Adapter {
    // Start unbound at the first ping byte, with no connected ports or scheduled transfer deadline.
    fn default() -> Self {
        Self {
            session_count: None,
            phase: Dmg07Phase::Ping { byte_index: 0 },
            connected_mask: 0,
            ping_replies: [[0; 4]; DMG07_PORT_COUNT],
            // Ping itself is fixed-speed. These defaults are only used if a
            // malformed Player 1 reply starts transmission without parameters.
            rate: 0xFF,
            ping_rate: 0,
            packet_size: 1,
            capture_packets: std::array::from_fn(|_| vec![0]),
            broadcast_packet: vec![0; DMG07_PORT_COUNT],
            restart_ff_runs: [0; DMG07_PORT_COUNT],
            restart_pending: false,
            next_transfer_cycle: None,
        }
    }
}

impl Dmg07Adapter {
    // Bind this persistent adapter to the first participant count and reject later count changes.
    // Topology validation by the runner supplies the separate two-through-four range check.
    fn bind_session_count(&mut self, session_count: usize) -> Result<(), LinkRunnerError> {
        if let Some(bound) = self.session_count {
            if bound != session_count {
                return Err(LinkRunnerError::InvalidTopology(format!(
                    "dmg07 runner is already bound to {} sessions, got {}",
                    bound, session_count
                )));
            }
        } else {
            self.session_count = Some(session_count);
        }
        Ok(())
    }

    // Apply one parallel byte exchange to the current protocol phase. Timing is deliberately
    // separate: the runner checks transfer_due and schedules the next deadline around this call.
    fn transfer(&mut self, outgoing: &[u8]) -> Result<Vec<u8>, LinkRunnerError> {
        self.bind_session_count(outgoing.len())?;
        match self.phase {
            Dmg07Phase::Ping { byte_index } => Ok(self.transfer_ping(outgoing, byte_index)),
            Dmg07Phase::Confirm { byte_index } => Ok(self.transfer_confirm(outgoing, byte_index)),
            Dmg07Phase::Transmission { byte_index } => {
                Ok(self.transfer_payload(outgoing, byte_index))
            }
            Dmg07Phase::Restart { byte_index } => {
                Ok(self.transfer_restart(outgoing.len(), byte_index))
            }
        }
    }

    // Convert modeled microsecond delays to base-clock cycles, rounding up with saturating arithmetic.
    fn cycles_for_micros(micros: u64) -> u64 {
        DMG07_CPU_HZ.saturating_mul(micros).saturating_add(999_999) / 1_000_000
    }

    // Permit the first transfer immediately; subsequent transfers wait for the recorded absolute cycle.
    fn transfer_due(&self, cycle: u64) -> bool {
        self.next_transfer_cycle.is_none_or(|next| cycle >= next)
    }

    // Use the rate high nibble to extend the modeled per-byte transmission interval.
    fn transmission_byte_interval_cycles(&self) -> u64 {
        let micros = DMG07_TX_BASE_BYTE_INTERVAL_US
            + u64::from(self.rate >> 4) * DMG07_TX_RATE_NIBBLE_INTERVAL_US;
        Self::cycles_for_micros(micros)
    }

    // Choose a packet period covering both the rate-dependent minimum and serialized
    // bytes plus overhead, then subtract elapsed byte intervals to obtain the final gap.
    fn transmission_packet_tail_cycles(&self, byte_count: usize) -> u64 {
        let interval = self.transmission_byte_interval_cycles();
        let minimum_period = Self::cycles_for_micros(
            DMG07_TX_MIN_PACKET_US + u64::from(self.rate & 0x0F) * DMG07_TX_PACKET_RATE_US,
        );
        let serialized_period = interval
            .saturating_mul(byte_count as u64)
            .saturating_add(Self::cycles_for_micros(DMG07_TX_PACKET_ADDON_US));
        let period = minimum_period.max(serialized_period);
        let elapsed = interval.saturating_mul(byte_count.saturating_sub(1) as u64);
        period.saturating_sub(elapsed).max(interval)
    }

    // Schedule from the phase and byte index that just completed, even when transfer
    // already changed the current phase. Deadline time uses the runner's shared cycle observation.
    fn schedule_after_transfer(&mut self, cycle: u64, completed_phase: Dmg07Phase) {
        let delay = match completed_phase {
            Dmg07Phase::Ping { byte_index } => {
                if byte_index < 3 {
                    Self::cycles_for_micros(DMG07_PING_BYTE_INTERVAL_US)
                } else {
                    Self::cycles_for_micros(
                        DMG07_PING_PACKET_DELAY_US
                            + u64::from(self.ping_rate & 0x0F) * DMG07_TX_PACKET_RATE_US,
                    )
                }
            }
            Dmg07Phase::Confirm { .. } => self.transmission_byte_interval_cycles(),
            Dmg07Phase::Transmission { byte_index } => {
                if byte_index + 1 < self.broadcast_packet.len() {
                    self.transmission_byte_interval_cycles()
                } else {
                    self.transmission_packet_tail_cycles(self.broadcast_packet.len())
                }
            }
            Dmg07Phase::Restart { byte_index } => {
                let byte_count = usize::from(self.packet_size) * DMG07_PORT_COUNT;
                if byte_index + 1 < byte_count {
                    self.transmission_byte_interval_cycles()
                } else {
                    Self::cycles_for_micros(DMG07_PING_PACKET_DELAY_US)
                }
            }
        };
        self.next_transfer_cycle = Some(cycle.saturating_add(delay));
    }

    // Return the current ping header/status, collect per-port replies and recognize ACK
    // or Player-1 start packets. Rate and bounded packet size are accepted from Player 1 only.
    fn transfer_ping(&mut self, outgoing: &[u8], byte_index: usize) -> Vec<u8> {
        // Compute the returned status before consuming this byte's reply; newly recognized
        // ACK prefixes therefore become visible in subsequent bytes.
        let incoming = (0..outgoing.len())
            .map(|slot| {
                if byte_index == 0 {
                    DMG07_PING_HEADER
                } else {
                    self.status_for_slot(slot)
                }
            })
            .collect::<Vec<_>>();

        for (slot, value) in outgoing.iter().copied().enumerate() {
            self.ping_replies[slot][byte_index] = value;
        }

        // A completed 88 88 prefix changes the status visible later in the
        // same ping. AA AA from Player 1 is held until the full start packet is
        // known, so beginning transmission does not spuriously disconnect it.
        if byte_index == 1 {
            for slot in 0..outgoing.len() {
                let prefix = &self.ping_replies[slot][..2];
                if prefix == [DMG07_ACK, DMG07_ACK] {
                    self.set_connected(slot, true);
                } else if !(slot == 0 && prefix == [DMG07_START, DMG07_START]) {
                    self.set_connected(slot, false);
                }
            }
        }

        if byte_index < 3 {
            self.phase = Dmg07Phase::Ping {
                byte_index: byte_index + 1,
            };
            return incoming;
        }

        let player_one_start = self.ping_replies[0] == [DMG07_START; 4];
        for slot in 0..outgoing.len() {
            let reply = self.ping_replies[slot];
            if reply[..2] == [DMG07_ACK, DMG07_ACK] {
                self.set_connected(slot, true);
            } else if !(slot == 0 && player_one_start) {
                self.set_connected(slot, false);
            }
        }

        if player_one_start {
            self.phase = Dmg07Phase::Confirm { byte_index: 0 };
        } else {
            let master_reply = self.ping_replies[0];
            if master_reply[..2] == [DMG07_ACK, DMG07_ACK] {
                // RATE=$00 means "keep the existing speed" in both phases.
                if master_reply[2] != 0 {
                    self.rate = master_reply[2];
                    self.ping_rate = master_reply[2];
                }
                // This in-process backplane intentionally bounds SIZE to 1-4:
                // zero cannot form a packet, and larger untrusted values would
                // make every emulated frame perform disproportionate buffering.
                self.packet_size = master_reply[3].clamp(1, 4);
            }
            self.phase = Dmg07Phase::Ping { byte_index: 0 };
        }
        self.ping_replies = [[0; 4]; DMG07_PORT_COUNT];
        incoming
    }

    // Broadcast four confirmation bytes, then initialize transmission buffers; outgoing bytes are ignored.
    fn transfer_confirm(&mut self, outgoing: &[u8], byte_index: usize) -> Vec<u8> {
        let incoming = vec![DMG07_CONFIRM; outgoing.len()];
        if byte_index == 3 {
            self.begin_transmission();
        } else {
            self.phase = Dmg07Phase::Confirm {
                byte_index: byte_index + 1,
            };
        }
        incoming
    }

    // Broadcast the previously captured packet while collecting each connected port
    // for the next packet. A restart request waits until the current full broadcast completes.
    fn transfer_payload(&mut self, outgoing: &[u8], byte_index: usize) -> Vec<u8> {
        let restart_requested = self.observe_restart_request(outgoing, byte_index);
        let incoming_byte = self.broadcast_packet[byte_index];
        let incoming = vec![incoming_byte; outgoing.len()];

        let packet_size = usize::from(self.packet_size);
        if byte_index < packet_size {
            for (slot, value) in outgoing.iter().copied().enumerate() {
                if self.is_connected(slot) {
                    self.capture_packets[slot][byte_index] = value;
                }
            }
        }

        if restart_requested {
            self.restart_pending = true;
        }

        if byte_index + 1 == self.broadcast_packet.len() {
            if self.restart_pending {
                // A physical adapter finishes the current SIZE*4 broadcast
                // packet before emitting the aligned all-FF indicator packet.
                self.restart_pending = false;
                self.phase = Dmg07Phase::Restart { byte_index: 0 };
            } else {
                // Concatenate all four port buffers in port order, including deterministic zero slots
                // for absent/unconnected ports; reset captures for the following packet.
                let mut next_broadcast = Vec::with_capacity(packet_size * DMG07_PORT_COUNT);
                for slot in 0..DMG07_PORT_COUNT {
                    next_broadcast.extend_from_slice(&self.capture_packets[slot]);
                    self.capture_packets[slot].fill(0);
                }
                self.broadcast_packet = next_broadcast;
                self.phase = Dmg07Phase::Transmission { byte_index: 0 };
            }
        } else {
            self.phase = Dmg07Phase::Transmission {
                byte_index: byte_index + 1,
            };
        }
        incoming
    }

    // Emit one aligned all-FF packet of SIZE times four bytes, then restart discovery.
    fn transfer_restart(&mut self, session_count: usize, byte_index: usize) -> Vec<u8> {
        let incoming = vec![0xFF; session_count];
        let restart_packet_len = usize::from(self.packet_size) * DMG07_PORT_COUNT;
        if byte_index + 1 == restart_packet_len {
            self.restart_ping();
        } else {
            self.phase = Dmg07Phase::Restart {
                byte_index: byte_index + 1,
            };
        }
        incoming
    }

    // Size all port buffers and start with deterministic zero warm-up data, retaining
    // the connected-port mask and negotiated rate.
    fn begin_transmission(&mut self) {
        let packet_size = usize::from(self.packet_size);
        self.capture_packets = std::array::from_fn(|_| vec![0; packet_size]);
        // Hardware exposes undefined warm-up data derived from the handshake.
        // KOKURA makes that deliberately ignorable packet deterministic while
        // preserving the physical one-full-packet latency.
        self.broadcast_packet = vec![0; packet_size * DMG07_PORT_COUNT];
        self.restart_ff_runs = [0; DMG07_PORT_COUNT];
        self.restart_pending = false;
        self.phase = Dmg07Phase::Transmission { byte_index: 0 };
    }

    // Clear discovery/restart state and return to ping while retaining the bound count,
    // rate and packet size. The caller schedules the next deadline separately.
    fn restart_ping(&mut self) {
        self.phase = Dmg07Phase::Ping { byte_index: 0 };
        self.connected_mask = 0;
        self.ping_replies = [[0; 4]; DMG07_PORT_COUNT];
        self.restart_ff_runs = [0; DMG07_PORT_COUNT];
        self.restart_pending = false;
    }

    // Recognize three consecutive FF bytes beginning at the broadcast packet boundary
    // from any connected port. A run beginning later in the packet does not qualify.
    fn observe_restart_request(&mut self, outgoing: &[u8], byte_index: usize) -> bool {
        for (slot, value) in outgoing.iter().copied().enumerate() {
            if byte_index == 0 {
                self.restart_ff_runs[slot] = u8::from(self.is_connected(slot) && value == 0xFF);
            } else if self.restart_ff_runs[slot] != 0 && self.is_connected(slot) && value == 0xFF {
                self.restart_ff_runs[slot] = self.restart_ff_runs[slot].saturating_add(1);
            } else {
                self.restart_ff_runs[slot] = 0;
            }
        }
        self.restart_ff_runs.iter().any(|run| *run >= 3)
    }

    // Combine the upper-nibble connection mask with the one-based player ID in the low nibble.
    fn status_for_slot(&self, slot: usize) -> u8 {
        self.connected_mask | ((slot + 1) as u8)
    }

    // Test the connection bit for a validated zero-based port slot.
    fn is_connected(&self, slot: usize) -> bool {
        self.connected_mask & (0x10_u8 << slot) != 0
    }

    // Change only the selected port's connection bit, preserving the other three ports.
    fn set_connected(&mut self, slot: usize, connected: bool) {
        let bit = 0x10_u8 << slot;
        if connected {
            self.connected_mask |= bit;
        } else {
            self.connected_mask &= !bit;
        }
    }
}

#[derive(Debug, Clone)]
// A runner owns persistent DMG-07 state through RefCell. Use a separate runner for
// an independent group; simultaneous reentrant adapter access is not supported.
pub struct TimingAwareLinkRunner {
    topology: LinkTopology,
    dmg07: RefCell<Dmg07Adapter>,
}

impl TimingAwareLinkRunner {
    // Store the topology and fresh interior-mutable adapter state; session-count validation waits until a run.
    pub fn new(topology: LinkTopology) -> Self {
        Self {
            topology,
            dmg07: RefCell::new(Dmg07Adapter::default()),
        }
    }

    // Return the configured topology without resetting discovery or transmission state.
    pub fn topology(&self) -> LinkTopology {
        self.topology
    }

    /// Runs two ordinary `Machine` instances through a normal Game Boy link
    /// cable without collecting debugger events. One side must arm the
    /// internal clock and the other the external clock before a byte is
    /// exchanged.
    // Attach two machines and advance the eligible one with the lowest cycle count,
    // using index to break ties. The attachment remains after return; errors retain partial execution.
    pub fn run_pair_machine_frames(
        &self,
        machines: &mut [&mut Machine],
        frames: u64,
    ) -> Result<LinkRunSummary, LinkRunnerError> {
        if self.topology != LinkTopology::Pair {
            return Err(LinkRunnerError::InvalidTopology(
                "run_pair_machine_frames requires the pair topology".to_string(),
            ));
        }
        self.topology.validate_session_count(machines.len())?;
        for machine in machines.iter_mut() {
            machine.set_serial_link_attached(true);
        }

        let mut summary = LinkRunSummary::new(self.topology, machines.len())?;
        summary.active_pair = Some((0, 1));
        if frames == 0 {
            return Ok(summary);
        }
        let targets = machines
            .iter()
            .map(|machine| machine.clocks.frames.saturating_add(frames))
            .collect::<Vec<_>>();

        loop {
            let Some(next_index) = (0..machines.len())
                .filter(|&index| machines[index].clocks.frames < targets[index])
                .min_by_key(|&index| (machines[index].clocks.cycles, index))
            else {
                break;
            };
            let frame_before = machines[next_index].clocks.frames;
            machines[next_index].step_instruction_fast()?;
            summary.total_steps = summary.total_steps.saturating_add(1);
            summary.session_steps[next_index] = summary.session_steps[next_index].saturating_add(1);
            if machines[next_index].clocks.frames != frame_before {
                summary.frame_advances[next_index] =
                    summary.frame_advances[next_index].saturating_add(1);
            }

            if machines[0].serial.transfer_active()
                && machines[1].serial.transfer_active()
                && machines[0].serial.internal_clock() != machines[1].serial.internal_clock()
            {
                let (left, right) = machines.split_at_mut(1);
                if left[0].exchange_serial_with_peer(right[0]).completed {
                    summary.exchange_count = summary.exchange_count.saturating_add(1);
                    summary.session_exchange_counts[0] =
                        summary.session_exchange_counts[0].saturating_add(1);
                    summary.session_exchange_counts[1] =
                        summary.session_exchange_counts[1].saturating_add(1);
                }
            }
        }
        Ok(summary)
    }

    /// Runs ordinary `Machine` instances against the DMG-07 backplane without
    /// collecting the debugger's per-instruction event stream.  Realtime
    /// frontends use this path so a four-player session can run indefinitely
    /// without accumulating debug events.
    // Advance each machine toward its relative frame target while retaining the adapter
    // across calls. All configured ports must arm external-clock transfers before any adapter byte is delivered.
    pub fn run_dmg07_machine_frames(
        &self,
        machines: &mut [&mut Machine],
        frames: u64,
    ) -> Result<LinkRunSummary, LinkRunnerError> {
        if self.topology != LinkTopology::Dmg07 {
            return Err(LinkRunnerError::InvalidTopology(
                "run_dmg07_machine_frames requires the dmg07 topology".to_string(),
            ));
        }
        self.topology.validate_session_count(machines.len())?;
        self.prepare_backplane(machines.len())?;
        for machine in machines.iter_mut() {
            machine.set_serial_link_attached(true);
        }

        let mut summary = LinkRunSummary::new(self.topology, machines.len())?;
        if frames == 0 {
            return Ok(summary);
        }
        let targets = machines
            .iter()
            .map(|machine| machine.clocks.frames.saturating_add(frames))
            .collect::<Vec<_>>();

        loop {
            let Some(next_index) = (0..machines.len())
                .filter(|&index| machines[index].clocks.frames < targets[index])
                .min_by_key(|&index| (machines[index].clocks.cycles, index))
            else {
                break;
            };
            let frame_before = machines[next_index].clocks.frames;
            machines[next_index].step_instruction_fast()?;
            summary.total_steps = summary.total_steps.saturating_add(1);
            summary.session_steps[next_index] = summary.session_steps[next_index].saturating_add(1);
            if machines[next_index].clocks.frames != frame_before {
                summary.frame_advances[next_index] =
                    summary.frame_advances[next_index].saturating_add(1);
            }

            if machines
                .iter()
                .all(|machine| machine.serial.transfer_active() && !machine.serial.internal_clock())
            {
                let cycle = machines
                    .iter()
                    .map(|machine| machine.clocks.cycles)
                    .min()
                    .unwrap_or(0);
                if !self.dmg07.borrow().transfer_due(cycle) {
                    continue;
                }
                let outgoing = machines
                    .iter()
                    .map(|machine| machine.serial.sb)
                    .collect::<Vec<_>>();
                let mut adapter = self.dmg07.borrow_mut();
                let completed_phase = adapter.phase;
                let incoming = adapter.transfer(&outgoing)?;
                adapter.schedule_after_transfer(cycle, completed_phase);
                drop(adapter);
                for (slot, (machine, value)) in
                    machines.iter_mut().zip(incoming.into_iter()).enumerate()
                {
                    let completed = machine.clock_external_serial_byte(value);
                    debug_assert_eq!(
                        completed.completed.then_some(completed.outgoing),
                        Some(outgoing[slot])
                    );
                }
                summary.exchange_count = summary.exchange_count.saturating_add(1);
                for count in &mut summary.session_exchange_counts {
                    *count = count.saturating_add(1);
                }
            }
        }
        Ok(summary)
    }

    // Run a total debug-step budget shared across sessions in cycle order. Exchange after
    // each step, then stop the whole pump when that step reports a breakpoint or unsupported opcode.
    pub fn pump_instructions(
        &self,
        sessions: &mut [&mut DebugSession],
        max_total_steps: u64,
    ) -> Result<LinkRunSummary, LinkRunnerError> {
        self.topology.validate_session_count(sessions.len())?;
        Self::attach_backplane(sessions);
        self.prepare_backplane(sessions.len())?;
        let mut summary = LinkRunSummary::new(self.topology, sessions.len())?;
        if max_total_steps == 0 {
            return Ok(summary);
        }

        while summary.total_steps < max_total_steps {
            self.refresh_active_pair(sessions, &mut summary)?;
            let Some(next_index) = Self::next_session_index(sessions, None) else {
                break;
            };
            let outcome = sessions[next_index].run_debug_step()?;
            summary.total_steps += 1;
            summary.session_steps[next_index] += 1;
            if outcome.frame_completed {
                summary.frame_advances[next_index] += 1;
            }
            self.try_exchange(sessions, &mut summary)?;
            if outcome.stop_triggered || outcome.halted_on_unsupported_opcode {
                summary.stopped_session = Some(next_index);
                summary.halted_on_unsupported_opcode = outcome.halted_on_unsupported_opcode;
                break;
            }
        }

        Ok(summary)
    }

    // Give each session a relative frame target and schedule eligible debug steps by cycle count.
    // A newly triggered debug stop ends this call before all targets necessarily complete.
    pub fn run_frames(
        &self,
        sessions: &mut [&mut DebugSession],
        frames: u64,
    ) -> Result<LinkRunSummary, LinkRunnerError> {
        self.topology.validate_session_count(sessions.len())?;
        Self::attach_backplane(sessions);
        self.prepare_backplane(sessions.len())?;
        let mut summary = LinkRunSummary::new(self.topology, sessions.len())?;
        if frames == 0 {
            return Ok(summary);
        }

        let targets: Vec<u64> = sessions
            .iter()
            .map(|session| session.machine.clocks.frames.saturating_add(frames))
            .collect();

        loop {
            self.refresh_active_pair(sessions, &mut summary)?;
            let Some(next_index) = Self::next_session_index(sessions, Some(&targets)) else {
                break;
            };
            let outcome = sessions[next_index].run_debug_step()?;
            summary.total_steps += 1;
            summary.session_steps[next_index] += 1;
            if outcome.frame_completed {
                summary.frame_advances[next_index] += 1;
            }
            self.try_exchange(sessions, &mut summary)?;
            if outcome.stop_triggered || outcome.halted_on_unsupported_opcode {
                summary.stopped_session = Some(next_index);
                summary.halted_on_unsupported_opcode = outcome.halted_on_unsupported_opcode;
                break;
            }
        }

        Ok(summary)
    }

    // Refresh symbol-driven peer selection and record observed pair changes after execution has begun.
    fn refresh_active_pair(
        &self,
        sessions: &[&mut DebugSession],
        summary: &mut LinkRunSummary,
    ) -> Result<(), LinkRunnerError> {
        let (active_pair, dynamic_observed) = self.resolve_active_pair(sessions)?;
        if summary.active_pair != active_pair {
            if summary.total_steps > 0 && summary.active_pair.is_some() && active_pair.is_some() {
                summary.peer_switch_count = summary.peer_switch_count.saturating_add(1);
            }
            summary.active_pair = active_pair;
        }
        summary.dynamic_peer_selection_observed |= dynamic_observed;
        Ok(())
    }

    // Prefer the host's recognized link-library peer variable when available, otherwise
    // use the configured peer. The DMG-07 broadcast path does not use this selector.
    fn resolve_active_pair(
        &self,
        sessions: &[&mut DebugSession],
    ) -> Result<(Option<(usize, usize)>, bool), LinkRunnerError> {
        self.topology.validate_session_count(sessions.len())?;
        Ok(match self.topology {
            LinkTopology::Pair => (Some((0, 1)), false),
            LinkTopology::FourPlayerAdapter {
                host_session,
                active_peer,
            } => {
                let dynamic_peer =
                    sessions[host_session].link4_selected_peer_session(sessions.len());
                if let Some(peer) = dynamic_peer {
                    (Some((host_session, peer)), true)
                } else {
                    (Some((host_session, active_peer)), false)
                }
            }
            LinkTopology::Dmg07 => (None, false),
        })
    }

    // Keep DMG-07 port count consistent with previous calls without resetting its protocol phase.
    fn prepare_backplane(&self, session_count: usize) -> Result<(), LinkRunnerError> {
        if self.topology == LinkTopology::Dmg07 {
            self.dmg07.borrow_mut().bind_session_count(session_count)?;
        }
        Ok(())
    }

    // Dispatch to broadcast or selected-pair exchange and count only completed byte transactions.
    fn try_exchange(
        &self,
        sessions: &mut [&mut DebugSession],
        summary: &mut LinkRunSummary,
    ) -> Result<(), LinkRunnerError> {
        match self.topology {
            LinkTopology::Dmg07 => {
                if self.try_exchange_dmg07(sessions)? {
                    summary.exchange_count = summary.exchange_count.saturating_add(1);
                    for count in &mut summary.session_exchange_counts {
                        *count = count.saturating_add(1);
                    }
                }
            }
            LinkTopology::Pair | LinkTopology::FourPlayerAdapter { .. } => {
                if let Some((left, right)) = summary.active_pair {
                    if Self::try_exchange_pair(sessions, left, right) {
                        summary.exchange_count = summary.exchange_count.saturating_add(1);
                        summary.session_exchange_counts[left] =
                            summary.session_exchange_counts[left].saturating_add(1);
                        summary.session_exchange_counts[right] =
                            summary.session_exchange_counts[right].saturating_add(1);
                    }
                }
            }
        }
        Ok(())
    }

    // Require every session to be armed on external clock and the minimum machine cycle
    // to reach the adapter deadline, then complete one parallel byte through each debug session.
    fn try_exchange_dmg07(
        &self,
        sessions: &mut [&mut DebugSession],
    ) -> Result<bool, LinkRunnerError> {
        // The physical adapter owns the clock. Waiting for every configured
        // slot keeps the in-process machines on the same byte boundary; an
        // internal-clock request is intentionally never treated as DMG-07 I/O.
        if sessions.iter().any(|session| {
            !session.machine.serial.transfer_active() || session.machine.serial.internal_clock()
        }) {
            return Ok(false);
        }

        let cycle = sessions
            .iter()
            .map(|session| session.machine.clocks.cycles)
            .min()
            .unwrap_or(0);
        if !self.dmg07.borrow().transfer_due(cycle) {
            return Ok(false);
        }

        let outgoing = sessions
            .iter()
            .map(|session| session.machine.serial.sb)
            .collect::<Vec<_>>();
        let mut adapter = self.dmg07.borrow_mut();
        let completed_phase = adapter.phase;
        let incoming = adapter.transfer(&outgoing)?;
        adapter.schedule_after_transfer(cycle, completed_phase);
        drop(adapter);
        for (slot, (session, value)) in sessions.iter_mut().zip(incoming).enumerate() {
            let completed = session.clock_external_serial_byte(value);
            debug_assert_eq!(completed, Some(outgoing[slot]));
        }
        Ok(true)
    }

    // Skip stopped or target-complete sessions and select the smallest (cycle, index) pair.
    // A supplied target slice must have an entry for every session.
    fn next_session_index(
        sessions: &[&mut DebugSession],
        frame_targets: Option<&[u64]>,
    ) -> Option<usize> {
        (0..sessions.len())
            .filter(|&index| {
                !sessions[index].is_execution_stopped()
                    && frame_targets.is_none_or(|targets| {
                        sessions[index].machine.clocks.frames < targets[index]
                    })
            })
            .min_by_key(|&index| (sessions[index].machine.clocks.cycles, index))
    }

    // Mark every machine as cable-attached; this helper does not restore prior attachment flags.
    fn attach_backplane(sessions: &mut [&mut DebugSession]) {
        for session in sessions {
            session.machine.set_serial_link_attached(true);
        }
    }

    // Borrow distinct sessions safely, require both transfers active with opposite clock
    // roles, then delegate byte completion to the debug-session exchange path.
    fn try_exchange_pair(sessions: &mut [&mut DebugSession], left: usize, right: usize) -> bool {
        if left == right || left >= sessions.len() || right >= sessions.len() {
            return false;
        }
        let (left_session, right_session) = if left < right {
            let (head, tail) = sessions.split_at_mut(right);
            (&mut *head[left], &mut *tail[0])
        } else {
            let (head, tail) = sessions.split_at_mut(left);
            (&mut *tail[0], &mut *head[right])
        };
        if !left_session.machine.serial.transfer_active()
            || !right_session.machine.serial.transfer_active()
        {
            return false;
        }
        if left_session.machine.serial.internal_clock()
            == right_session.machine.serial.internal_clock()
        {
            return false;
        }
        left_session.exchange_serial_with_peer(right_session)
    }
}

#[cfg(test)]
mod tests {
    use kokura_core::Machine;

    use crate::DebugEvent;

    use super::{
        Dmg07Adapter, Dmg07Phase, LinkTopology, TimingAwareLinkRunner, DMG07_CPU_HZ,
        DMG07_PING_BYTE_INTERVAL_US, DMG07_PING_PACKET_DELAY_US,
    };
    use crate::session::DebugSession;

    #[test]
    // Check one armed byte exchange, received bytes, interrupt counts and a debug completion event.
    fn pair_runner_completes_serial_exchange() {
        let mut left = DebugSession::new(Machine::new());
        let mut right = DebugSession::new(Machine::new());
        left.machine.write8(0xFF01, 0xA5);
        left.machine.write8(0xFF02, 0x81);
        right.machine.write8(0xFF01, 0x3C);
        right.machine.write8(0xFF02, 0x80);

        let runner = TimingAwareLinkRunner::new(LinkTopology::Pair);
        let mut sessions: Vec<&mut DebugSession> = vec![&mut left, &mut right];
        let summary = runner
            .pump_instructions(&mut sessions, 1)
            .expect("pump instructions");

        assert_eq!(summary.exchange_count, 1);
        assert_eq!(summary.active_pair, Some((0, 1)));
        assert_eq!(summary.session_exchange_counts, vec![1, 1]);
        assert!(left.machine.serial_link_attached());
        assert!(right.machine.serial_link_attached());
        assert_eq!(left.machine.serial.sb, 0x3C);
        assert_eq!(right.machine.serial.sb, 0xA5);
        assert_eq!(left.serial_interrupt_count, 1);
        assert_eq!(right.serial_interrupt_count, 1);
        assert!(left
            .event_log
            .iter()
            .any(|event| matches!(event, DebugEvent::SerialTransferComplete { sb: 0x3C, .. })));
    }

    #[test]
    // Use original synthetic ROM bytes to check the non-debug pair runner exchanges one armed byte.
    fn realtime_pair_runner_exchanges_machine_bytes() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0x00;
        let mut left = Machine::new();
        let mut right = Machine::new();
        left.load_rom(rom.clone()).expect("load left ROM");
        right.load_rom(rom).expect("load right ROM");
        left.write8(0xFF01, 0xA5);
        left.write8(0xFF02, 0x81);
        right.write8(0xFF01, 0x3C);
        right.write8(0xFF02, 0x80);

        let runner = TimingAwareLinkRunner::new(LinkTopology::Pair);
        let mut machines = vec![&mut left, &mut right];
        let summary = runner
            .run_pair_machine_frames(&mut machines, 1)
            .expect("run realtime pair");

        assert_eq!(summary.exchange_count, 1);
        assert_eq!(summary.session_exchange_counts, vec![1, 1]);
        assert_eq!(left.serial.sb, 0x3C);
        assert_eq!(right.serial.sb, 0xA5);
    }

    #[test]
    // Check that only the configured host/peer exchange while another armed peer stays pending.
    fn four_player_adapter_targets_selected_peer_only() {
        let mut host = DebugSession::new(Machine::new());
        let mut peer_a = DebugSession::new(Machine::new());
        let mut peer_b = DebugSession::new(Machine::new());
        let mut peer_c = DebugSession::new(Machine::new());

        host.machine.write8(0xFF01, 0x11);
        host.machine.write8(0xFF02, 0x81);
        peer_a.machine.write8(0xFF01, 0x22);
        peer_a.machine.write8(0xFF02, 0x80);
        peer_b.machine.write8(0xFF01, 0x33);
        peer_b.machine.write8(0xFF02, 0x80);
        peer_c.machine.write8(0xFF01, 0x44);
        peer_c.machine.write8(0xFF02, 0x00);

        let runner = TimingAwareLinkRunner::new(LinkTopology::FourPlayerAdapter {
            host_session: 0,
            active_peer: 2,
        });
        let mut sessions: Vec<&mut DebugSession> =
            vec![&mut host, &mut peer_a, &mut peer_b, &mut peer_c];
        let summary = runner
            .pump_instructions(&mut sessions, 1)
            .expect("pump instructions");

        assert_eq!(summary.exchange_count, 1);
        assert_eq!(summary.active_pair, Some((0, 2)));
        assert_eq!(summary.session_exchange_counts, vec![1, 0, 1, 0]);
        assert_eq!(host.machine.serial.sb, 0x33);
        assert_eq!(peer_b.machine.serial.sb, 0x11);
        assert_eq!(peer_a.machine.serial.sb, 0x22);
        assert!(peer_a.machine.serial.transfer_active());
        assert_eq!(host.serial_interrupt_count, 1);
        assert_eq!(peer_b.serial_interrupt_count, 1);
        assert_eq!(peer_a.serial_interrupt_count, 0);
    }

    #[test]
    // Supply link-library variable metadata and verify its peer selection overrides the configured default.
    fn four_player_adapter_follows_host_selected_peer_variable() {
        let mut host = DebugSession::new(Machine::new());
        let mut peer_a = DebugSession::new(Machine::new());
        let mut peer_b = DebugSession::new(Machine::new());

        host.set_symbol_table(kokura_bridge::SymbolTable {
            symbols: Vec::new(),
            source_locations: Vec::new(),
            functions: Vec::new(),
            variables: vec![
                kokura_bridge::VariableInfo {
                    name: "Link4_ModeState".to_string(),
                    address: 0xFF80,
                    size: 1,
                    region: "hram".to_string(),
                    bank: None,
                },
                kokura_bridge::VariableInfo {
                    name: "Link4_SelectedPeer".to_string(),
                    address: 0xFF81,
                    size: 1,
                    region: "hram".to_string(),
                    bank: None,
                },
                kokura_bridge::VariableInfo {
                    name: "Link4_SlotCount".to_string(),
                    address: 0xFF82,
                    size: 1,
                    region: "hram".to_string(),
                    bank: None,
                },
            ],
            static_estimates: Vec::new(),
            call_edges: Vec::new(),
        });
        host.machine.write8(0xFF80, 1);
        host.machine.write8(0xFF81, 2);
        host.machine.write8(0xFF82, 3);

        host.machine.write8(0xFF01, 0x55);
        host.machine.write8(0xFF02, 0x81);
        peer_a.machine.write8(0xFF01, 0x11);
        peer_a.machine.write8(0xFF02, 0x80);
        peer_b.machine.write8(0xFF01, 0x22);
        peer_b.machine.write8(0xFF02, 0x80);

        let runner = TimingAwareLinkRunner::new(LinkTopology::FourPlayerAdapter {
            host_session: 0,
            active_peer: 1,
        });
        let mut sessions: Vec<&mut DebugSession> = vec![&mut host, &mut peer_a, &mut peer_b];
        let summary = runner
            .pump_instructions(&mut sessions, 1)
            .expect("pump instructions");

        assert_eq!(summary.exchange_count, 1);
        assert_eq!(summary.active_pair, Some((0, 2)));
        assert_eq!(summary.session_exchange_counts, vec![1, 0, 1]);
        assert!(summary.dynamic_peer_selection_observed);
        assert_eq!(host.machine.serial.sb, 0x22);
        assert_eq!(peer_b.machine.serial.sb, 0x55);
        assert_eq!(peer_a.machine.serial.sb, 0x11);
    }

    #[test]
    // Check discovery replies, connected status, Player-1 parameters and the model's maximum packet-size clamp.
    fn dmg07_ping_parses_ack_rate_size_and_updates_status() {
        let mut adapter = Dmg07Adapter::default();

        assert_eq!(adapter.transfer(&[0x88, 0x88]).unwrap(), vec![0xFE, 0xFE]);
        assert_eq!(adapter.transfer(&[0x88, 0x88]).unwrap(), vec![0x01, 0x02]);
        // Both completed ACK prefixes are visible in STAT2 and STAT3.
        assert_eq!(adapter.transfer(&[0x10, 0x10]).unwrap(), vec![0x31, 0x32]);
        assert_eq!(adapter.transfer(&[0x02, 0x02]).unwrap(), vec![0x31, 0x32]);

        assert_eq!(adapter.connected_mask, 0x30);
        assert_eq!(adapter.rate, 0x10);
        assert_eq!(adapter.packet_size, 2);
        assert_eq!(adapter.phase, Dmg07Phase::Ping { byte_index: 0 });

        for outgoing in [[0x88, 0x88], [0x88, 0x88], [0x20, 0x20], [0xFF, 0xFF]] {
            adapter.transfer(&outgoing).unwrap();
        }
        assert_eq!(adapter.rate, 0x20);
        assert_eq!(adapter.packet_size, 4);
    }

    #[test]
    // Check modeled ping deadlines just before/at due time and the resulting approximate packet period.
    // These assertions compare model constants, not measurements from a physical adapter.
    fn dmg07_power_up_ping_uses_hardware_byte_and_packet_spacing() {
        let mut adapter = Dmg07Adapter::default();
        let mut cycle = 0u64;

        for byte_index in 0..4 {
            assert!(adapter.transfer_due(cycle));
            let phase = adapter.phase;
            adapter.transfer(&[0, 0, 0, 0]).unwrap();
            adapter.schedule_after_transfer(cycle, phase);
            let next = adapter.next_transfer_cycle.expect("scheduled transfer");
            assert!(!adapter.transfer_due(next - 1));
            assert!(adapter.transfer_due(next));
            let expected = if byte_index < 3 {
                Dmg07Adapter::cycles_for_micros(DMG07_PING_BYTE_INTERVAL_US)
            } else {
                Dmg07Adapter::cycles_for_micros(DMG07_PING_PACKET_DELAY_US)
            };
            assert_eq!(next - cycle, expected);
            cycle = next;
        }

        let packet_period_micros = cycle.saturating_mul(1_000_000) / DMG07_CPU_HZ;
        assert!((16_900..=17_100).contains(&packet_period_micros));
    }

    #[test]
    // Check that a zero rate reply retains the previous transmission and ping rate.
    fn dmg07_zero_rate_reply_preserves_existing_speed() {
        let mut adapter = Dmg07Adapter::default();
        adapter.rate = 0x28;
        adapter.ping_rate = 0x28;
        for outgoing in [[0x88; 2], [0x88; 2], [0x00; 2], [0x04; 2]] {
            adapter.transfer(&outgoing).unwrap();
        }
        assert_eq!(adapter.rate, 0x28);
        assert_eq!(adapter.ping_rate, 0x28);
        assert_eq!(adapter.packet_size, 4);
    }

    #[test]
    // Check aligned start, confirmation, deterministic warm-up and the next packet
    // containing captured port data with zeros for unused ports.
    fn dmg07_player_one_start_confirms_then_broadcasts_with_one_packet_delay() {
        let mut adapter = Dmg07Adapter::default();

        // Establish Players 1 and 2 with RATE=0x10 and SIZE=2.
        for outgoing in [[0x88, 0x88], [0x88, 0x88], [0x10, 0x10], [0x02, 0x02]] {
            adapter.transfer(&outgoing).unwrap();
        }

        // Player 1 replaces its next aligned ping reply with AA AA AA AA.
        for outgoing in [[0xAA, 0x88], [0xAA, 0x88], [0xAA, 0x10], [0xAA, 0x02]] {
            adapter.transfer(&outgoing).unwrap();
        }
        assert_eq!(adapter.phase, Dmg07Phase::Confirm { byte_index: 0 });

        for index in 0..4 {
            assert_eq!(adapter.transfer(&[0, 0]).unwrap(), vec![0xCC, 0xCC]);
            let expected = if index == 3 {
                Dmg07Phase::Transmission { byte_index: 0 }
            } else {
                Dmg07Phase::Confirm {
                    byte_index: index + 1,
                }
            };
            assert_eq!(adapter.phase, expected);
        }

        // SIZE=2 collects two bytes from each connected port, but the first
        // SIZE*4-byte broadcast is only the deterministic warm-up packet.
        let first_packet_outgoing = [
            [0x11, 0x21],
            [0x12, 0x22],
            [0x00, 0x00],
            [0x00, 0x00],
            [0x00, 0x00],
            [0x00, 0x00],
            [0x00, 0x00],
            [0x00, 0x00],
        ];
        for outgoing in first_packet_outgoing {
            assert_eq!(adapter.transfer(&outgoing).unwrap(), vec![0, 0]);
        }

        let expected_broadcast = [0x11, 0x12, 0x21, 0x22, 0x00, 0x00, 0x00, 0x00];
        for expected in expected_broadcast {
            assert_eq!(adapter.transfer(&[0, 0]).unwrap(), vec![expected, expected]);
        }
    }

    #[test]
    // Check that one internal-clock participant prevents adapter exchange until rearmed externally.
    fn dmg07_runner_only_clocks_armed_external_transfers() {
        let mut player_one = DebugSession::new(Machine::new());
        let mut player_two = DebugSession::new(Machine::new());
        player_one.machine.write8(0xFF01, 0x88);
        player_one.machine.write8(0xFF02, 0x80);
        player_two.machine.write8(0xFF01, 0x88);
        player_two.machine.write8(0xFF02, 0x81);

        let runner = TimingAwareLinkRunner::new(LinkTopology::Dmg07);
        let mut sessions: Vec<&mut DebugSession> = vec![&mut player_one, &mut player_two];
        let blocked = runner.pump_instructions(&mut sessions, 1).unwrap();
        assert_eq!(blocked.exchange_count, 0);
        assert!(player_one.machine.serial.transfer_active());
        assert!(player_two.machine.serial.transfer_active());

        player_two.machine.write8(0xFF02, 0x80);
        let mut sessions: Vec<&mut DebugSession> = vec![&mut player_one, &mut player_two];
        let completed = runner.pump_instructions(&mut sessions, 1).unwrap();
        assert_eq!(completed.topology, "dmg07");
        assert_eq!(completed.active_pair, None);
        assert_eq!(completed.exchange_count, 1);
        assert_eq!(completed.session_exchange_counts, vec![1, 1]);
        assert_eq!(player_one.machine.serial.sb, 0xFE);
        assert_eq!(player_two.machine.serial.sb, 0xFE);
        assert_eq!(player_one.serial_interrupt_count, 1);
        assert_eq!(player_two.serial_interrupt_count, 1);
    }

    #[test]
    // Check four machines each advance one frame and receive the initial ping header.
    fn dmg07_realtime_machine_runner_advances_four_players_and_exchanges() {
        let mut players = std::array::from_fn::<_, 4, _>(|_| Machine::new());
        for player in &mut players {
            player.write8(0xFF01, 0x88);
            player.write8(0xFF02, 0x80);
        }
        let runner = TimingAwareLinkRunner::new(LinkTopology::Dmg07);
        let mut machines = players.iter_mut().collect::<Vec<_>>();
        let summary = runner.run_dmg07_machine_frames(&mut machines, 1).unwrap();
        assert_eq!(summary.exchange_count, 1);
        assert_eq!(summary.session_exchange_counts, vec![1, 1, 1, 1]);
        assert_eq!(summary.frame_advances, vec![1, 1, 1, 1]);
        drop(machines);
        assert!(players.iter().all(|player| player.serial.sb == 0xFE));
        assert!(players.iter().all(|player| player.clocks.frames == 1));
    }

    #[test]
    // Reject unaligned FF runs, accept a connected peer's aligned request and complete
    // the old packet before emitting the restart indicator and returning to discovery.
    fn dmg07_connected_peer_can_request_ff_restart_indicator_then_ping() {
        let mut adapter = Dmg07Adapter::default();

        for outgoing in [[0x88, 0x88], [0x88, 0x88], [0x10, 0x10], [0x01, 0x01]] {
            adapter.transfer(&outgoing).unwrap();
        }
        for outgoing in [[0xAA, 0x88], [0xAA, 0x88], [0xAA, 0x10], [0xAA, 0x01]] {
            adapter.transfer(&outgoing).unwrap();
        }
        for _ in 0..4 {
            adapter.transfer(&[0, 0]).unwrap();
        }

        // Three FF bytes that begin after the packet boundary must not request
        // restart, even when they are consecutive.
        adapter.transfer(&[0x00, 0x00]).unwrap();
        adapter.transfer(&[0x00, 0xFF]).unwrap();
        adapter.transfer(&[0x00, 0xFF]).unwrap();
        adapter.transfer(&[0x00, 0xFF]).unwrap();
        assert_eq!(adapter.phase, Dmg07Phase::Transmission { byte_index: 0 });

        // Any connected player can request restart with three consecutive FF
        // bytes aligned to the packet start. Player 2 proves this is not
        // host-only.
        adapter.transfer(&[0x00, 0xFF]).unwrap();
        adapter.transfer(&[0x00, 0xFF]).unwrap();
        adapter.transfer(&[0x00, 0xFF]).unwrap();
        assert_eq!(adapter.phase, Dmg07Phase::Transmission { byte_index: 3 });
        // The fourth aligned byte completes the old packet. The all-FF
        // indicator starts on the following transfer, never mid-packet.
        assert_eq!(adapter.transfer(&[0x00, 0xFF]).unwrap(), vec![0, 0]);
        assert_eq!(adapter.phase, Dmg07Phase::Restart { byte_index: 0 });

        for _ in 0..4 {
            assert_eq!(adapter.transfer(&[0, 0]).unwrap(), vec![0xFF, 0xFF]);
        }
        assert_eq!(adapter.phase, Dmg07Phase::Ping { byte_index: 0 });
        assert_eq!(adapter.connected_mask, 0);
        assert_eq!(adapter.transfer(&[0x88, 0x88]).unwrap(), vec![0xFE, 0xFE]);
    }

    #[test]
    // Optionally load a caller-provided ROM and log 64 model exchanges within a step cap.
    // This fixture bypasses adapter scheduling and asserts the count, not particular initial reply bytes.
    fn dmg07_external_rom_fixture_reports_initial_ping_bytes() {
        // With no external fixture configured this test returns immediately; a passing harness
        // result then provides no external-ROM execution evidence.
        let Some(path) = std::env::var_os("KOKURA_DMG07_ROM") else {
            return;
        };
        let bytes = std::fs::read(&path).expect("read external DMG-07 ROM fixture");
        let mut machines = (0..4)
            .map(|_| {
                let mut machine = Machine::new();
                machine.load_rom(bytes.clone()).expect("load ROM fixture");
                machine.set_serial_link_attached(true);
                machine
            })
            .collect::<Vec<_>>();
        let mut adapter = Dmg07Adapter::default();
        let mut trace = Vec::new();
        let mut steps = 0u64;

        while trace.len() < 64 && steps < 5_000_000 {
            let next = (0..machines.len())
                .min_by_key(|&index| (machines[index].clocks.cycles, index))
                .expect("four machines");
            machines[next]
                .step_instruction_fast()
                .expect("step ROM fixture");
            steps += 1;
            if machines
                .iter()
                .all(|machine| machine.serial.transfer_active() && !machine.serial.internal_clock())
            {
                let outgoing = machines
                    .iter()
                    .map(|machine| machine.serial.sb)
                    .collect::<Vec<_>>();
                let phase = adapter.phase;
                let incoming = adapter.transfer(&outgoing).expect("adapter transfer");
                trace.push((phase, outgoing.clone(), incoming.clone()));
                for (machine, value) in machines.iter_mut().zip(incoming) {
                    assert!(machine.clock_external_serial_byte(value).completed);
                }
            }
        }

        for (index, (phase, outgoing, incoming)) in trace.iter().enumerate() {
            eprintln!("{index:02} {phase:?} out={outgoing:02X?} in={incoming:02X?}");
        }
        assert_eq!(trace.len(), 64, "did not observe 64 DMG-07 transfers");
    }

    #[test]
    // Check count rejection at one/five and acceptance at two/four sessions.
    fn dmg07_topology_rejects_non_physical_session_counts() {
        assert!(LinkTopology::Dmg07.validate_session_count(1).is_err());
        assert!(LinkTopology::Dmg07.validate_session_count(2).is_ok());
        assert!(LinkTopology::Dmg07.validate_session_count(4).is_ok());
        assert!(LinkTopology::Dmg07.validate_session_count(5).is_err());
    }
}
