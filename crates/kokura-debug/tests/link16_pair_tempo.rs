use std::{fs, path::Path};

use kokura_core::Machine;
use kokura_debug::{LinkTopology, TimingAwareLinkRunner};

// Read a whitespace-separated map, match the final column exactly and decode the
// first column as a 16-bit hexadecimal address. A missing symbol fails this fixture.
fn symbol_address(map_path: &Path, name: &str) -> u16 {
    let map = fs::read_to_string(map_path).expect("read LINK16 map");
    map.lines()
        .find_map(|line| {
            let columns = line.split_whitespace().collect::<Vec<_>>();
            if columns.last().copied() == Some(name) {
                u16::from_str_radix(columns[0], 16).ok()
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("symbol {name} not found in {}", map_path.display()))
}

#[test]
// Exercise cable transport cadence with two copies of the optional LINK16 ROM.
// Missing ROM/map files return early, so a passing harness alone does not prove this test ran its scenario.
fn link16_unified_cable_selects_roles_and_starts_transport() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../link16_sequencer/out");
    let cable_rom = root.join("link16_cable.gb");
    let cable_map = root.join("link16_cable.map");
    // The published workspace may omit these external fixture outputs; no ROM is downloaded or built here.
    if !cable_rom.exists() || !cable_map.exists() {
        return;
    }

    // Resolve current build symbols instead of hard-coding the sequencer's RAM layout.
    let applied_addr = symbol_address(&cable_map, "Link16_TransportAppliedCount");
    let playing_addr = symbol_address(&cable_map, "Link16_TransportPlaying");
    let step_frames_addr = symbol_address(&cable_map, "Link16_StepFrames");
    let late_addr = symbol_address(&cable_map, "Link16_TransportLateCount");
    let error_addr = symbol_address(&cable_map, "Link16_LinkErrorCount");
    let overrun_addr = symbol_address(&cable_map, "Link16_OverrunCount");
    let link_state_addr = symbol_address(&cable_map, "Link16_LinkState");
    let mode_addr = symbol_address(&cable_map, "Link16_Mode");
    let peer_pattern_ready_addr = symbol_address(&cable_map, "Link16_PeerPatternReady");
    let peer_tx_state_addr = symbol_address(&cable_map, "Link16_PeerTxState");
    let peer_rx_state_addr = symbol_address(&cable_map, "Link16_PeerRxState");

    let rom = fs::read(cable_rom).expect("read unified cable ROM");
    // Create independent machine states while sharing identical starting program bytes.
    let mut master = Machine::new();
    master.load_rom(rom.clone()).unwrap();
    let mut peer = Machine::new();
    peer.load_rom(rom).unwrap();
    let runner = TimingAwareLinkRunner::new(LinkTopology::Pair);

    let mut start_frame = None;
    let mut master_events = Vec::new();
    let mut peer_events = Vec::new();
    let mut last_master_applied = 0;
    let mut last_peer_applied = 0;
    let mut overrun_at_start = None;
    let mut start_sent = false;
    let mut start_press_frames = 0u8;
    let mut start_request_frame = None;

    for frame in 0..1200u64 {
        // Both machines boot the same ROM. Select the one internal-clock owner
        // and one external-clock peer through the ROM's role menu.
        let master_role = frame == 20;
        let peer_role = frame == 20;
        // Arm transport only once both links report readiness and the master's transfer state is idle.
        let ready = master.peek8(link_state_addr) == 3
            && peer.peek8(link_state_addr) == 3
            && master.peek8(peer_tx_state_addr) == 0
            && master.peek8(peer_rx_state_addr) == 0;
        let arm_start = ready && !start_sent;
        if arm_start {
            // The default patterns are byte-identical in this synthetic test,
            // so mark the initial full-pattern transfer complete and exercise
            // the real transport protocol without spending thousands of
            // frames copying data that is already identical.
            master.write8(peer_pattern_ready_addr, 1);
            peer.write8(peer_pattern_ready_addr, 1);
            start_sent = true;
            start_press_frames = 2;
            start_request_frame = Some(frame);
        }
        let start_pressed = start_press_frames != 0;
        master.set_joypad_mask(if master_role {
            0x10
        } else if start_pressed {
            0x80
        } else {
            0
        });
        peer.set_joypad_mask(if peer_role { 0x20 } else { 0 });
        runner
            .run_pair_machine_frames(&mut [&mut master, &mut peer], 1)
            .unwrap();
        if start_press_frames != 0 {
            start_press_frames -= 1;
        }

        // Capture the first observed playing frame and baseline overrun counters for later comparison.
        if start_frame.is_none() && master.peek8(playing_addr) != 0 {
            start_frame = Some(master.clocks.frames);
            overrun_at_start = Some((master.peek8(overrun_addr), peer.peek8(overrun_addr)));
        }
        // Observe transport counter changes once per emulated frame using side-effect-free reads.
        // Multiple updates within a frame are not individually timestamped by this test.
        let master_applied = master.peek8(applied_addr);
        if master_applied != last_master_applied {
            master_events.push(master.clocks.frames);
            last_master_applied = master_applied;
        }
        let peer_applied = peer.peek8(applied_addr);
        if peer_applied != last_peer_applied {
            peer_events.push(peer.clocks.frames);
            last_peer_applied = peer_applied;
        }
        // Bound the scenario by enough observations, start timeout or the outer 1200-frame budget.
        if master_events.len() >= 12 && peer_events.len() >= 12 {
            break;
        }
        if start_frame.is_none() && start_request_frame.is_some_and(|request| frame > request + 300)
        {
            break;
        }
    }

    println!(
        "final mode={}/{} link={}/{} ready={}/{} tx={}/{} rx={}/{} playing={}/{} applied={}/{} start_sent={start_sent} start_request={start_request_frame:?}",
        master.peek8(mode_addr),
        peer.peek8(mode_addr),
        master.peek8(link_state_addr),
        peer.peek8(link_state_addr),
        master.peek8(peer_pattern_ready_addr),
        peer.peek8(peer_pattern_ready_addr),
        master.peek8(peer_tx_state_addr),
        peer.peek8(peer_tx_state_addr),
        master.peek8(peer_rx_state_addr),
        peer.peek8(peer_rx_state_addr),
        master.peek8(playing_addr),
        peer.peek8(playing_addr),
        master.peek8(applied_addr),
        peer.peek8(applied_addr),
    );

    assert_eq!(
        master.peek8(step_frames_addr),
        peer.peek8(step_frames_addr),
        "the unified ROM selected different tempo state on each machine"
    );
    assert_eq!(master.peek8(mode_addr), 1, "P1 did not select MASTER");
    assert_eq!(peer.peek8(mode_addr), 2, "P2 did not select PEER");
    assert!(
        master_events.len() >= 12,
        "master did not produce enough events"
    );
    assert!(
        peer_events.len() >= 12,
        "peer did not produce enough events"
    );
    let start_frame = start_frame.expect("transport never armed");
    // Measure startup lead from the first observed playing frame, then compare adjacent event intervals.
    let initial_lead = master_events[0].saturating_sub(start_frame);
    let master_intervals = master_events
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .collect::<Vec<_>>();
    let peer_intervals = peer_events
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .collect::<Vec<_>>();
    println!(
        "start={start_frame} first={} lead={initial_lead} master={master_intervals:?} peer={peer_intervals:?} late={}/{} error={}/{} overrun={:?}->{}/{}",
        master_events[0],
        master.peek8(late_addr),
        peer.peek8(late_addr),
        master.peek8(error_addr),
        peer.peek8(error_addr),
        overrun_at_start,
        master.peek8(overrun_addr),
        peer.peek8(overrun_addr),
    );

    assert!(
        (6..=8).contains(&initial_lead),
        "the first beat did not begin within the cable reset/cadence window"
    );
    assert!(
        master_intervals
            .iter()
            .all(|interval| (7..=8).contains(interval)),
        "the MASTER cadence was not steady from the first interval"
    );
    assert!(
        peer_intervals
            .iter()
            .all(|interval| (6..=9).contains(interval)),
        "the PEER cadence exceeded the one-frame observation tolerance"
    );
    assert!(
        master_events
            .iter()
            .zip(peer_events.iter())
            .all(|(master_frame, peer_frame)| master_frame.abs_diff(*peer_frame) <= 1),
        "master/peer event frames diverged by more than one emulator frame"
    );
    // Require clean late/error counters and no added overrun after playback began.
    // These assertions cover the emulated fixture, not a physical link cable.
    assert_eq!(master.peek8(late_addr), 0);
    assert_eq!(peer.peek8(late_addr), 0);
    assert_eq!(master.peek8(error_addr), 0);
    assert_eq!(peer.peek8(error_addr), 0);
    assert_eq!(
        overrun_at_start,
        Some((master.peek8(overrun_addr), peer.peek8(overrun_addr))),
        "playback introduced a frame overrun",
    );
}
