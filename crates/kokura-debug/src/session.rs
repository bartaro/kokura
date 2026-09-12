use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::Serialize;

use kokura_bridge::{
    classify_symbol_name, describe_bank_switch, FunctionInfo, KitaqgbIntrinsicKind,
    KitaqgbIntrinsicMatch, SourceLocationInfo, StaticEstimateInfo, SymbolInfo, SymbolTable,
};
use kokura_core::{
    interrupt::{INT_JOYPAD, INT_SERIAL, INT_TIMER},
    ppu::PpuMode,
    state::MachineState,
    types::{
        ApuTraceEvent, CgbTraceEvent, DmaTraceEvent, HdmaDeferredReason, InterruptSource,
        InterruptTraceEvent, IoTraceEvent, JoypadTraceEvent, MapperTraceEvent, PpuTraceEvent,
        SerialTraceEvent, TimerTraceEvent,
    },
    Machine,
};

use crate::{
    diagnostics::{
        analyze_basic, build_auto_diagnosis, build_timing_pack_report, AutoDiagnosisReport,
        Diagnostic, DiagnosticInput,
    },
    events::DebugEvent,
    report::{
        AbiBoundaryRegisterCheckReport, AbiCallBoundaryReport, AbiIntrinsicBoundaryReport,
        AbiIntrinsicRegisterCheckReport, AbiRegisterContractRuleReport, AbiRegisterSnapshotReport,
        AbiReturnBoundaryReport, AbiReturnRegisterCheckReport, AbiStackWindowReport,
        AbiVerificationReport, DebugReport, EventSummary, ForensicHotLoopReport,
        ForensicHotspotReport, ForensicMapperActivityReport, ForensicUnsupportedOpcodeReport,
        HeatmapBucketReport, HeatmapCulpritBankReport, HeatmapCulpritFunctionReport,
        HeatmapRegionTotalReport, HeatmapReport, ProfilerBankActivityReport,
        ProfilerBankTransitionReport, ProfilerFunctionActivityReport, ProfilerReport,
        ReleasePackagingSurfaceReport, ReleaseReadinessReport, ReleaseSmokeSurfaceReport,
        ReleaseStabilitySurfaceReport, ReplayCheckpointReport, ReplayDivergenceReport,
        ReplayReport, ReplayRewindReport, ReplaySliceReport, ReplayWatchDigestReport, ReportMeta,
        RomForensicsReport, SourceLocationReport, SymbolReport, ToolchainBuildReport,
        ToolchainFunctionReport, ToolchainStaticEstimateReport, UnsupportedOpcodeSummary,
        VisualizationApuReport, VisualizationBankStateReport, VisualizationDmaReport,
        VisualizationLayerReport, VisualizationOamReport, VisualizationPalettePreviewReport,
        VisualizationPaletteReport, VisualizationSpriteReport, VisualizationTilePreviewReport,
        VisualizationTileReport, VisualizationsReport, DEBUG_REPORT_SCHEMA_VERSION,
    },
    snapshot::{
        compute_bg_hash, compute_frame_hash, compute_oam_hash, compute_sprite_hash,
        compute_vram_hash, compute_window_hash, hash_bytes, CpuSnapshot, TimerSnapshot,
        VideoSnapshot,
    },
    stop::{
        ExecutionContextFrame, InterruptStopPhase, ReplayControlSet, SourceLocationStop,
        StopConditionSet, StopReason,
    },
    watch::{MemoryWatchBaselineMode, MemoryWatchInsight, MemoryWatchResult, MemoryWatchSpec},
};

const BANK_RETURN_GRACE_FRAMES: u64 = 120;

#[derive(Debug, Clone, Copy)]
struct PendingBankReturn {
    expected_bank: u16,
    started_frame: u64,
    reported: bool,
}

fn kitaqgb_thunk_target_bank(name: &str) -> Option<u16> {
    let start = name.find("__kq_thunk_b")?;
    let suffix = name[start..].strip_prefix("__kq_thunk_b")?;
    let digits = suffix.split('_').next()?;
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

#[derive(Debug)]
pub struct DebugSession {
    pub machine: Machine,
    pub last_frame_hash: Option<u32>,
    pub previous_frame_hash: Option<u32>,
    pub previous_vram_hash: Option<u32>,
    pub previous_oam_hash: Option<u32>,
    pub previous_bg_hash: Option<u32>,
    pub previous_window_hash: Option<u32>,
    pub previous_sprite_hash: Option<u32>,
    pub event_log: Vec<DebugEvent>,
    pub diagnostics: Vec<Diagnostic>,
    pub vblank_count: u64,
    pub timer_interrupt_count: u64,
    pub timer_overflow_count: u64,
    pub timer_reload_count: u64,
    pub timer_control_write_count: u64,
    pub div_reset_count: u64,
    pub serial_interrupt_count: u64,
    pub serial_transfer_count: u64,
    pub mapper_control_write_count: u64,
    pub mapper_rom_bank_change_count: u64,
    pub mapper_ram_bank_change_count: u64,
    pub apu_master_toggle_count: u64,
    pub apu_trigger_count: u64,
    pub apu_length_expire_count: u64,
    pub apu_envelope_step_count: u64,
    pub apu_sweep_step_count: u64,
    pub apu_channel_disabled_count: u64,
    pub apu_dac_change_count: u64,
    pub apu_pop_risk_count: u64,
    pub apu_frame_step_count: u64,
    pub apu_mixer_write_count: u64,
    pub apu_mix_output_count: u64,
    pub apu_pcm_frame_count: u64,
    pub apu_pcm_drop_count: u64,
    pub apu_pcm_peak_buffer_frames: u32,
    pub apu_wave_write_count: u64,
    pub apu_wave_alias_count: u64,
    pub apu_ch3_hold_count: u64,
    pub apu_noise_lock_count: u64,
    pub cgb_mode_select_count: u64,
    pub cgb_vram_bank_switch_count: u64,
    pub cgb_wram_bank_switch_count: u64,
    pub cgb_bg_palette_write_count: u64,
    pub cgb_obj_palette_write_count: u64,
    pub cgb_palette_blocked_count: u64,
    pub cgb_key1_write_count: u64,
    pub cgb_speed_switch_count: u64,
    pub cgb_speed_switch_freeze_count: u64,
    pub cgb_speed_switch_freeze_cycles: u64,
    pub joypad_interrupt_count: u64,
    pub joypad_read_count: u64,
    pub joypad_button_read_count: u64,
    pub joypad_dpad_read_count: u64,
    pub joypad_edge_count: u64,
    pub joypad_selection_write_count: u64,
    pub interrupt_service_count: u64,
    pub interrupt_pending_blocked_count: u64,
    pub bank_switch_count: u64,
    pub far_call_count: u64,
    pub intrinsic_count: u64,
    pub settile_flush_count: u64,
    pub buffered_settile_count: u64,
    pub cgb_settile_count: u64,
    pub flush_rows_count: u64,
    pub oam_dma_count: u64,
    pub oam_dma_start_count: u64,
    pub oam_dma_complete_count: u64,
    pub hdma_start_count: u64,
    pub hdma_block_count: u64,
    pub hdma_complete_count: u64,
    pub hdma_cancel_count: u64,
    pub gdma_stall_cycles_estimate: u64,
    pub hdma_stall_cycles_estimate: u64,
    pub hdma_deferred_count: u64,
    pub hdma_ignored_write_count: u64,
    pub wait_vblank_count: u64,
    pub present_count: u64,
    pub cgb_palette_count: u64,
    pub bank_guard_count: u64,
    pub trap_check_count: u64,
    pub oam_transfer_count: u64,
    pub input_path_count: u64,
    pub unsupported_opcode_count: u64,
    pub lcd_toggle_count: u64,
    pub stat_write_count: u64,
    pub lyc_write_count: u64,
    pub scanline_render_count: u64,
    pub replay_checkpoint_count: u64,
    pub replay_rewind_count: u64,
    pub replay_divergence_count: u64,
    pub bank_thrash_score: u32,
    bank_thrash_streak: u32,
    pub max_repeated_pc_hits: u32,
    pub symbol_table: Option<SymbolTable>,
    toolchain_build_report: Option<ToolchainBuildReport>,
    recent_pcs: VecDeque<(u16, u16)>,
    pc_hit_histogram: BTreeMap<(u16, u16), u64>,
    pc_cycle_histogram: BTreeMap<(u16, u16), u64>,
    executed_instruction_samples: u64,
    execution_context_trail: VecDeque<ExecutionContextFrame>,
    last_iflag: u8,
    last_symbol_name: Option<String>,
    last_source_label: Option<String>,
    start_frame_counter: u64,
    start_cycle_counter: u64,
    start_rom_bank: u16,
    start_pc: u16,
    end_rom_bank: u16,
    bank_restored: bool,
    last_intrinsic: Option<String>,
    last_far_call_symbol: Option<String>,
    last_bank_switch_from_symbol: Option<String>,
    last_bank_switch_to_symbol: Option<String>,
    previous_bank_switch: Option<(u16, u16)>,
    pending_bank_returns: Vec<PendingBankReturn>,
    seen_wait_vblank: bool,
    wait_before_present: bool,
    screen_changed: bool,
    vram_changed: bool,
    oam_changed: bool,
    bg_hash_changed: bool,
    window_hash_changed: bool,
    sprite_hash_changed: bool,
    bg_changed: bool,
    window_changed: bool,
    sprite_changed: bool,
    last_ly: u8,
    last_ppu_mode: PpuMode,
    last_stat_coincidence: bool,
    scanline_event_count: u64,
    stat_signal_count: u64,
    halted_on_unsupported_opcode: bool,
    unsupported_opcodes: BTreeMap<u8, UnsupportedOpcodeSummary>,
    watch_windows: Vec<MemoryWatchSpec>,
    watch_window_baselines: Vec<Vec<u8>>,
    watch_previous_frame_baselines: Vec<Vec<u8>>,
    watch_named_baselines: BTreeMap<String, Vec<Vec<u8>>>,
    watch_baseline_mode: MemoryWatchBaselineMode,
    watch_named_baseline_active: Option<String>,
    stop_conditions: StopConditionSet,
    stop_watchpoint_baselines: Vec<Vec<u8>>,
    replay_control: ReplayControlSet,
    replay_checkpoints: VecDeque<ReplayCheckpoint>,
    replay_frame_digests: BTreeMap<u64, HistoricalReplayDigest>,
    replay_generation: u32,
    replay_last_checkpoint_key: Option<(u64, u64)>,
    replay_last_rewind: Option<ReplayRewindReport>,
    replay_divergence: Option<ReplayDivergenceReport>,
    last_stop_reason: Option<StopReason>,
    stopped_by_debugger: bool,
    observed_frame_start: Option<u64>,
}

fn interrupt_source_name(source: InterruptSource) -> &'static str {
    match source {
        InterruptSource::Vblank => "vblank",
        InterruptSource::LcdStat => "lcd_stat",
        InterruptSource::Timer => "timer",
        InterruptSource::Serial => "serial",
        InterruptSource::Joypad => "joypad",
    }
}

#[derive(Debug, Clone)]
struct ReplayCheckpoint {
    checkpoint_index: u64,
    generation: u32,
    frame: u64,
    cycle: u64,
    rom_bank: u16,
    pc: u16,
    frame_hash: u32,
    digest: u64,
    symbol: Option<String>,
    source: Option<SourceLocationStop>,
    watch_hashes: Vec<ReplayWatchDigestReport>,
    watch_digest: u64,
    event_log_len: usize,
    instruction_samples_total: u64,
    state: MachineState,
}

#[derive(Debug, Clone)]
struct HistoricalReplayDigest {
    digest: u64,
    generation: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DebugStepOutcome {
    pub frame_completed: bool,
    pub stop_triggered: bool,
    pub halted_on_unsupported_opcode: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObservationSnapshot {
    pub completed_frames: u64,
    pub active_frame: u64,
    pub cycle: u64,
    pub rom_bank: u16,
    pub pc: u16,
    pub sp: u16,
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub ly: u8,
    pub ppu_mode: String,
    pub symbol: Option<String>,
    pub source: Option<SourceLocationStop>,
}

impl DebugSession {
    pub fn new(machine: Machine) -> Self {
        let last_iflag = machine.interrupt.iflag;
        let initial_bank = machine.current_rom_bank();
        let initial_pc = machine.cpu.pc;
        let initial_frame_counter = machine.clocks.frames;
        let initial_cycle_counter = machine.clocks.cycles;
        let initial_ppu_mode = machine.ppu.current_mode();
        let initial_stat_coincidence = machine.ppu.stat_coincidence();
        Self {
            machine,
            last_frame_hash: None,
            previous_frame_hash: None,
            previous_vram_hash: None,
            previous_oam_hash: None,
            previous_bg_hash: None,
            previous_window_hash: None,
            previous_sprite_hash: None,
            event_log: Vec::new(),
            diagnostics: Vec::new(),
            vblank_count: 0,
            timer_interrupt_count: 0,
            timer_overflow_count: 0,
            timer_reload_count: 0,
            timer_control_write_count: 0,
            div_reset_count: 0,
            serial_interrupt_count: 0,
            serial_transfer_count: 0,
            mapper_control_write_count: 0,
            mapper_rom_bank_change_count: 0,
            mapper_ram_bank_change_count: 0,
            apu_master_toggle_count: 0,
            apu_trigger_count: 0,
            apu_length_expire_count: 0,
            apu_envelope_step_count: 0,
            apu_sweep_step_count: 0,
            apu_channel_disabled_count: 0,
            apu_dac_change_count: 0,
            apu_pop_risk_count: 0,
            apu_frame_step_count: 0,
            apu_mixer_write_count: 0,
            apu_mix_output_count: 0,
            apu_pcm_frame_count: 0,
            apu_pcm_drop_count: 0,
            apu_pcm_peak_buffer_frames: 0,
            apu_wave_write_count: 0,
            apu_wave_alias_count: 0,
            apu_ch3_hold_count: 0,
            apu_noise_lock_count: 0,
            cgb_mode_select_count: 0,
            cgb_vram_bank_switch_count: 0,
            cgb_wram_bank_switch_count: 0,
            cgb_bg_palette_write_count: 0,
            cgb_obj_palette_write_count: 0,
            cgb_palette_blocked_count: 0,
            cgb_key1_write_count: 0,
            cgb_speed_switch_count: 0,
            cgb_speed_switch_freeze_count: 0,
            cgb_speed_switch_freeze_cycles: 0,
            joypad_interrupt_count: 0,
            joypad_read_count: 0,
            joypad_button_read_count: 0,
            joypad_dpad_read_count: 0,
            joypad_edge_count: 0,
            joypad_selection_write_count: 0,
            interrupt_service_count: 0,
            interrupt_pending_blocked_count: 0,
            bank_switch_count: 0,
            far_call_count: 0,
            intrinsic_count: 0,
            settile_flush_count: 0,
            buffered_settile_count: 0,
            cgb_settile_count: 0,
            flush_rows_count: 0,
            oam_dma_count: 0,
            oam_dma_start_count: 0,
            oam_dma_complete_count: 0,
            hdma_start_count: 0,
            hdma_block_count: 0,
            hdma_complete_count: 0,
            hdma_cancel_count: 0,
            gdma_stall_cycles_estimate: 0,
            hdma_stall_cycles_estimate: 0,
            hdma_deferred_count: 0,
            hdma_ignored_write_count: 0,
            wait_vblank_count: 0,
            present_count: 0,
            cgb_palette_count: 0,
            bank_guard_count: 0,
            trap_check_count: 0,
            oam_transfer_count: 0,
            input_path_count: 0,
            unsupported_opcode_count: 0,
            lcd_toggle_count: 0,
            stat_write_count: 0,
            lyc_write_count: 0,
            scanline_render_count: 0,
            replay_checkpoint_count: 0,
            replay_rewind_count: 0,
            replay_divergence_count: 0,
            bank_thrash_score: 0,
            bank_thrash_streak: 0,
            max_repeated_pc_hits: 0,
            symbol_table: None,
            toolchain_build_report: None,
            recent_pcs: VecDeque::with_capacity(64),
            pc_hit_histogram: BTreeMap::new(),
            pc_cycle_histogram: BTreeMap::new(),
            executed_instruction_samples: 0,
            execution_context_trail: VecDeque::with_capacity(16),
            last_iflag,
            last_symbol_name: None,
            last_source_label: None,
            start_frame_counter: initial_frame_counter,
            start_cycle_counter: initial_cycle_counter,
            start_rom_bank: initial_bank,
            start_pc: initial_pc,
            end_rom_bank: initial_bank,
            bank_restored: true,
            last_intrinsic: None,
            last_far_call_symbol: None,
            last_bank_switch_from_symbol: None,
            last_bank_switch_to_symbol: None,
            previous_bank_switch: None,
            pending_bank_returns: Vec::new(),
            seen_wait_vblank: false,
            wait_before_present: false,
            screen_changed: false,
            vram_changed: false,
            oam_changed: false,
            bg_hash_changed: false,
            window_hash_changed: false,
            sprite_hash_changed: false,
            bg_changed: false,
            window_changed: false,
            sprite_changed: false,
            last_ly: 0,
            last_ppu_mode: initial_ppu_mode,
            last_stat_coincidence: initial_stat_coincidence,
            scanline_event_count: 0,
            stat_signal_count: 0,
            halted_on_unsupported_opcode: false,
            unsupported_opcodes: BTreeMap::new(),
            watch_windows: Vec::new(),
            watch_window_baselines: Vec::new(),
            watch_previous_frame_baselines: Vec::new(),
            watch_named_baselines: BTreeMap::new(),
            watch_baseline_mode: MemoryWatchBaselineMode::Initial,
            watch_named_baseline_active: None,
            stop_conditions: StopConditionSet::default(),
            stop_watchpoint_baselines: Vec::new(),
            replay_control: ReplayControlSet::default(),
            replay_checkpoints: VecDeque::new(),
            replay_frame_digests: BTreeMap::new(),
            replay_generation: 0,
            replay_last_checkpoint_key: None,
            replay_last_rewind: None,
            replay_divergence: None,
            last_stop_reason: None,
            stopped_by_debugger: false,
            observed_frame_start: None,
        }
    }

    pub fn run_frames(&mut self, frames: u64) -> Result<(), kokura_core::error::CoreError> {
        self.run_frames_with_callback(frames, |_, _| {})
    }

    pub fn debug_step(&mut self) -> Result<DebugStepOutcome, kokura_core::error::CoreError> {
        self.run_debug_step()
    }

    pub fn observation_snapshot(&self) -> ObservationSnapshot {
        let completed_frames = self.machine.clocks.frames;
        ObservationSnapshot {
            completed_frames,
            active_frame: completed_frames.saturating_add(1),
            cycle: self.machine.clocks.cycles,
            rom_bank: self.machine.current_rom_bank(),
            pc: self.machine.cpu.pc,
            sp: self.machine.cpu.sp,
            a: self.machine.cpu.a,
            f: self.machine.cpu.f.0,
            b: self.machine.cpu.b,
            c: self.machine.cpu.c,
            d: self.machine.cpu.d,
            e: self.machine.cpu.e,
            h: self.machine.cpu.h,
            l: self.machine.cpu.l,
            ly: self.machine.ppu.ly,
            ppu_mode: format!("{:?}", self.machine.ppu.current_mode()),
            symbol: self.current_pc_symbol(),
            source: self.current_source_location(),
        }
    }

    pub fn watched_memory_results_snapshot(&self) -> Vec<MemoryWatchResult> {
        self.watched_memory_results()
    }

    pub fn machine(&self) -> &Machine {
        &self.machine
    }

    pub fn exchange_serial_with_peer(&mut self, peer: &mut Self) -> bool {
        let result = self.machine.exchange_serial_with_peer(&mut peer.machine);
        self.capture_io_trace(&result.self_io_trace);
        self.capture_interrupt_edges(&result.self_interrupt_trace);
        peer.capture_io_trace(&result.peer_io_trace);
        peer.capture_interrupt_edges(&result.peer_interrupt_trace);
        result.completed
    }

    /// Completes one byte driven by an attached external-clock serial device.
    /// Returns the byte the emulated Game Boy transmitted when a transfer was
    /// armed, or `None` when SC was not in external-clock transfer mode.
    pub fn clock_external_serial_byte(&mut self, incoming: u8) -> Option<u8> {
        let result = self.machine.clock_external_serial_byte(incoming);
        self.capture_io_trace(&result.io_trace);
        self.capture_interrupt_edges(&result.interrupt_trace);
        result.completed.then_some(result.outgoing)
    }

    pub fn run_frames_collect_audio(
        &mut self,
        frames: u64,
        audio_out: &mut Vec<i16>,
    ) -> Result<(), kokura_core::error::CoreError> {
        self.run_frames_with_callback(frames, |session, _| {
            let available = session.machine.audio_frames_available();
            if available > 0 {
                audio_out.extend(
                    session
                        .machine
                        .drain_audio_frames_interleaved_i16(available),
                );
            }
        })
    }

    pub fn run_frames_with_callback<F>(
        &mut self,
        frames: u64,
        mut after_frame: F,
    ) -> Result<(), kokura_core::error::CoreError>
    where
        F: FnMut(&mut Self, u64),
    {
        let start_frames = self.machine.clocks.frames;
        let start_hash = self
            .last_frame_hash
            .or_else(|| Some(compute_frame_hash(&self.machine)));
        self.previous_frame_hash = start_hash;
        self.vblank_count = 0;
        self.timer_interrupt_count = 0;
        self.timer_overflow_count = 0;
        self.timer_reload_count = 0;
        self.timer_control_write_count = 0;
        self.div_reset_count = 0;
        self.serial_interrupt_count = 0;
        self.serial_transfer_count = 0;
        self.joypad_interrupt_count = 0;
        self.joypad_read_count = 0;
        self.joypad_button_read_count = 0;
        self.joypad_dpad_read_count = 0;
        self.joypad_edge_count = 0;
        self.joypad_selection_write_count = 0;
        self.interrupt_service_count = 0;
        self.interrupt_pending_blocked_count = 0;
        self.bank_switch_count = 0;
        self.far_call_count = 0;
        self.intrinsic_count = 0;
        self.settile_flush_count = 0;
        self.buffered_settile_count = 0;
        self.cgb_settile_count = 0;
        self.flush_rows_count = 0;
        self.oam_dma_count = 0;
        self.oam_dma_start_count = 0;
        self.oam_dma_complete_count = 0;
        self.hdma_start_count = 0;
        self.hdma_block_count = 0;
        self.hdma_complete_count = 0;
        self.hdma_cancel_count = 0;
        self.gdma_stall_cycles_estimate = 0;
        self.hdma_stall_cycles_estimate = 0;
        self.hdma_deferred_count = 0;
        self.hdma_ignored_write_count = 0;
        self.wait_vblank_count = 0;
        self.present_count = 0;
        self.cgb_palette_count = 0;
        self.cgb_speed_switch_freeze_count = 0;
        self.cgb_speed_switch_freeze_cycles = 0;
        self.bank_guard_count = 0;
        self.trap_check_count = 0;
        self.oam_transfer_count = 0;
        self.input_path_count = 0;
        self.unsupported_opcode_count = 0;
        self.lcd_toggle_count = 0;
        self.stat_write_count = 0;
        self.lyc_write_count = 0;
        self.scanline_render_count = 0;
        self.replay_checkpoint_count = 0;
        self.replay_rewind_count = 0;
        self.replay_divergence_count = 0;
        self.replay_last_rewind = None;
        self.replay_divergence = None;
        self.bank_thrash_score = 0;
        self.bank_thrash_streak = 0;
        self.max_repeated_pc_hits = 0;
        self.halted_on_unsupported_opcode = false;
        self.last_stop_reason = None;
        self.stopped_by_debugger = false;
        self.unsupported_opcodes.clear();
        self.recent_pcs.clear();
        self.pc_hit_histogram.clear();
        self.pc_cycle_histogram.clear();
        self.executed_instruction_samples = 0;
        self.execution_context_trail.clear();
        self.last_iflag = self.machine.interrupt.iflag;
        self.last_symbol_name = self.current_pc_symbol();
        self.last_source_label = self.current_source_label();
        self.start_frame_counter = self.machine.clocks.frames;
        self.start_cycle_counter = self.machine.clocks.cycles;
        self.start_rom_bank = self.machine.current_rom_bank();
        self.start_pc = self.machine.cpu.pc;
        self.end_rom_bank = self.start_rom_bank;
        self.bank_restored = true;
        self.last_intrinsic = None;
        self.last_far_call_symbol = None;
        self.last_bank_switch_from_symbol = None;
        self.last_bank_switch_to_symbol = None;
        self.previous_bank_switch = None;
        self.pending_bank_returns.clear();
        self.seen_wait_vblank = false;
        self.wait_before_present = false;
        self.screen_changed = false;
        self.vram_changed = false;
        self.oam_changed = false;
        self.bg_hash_changed = false;
        self.window_hash_changed = false;
        self.sprite_hash_changed = false;
        self.bg_changed = false;
        self.window_changed = false;
        self.sprite_changed = false;
        self.scanline_event_count = 0;
        self.stat_signal_count = 0;
        self.last_ppu_mode = self.machine.ppu.current_mode();
        self.last_stat_coincidence = self.machine.ppu.stat_coincidence();
        self.observed_frame_start = None;
        self.refresh_watch_window_baselines();
        self.refresh_stop_watchpoint_baselines();
        for frame_idx in 0..frames {
            self.run_single_frame()?;
            after_frame(self, frame_idx + 1);
            self.maybe_record_replay_checkpoint();
            if self.last_stop_reason.is_some() {
                if let Some(frames_back) = self.replay_control.auto_rewind_on_stop_frames {
                    let _ = self.rewind_frames(frames_back);
                }
            }
            if self.halted_on_unsupported_opcode || self.last_stop_reason.is_some() {
                break;
            }
        }
        self.finalize_bank_return_tracking();
        self.diagnostics = analyze_basic(
            &self.machine,
            DiagnosticInput {
                vblank_events: self.vblank_count,
                screen_changed: self.screen_changed,
                repeated_pc_hits: self.max_repeated_pc_hits,
                executed_frames: self.machine.clocks.frames.saturating_sub(start_frames),
                timer_interrupts: self.timer_interrupt_count,
                timer_overflows: self.timer_overflow_count,
                serial_interrupts: self.serial_interrupt_count,
                joypad_interrupts: self.joypad_interrupt_count,
                joypad_read_count: self.joypad_read_count,
                joypad_button_read_count: self.joypad_button_read_count,
                joypad_dpad_read_count: self.joypad_dpad_read_count,
                interrupt_services: self.interrupt_service_count,
                interrupt_pending_blocked_count: self.interrupt_pending_blocked_count,
                apu_trigger_count: self.apu_trigger_count,
                apu_mix_output_count: self.apu_mix_output_count,
                apu_pop_risk_count: self.apu_pop_risk_count,
                apu_wave_alias_count: self.apu_wave_alias_count,
                apu_noise_lock_count: self.apu_noise_lock_count,
                apu_pcm_frame_count: self.apu_pcm_frame_count,
                apu_pcm_drop_count: self.apu_pcm_drop_count,
                apu_pcm_peak_buffer_frames: self.apu_pcm_peak_buffer_frames,
                far_call_count: self.far_call_count,
                intrinsic_count: self.intrinsic_count,
                settile_flush_count: self.settile_flush_count,
                bank_switches: self.bank_switch_count,
                bank_restored: self.bank_restored,
                bank_thrash_score: self.bank_thrash_score,
                buffered_settile_count: self.buffered_settile_count,
                cgb_settile_count: self.cgb_settile_count,
                flush_rows_count: self.flush_rows_count,
                oam_dma_count: self.oam_dma_count,
                oam_dma_start_count: self.oam_dma_start_count,
                oam_dma_complete_count: self.oam_dma_complete_count,
                hdma_start_count: self.hdma_start_count,
                hdma_block_count: self.hdma_block_count,
                hdma_complete_count: self.hdma_complete_count,
                hdma_cancel_count: self.hdma_cancel_count,
                gdma_stall_cycles_estimate: self.gdma_stall_cycles_estimate,
                hdma_stall_cycles_estimate: self.hdma_stall_cycles_estimate,
                hdma_deferred_count: self.hdma_deferred_count,
                hdma_ignored_write_count: self.hdma_ignored_write_count,
                wait_vblank_count: self.wait_vblank_count,
                cgb_palette_count: self.cgb_palette_count,
                bank_guard_count: self.bank_guard_count,
                trap_check_count: self.trap_check_count,
                oam_transfer_count: self.oam_transfer_count,
                dmg_mode: matches!(self.machine.mode, kokura_core::types::HardwareMode::Dmg),
                vram_changed: self.vram_changed,
                bg_enabled: self.machine.ppu.lcdc & 0x01 != 0,
                present_path_count: self.present_count,
                input_path_count: self.input_path_count,
                input_active: self.machine.joypad.mask != 0,
                wait_before_present: self.wait_before_present,
                oam_changed: self.oam_changed,
                visible_sprite_count: self
                    .machine
                    .ppu
                    .visible_sprite_count_estimate(self.machine.memory.oam()),
                nonzero_oam_entries: self
                    .machine
                    .ppu
                    .nonzero_oam_entries(self.machine.memory.oam()),
                sprite_enabled: self.machine.ppu.sprite_enabled(),
                window_enabled: self.machine.ppu.window_enabled(),
                bg_changed: self.bg_changed,
                window_changed: self.window_changed,
                sprite_changed: self.sprite_changed,
                bg_hash_changed: self.bg_hash_changed,
                window_hash_changed: self.window_hash_changed,
                sprite_hash_changed: self.sprite_hash_changed,
                stat_signal_count: self.stat_signal_count,
                scanline_event_count: self.scanline_event_count,
                unsupported_opcode_count: self.unsupported_opcode_count,
                halted_on_unsupported_opcode: self.halted_on_unsupported_opcode,
                lcd_toggle_count: self.lcd_toggle_count,
                stat_write_count: self.stat_write_count,
                lyc_write_count: self.lyc_write_count,
                scanline_render_count: self.scanline_render_count,
            },
        );
        Ok(())
    }

    pub fn is_execution_stopped(&self) -> bool {
        self.halted_on_unsupported_opcode || self.last_stop_reason.is_some()
    }

    fn prepare_frame_tracking_if_needed(&mut self) {
        let current_frame = self.machine.clocks.frames;
        if self.observed_frame_start == Some(current_frame) {
            return;
        }
        let start_hash = self
            .last_frame_hash
            .or_else(|| Some(compute_frame_hash(&self.machine)));
        self.previous_frame_hash = start_hash;
        self.previous_vram_hash = Some(compute_vram_hash(&self.machine));
        self.previous_oam_hash = Some(compute_oam_hash(&self.machine));
        self.previous_bg_hash = Some(compute_bg_hash(&self.machine));
        self.previous_window_hash = Some(compute_window_hash(&self.machine));
        self.previous_sprite_hash = Some(compute_sprite_hash(&self.machine));
        self.last_ly = self.machine.ppu.ly;
        self.observed_frame_start = Some(current_frame);
    }

    fn finish_observed_frame(&mut self) {
        let current_watch_baselines = self
            .watch_windows
            .iter()
            .map(|spec| self.read_watch_window(spec.addr, spec.size))
            .collect::<Vec<_>>();
        let current_frame_hash = compute_frame_hash(&self.machine);
        let current_vram_hash = compute_vram_hash(&self.machine);
        let current_oam_hash = compute_oam_hash(&self.machine);
        let current_bg_hash = compute_bg_hash(&self.machine);
        let current_window_hash = compute_window_hash(&self.machine);
        let current_sprite_hash = compute_sprite_hash(&self.machine);
        let screen_changed = self
            .previous_frame_hash
            .is_some_and(|prev| prev != current_frame_hash);
        let vram_changed = self
            .previous_vram_hash
            .is_some_and(|prev| prev != current_vram_hash);
        let oam_changed = self
            .previous_oam_hash
            .is_some_and(|prev| prev != current_oam_hash);
        let bg_hash_changed = self
            .previous_bg_hash
            .is_some_and(|prev| prev != current_bg_hash);
        let window_hash_changed = self
            .previous_window_hash
            .is_some_and(|prev| prev != current_window_hash);
        let sprite_hash_changed = self
            .previous_sprite_hash
            .is_some_and(|prev| prev != current_sprite_hash);

        self.last_frame_hash = Some(current_frame_hash);
        self.screen_changed |= screen_changed;
        self.vram_changed |= vram_changed;
        self.oam_changed |= oam_changed;
        self.bg_hash_changed |= bg_hash_changed;
        self.window_hash_changed |= window_hash_changed;
        self.sprite_hash_changed |= sprite_hash_changed;
        self.bg_changed |= self.machine.ppu.bg_enabled() && bg_hash_changed;
        self.window_changed |= self.machine.ppu.window_enabled() && window_hash_changed;
        self.sprite_changed |=
            self.machine.ppu.sprite_enabled() && (sprite_hash_changed || oam_changed);

        self.previous_frame_hash = self.last_frame_hash;
        self.previous_vram_hash = Some(current_vram_hash);
        self.previous_oam_hash = Some(current_oam_hash);
        self.previous_bg_hash = Some(current_bg_hash);
        self.previous_window_hash = Some(current_window_hash);
        self.previous_sprite_hash = Some(current_sprite_hash);
        self.watch_previous_frame_baselines = current_watch_baselines;
        self.last_ly = self.machine.ppu.ly;
        self.observed_frame_start = Some(self.machine.clocks.frames);
    }

    pub fn run_debug_step(&mut self) -> Result<DebugStepOutcome, kokura_core::error::CoreError> {
        self.prepare_frame_tracking_if_needed();
        let frame_before = self.machine.clocks.frames;
        let pc_before = self.machine.cpu.pc;
        let code_bank_before = self.current_code_bank();
        let selected_bank_before = self.machine.current_rom_bank();
        let symbol_before = self.current_pc_symbol();
        if let Some(reason) = self.check_execute_breakpoints_before_step(
            pc_before,
            code_bank_before,
            symbol_before.as_deref(),
        ) {
            self.record_stop_reason(reason);
            return Ok(DebugStepOutcome {
                stop_triggered: true,
                ..DebugStepOutcome::default()
            });
        }

        let mmio_before = self.read_mmio_stop_values();
        let lcdc_before = self.machine.ppu.lcdc;
        let stat_before = self.machine.ppu.read_stat();
        let lyc_before = self.machine.ppu.lyc;
        self.record_pc(code_bank_before, pc_before);
        let step = match self.machine.step_instruction() {
            Ok(step) => step,
            Err(kokura_core::error::CoreError::UnsupportedOpcode { opcode, pc }) => {
                self.record_unsupported_opcode(opcode, pc);
                return Ok(DebugStepOutcome {
                    halted_on_unsupported_opcode: true,
                    ..DebugStepOutcome::default()
                });
            }
            Err(err) => return Err(err),
        };
        self.executed_instruction_samples = self.executed_instruction_samples.saturating_add(1);
        self.record_pc_cycles(code_bank_before, pc_before, step.cycles);
        self.capture_ppu_trace(&step.ppu_trace);
        self.capture_dma_trace(&step.dma_trace);
        self.capture_mapper_trace(&step.mapper_trace);
        self.capture_io_trace(&step.io_trace);
        self.capture_apu_trace(&step.apu_trace);
        self.capture_cgb_trace(&step.cgb_trace);
        self.capture_ppu_register_writes(lcdc_before, stat_before, lyc_before);
        self.capture_interrupt_edges(&step.interrupt_trace);
        self.capture_symbol_event();

        let current_bank = self.machine.current_rom_bank();
        if current_bank != selected_bank_before {
            let trace = describe_bank_switch(
                self.symbol_table.as_ref(),
                selected_bank_before,
                current_bank,
                self.machine.cpu.pc,
            );
            self.bank_switch_count += 1;
            let from_symbol = trace.from_symbol.clone();
            let to_symbol = trace.to_symbol.clone();
            self.last_bank_switch_from_symbol = from_symbol.clone();
            self.last_bank_switch_to_symbol = to_symbol.clone();
            self.push_execution_context("bank_switch", current_bank, self.machine.cpu.pc);
            self.event_log.push(DebugEvent::BankSwitch {
                frame: self.machine.clocks.frames,
                cycle: self.machine.clocks.cycles,
                from: selected_bank_before,
                to: current_bank,
                from_symbol: from_symbol.clone(),
                to_symbol: to_symbol.clone(),
            });

            let thunk_target = from_symbol
                .as_deref()
                .and_then(kitaqgb_thunk_target_bank)
                .or_else(|| to_symbol.as_deref().and_then(kitaqgb_thunk_target_bank));
            let known_thunk_return = thunk_target
                .is_some_and(|target| selected_bank_before == target && current_bank != target);
            let known_thunk_entry = thunk_target
                .is_some_and(|target| current_bank == target && selected_bank_before != target);
            self.update_bank_thrash(
                selected_bank_before,
                current_bank,
                known_thunk_entry || known_thunk_return,
            );
            if trace.suspected_far_call && known_thunk_return {
                if self
                    .pending_bank_returns
                    .last()
                    .is_some_and(|pending| pending.expected_bank == current_bank)
                {
                    self.pending_bank_returns.pop();
                }
            } else if trace.suspected_far_call && known_thunk_entry {
                self.far_call_count += 1;
                self.last_far_call_symbol = to_symbol.clone().or(from_symbol.clone());
                self.pending_bank_returns.push(PendingBankReturn {
                    expected_bank: selected_bank_before,
                    started_frame: self.machine.clocks.frames,
                    reported: false,
                });
                self.push_execution_context(
                    "far_call_suspected",
                    current_bank,
                    self.machine.cpu.pc,
                );
                self.event_log.push(DebugEvent::FarCallSuspected {
                    frame: self.machine.clocks.frames,
                    cycle: self.machine.clocks.cycles,
                    from_bank: selected_bank_before,
                    to_bank: current_bank,
                    from_symbol,
                    to_symbol,
                });
            }
            self.end_rom_bank = current_bank;
        }

        let stop_reason = self
            .check_watchpoints_after_step()
            .or_else(|| self.check_mmio_stops_after_step(&mmio_before))
            .or_else(|| self.check_interrupt_stops_after_step(&step.interrupt_trace))
            .or_else(|| self.check_dma_stops_after_step(&step.dma_trace));

        let frame_completed = step.frame_completed || self.machine.clocks.frames != frame_before;
        if frame_completed {
            self.finish_observed_frame();
        }

        if let Some(reason) = stop_reason {
            self.record_stop_reason(reason);
        }

        Ok(DebugStepOutcome {
            frame_completed,
            stop_triggered: self.last_stop_reason.is_some(),
            halted_on_unsupported_opcode: self.halted_on_unsupported_opcode,
        })
    }

    fn run_single_frame(&mut self) -> Result<(), kokura_core::error::CoreError> {
        let start_frame = self.machine.clocks.frames;
        self.prepare_frame_tracking_if_needed();

        while self.machine.clocks.frames == start_frame {
            let outcome = self.run_debug_step()?;
            if outcome.stop_triggered || outcome.halted_on_unsupported_opcode {
                break;
            }
        }

        if self.last_frame_hash.is_none() {
            self.previous_frame_hash = self.last_frame_hash;
            self.last_frame_hash = Some(compute_frame_hash(&self.machine));
        }

        Ok(())
    }

    fn capture_ppu_trace(&mut self, trace: &[PpuTraceEvent]) {
        for event in trace {
            match *event {
                PpuTraceEvent::ScanlineAdvance { from_ly, to_ly } => {
                    self.scanline_event_count += 1;
                    self.event_log.push(DebugEvent::ScanlineAdvance {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        from_ly,
                        to_ly,
                    });
                    self.last_ly = to_ly;
                }
                PpuTraceEvent::PpuModeChange {
                    from_mode,
                    to_mode,
                    ly,
                } => {
                    let from_mode = match from_mode {
                        0 => PpuMode::HBlank,
                        1 => PpuMode::VBlank,
                        2 => PpuMode::OamSearch,
                        _ => PpuMode::Transfer,
                    };
                    let to_mode = match to_mode {
                        0 => PpuMode::HBlank,
                        1 => PpuMode::VBlank,
                        2 => PpuMode::OamSearch,
                        _ => PpuMode::Transfer,
                    };
                    self.event_log.push(DebugEvent::PpuModeChange {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        from_mode: format!("{:?}", from_mode),
                        to_mode: format!("{:?}", to_mode),
                        ly,
                    });
                    self.last_ppu_mode = to_mode;
                }
                PpuTraceEvent::StatSignal {
                    coincidence,
                    ly,
                    lyc,
                } => {
                    self.stat_signal_count += 1;
                    self.event_log.push(DebugEvent::StatSignal {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        stat: self.machine.ppu.read_stat(),
                        ly,
                        lyc,
                        coincidence,
                    });
                    self.last_stat_coincidence = coincidence;
                }
                PpuTraceEvent::VblankEnter { ly } => {
                    self.vblank_count += 1;
                    self.event_log.push(DebugEvent::VblankEnter {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        ly,
                    });
                }
                PpuTraceEvent::FrameComplete { frame_serial: _ } => {
                    let hash = compute_frame_hash(&self.machine);
                    self.event_log.push(DebugEvent::FrameComplete {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        frame_hash: hash,
                    });
                    self.last_frame_hash = Some(hash);
                }
                PpuTraceEvent::ScanlineRender { ly } => {
                    self.scanline_render_count += 1;
                    self.event_log.push(DebugEvent::ScanlineRender {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        ly,
                    });
                }
            }
        }
    }

    fn capture_dma_trace(&mut self, trace: &[DmaTraceEvent]) {
        for event in trace {
            match *event {
                DmaTraceEvent::OamDmaStart { source } => {
                    self.oam_dma_start_count += 1;
                    self.event_log.push(DebugEvent::OamDmaStart {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        source,
                    });
                }
                DmaTraceEvent::OamDmaComplete { source, bytes } => {
                    self.oam_dma_complete_count += 1;
                    self.event_log.push(DebugEvent::OamDmaComplete {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        source,
                        bytes,
                    });
                }
                DmaTraceEvent::GdmaStallEstimate {
                    blocks,
                    stall_cycles,
                } => {
                    self.gdma_stall_cycles_estimate += u64::from(stall_cycles);
                    self.event_log.push(DebugEvent::GdmaStallEstimate {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        blocks,
                        stall_cycles,
                    });
                }
                DmaTraceEvent::HdmaStart {
                    source,
                    dest,
                    blocks,
                    hblank_mode,
                } => {
                    self.hdma_start_count += 1;
                    self.event_log.push(DebugEvent::HdmaStart {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        source,
                        dest,
                        blocks,
                        hblank_mode,
                    });
                }
                DmaTraceEvent::HdmaBlock {
                    source,
                    dest,
                    block_index,
                    remaining_blocks,
                    hblank_mode,
                    stall_cycles,
                } => {
                    self.hdma_block_count += 1;
                    if hblank_mode {
                        self.hdma_stall_cycles_estimate += u64::from(stall_cycles);
                    }
                    self.event_log.push(DebugEvent::HdmaBlock {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        source,
                        dest,
                        block_index,
                        remaining_blocks,
                        hblank_mode,
                        stall_cycles,
                    });
                }
                DmaTraceEvent::HdmaComplete {
                    source,
                    dest,
                    blocks,
                    hblank_mode,
                } => {
                    self.hdma_complete_count += 1;
                    self.event_log.push(DebugEvent::HdmaComplete {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        source,
                        dest,
                        blocks,
                        hblank_mode,
                    });
                }
                DmaTraceEvent::HdmaCancel {
                    source,
                    dest,
                    remaining_blocks,
                } => {
                    self.hdma_cancel_count += 1;
                    self.event_log.push(DebugEvent::HdmaCancel {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        source,
                        dest,
                        remaining_blocks,
                    });
                }
                DmaTraceEvent::HdmaDeferred {
                    remaining_blocks,
                    ly,
                    reason,
                } => {
                    self.hdma_deferred_count += 1;
                    let reason_text = match reason {
                        HdmaDeferredReason::CpuHalted => "cpu_halted".to_string(),
                    };
                    self.event_log.push(DebugEvent::HdmaDeferred {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        remaining_blocks,
                        ly,
                        reason: reason_text,
                    });
                }
                DmaTraceEvent::HdmaWriteIgnored {
                    value,
                    remaining_blocks,
                } => {
                    self.hdma_ignored_write_count += 1;
                    self.event_log.push(DebugEvent::HdmaWriteIgnored {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        value,
                        remaining_blocks,
                    });
                }
            }
        }
    }

    fn capture_ppu_register_writes(&mut self, lcdc_before: u8, stat_before: u8, lyc_before: u8) {
        let lcdc_after = self.machine.ppu.lcdc;
        let stat_after = self.machine.ppu.read_stat();
        let lyc_after = self.machine.ppu.lyc;
        if (lcdc_before ^ lcdc_after) & 0x80 != 0 {
            self.lcd_toggle_count += 1;
            self.event_log.push(DebugEvent::LcdToggle {
                frame: self.machine.clocks.frames,
                cycle: self.machine.clocks.cycles,
                enabled: lcdc_after & 0x80 != 0,
                lcdc: lcdc_after,
            });
        }
        if (stat_before & 0x78) != (stat_after & 0x78) {
            self.stat_write_count += 1;
            self.event_log.push(DebugEvent::StatWrite {
                frame: self.machine.clocks.frames,
                cycle: self.machine.clocks.cycles,
                old_stat: stat_before,
                new_stat: stat_after,
            });
        }
        if lyc_before != lyc_after {
            self.lyc_write_count += 1;
            self.event_log.push(DebugEvent::LycWrite {
                frame: self.machine.clocks.frames,
                cycle: self.machine.clocks.cycles,
                old_lyc: lyc_before,
                new_lyc: lyc_after,
            });
        }
    }

    fn capture_mapper_trace(&mut self, trace: &[MapperTraceEvent]) {
        for event in trace {
            match *event {
                MapperTraceEvent::ControlWrite {
                    mapper,
                    addr,
                    value,
                    rom_bank,
                    ram_bank,
                } => {
                    self.mapper_control_write_count += 1;
                    self.event_log.push(DebugEvent::MapperControlWrite {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        mapper: mapper.name().to_string(),
                        addr,
                        value,
                        rom_bank,
                        ram_bank,
                    });
                }
                MapperTraceEvent::RomBankChange {
                    mapper,
                    addr,
                    value,
                    from,
                    to,
                } => {
                    self.mapper_rom_bank_change_count += 1;
                    self.event_log.push(DebugEvent::MapperRomBankChange {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        mapper: mapper.name().to_string(),
                        addr,
                        value,
                        from,
                        to,
                    });
                }
                MapperTraceEvent::RamBankChange {
                    mapper,
                    addr,
                    value,
                    from,
                    to,
                } => {
                    self.mapper_ram_bank_change_count += 1;
                    self.event_log.push(DebugEvent::MapperRamBankChange {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        mapper: mapper.name().to_string(),
                        addr,
                        value,
                        from,
                        to,
                    });
                }
            }
        }
    }

    fn capture_apu_trace(&mut self, trace: &[ApuTraceEvent]) {
        for event in trace {
            match *event {
                ApuTraceEvent::MasterToggle { enabled, nr52 } => {
                    self.apu_master_toggle_count += 1;
                    self.event_log.push(DebugEvent::ApuMasterToggle {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        enabled,
                        nr52,
                    });
                }
                ApuTraceEvent::ChannelTrigger {
                    channel,
                    reg,
                    value,
                } => {
                    self.apu_trigger_count += 1;
                    self.event_log.push(DebugEvent::ApuChannelTrigger {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        channel,
                        reg,
                        value,
                    });
                }
                ApuTraceEvent::ChannelLengthExpired { channel } => {
                    self.apu_length_expire_count += 1;
                    self.event_log.push(DebugEvent::ApuChannelLengthExpired {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        channel,
                    });
                }
                ApuTraceEvent::ChannelEnvelopeStep {
                    channel,
                    volume,
                    increasing,
                } => {
                    self.apu_envelope_step_count += 1;
                    self.event_log.push(DebugEvent::ApuEnvelopeStep {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        channel,
                        volume,
                        increasing,
                    });
                }
                ApuTraceEvent::ChannelSweepStep {
                    old_frequency,
                    new_frequency,
                    negate,
                    shift,
                } => {
                    self.apu_sweep_step_count += 1;
                    self.event_log.push(DebugEvent::ApuSweepStep {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        old_frequency,
                        new_frequency,
                        negate,
                        shift,
                    });
                }
                ApuTraceEvent::ChannelDisabled {
                    channel,
                    reason,
                    reg,
                } => {
                    self.apu_channel_disabled_count += 1;
                    let reason = match reason {
                        1 => "dac_off",
                        2 => "master_off",
                        3 => "sweep_overflow",
                        4 => "sweep_negate_cleared",
                        _ => "other",
                    }
                    .to_string();
                    self.event_log.push(DebugEvent::ApuChannelDisabled {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        channel,
                        reason,
                        reg,
                    });
                }
                ApuTraceEvent::DacStateChange {
                    channel,
                    enabled,
                    reg,
                    active,
                } => {
                    self.apu_dac_change_count += 1;
                    self.event_log.push(DebugEvent::ApuDacStateChange {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        channel,
                        enabled,
                        reg,
                        active,
                    });
                }
                ApuTraceEvent::PopRisk { source, reg } => {
                    self.apu_pop_risk_count += 1;
                    let source = match source {
                        1 => "dac_toggle",
                        2 => "nr50_change",
                        3 => "nr51_change",
                        _ => "other",
                    }
                    .to_string();
                    self.event_log.push(DebugEvent::ApuPopRisk {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        source,
                        reg,
                    });
                }
                ApuTraceEvent::FrameSequencerStep { step } => {
                    self.apu_frame_step_count += 1;
                    self.event_log.push(DebugEvent::ApuFrameSequencerStep {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        step,
                    });
                }
                ApuTraceEvent::MixerControlWrite { nr50, nr51, nr52 } => {
                    self.apu_mixer_write_count += 1;
                    self.event_log.push(DebugEvent::ApuMixerControl {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        nr50,
                        nr51,
                        nr52,
                    });
                }
                ApuTraceEvent::MixedOutput {
                    left,
                    right,
                    active_mask,
                } => {
                    self.apu_mix_output_count += 1;
                    self.event_log.push(DebugEvent::ApuMixedOutput {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        left,
                        right,
                        active_mask,
                    });
                }
                ApuTraceEvent::PcmFramesBuffered {
                    frames,
                    buffered_frames,
                } => {
                    self.apu_pcm_frame_count += u64::from(frames);
                    self.apu_pcm_peak_buffer_frames =
                        self.apu_pcm_peak_buffer_frames.max(buffered_frames);
                    self.event_log.push(DebugEvent::ApuPcmFramesBuffered {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        frames,
                        buffered_frames,
                    });
                }
                ApuTraceEvent::PcmBufferWrapped {
                    dropped_frames,
                    dropped_total,
                } => {
                    self.apu_pcm_drop_count += u64::from(dropped_frames);
                    self.event_log.push(DebugEvent::ApuPcmBufferWrapped {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        dropped_frames,
                        dropped_total,
                    });
                }
                ApuTraceEvent::WaveRamWrite { index, value } => {
                    self.apu_wave_write_count += 1;
                    self.event_log.push(DebugEvent::ApuWaveRamWrite {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        index,
                        value,
                    });
                }
                ApuTraceEvent::WaveRamAccessAliased {
                    requested_index,
                    actual_index,
                    is_write,
                } => {
                    self.apu_wave_alias_count += 1;
                    self.event_log.push(DebugEvent::ApuWaveRamAccessAliased {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        requested_index,
                        actual_index,
                        is_write,
                    });
                }
                ApuTraceEvent::Ch3TriggerRetainsSample {
                    buffered_sample,
                    next_index,
                } => {
                    self.apu_ch3_hold_count += 1;
                    self.event_log.push(DebugEvent::ApuCh3TriggerRetainsSample {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        buffered_sample,
                        next_index,
                    });
                }
                ApuTraceEvent::NoiseClockFrozen {
                    shift,
                    divisor_code,
                } => {
                    self.apu_noise_lock_count += 1;
                    self.event_log.push(DebugEvent::ApuNoiseClockFrozen {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        shift,
                        divisor_code,
                    });
                }
            }
        }
    }

    fn capture_cgb_trace(&mut self, trace: &[CgbTraceEvent]) {
        for event in trace {
            match *event {
                CgbTraceEvent::ModeSelected {
                    cgb_enabled,
                    cgb_only,
                } => {
                    self.cgb_mode_select_count += 1;
                    self.event_log.push(DebugEvent::CgbModeSelected {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        cgb_enabled,
                        cgb_only,
                    });
                }
                CgbTraceEvent::VramBankSwitch { bank, value } => {
                    self.cgb_vram_bank_switch_count += 1;
                    self.event_log.push(DebugEvent::CgbVramBankSwitch {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        bank,
                        value,
                    });
                }
                CgbTraceEvent::WramBankSwitch { bank, value } => {
                    self.cgb_wram_bank_switch_count += 1;
                    self.event_log.push(DebugEvent::CgbWramBankSwitch {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        bank,
                        value,
                    });
                }
                CgbTraceEvent::BgPaletteIndexWrite {
                    index,
                    auto_increment,
                } => {
                    self.event_log.push(DebugEvent::CgbBgPaletteIndexWrite {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        index,
                        auto_increment,
                    });
                }
                CgbTraceEvent::BgPaletteDataWrite {
                    index,
                    value,
                    blocked,
                    auto_increment,
                } => {
                    self.cgb_bg_palette_write_count += 1;
                    if blocked {
                        self.cgb_palette_blocked_count += 1;
                    }
                    self.event_log.push(DebugEvent::CgbBgPaletteDataWrite {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        index,
                        value,
                        blocked,
                        auto_increment,
                    });
                }
                CgbTraceEvent::ObjPaletteIndexWrite {
                    index,
                    auto_increment,
                } => {
                    self.event_log.push(DebugEvent::CgbObjPaletteIndexWrite {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        index,
                        auto_increment,
                    });
                }
                CgbTraceEvent::ObjPaletteDataWrite {
                    index,
                    value,
                    blocked,
                    auto_increment,
                } => {
                    self.cgb_obj_palette_write_count += 1;
                    if blocked {
                        self.cgb_palette_blocked_count += 1;
                    }
                    self.event_log.push(DebugEvent::CgbObjPaletteDataWrite {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        index,
                        value,
                        blocked,
                        auto_increment,
                    });
                }
                CgbTraceEvent::Key1Write {
                    armed,
                    double_speed,
                    value,
                } => {
                    self.cgb_key1_write_count += 1;
                    self.event_log.push(DebugEvent::CgbKey1Write {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        armed,
                        double_speed,
                        value,
                    });
                }
                CgbTraceEvent::SpeedSwitch {
                    double_speed,
                    stop_stall_cycles,
                } => {
                    self.cgb_speed_switch_count += 1;
                    self.event_log.push(DebugEvent::CgbSpeedSwitch {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        double_speed,
                        stop_stall_cycles,
                    });
                }
                CgbTraceEvent::SpeedSwitchFreeze {
                    cpu_cycles,
                    ppu_mode,
                } => {
                    self.cgb_speed_switch_freeze_count += 1;
                    self.cgb_speed_switch_freeze_cycles += u64::from(cpu_cycles);
                    self.event_log.push(DebugEvent::CgbSpeedSwitchFreeze {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        cpu_cycles,
                        ppu_mode,
                    });
                }
            }
        }
    }

    fn capture_io_trace(&mut self, trace: &[IoTraceEvent]) {
        for event in trace {
            match *event {
                IoTraceEvent::Timer(TimerTraceEvent::DivResetEdge { old_div }) => {
                    self.div_reset_count += 1;
                    self.event_log.push(DebugEvent::DivResetEdge {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        old_div,
                    });
                }
                IoTraceEvent::Timer(TimerTraceEvent::TacWrite { old_tac, new_tac }) => {
                    self.timer_control_write_count += 1;
                    self.event_log.push(DebugEvent::TimerControlWrite {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        old_tac,
                        new_tac,
                    });
                }
                IoTraceEvent::Timer(TimerTraceEvent::TimaOverflow { old_tima }) => {
                    self.timer_overflow_count += 1;
                    self.event_log.push(DebugEvent::TimerOverflow {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        old_tima,
                    });
                }
                IoTraceEvent::Timer(TimerTraceEvent::TimaReload { reloaded_tima, tma }) => {
                    self.timer_reload_count += 1;
                    self.event_log.push(DebugEvent::TimerReload {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        reloaded_tima,
                        tma,
                    });
                }
                IoTraceEvent::Serial(SerialTraceEvent::TransferStart {
                    sb,
                    sc,
                    internal_clock,
                }) => {
                    self.serial_transfer_count += 1;
                    self.event_log.push(DebugEvent::SerialTransferStart {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        sb,
                        sc,
                        internal_clock,
                    });
                }
                IoTraceEvent::Serial(SerialTraceEvent::TransferComplete {
                    sb,
                    sc,
                    internal_clock,
                }) => {
                    self.event_log.push(DebugEvent::SerialTransferComplete {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        sb,
                        sc,
                        internal_clock,
                    });
                }
                IoTraceEvent::Joypad(JoypadTraceEvent::InputEdge {
                    old_mask,
                    new_mask,
                    p1,
                }) => {
                    self.joypad_edge_count += 1;
                    self.event_log.push(DebugEvent::JoypadEdge {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        old_mask,
                        new_mask,
                        p1,
                    });
                }
                IoTraceEvent::Joypad(JoypadTraceEvent::Read { p1, select, mask }) => {
                    self.joypad_read_count += 1;
                    if select & 0x20 == 0 {
                        self.joypad_button_read_count += 1;
                    }
                    if select & 0x10 == 0 {
                        self.joypad_dpad_read_count += 1;
                    }
                    self.event_log.push(DebugEvent::JoypadRead {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        p1,
                        select,
                        mask,
                    });
                }
                IoTraceEvent::Joypad(JoypadTraceEvent::SelectionWrite { old_p1, new_p1 }) => {
                    self.joypad_selection_write_count += 1;
                    self.event_log.push(DebugEvent::JoypadSelectionWrite {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        old_p1,
                        new_p1,
                    });
                }
                IoTraceEvent::Joypad(JoypadTraceEvent::InterruptEdge { p1 }) => {
                    self.event_log.push(DebugEvent::JoypadInterrupt {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        p1,
                    });
                }
            }
        }
    }

    fn capture_interrupt_edges(&mut self, trace: &[InterruptTraceEvent]) {
        let raised = self.machine.interrupt.iflag & !self.last_iflag;
        if raised & INT_TIMER != 0 {
            self.timer_interrupt_count += 1;
            self.event_log.push(DebugEvent::TimerInterrupt {
                frame: self.machine.clocks.frames,
                cycle: self.machine.clocks.cycles,
                tima: self.machine.timer.tima,
                tma: self.machine.timer.tma,
            });
        }
        if raised & INT_SERIAL != 0 {
            self.serial_interrupt_count += 1;
        }
        if raised & INT_JOYPAD != 0 {
            self.joypad_interrupt_count += 1;
        }
        for event in trace {
            match *event {
                InterruptTraceEvent::Requested { source, iflag } => {
                    self.event_log.push(DebugEvent::InterruptRequested {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        source: interrupt_source_name(source).to_string(),
                        iflag,
                    });
                }
                InterruptTraceEvent::Serviced { source, vector } => {
                    self.interrupt_service_count += 1;
                    self.event_log.push(DebugEvent::InterruptServiced {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        source: interrupt_source_name(source).to_string(),
                        vector,
                    });
                }
                InterruptTraceEvent::PendingBlocked {
                    pending_mask,
                    ime,
                    halted,
                } => {
                    self.interrupt_pending_blocked_count += 1;
                    self.event_log.push(DebugEvent::InterruptPendingBlocked {
                        frame: self.machine.clocks.frames,
                        cycle: self.machine.clocks.cycles,
                        pending_mask,
                        ime,
                        halted,
                    });
                }
            }
        }
        self.last_iflag = self.machine.interrupt.iflag;
    }

    fn capture_symbol_event(&mut self) {
        let current_symbol = self.current_pc_symbol();
        let current_source = self.current_source_label();
        if current_symbol != self.last_symbol_name || current_source != self.last_source_label {
            if let Some(symbol) = current_symbol.clone() {
                if let Some(intrinsic) = classify_symbol_name(&symbol) {
                    self.capture_intrinsic(intrinsic);
                }
            }
            self.push_execution_context(
                "symbol_or_source_change",
                self.current_code_bank(),
                self.machine.cpu.pc,
            );
            self.event_log.push(DebugEvent::SymbolContextChange {
                frame: self.machine.clocks.frames,
                cycle: self.machine.clocks.cycles,
                rom_bank: self.current_code_bank(),
                pc: self.machine.cpu.pc,
                symbol: current_symbol.clone(),
                source: current_source.clone(),
            });
            self.last_symbol_name = current_symbol;
            self.last_source_label = current_source;
        }
    }

    fn capture_intrinsic(&mut self, intrinsic: KitaqgbIntrinsicMatch) {
        let symbol = intrinsic.canonical_name.clone();
        let kind = intrinsic.kind.clone();
        let likely_visible_effect = intrinsic.has_likely_visual_effect();
        let flush_like = intrinsic.is_flush_like();

        self.intrinsic_count += 1;
        self.last_intrinsic = Some(symbol.clone());
        if flush_like {
            self.settile_flush_count += 1;
        }
        match kind {
            KitaqgbIntrinsicKind::SetTileBuffered => self.buffered_settile_count += 1,
            KitaqgbIntrinsicKind::SetTileCgb => self.cgb_settile_count += 1,
            KitaqgbIntrinsicKind::FlushRows => self.flush_rows_count += 1,
            KitaqgbIntrinsicKind::OamDma => self.oam_dma_count += 1,
            KitaqgbIntrinsicKind::WaitVBlank => {
                self.wait_vblank_count += 1;
                self.seen_wait_vblank = true;
            }
            KitaqgbIntrinsicKind::Present => {
                self.present_count += 1;
                if self.seen_wait_vblank {
                    self.wait_before_present = true;
                }
            }
            KitaqgbIntrinsicKind::Input => self.input_path_count += 1,
            KitaqgbIntrinsicKind::CgbPalette => self.cgb_palette_count += 1,
            KitaqgbIntrinsicKind::BankGuard => self.bank_guard_count += 1,
            KitaqgbIntrinsicKind::TrapCheck => self.trap_check_count += 1,
            KitaqgbIntrinsicKind::OamTransfer => self.oam_transfer_count += 1,
            _ => {}
        }
        self.event_log.push(DebugEvent::KitaqgbIntrinsic {
            frame: self.machine.clocks.frames,
            cycle: self.machine.clocks.cycles,
            symbol,
            intrinsic_kind: format!("{kind:?}"),
            likely_visible_effect,
            flush_like,
        });
    }

    fn symbol_name_at(&self, selected_bank: u16, pc: u16) -> Option<String> {
        let bank = Self::code_bank_for_pc(pc, selected_bank);
        self.symbol_table.as_ref().and_then(|table| {
            table
                .lookup(bank, pc)
                .or_else(|| table.lookup_nearest_before(bank, pc))
                .map(|symbol| symbol.name.clone())
        })
    }

    fn record_unsupported_opcode(&mut self, opcode: u8, pc: u16) {
        let rom_bank = Self::code_bank_for_pc(pc, self.machine.current_rom_bank());
        let symbol = self.symbol_name_at(rom_bank, pc);

        self.unsupported_opcode_count += 1;
        self.halted_on_unsupported_opcode = true;
        self.unsupported_opcodes
            .entry(opcode)
            .and_modify(|entry| {
                entry.count += 1;
                entry.last_pc = pc;
                entry.last_rom_bank = rom_bank;
                entry.last_symbol = symbol.clone();
            })
            .or_insert_with(|| UnsupportedOpcodeSummary {
                opcode,
                count: 1,
                last_pc: pc,
                last_rom_bank: rom_bank,
                last_symbol: symbol.clone(),
            });
        self.event_log.push(DebugEvent::UnsupportedOpcode {
            frame: self.machine.clocks.frames,
            cycle: self.machine.clocks.cycles,
            opcode,
            pc,
            rom_bank,
            symbol,
            source: self
                .source_location_at(rom_bank, pc)
                .as_ref()
                .map(|s| match s.column {
                    Some(col) => format!("{}:{}:{}", s.path, s.line, col),
                    None => format!("{}:{}", s.path, s.line),
                }),
        });
    }

    fn update_bank_thrash(
        &mut self,
        previous_bank: u16,
        current_bank: u16,
        compiler_thunk_transition: bool,
    ) {
        if compiler_thunk_transition {
            self.previous_bank_switch = None;
            self.bank_thrash_streak = 0;
            return;
        }

        let reversed = self
            .previous_bank_switch
            .is_some_and(|(last_from, last_to)| {
                last_from == current_bank && last_to == previous_bank
            });
        self.bank_thrash_streak = if reversed {
            self.bank_thrash_streak.saturating_add(1)
        } else {
            0
        };
        self.bank_thrash_score = self.bank_thrash_score.max(self.bank_thrash_streak);
        if self.bank_thrash_streak == 4 {
            self.event_log.push(DebugEvent::BankThrashSuspected {
                frame: self.machine.clocks.frames,
                cycle: self.machine.clocks.cycles,
                score: self.bank_thrash_streak,
                bank_a: previous_bank,
                bank_b: current_bank,
            });
        }
        self.previous_bank_switch = Some((previous_bank, current_bank));
    }

    fn finalize_bank_return_tracking(&mut self) {
        self.end_rom_bank = self.machine.current_rom_bank();
        let current_frame = self.machine.clocks.frames;
        let unresolved_index = self.pending_bank_returns.iter().position(|pending| {
            current_frame.saturating_sub(pending.started_frame) > BANK_RETURN_GRACE_FRAMES
        });
        self.bank_restored = unresolved_index.is_none();
        if let Some(index) = unresolved_index {
            let pending = &mut self.pending_bank_returns[index];
            if pending.reported {
                return;
            }
            pending.reported = true;
            let expected_bank = pending.expected_bank;
            self.event_log.push(DebugEvent::BankReturnMissing {
                frame: current_frame,
                cycle: self.machine.clocks.cycles,
                expected_bank,
                current_bank: self.end_rom_bank,
            });
        }
    }

    fn record_pc(&mut self, bank: u16, pc: u16) {
        if self.recent_pcs.len() == 64 {
            self.recent_pcs.pop_front();
        }
        self.recent_pcs.push_back((bank, pc));
        *self.pc_hit_histogram.entry((bank, pc)).or_insert(0) += 1;
        let repeated = self
            .recent_pcs
            .iter()
            .filter(|&&(b, v)| b == bank && v == pc)
            .count() as u32;
        self.max_repeated_pc_hits = self.max_repeated_pc_hits.max(repeated);
    }

    fn record_pc_cycles(&mut self, bank: u16, pc: u16, cycles: u32) {
        *self.pc_cycle_histogram.entry((bank, pc)).or_insert(0) += u64::from(cycles);
    }

    pub fn set_symbol_table(&mut self, symbol_table: SymbolTable) {
        self.symbol_table = Some(symbol_table);
        self.last_symbol_name = self.current_pc_symbol();
        self.last_source_label = self.current_source_label();
        self.push_execution_context(
            "symbol_table_loaded",
            self.current_code_bank(),
            self.machine.cpu.pc,
        );
    }

    pub fn set_toolchain_build_report(&mut self, report: ToolchainBuildReport) {
        self.toolchain_build_report = Some(report);
    }

    pub fn clear_symbol_table(&mut self) {
        self.symbol_table = None;
        self.last_symbol_name = None;
        self.last_source_label = None;
    }

    pub fn named_variable_addr(&self, name: &str) -> Option<u16> {
        self.symbol_table
            .as_ref()?
            .lookup_variable_by_name(name)
            .map(|variable| variable.address)
    }

    pub fn read_named_variable_u8(&self, name: &str) -> Option<u8> {
        let addr = self.named_variable_addr(name)?;
        Some(self.machine.peek8(addr))
    }

    pub fn link4_selected_peer_session(&self, session_count: usize) -> Option<usize> {
        let mode = self.read_named_variable_u8("Link4_ModeState")?;
        if mode != 1 {
            return None;
        }

        let selected_peer = usize::from(self.read_named_variable_u8("Link4_SelectedPeer")?);
        let slot_count = self
            .read_named_variable_u8("Link4_SlotCount")
            .map(usize::from)
            .unwrap_or(session_count);
        if selected_peer == 0 || selected_peer >= slot_count || selected_peer >= session_count {
            return None;
        }
        Some(selected_peer)
    }

    pub fn set_watch_windows(&mut self, watch_windows: Vec<MemoryWatchSpec>) {
        self.watch_windows = watch_windows;
        self.refresh_watch_window_baselines();
        self.refresh_stop_watchpoint_baselines();
    }

    pub fn clear_watch_windows(&mut self) {
        self.watch_windows.clear();
        self.watch_window_baselines.clear();
        self.watch_previous_frame_baselines.clear();
        self.watch_named_baselines.clear();
        self.watch_named_baseline_active = None;
    }

    pub fn watch_windows(&self) -> &[MemoryWatchSpec] {
        &self.watch_windows
    }

    pub fn set_watch_baseline_mode(
        &mut self,
        mode: MemoryWatchBaselineMode,
        named_baseline: Option<String>,
    ) -> Result<(), String> {
        if matches!(mode, MemoryWatchBaselineMode::Named) {
            let Some(name) = named_baseline
                .as_ref()
                .map(|name| name.trim())
                .filter(|name| !name.is_empty())
            else {
                return Err(
                    "named watch baseline mode requires a non-empty baseline name".to_string(),
                );
            };
            if !self.watch_named_baselines.contains_key(name) {
                return Err(format!("watch baseline '{name}' has not been captured"));
            }
        }
        self.watch_baseline_mode = mode;
        self.watch_named_baseline_active = named_baseline
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty());
        Ok(())
    }

    pub fn watch_baseline_mode(&self) -> MemoryWatchBaselineMode {
        self.watch_baseline_mode
    }

    pub fn active_watch_baseline_name(&self) -> Option<&str> {
        self.watch_named_baseline_active.as_deref()
    }

    pub fn capture_watch_baseline(&mut self, name: &str) -> Result<(), String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("watch baseline name must not be empty".to_string());
        }
        self.watch_named_baselines.insert(
            trimmed.to_string(),
            self.watch_windows
                .iter()
                .map(|spec| self.read_watch_window(spec.addr, spec.size))
                .collect(),
        );
        Ok(())
    }

    pub fn watch_baseline_names(&self) -> Vec<String> {
        self.watch_named_baselines.keys().cloned().collect()
    }

    pub fn set_stop_conditions(&mut self, stop_conditions: StopConditionSet) -> Result<(), String> {
        let validated = stop_conditions.validate()?;
        self.stop_conditions = validated;
        self.last_stop_reason = None;
        self.stopped_by_debugger = false;
        self.refresh_stop_watchpoint_baselines();
        Ok(())
    }

    pub fn clear_stop_conditions(&mut self) {
        self.stop_conditions = StopConditionSet::default();
        self.stop_watchpoint_baselines.clear();
        self.last_stop_reason = None;
        self.stopped_by_debugger = false;
    }

    pub fn stop_conditions(&self) -> &StopConditionSet {
        &self.stop_conditions
    }

    pub fn set_replay_control(&mut self, replay_control: ReplayControlSet) -> Result<(), String> {
        self.replay_control = replay_control.validate()?;
        self.replay_last_checkpoint_key = None;
        self.replay_last_rewind = None;
        self.replay_divergence = None;
        Ok(())
    }

    pub fn replay_control(&self) -> &ReplayControlSet {
        &self.replay_control
    }

    pub fn oldest_replay_checkpoint_frame(&self) -> Option<u64> {
        self.replay_checkpoints
            .front()
            .map(|checkpoint| checkpoint.frame)
    }

    pub fn rewind_frames(&mut self, frames_back: u64) -> bool {
        if !self.replay_control.enabled || frames_back == 0 || self.replay_checkpoints.is_empty() {
            return false;
        }
        let from_frame = self.machine.clocks.frames;
        let target_frame = from_frame.saturating_sub(frames_back);
        let checkpoint = self
            .replay_checkpoints
            .iter()
            .rev()
            .find(|checkpoint| checkpoint.frame <= target_frame)
            .cloned();
        let Some(checkpoint) = checkpoint else {
            return false;
        };
        checkpoint.state.apply_to(&mut self.machine);
        let now_frame_hash = compute_frame_hash(&self.machine);
        self.previous_frame_hash = Some(now_frame_hash);
        self.last_frame_hash = Some(now_frame_hash);
        self.previous_vram_hash = Some(compute_vram_hash(&self.machine));
        self.previous_oam_hash = Some(compute_oam_hash(&self.machine));
        self.previous_bg_hash = Some(compute_bg_hash(&self.machine));
        self.previous_window_hash = Some(compute_window_hash(&self.machine));
        self.previous_sprite_hash = Some(compute_sprite_hash(&self.machine));
        self.last_symbol_name = self.current_pc_symbol();
        self.last_source_label = self.current_source_label();
        self.last_ppu_mode = self.machine.ppu.current_mode();
        self.last_stat_coincidence = self.machine.ppu.stat_coincidence();
        self.refresh_watch_window_baselines();
        self.refresh_stop_watchpoint_baselines();
        self.replay_generation = self.replay_generation.saturating_add(1);
        self.replay_rewind_count = self.replay_rewind_count.saturating_add(1);
        self.replay_last_rewind = Some(ReplayRewindReport {
            requested_frames: frames_back,
            from_frame,
            to_frame: checkpoint.frame,
            checkpoint_index: checkpoint.checkpoint_index,
            generation: self.replay_generation,
        });
        self.event_log.push(DebugEvent::ReplayRewindApplied {
            frame: self.machine.clocks.frames,
            cycle: self.machine.clocks.cycles,
            requested_frames: frames_back,
            from_frame,
            to_frame: checkpoint.frame,
            checkpoint_index: checkpoint.checkpoint_index,
            generation: self.replay_generation,
        });
        true
    }

    pub fn stop_reason(&self) -> Option<&StopReason> {
        self.last_stop_reason.as_ref()
    }

    pub fn halted_on_unsupported_opcode(&self) -> bool {
        self.halted_on_unsupported_opcode
    }

    fn code_bank_for_pc(pc: u16, selected_bank: u16) -> u16 {
        if pc < 0x4000 {
            0
        } else {
            selected_bank
        }
    }

    fn current_code_bank(&self) -> u16 {
        Self::code_bank_for_pc(self.machine.cpu.pc, self.machine.current_rom_bank())
    }

    fn current_pc_symbol_info(&self) -> Option<&SymbolInfo> {
        self.symbol_table
            .as_ref()
            .and_then(|table| table.lookup(self.current_code_bank(), self.machine.cpu.pc))
    }

    fn current_pc_symbol(&self) -> Option<String> {
        self.current_pc_symbol_info().map(|s| s.name.clone())
    }

    fn previous_pc_symbol(&self) -> Option<String> {
        self.symbol_table.as_ref().and_then(|table| {
            self.recent_pcs
                .iter()
                .rev()
                .skip(1)
                .find_map(|&(bank, pc)| table.lookup(bank, pc).map(|s| s.name.clone()))
        })
    }

    fn nearest_symbol_name(&self) -> Option<String> {
        let bank = self.current_code_bank();
        self.symbol_table
            .as_ref()
            .and_then(|table| table.lookup_nearest_before(bank, self.machine.cpu.pc))
            .map(|s| s.name.clone())
    }

    fn next_symbol_name(&self) -> Option<String> {
        let bank = self.current_code_bank();
        self.symbol_table
            .as_ref()
            .and_then(|table| table.lookup_next_after(bank, self.machine.cpu.pc))
            .map(|s| s.name.clone())
    }

    fn source_location_at(&self, selected_bank: u16, pc: u16) -> Option<SourceLocationStop> {
        let bank = Self::code_bank_for_pc(pc, selected_bank);
        self.symbol_table
            .as_ref()
            .and_then(|table| table.lookup_source(bank, pc))
            .map(Self::source_location_stop_from_info)
    }

    fn source_location_stop_from_info(info: &SourceLocationInfo) -> SourceLocationStop {
        SourceLocationStop {
            path: info.path.clone(),
            line: info.line,
            column: info.column,
            bank: info.bank,
            addr: info.addr,
            symbol: info.symbol.clone(),
            section: info.section.clone(),
        }
    }

    fn source_location_report_from_stop(source: &SourceLocationStop) -> SourceLocationReport {
        SourceLocationReport {
            path: source.path.clone(),
            line: source.line,
            column: source.column,
            bank: source.bank,
            addr: source.addr,
            symbol: source.symbol.clone(),
            section: source.section.clone(),
        }
    }

    fn current_source_location(&self) -> Option<SourceLocationStop> {
        self.source_location_at(self.current_code_bank(), self.machine.cpu.pc)
    }

    fn previous_source_location(&self) -> Option<SourceLocationStop> {
        self.recent_pcs
            .iter()
            .rev()
            .skip(1)
            .find_map(|&(bank, pc)| self.source_location_at(bank, pc))
    }

    fn next_source_location(&self) -> Option<SourceLocationStop> {
        let bank = self.current_code_bank();
        self.symbol_table
            .as_ref()
            .and_then(|table| table.lookup_source_next_after(bank, self.machine.cpu.pc))
            .map(Self::source_location_stop_from_info)
    }

    fn current_source_label(&self) -> Option<String> {
        self.current_source_location().map(|s| match s.column {
            Some(col) => format!("{}:{}:{}", s.path, s.line, col),
            None => format!("{}:{}", s.path, s.line),
        })
    }

    fn current_section(&self) -> Option<String> {
        self.current_source_location()
            .and_then(|source| source.section)
            .or_else(|| {
                self.current_function_info()
                    .and_then(|function| function.section.clone())
            })
            .or_else(|| {
                self.current_pc_symbol_info()
                    .and_then(|symbol| symbol.section.clone())
            })
    }

    fn current_function_info(&self) -> Option<&FunctionInfo> {
        let bank = self.current_code_bank();
        self.symbol_table
            .as_ref()
            .and_then(|table| table.lookup_function(bank, self.machine.cpu.pc))
    }

    fn current_static_estimate_info(&self) -> Option<&StaticEstimateInfo> {
        let bank = self.current_code_bank();
        self.symbol_table
            .as_ref()
            .and_then(|table| table.lookup_static_estimate(bank, self.machine.cpu.pc))
    }

    fn function_info_at(&self, selected_bank: u16, pc: u16) -> Option<&FunctionInfo> {
        let bank = Self::code_bank_for_pc(pc, selected_bank);
        self.symbol_table
            .as_ref()
            .and_then(|table| table.lookup_function(bank, pc))
    }

    fn static_estimate_at(&self, selected_bank: u16, pc: u16) -> Option<&StaticEstimateInfo> {
        let bank = Self::code_bank_for_pc(pc, selected_bank);
        self.symbol_table
            .as_ref()
            .and_then(|table| table.lookup_static_estimate(bank, pc))
    }

    fn build_hotspot_score_for_function(&self, name: &str) -> Option<u32> {
        self.toolchain_build_report
            .as_ref()
            .and_then(|report| report.hotspots.iter().find(|row| row.name == name))
            .map(|row| row.score)
    }

    fn toolchain_function_report(info: &FunctionInfo) -> ToolchainFunctionReport {
        ToolchainFunctionReport {
            name: info.name.clone(),
            bank: info.bank,
            start: info.start,
            end: info.end,
            size_bytes: info.size_bytes,
            section: info.section.clone(),
            is_stack_call: info.is_stack_call,
            is_fast_call: info.is_fast_call,
            has_fixed_bank: info.has_fixed_bank,
            has_fixed_order: info.has_fixed_order,
            return_size: info.return_size,
            param_sizes: info.param_sizes.clone(),
        }
    }

    fn toolchain_static_estimate_report(
        info: &StaticEstimateInfo,
    ) -> ToolchainStaticEstimateReport {
        ToolchainStaticEstimateReport {
            name: info.name.clone(),
            bank: info.bank,
            size_bytes: info.size_bytes,
            incoming_call_count: info.incoming_call_count,
            outgoing_call_count: info.outgoing_call_count,
            cross_bank_outgoing_count: info.cross_bank_outgoing_count,
            far_call_count: info.far_call_count,
            is_stack_call: info.is_stack_call,
            is_fast_call: info.is_fast_call,
            return_size: info.return_size,
            param_sizes: info.param_sizes.clone(),
        }
    }

    fn push_execution_context(&mut self, reason: impl Into<String>, bank: u16, pc: u16) {
        if self.execution_context_trail.len() == 16 {
            self.execution_context_trail.pop_front();
        }
        self.execution_context_trail
            .push_back(self.execution_context_frame(reason.into(), bank, pc));
    }

    fn execution_context_trail(&self) -> Vec<ExecutionContextFrame> {
        self.execution_context_trail.iter().cloned().collect()
    }

    fn execution_context_frame(&self, reason: String, bank: u16, pc: u16) -> ExecutionContextFrame {
        let cpu = &self.machine.cpu;
        ExecutionContextFrame {
            reason,
            rom_bank: bank,
            pc,
            sp: cpu.sp,
            a: cpu.a,
            b: cpu.b,
            c: cpu.c,
            d: cpu.d,
            e: cpu.e,
            bc: (u16::from(cpu.b) << 8) | u16::from(cpu.c),
            de: (u16::from(cpu.d) << 8) | u16::from(cpu.e),
            hl: cpu.hl(),
            stack_preview: self.read_watch_window(cpu.sp, 6),
            symbol: self.symbol_name_at(bank, pc),
            source: self.source_location_at(bank, pc),
        }
    }

    fn replay_checkpoint_report(&self, checkpoint: &ReplayCheckpoint) -> ReplayCheckpointReport {
        ReplayCheckpointReport {
            checkpoint_index: checkpoint.checkpoint_index,
            generation: checkpoint.generation,
            frame: checkpoint.frame,
            cycle: checkpoint.cycle,
            rom_bank: checkpoint.rom_bank,
            pc: checkpoint.pc,
            frame_hash: checkpoint.frame_hash,
            digest: checkpoint.digest,
            symbol: checkpoint.symbol.clone(),
            source: checkpoint
                .source
                .as_ref()
                .map(Self::source_location_report_from_stop),
            watch_hashes: checkpoint.watch_hashes.clone(),
            watch_digest: checkpoint.watch_digest,
        }
    }

    fn replay_slice_digest(slice: &ReplaySliceReport) -> u64 {
        let mut acc = 0xcbf29ce484222325u64;
        for value in [
            slice.from_checkpoint_index.unwrap_or(u64::MAX),
            slice.to_checkpoint_index,
            u64::from(slice.generation),
            slice.start_frame,
            slice.end_frame,
            slice.start_cycle,
            slice.end_cycle,
            u64::from(slice.start_rom_bank),
            u64::from(slice.start_pc),
            u64::from(slice.end_rom_bank),
            u64::from(slice.end_pc),
            slice.instruction_samples,
            slice.cpu_cycles,
            slice.event_count,
            slice.bank_switch_count,
            slice.far_call_count,
            slice.joypad_read_count,
            slice.dma_event_count,
            slice.interrupt_event_count,
            slice.render_event_count,
            slice.mmio_event_count,
        ] {
            acc = acc.rotate_left(5) ^ value.wrapping_mul(0x100000001b3);
        }
        if let Some(symbol) = &slice.start_symbol {
            acc ^= u64::from(hash_bytes(symbol.as_bytes()));
        }
        if let Some(symbol) = &slice.end_symbol {
            acc ^= u64::from(hash_bytes(symbol.as_bytes())).rotate_left(7);
        }
        if let Some(source) = &slice.start_source {
            acc ^= u64::from(hash_bytes(source.path.as_bytes())).rotate_left(11);
            acc ^= (u64::from(source.line) << 8) ^ u64::from(source.addr);
        }
        if let Some(source) = &slice.end_source {
            acc ^= u64::from(hash_bytes(source.path.as_bytes())).rotate_left(13);
            acc ^= (u64::from(source.line) << 10) ^ u64::from(source.addr);
        }
        acc
    }

    fn build_replay_slices(&self) -> Vec<ReplaySliceReport> {
        let mut slices = Vec::new();
        let mut previous_checkpoint_index = None;
        let mut previous_event_log_len = 0usize;
        let mut previous_instruction_samples_total = 0u64;
        let mut start_frame = self.start_frame_counter;
        let mut start_cycle = self.start_cycle_counter;
        let mut start_rom_bank = self.start_rom_bank;
        let mut start_pc = self.start_pc;
        let mut start_symbol = self.symbol_name_at(start_rom_bank, start_pc);
        let mut start_source = self
            .source_location_at(start_rom_bank, start_pc)
            .as_ref()
            .map(Self::source_location_report_from_stop);

        for checkpoint in &self.replay_checkpoints {
            let event_end = checkpoint.event_log_len.min(self.event_log.len());
            let events = &self.event_log[previous_event_log_len..event_end];
            let mut bank_switch_count = 0u64;
            let mut far_call_count = 0u64;
            let mut joypad_read_count = 0u64;
            let mut dma_event_count = 0u64;
            let mut interrupt_event_count = 0u64;
            let mut render_event_count = 0u64;
            let mut mmio_event_count = 0u64;

            for event in events {
                match event {
                    DebugEvent::BankSwitch { .. } => bank_switch_count += 1,
                    DebugEvent::FarCallSuspected { .. } => far_call_count += 1,
                    DebugEvent::JoypadRead { .. } => joypad_read_count += 1,
                    DebugEvent::OamDmaStart { .. }
                    | DebugEvent::OamDmaComplete { .. }
                    | DebugEvent::GdmaStallEstimate { .. }
                    | DebugEvent::HdmaStart { .. }
                    | DebugEvent::HdmaBlock { .. }
                    | DebugEvent::HdmaComplete { .. }
                    | DebugEvent::HdmaCancel { .. }
                    | DebugEvent::HdmaDeferred { .. }
                    | DebugEvent::HdmaWriteIgnored { .. } => dma_event_count += 1,
                    DebugEvent::TimerInterrupt { .. }
                    | DebugEvent::JoypadInterrupt { .. }
                    | DebugEvent::InterruptRequested { .. }
                    | DebugEvent::InterruptServiced { .. }
                    | DebugEvent::InterruptPendingBlocked { .. } => interrupt_event_count += 1,
                    DebugEvent::ScanlineAdvance { .. }
                    | DebugEvent::PpuModeChange { .. }
                    | DebugEvent::ScanlineRender { .. }
                    | DebugEvent::LcdToggle { .. }
                    | DebugEvent::StatWrite { .. }
                    | DebugEvent::LycWrite { .. }
                    | DebugEvent::StatSignal { .. }
                    | DebugEvent::VblankEnter { .. }
                    | DebugEvent::FrameComplete { .. } => render_event_count += 1,
                    DebugEvent::MapperControlWrite { .. }
                    | DebugEvent::MapperRomBankChange { .. }
                    | DebugEvent::MapperRamBankChange { .. }
                    | DebugEvent::TimerControlWrite { .. }
                    | DebugEvent::DivResetEdge { .. }
                    | DebugEvent::JoypadSelectionWrite { .. }
                    | DebugEvent::CgbVramBankSwitch { .. }
                    | DebugEvent::CgbWramBankSwitch { .. }
                    | DebugEvent::CgbBgPaletteIndexWrite { .. }
                    | DebugEvent::CgbBgPaletteDataWrite { .. }
                    | DebugEvent::CgbObjPaletteIndexWrite { .. }
                    | DebugEvent::CgbObjPaletteDataWrite { .. }
                    | DebugEvent::CgbKey1Write { .. }
                    | DebugEvent::ApuMasterToggle { .. }
                    | DebugEvent::ApuChannelTrigger { .. }
                    | DebugEvent::ApuMixerControl { .. }
                    | DebugEvent::ApuWaveRamWrite { .. } => mmio_event_count += 1,
                    _ => {}
                }
            }

            let mut slice = ReplaySliceReport {
                from_checkpoint_index: previous_checkpoint_index,
                to_checkpoint_index: checkpoint.checkpoint_index,
                generation: checkpoint.generation,
                start_frame,
                end_frame: checkpoint.frame,
                start_cycle,
                end_cycle: checkpoint.cycle,
                start_rom_bank,
                start_pc,
                start_symbol: start_symbol.clone(),
                start_source: start_source.clone(),
                end_rom_bank: checkpoint.rom_bank,
                end_pc: checkpoint.pc,
                end_symbol: checkpoint.symbol.clone(),
                end_source: checkpoint
                    .source
                    .as_ref()
                    .map(Self::source_location_report_from_stop),
                instruction_samples: checkpoint
                    .instruction_samples_total
                    .saturating_sub(previous_instruction_samples_total),
                cpu_cycles: checkpoint.cycle.saturating_sub(start_cycle),
                event_count: events.len() as u64,
                bank_switch_count,
                far_call_count,
                joypad_read_count,
                dma_event_count,
                interrupt_event_count,
                render_event_count,
                mmio_event_count,
                slice_digest: 0,
            };
            slice.slice_digest = Self::replay_slice_digest(&slice);
            slices.push(slice);

            previous_checkpoint_index = Some(checkpoint.checkpoint_index);
            previous_event_log_len = checkpoint.event_log_len;
            previous_instruction_samples_total = checkpoint.instruction_samples_total;
            start_frame = checkpoint.frame;
            start_cycle = checkpoint.cycle;
            start_rom_bank = checkpoint.rom_bank;
            start_pc = checkpoint.pc;
            start_symbol = checkpoint.symbol.clone();
            start_source = checkpoint
                .source
                .as_ref()
                .map(Self::source_location_report_from_stop);
        }

        slices
    }

    fn build_profiler(&self) -> Option<ProfilerReport> {
        #[derive(Default)]
        struct FunctionAcc {
            samples: u64,
            cycles: u64,
            unique_pcs: BTreeSet<u16>,
            hottest_pc: u16,
            hottest_samples: u64,
            section: Option<String>,
            source: Option<SourceLocationReport>,
            static_estimate: Option<ToolchainStaticEstimateReport>,
            build_hotspot_score: Option<u32>,
        }

        #[derive(Default)]
        struct BankAcc {
            samples: u64,
            cycles: u64,
            unique_pcs: BTreeSet<u16>,
            switch_in_count: u64,
            switch_out_count: u64,
            far_call_count: u64,
            top_functions: BTreeMap<String, u64>,
        }

        #[derive(Default)]
        struct TransitionAcc {
            count: u64,
            far_call_count: u64,
        }

        let mut function_acc: BTreeMap<(u16, String), FunctionAcc> = BTreeMap::new();
        let mut bank_acc: BTreeMap<u16, BankAcc> = BTreeMap::new();

        for (&(bank, pc), &samples) in &self.pc_hit_histogram {
            let cycles = self
                .pc_cycle_histogram
                .get(&(bank, pc))
                .copied()
                .unwrap_or(0);
            let function_info = self.function_info_at(bank, pc);
            let function_name = function_info
                .map(|info| info.name.clone())
                .or_else(|| self.symbol_name_at(bank, pc))
                .unwrap_or_else(|| format!("bank{:02X}:{:04X}", bank, pc));
            let function_bank = function_info.map(|info| info.bank).unwrap_or(bank);

            let entry = function_acc
                .entry((function_bank, function_name.clone()))
                .or_default();
            entry.samples += samples;
            entry.cycles += cycles;
            entry.unique_pcs.insert(pc);
            if samples > entry.hottest_samples {
                entry.hottest_samples = samples;
                entry.hottest_pc = pc;
            }
            if entry.section.is_none() {
                entry.section = function_info.and_then(|info| info.section.clone());
            }
            if entry.source.is_none() {
                entry.source = function_info
                    .and_then(|info| self.source_location_at(info.bank, info.start))
                    .as_ref()
                    .map(Self::source_location_report_from_stop)
                    .or_else(|| {
                        self.source_location_at(bank, pc)
                            .as_ref()
                            .map(Self::source_location_report_from_stop)
                    });
            }
            if entry.static_estimate.is_none() {
                entry.static_estimate = self
                    .static_estimate_at(bank, pc)
                    .map(Self::toolchain_static_estimate_report);
            }
            if entry.build_hotspot_score.is_none() {
                entry.build_hotspot_score = self.build_hotspot_score_for_function(&function_name);
            }

            let bank_entry = bank_acc.entry(bank).or_default();
            bank_entry.samples += samples;
            bank_entry.cycles += cycles;
            bank_entry.unique_pcs.insert(pc);
            *bank_entry.top_functions.entry(function_name).or_insert(0) += samples;
        }

        let mut transition_acc: BTreeMap<
            (u16, u16, Option<String>, Option<String>),
            TransitionAcc,
        > = BTreeMap::new();
        for event in &self.event_log {
            match event {
                DebugEvent::BankSwitch {
                    from,
                    to,
                    from_symbol,
                    to_symbol,
                    ..
                } => {
                    bank_acc.entry(*from).or_default().switch_out_count += 1;
                    bank_acc.entry(*to).or_default().switch_in_count += 1;
                    transition_acc
                        .entry((*from, *to, from_symbol.clone(), to_symbol.clone()))
                        .or_default()
                        .count += 1;
                }
                DebugEvent::FarCallSuspected {
                    from_bank,
                    to_bank,
                    from_symbol,
                    to_symbol,
                    ..
                } => {
                    bank_acc.entry(*to_bank).or_default().far_call_count += 1;
                    transition_acc
                        .entry((*from_bank, *to_bank, from_symbol.clone(), to_symbol.clone()))
                        .or_default()
                        .far_call_count += 1;
                }
                _ => {}
            }
        }

        let mut function_activity: Vec<_> = function_acc
            .into_iter()
            .map(|((bank, name), item)| ProfilerFunctionActivityReport {
                name,
                bank,
                section: item.section,
                samples: item.samples,
                cycles: item.cycles,
                unique_pcs: item.unique_pcs.len() as u32,
                hottest_pc: item.hottest_pc,
                last_pc: item.hottest_pc,
                source: item.source,
                static_estimate: item.static_estimate,
                build_hotspot_score: item.build_hotspot_score,
            })
            .collect();
        function_activity.sort_by(|a, b| {
            b.cycles
                .cmp(&a.cycles)
                .then_with(|| b.samples.cmp(&a.samples))
                .then_with(|| a.bank.cmp(&b.bank))
                .then_with(|| a.name.cmp(&b.name))
        });
        function_activity.truncate(16);

        let mut bank_activity: Vec<_> = bank_acc
            .into_iter()
            .map(|(bank, item)| {
                let mut top_functions: Vec<_> = item.top_functions.into_iter().collect();
                top_functions.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
                top_functions.truncate(4);
                ProfilerBankActivityReport {
                    bank,
                    samples: item.samples,
                    cycles: item.cycles,
                    unique_pcs: item.unique_pcs.len() as u32,
                    switch_in_count: item.switch_in_count,
                    switch_out_count: item.switch_out_count,
                    far_call_count: item.far_call_count,
                    top_functions: top_functions.into_iter().map(|(name, _)| name).collect(),
                }
            })
            .collect();
        bank_activity.sort_by(|a, b| {
            b.cycles
                .cmp(&a.cycles)
                .then_with(|| b.samples.cmp(&a.samples))
                .then_with(|| a.bank.cmp(&b.bank))
        });
        bank_activity.truncate(16);

        let mut bank_transitions: Vec<_> = transition_acc
            .into_iter()
            .map(|((from_bank, to_bank, from_symbol, to_symbol), item)| {
                ProfilerBankTransitionReport {
                    from_bank,
                    to_bank,
                    count: item.count,
                    far_call_count: item.far_call_count,
                    from_symbol,
                    to_symbol,
                }
            })
            .collect();
        bank_transitions.sort_by(|a, b| {
            b.far_call_count
                .cmp(&a.far_call_count)
                .then_with(|| b.count.cmp(&a.count))
                .then_with(|| a.from_bank.cmp(&b.from_bank))
                .then_with(|| a.to_bank.cmp(&b.to_bank))
        });
        bank_transitions.truncate(16);

        let generated = !function_activity.is_empty()
            || !bank_activity.is_empty()
            || !bank_transitions.is_empty();
        generated.then_some(ProfilerReport {
            generated,
            sample_unit: "instruction_samples".to_string(),
            function_activity,
            bank_activity,
            bank_transitions,
            carry_forward_notes: vec![
                "Profiler values are sampled from executed instruction PCs plus per-step CPU cycles, not full hardware bus accounting.".to_string(),
                "Function grouping prefers toolchain function ranges when metadata is available; otherwise it falls back to symbol names or raw bank:pc labels.".to_string(),
            ],
        })
    }

    fn build_heatmap(&self, profiler: Option<&ProfilerReport>) -> Option<HeatmapReport> {
        #[derive(Default)]
        struct BucketAcc {
            count: u64,
            detail: String,
        }

        fn record_bucket(
            map: &mut BTreeMap<(String, u16, u16), BucketAcc>,
            region: &str,
            start: u16,
            end: u16,
            detail: String,
        ) {
            let entry = map.entry((region.to_string(), start, end)).or_default();
            entry.count += 1;
            entry.detail = detail;
        }

        fn record_mmio(
            map: &mut BTreeMap<(String, u16, u16), BucketAcc>,
            addr: u16,
            detail: String,
        ) {
            record_bucket(map, "io", addr, addr, detail);
        }

        fn mapper_window(addr: u16) -> (u16, u16) {
            let start = addr & 0xE000;
            let end = start.saturating_add(0x1FFF);
            (start, end)
        }

        fn page_window(addr: u16) -> (u16, u16) {
            let start = addr & 0xFF00;
            let end = start.saturating_add(0x00FF);
            (start, end)
        }

        let mut buckets: BTreeMap<(String, u16, u16), BucketAcc> = BTreeMap::new();
        for event in &self.event_log {
            match event {
                DebugEvent::JoypadRead { .. }
                | DebugEvent::JoypadSelectionWrite { .. }
                | DebugEvent::JoypadInterrupt { .. } => {
                    record_mmio(&mut buckets, 0xFF00, "joypad".to_string());
                }
                DebugEvent::DivResetEdge { .. }
                | DebugEvent::TimerOverflow { .. }
                | DebugEvent::TimerReload { .. }
                | DebugEvent::TimerInterrupt { .. } => {
                    record_bucket(
                        &mut buckets,
                        "io",
                        0xFF04,
                        0xFF07,
                        "timer block".to_string(),
                    );
                }
                DebugEvent::TimerControlWrite { .. } => {
                    record_mmio(&mut buckets, 0xFF07, "timer control".to_string());
                }
                DebugEvent::LcdToggle { .. }
                | DebugEvent::StatWrite { .. }
                | DebugEvent::LycWrite { .. }
                | DebugEvent::StatSignal { .. } => {
                    record_bucket(
                        &mut buckets,
                        "io",
                        0xFF40,
                        0xFF45,
                        "lcd/stat block".to_string(),
                    );
                }
                DebugEvent::OamDmaStart { source, .. }
                | DebugEvent::OamDmaComplete { source, .. } => {
                    record_mmio(&mut buckets, 0xFF46, "oam dma control".to_string());
                    let (start, end) = page_window(*source);
                    record_bucket(
                        &mut buckets,
                        "dma_source",
                        start,
                        end,
                        format!("source page around {:#06X}", source),
                    );
                }
                DebugEvent::HdmaStart { source, dest, .. }
                | DebugEvent::HdmaBlock { source, dest, .. }
                | DebugEvent::HdmaComplete { source, dest, .. }
                | DebugEvent::HdmaCancel { source, dest, .. } => {
                    record_bucket(
                        &mut buckets,
                        "io",
                        0xFF51,
                        0xFF55,
                        "hdma/gdma control".to_string(),
                    );
                    let (src_start, src_end) = page_window(*source);
                    record_bucket(
                        &mut buckets,
                        "dma_source",
                        src_start,
                        src_end,
                        format!("source page around {:#06X}", source),
                    );
                    let (dst_start, dst_end) = page_window(*dest);
                    record_bucket(
                        &mut buckets,
                        "dma_dest",
                        dst_start,
                        dst_end,
                        format!("dest page around {:#06X}", dest),
                    );
                }
                DebugEvent::HdmaDeferred { .. } | DebugEvent::HdmaWriteIgnored { .. } => {
                    record_bucket(
                        &mut buckets,
                        "io",
                        0xFF51,
                        0xFF55,
                        "hdma/gdma control".to_string(),
                    );
                }
                DebugEvent::MapperControlWrite { addr, .. }
                | DebugEvent::MapperRomBankChange { addr, .. }
                | DebugEvent::MapperRamBankChange { addr, .. } => {
                    let (start, end) = mapper_window(*addr);
                    record_bucket(
                        &mut buckets,
                        "mapper_ctrl",
                        start,
                        end,
                        format!("mapper control window around {:#06X}", addr),
                    );
                }
                DebugEvent::CgbBgPaletteIndexWrite { .. }
                | DebugEvent::CgbBgPaletteDataWrite { .. } => {
                    record_bucket(
                        &mut buckets,
                        "io",
                        0xFF68,
                        0xFF69,
                        "cgb bg palette".to_string(),
                    );
                }
                DebugEvent::CgbObjPaletteIndexWrite { .. }
                | DebugEvent::CgbObjPaletteDataWrite { .. } => {
                    record_bucket(
                        &mut buckets,
                        "io",
                        0xFF6A,
                        0xFF6B,
                        "cgb obj palette".to_string(),
                    );
                }
                DebugEvent::CgbKey1Write { .. } | DebugEvent::CgbSpeedSwitch { .. } => {
                    record_mmio(&mut buckets, 0xFF4D, "cgb speed switch".to_string());
                }
                DebugEvent::CgbVramBankSwitch { .. } => {
                    record_mmio(&mut buckets, 0xFF4F, "cgb vram bank".to_string());
                }
                DebugEvent::CgbWramBankSwitch { .. } => {
                    record_mmio(&mut buckets, 0xFF70, "cgb wram bank".to_string());
                }
                DebugEvent::ApuChannelTrigger { reg, .. }
                | DebugEvent::ApuChannelDisabled { reg, .. }
                | DebugEvent::ApuDacStateChange { reg, .. }
                | DebugEvent::ApuPopRisk { reg, .. } => {
                    record_mmio(&mut buckets, *reg, "apu register".to_string());
                }
                DebugEvent::ApuMixerControl { .. } => {
                    record_bucket(&mut buckets, "io", 0xFF24, 0xFF26, "apu mixer".to_string());
                }
                DebugEvent::SerialTransferStart { .. }
                | DebugEvent::SerialTransferComplete { .. } => {
                    record_bucket(
                        &mut buckets,
                        "io",
                        0xFF01,
                        0xFF02,
                        "serial block".to_string(),
                    );
                }
                _ => {}
            }
        }

        let mut region_totals: BTreeMap<String, u64> = BTreeMap::new();
        for ((region, _, _), item) in &buckets {
            *region_totals.entry(region.clone()).or_insert(0) += item.count;
        }

        let mut bucket_reports: Vec<_> = buckets
            .into_iter()
            .map(|((region, start, end), item)| HeatmapBucketReport {
                region,
                start,
                end,
                count: item.count,
                detail: item.detail,
            })
            .collect();
        bucket_reports.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| a.region.cmp(&b.region))
                .then_with(|| a.start.cmp(&b.start))
        });
        bucket_reports.truncate(24);

        let mut region_total_reports: Vec<_> = region_totals
            .into_iter()
            .map(|(region, count)| HeatmapRegionTotalReport { region, count })
            .collect();
        region_total_reports
            .sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.region.cmp(&b.region)));

        let culprit_functions = profiler
            .map(|report| {
                report
                    .function_activity
                    .iter()
                    .take(8)
                    .map(|item| HeatmapCulpritFunctionReport {
                        name: item.name.clone(),
                        bank: item.bank,
                        samples: item.samples,
                        cycles: item.cycles,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let culprit_banks = profiler
            .map(|report| {
                report
                    .bank_activity
                    .iter()
                    .take(8)
                    .map(|item| HeatmapCulpritBankReport {
                        bank: item.bank,
                        samples: item.samples,
                        cycles: item.cycles,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let generated = !bucket_reports.is_empty()
            || !culprit_functions.is_empty()
            || !culprit_banks.is_empty();
        generated.then_some(HeatmapReport {
            generated,
            buckets: bucket_reports,
            region_totals: region_total_reports,
            culprit_functions,
            culprit_banks,
            carry_forward_notes: vec![
                "Heatmap buckets are event-derived coarse ranges, intended for triage rather than exact bus traces.".to_string(),
                "Use the top culprit functions/banks together with watch windows and stop conditions to zoom in on a narrower failing path.".to_string(),
            ],
        })
    }

    fn classify_stack_region(addr: u16) -> &'static str {
        match addr {
            0x0000..=0x7FFF => "rom",
            0x8000..=0x9FFF => "vram",
            0xA000..=0xBFFF => "cart_ram",
            0xC000..=0xDFFF => "wram",
            0xE000..=0xFDFF => "echo_ram",
            0xFE00..=0xFE9F => "oam",
            0xFEA0..=0xFEFF => "unusable",
            0xFF00..=0xFF7F => "io",
            0xFF80..=0xFFFE => "hram",
            0xFFFF => "ie",
        }
    }

    fn build_abi_register_snapshot(&self) -> AbiRegisterSnapshotReport {
        let cpu = &self.machine.cpu;
        AbiRegisterSnapshotReport {
            a: cpu.a,
            b: cpu.b,
            c: cpu.c,
            d: cpu.d,
            e: cpu.e,
            h: cpu.h,
            l: cpu.l,
            hl: cpu.hl(),
            sp: cpu.sp,
            pc: cpu.pc,
        }
    }

    fn build_abi_stack_window(
        &self,
        current_function_contract: Option<&ToolchainFunctionReport>,
    ) -> AbiStackWindowReport {
        let sp = self.machine.cpu.sp;
        let preview_len = 12u16;
        let preview_bytes = self.read_watch_window(sp, preview_len);
        let expected_stack_argument_bytes = current_function_contract
            .map(|function| {
                function
                    .param_sizes
                    .iter()
                    .map(|&size| u16::from(size))
                    .sum::<u16>()
            })
            .unwrap_or(0);
        let expected_return_size = current_function_contract
            .map(|function| function.return_size)
            .unwrap_or(0);
        let decoded_return_address = if preview_bytes.len() >= 2 {
            Some(u16::from(preview_bytes[0]) | (u16::from(preview_bytes[1]) << 8))
        } else {
            None
        };
        let decoded_return_region = decoded_return_address
            .map(Self::classify_stack_region)
            .map(str::to_string);
        let argument_base = current_function_contract
            .and_then(|function| function.is_stack_call.then(|| sp.saturating_add(2)));
        let argument_preview_cap = 8u16;
        let argument_preview_bytes = argument_base
            .map(|base| {
                self.read_watch_window(
                    base,
                    expected_stack_argument_bytes.min(argument_preview_cap),
                )
            })
            .unwrap_or_default();
        let preview_truncated = expected_stack_argument_bytes
            .checked_add(2)
            .map(|needed| needed > preview_len)
            .unwrap_or(false);
        let argument_preview_truncated = expected_stack_argument_bytes > argument_preview_cap;

        AbiStackWindowReport {
            sp,
            region: Self::classify_stack_region(sp).to_string(),
            preview_bytes,
            preview_truncated,
            decoded_return_address,
            decoded_return_region,
            expected_stack_argument_bytes,
            expected_return_size,
            argument_base,
            argument_preview_bytes,
            argument_preview_truncated,
        }
    }

    fn recent_abi_boundary_context(
        &self,
        current_function: &ToolchainFunctionReport,
    ) -> (Option<ExecutionContextFrame>, Option<ExecutionContextFrame>) {
        let mut callee_entry = self.execution_context_trail.iter().rev().find_map(|frame| {
            self.function_info_at(frame.rom_bank, frame.pc)
                .and_then(|info| {
                    (info.name == current_function.name && info.bank == current_function.bank)
                        .then(|| frame.clone())
                })
        });

        if callee_entry.is_none() {
            callee_entry = Some(self.execution_context_frame(
                "current_state".to_string(),
                self.current_code_bank(),
                self.machine.cpu.pc,
            ));
        }

        let caller_context = callee_entry.as_ref().and_then(|entry| {
            self.execution_context_trail.iter().rev().find_map(|frame| {
                if frame.pc == entry.pc && frame.rom_bank == entry.rom_bank {
                    return None;
                }
                self.function_info_at(frame.rom_bank, frame.pc)
                    .and_then(|info| {
                        (info.name != current_function.name || info.bank != current_function.bank)
                            .then(|| frame.clone())
                    })
            })
        });

        (caller_context, callee_entry)
    }

    fn abi_contract_rule(
        register: &str,
        role: &str,
        confidence: &str,
        detail: impl Into<String>,
    ) -> AbiRegisterContractRuleReport {
        AbiRegisterContractRuleReport {
            register: register.to_string(),
            role: role.to_string(),
            confidence: confidence.to_string(),
            detail: detail.into(),
        }
    }

    fn execution_context_register_value(
        frame: &ExecutionContextFrame,
        register: &str,
    ) -> Option<u16> {
        match register {
            "A" => Some(u16::from(frame.a)),
            "BC" => Some(frame.bc),
            "DE" => Some(frame.de),
            "HL" => Some(frame.hl),
            "SP" => Some(frame.sp),
            _ => None,
        }
    }

    fn intrinsic_contract_strength(intrinsic_kind: &KitaqgbIntrinsicKind) -> &'static str {
        match intrinsic_kind {
            KitaqgbIntrinsicKind::Bank => "preserve_return_carriers",
            KitaqgbIntrinsicKind::FarMemcpy
            | KitaqgbIntrinsicKind::Memcpy
            | KitaqgbIntrinsicKind::Memset => "memory_transport_helper",
            KitaqgbIntrinsicKind::SetTile8
            | KitaqgbIntrinsicKind::SetTile16
            | KitaqgbIntrinsicKind::SetTileBuffered
            | KitaqgbIntrinsicKind::SetTileCgb
            | KitaqgbIntrinsicKind::SetTileFlush
            | KitaqgbIntrinsicKind::SetTile16Flush
            | KitaqgbIntrinsicKind::FlushRows
            | KitaqgbIntrinsicKind::OamTransfer
            | KitaqgbIntrinsicKind::CgbPalette
            | KitaqgbIntrinsicKind::OamDma
            | KitaqgbIntrinsicKind::Vram => "video_pipeline_helper",
            KitaqgbIntrinsicKind::Music => "apu_side_effect_helper",
            KitaqgbIntrinsicKind::WaitVBlank
            | KitaqgbIntrinsicKind::Present
            | KitaqgbIntrinsicKind::Input
            | KitaqgbIntrinsicKind::BankGuard
            | KitaqgbIntrinsicKind::TrapCheck => "stack_safety_observation",
        }
    }

    fn build_call_boundary_contract_rules(
        &self,
        current_function: &ToolchainFunctionReport,
        cross_bank_edge: Option<&crate::report::ToolchainCrossBankEdgeReport>,
        cross_bank: bool,
    ) -> Vec<AbiRegisterContractRuleReport> {
        let mut rules = Vec::new();
        if current_function.is_fast_call {
            match current_function.param_sizes.first().copied().unwrap_or(0) {
                1 => rules.push(Self::abi_contract_rule(
                    "A",
                    "entry_carrier",
                    "toolchain_metadata",
                    "Current function metadata marks arg0 as a fastcall u8, so A should carry the value at callee entry.",
                )),
                2 => rules.push(Self::abi_contract_rule(
                    "HL",
                    "entry_carrier",
                    "toolchain_metadata",
                    "Current function metadata marks arg0 as a fastcall u16, so HL should carry the value at callee entry.",
                )),
                _ => {}
            }
        }
        if current_function.is_stack_call {
            rules.push(Self::abi_contract_rule(
                "SP",
                "stack_entry_layout",
                "toolchain_metadata",
                "Stack-call metadata means the callee should observe a valid return address at SP and stack arguments starting at SP+2.",
            ));
        }
        if cross_bank_edge.is_some_and(|edge| edge.via_thunk || edge.via_farcall) || cross_bank {
            rules.push(Self::abi_contract_rule(
                "BC",
                "scratch_allowed",
                "kitaqgb_codegen_comment",
                "Cross-bank thunk/farcall wrappers use BC as scratch while switching/restoring banks.",
            ));
            rules.push(Self::abi_contract_rule(
                "DE",
                "scratch_allowed",
                "kitaqgb_codegen_comment",
                "Cross-bank thunk/farcall wrappers use DE as scratch while shuttling preserved values.",
            ));
        }
        rules
    }

    fn build_return_boundary_contract_rules(
        &self,
        callee_function: &ToolchainFunctionReport,
        cross_bank_edge: Option<&crate::report::ToolchainCrossBankEdgeReport>,
        cross_bank: bool,
    ) -> Vec<AbiRegisterContractRuleReport> {
        let mut rules = Vec::new();
        rules.push(Self::abi_contract_rule(
            "SP",
            "stack_restored_required",
            "runtime_inference",
            "Caller-visible stack depth should match again when execution resumes after the call/helper boundary.",
        ));
        if cross_bank_edge.is_some_and(|edge| edge.via_thunk || edge.via_farcall) || cross_bank {
            rules.push(Self::abi_contract_rule(
                "A",
                "preserved_required",
                "kitaqgb_codegen_comment",
                "KITAQGB bank wrappers preserve return carrier A while restoring the previous ROM bank.",
            ));
            rules.push(Self::abi_contract_rule(
                "HL",
                "preserved_required",
                "kitaqgb_codegen_comment",
                "KITAQGB bank wrappers preserve return carrier HL while restoring the previous ROM bank.",
            ));
            rules.push(Self::abi_contract_rule(
                "BC",
                "scratch_allowed",
                "kitaqgb_codegen_comment",
                "KITAQGB bank wrappers use BC as scratch during restore; movement here is observational unless it leaks back into caller-visible state incorrectly.",
            ));
            rules.push(Self::abi_contract_rule(
                "DE",
                "scratch_allowed",
                "kitaqgb_codegen_comment",
                "KITAQGB bank wrappers use DE as scratch during restore; movement here is observational unless it leaks back into caller-visible state incorrectly.",
            ));
        }
        match callee_function.return_size {
            1 => rules.push(Self::abi_contract_rule(
                "A",
                "return_value_carrier",
                "toolchain_metadata",
                "Callee metadata says the return value is 1 byte, so caller-resume A should reflect the callee result.",
            )),
            2.. => rules.push(Self::abi_contract_rule(
                "HL",
                "return_value_carrier",
                "toolchain_metadata",
                "Callee metadata says the return value is 2+ bytes, so caller-resume HL should reflect the callee result.",
            )),
            _ => {}
        }
        rules
    }

    fn build_intrinsic_contract_rules(
        &self,
        intrinsic_kind: &KitaqgbIntrinsicKind,
    ) -> Vec<AbiRegisterContractRuleReport> {
        let mut rules = Vec::new();
        rules.push(Self::abi_contract_rule(
            "SP",
            "stack_restored_required",
            "generic_helper_expectation",
            "KITAQGB helpers should leave caller-visible stack depth unchanged when control returns to the caller.",
        ));
        match intrinsic_kind {
            KitaqgbIntrinsicKind::Bank => {
                rules.push(Self::abi_contract_rule(
                    "A",
                    "preserved_required",
                    "kitaqgb_codegen_comment",
                    "Bank helper thunks preserve caller-visible A while switching/restoring ROM banks.",
                ));
                rules.push(Self::abi_contract_rule(
                    "HL",
                    "preserved_required",
                    "kitaqgb_codegen_comment",
                    "Bank helper thunks preserve caller-visible HL while switching/restoring ROM banks.",
                ));
                rules.push(Self::abi_contract_rule(
                    "BC",
                    "scratch_allowed",
                    "kitaqgb_codegen_comment",
                    "Bank helper thunks use BC as scratch during bank-save/restore choreography.",
                ));
                rules.push(Self::abi_contract_rule(
                    "DE",
                    "scratch_allowed",
                    "kitaqgb_codegen_comment",
                    "Bank helper thunks use DE as scratch during bank-save/restore choreography.",
                ));
            }
            KitaqgbIntrinsicKind::FarMemcpy
            | KitaqgbIntrinsicKind::Memcpy
            | KitaqgbIntrinsicKind::Memset => {
                rules.push(Self::abi_contract_rule(
                    "BC",
                    "scratch_allowed",
                    "helper_name_inference",
                    "Copy/fill helpers commonly use BC as a length or loop counter while moving bytes.",
                ));
                rules.push(Self::abi_contract_rule(
                    "DE",
                    "scratch_allowed",
                    "helper_name_inference",
                    "Copy/fill helpers commonly use DE as the destination pointer while walking memory.",
                ));
                rules.push(Self::abi_contract_rule(
                    "HL",
                    "scratch_allowed",
                    "helper_name_inference",
                    "Copy/fill helpers commonly use HL as the source or pattern pointer while walking memory.",
                ));
                rules.push(Self::abi_contract_rule(
                    "A",
                    "observational_only",
                    "helper_name_inference",
                    "Copy/fill helpers may transiently use A for the fill byte or loop state, so treat A as observational unless codegen metadata says otherwise.",
                ));
            }
            KitaqgbIntrinsicKind::SetTile8
            | KitaqgbIntrinsicKind::SetTile16
            | KitaqgbIntrinsicKind::SetTileBuffered
            | KitaqgbIntrinsicKind::SetTileCgb
            | KitaqgbIntrinsicKind::SetTileFlush
            | KitaqgbIntrinsicKind::SetTile16Flush
            | KitaqgbIntrinsicKind::FlushRows
            | KitaqgbIntrinsicKind::OamTransfer
            | KitaqgbIntrinsicKind::CgbPalette
            | KitaqgbIntrinsicKind::OamDma
            | KitaqgbIntrinsicKind::Vram => {
                rules.push(Self::abi_contract_rule(
                    "BC",
                    "scratch_allowed",
                    "helper_name_inference",
                    "Tile/VRAM/OAM helpers commonly use BC as a counter, row span, or coordinate pair while staging visible data.",
                ));
                rules.push(Self::abi_contract_rule(
                    "DE",
                    "scratch_allowed",
                    "helper_name_inference",
                    "Tile/VRAM/OAM helpers commonly use DE as a destination pointer or packed coordinate pair while updating display memory.",
                ));
                rules.push(Self::abi_contract_rule(
                    "HL",
                    "scratch_allowed",
                    "helper_name_inference",
                    "Tile/VRAM/OAM helpers commonly use HL as a source pointer, tile cursor, or descriptor stream cursor.",
                ));
                rules.push(Self::abi_contract_rule(
                    "A",
                    "observational_only",
                    "helper_name_inference",
                    "Video helpers often consume A as a tile id, palette selector, or status byte, so treat A as observational unless a stronger helper contract is added later.",
                ));
            }
            KitaqgbIntrinsicKind::Music => {
                for register in ["A", "BC", "DE", "HL"] {
                    rules.push(Self::abi_contract_rule(
                        register,
                        "observational_only",
                        "helper_name_inference",
                        "Audio helpers often repack note/envelope/register arguments in caller-visible registers before touching APU state, so current verification keeps these registers observational.",
                    ));
                }
            }
            KitaqgbIntrinsicKind::WaitVBlank
            | KitaqgbIntrinsicKind::Present
            | KitaqgbIntrinsicKind::Input
            | KitaqgbIntrinsicKind::BankGuard
            | KitaqgbIntrinsicKind::TrapCheck => {
                for register in ["A", "BC", "DE", "HL"] {
                    rules.push(Self::abi_contract_rule(
                        register,
                        "observational_only",
                        "weak_runtime_only",
                        "This helper kind is currently tracked mainly for stack safety and caller-visible side effects; register preservation still needs stronger helper-specific proof.",
                    ));
                }
            }
        }
        rules
    }

    fn build_abi_call_boundary(
        &self,
        current_function: &ToolchainFunctionReport,
    ) -> Option<AbiCallBoundaryReport> {
        let (caller_context, callee_entry_context) =
            self.recent_abi_boundary_context(current_function);
        let caller_function = caller_context.as_ref().and_then(|frame| {
            self.function_info_at(frame.rom_bank, frame.pc)
                .map(Self::toolchain_function_report)
        });
        let cross_bank_edge = self.toolchain_build_report.as_ref().and_then(|report| {
            report.cross_bank_edges.iter().find(|edge| {
                edge.callee == current_function.name
                    && edge.callee_bank == i32::from(current_function.bank)
                    && caller_function.as_ref().is_none_or(|caller| {
                        edge.caller == caller.name && edge.caller_bank == i32::from(caller.bank)
                    })
            })
        });

        let cross_bank = cross_bank_edge.is_some_and(|edge| edge.caller_bank != edge.callee_bank)
            || caller_function
                .as_ref()
                .is_some_and(|caller| caller.bank != current_function.bank);
        let contract_rules =
            self.build_call_boundary_contract_rules(current_function, cross_bank_edge, cross_bank);

        let mut register_checks = Vec::new();
        let mut notes = Vec::new();

        if let (Some(caller), Some(callee_entry)) = (&caller_context, &callee_entry_context) {
            if current_function.is_fast_call {
                match current_function.param_sizes.first().copied().unwrap_or(0) {
                    1 => {
                        let status = if caller.a == callee_entry.a {
                            "stable_across_boundary"
                        } else {
                            "changed_across_boundary"
                        };
                        register_checks.push(AbiBoundaryRegisterCheckReport {
                            register: "A".to_string(),
                            caller_value: Some(u16::from(caller.a)),
                            callee_entry_value: Some(u16::from(callee_entry.a)),
                            expectation: "fastcall arg0 should arrive in A at callee entry"
                                .to_string(),
                            status: status.to_string(),
                            detail: if caller.a == callee_entry.a {
                                format!(
                                    "Caller A={:#04X} matched callee-entry A={:#04X}.",
                                    caller.a, callee_entry.a
                                )
                            } else {
                                format!(
                                    "Caller A={:#04X} differed from callee-entry A={:#04X}; inspect the immediate call/thunk boundary.",
                                    caller.a, callee_entry.a
                                )
                            },
                        });
                    }
                    2 => {
                        let status = if caller.hl == callee_entry.hl {
                            "stable_across_boundary"
                        } else {
                            "changed_across_boundary"
                        };
                        register_checks.push(AbiBoundaryRegisterCheckReport {
                            register: "HL".to_string(),
                            caller_value: Some(caller.hl),
                            callee_entry_value: Some(callee_entry.hl),
                            expectation: "fastcall arg0 should arrive in HL at callee entry"
                                .to_string(),
                            status: status.to_string(),
                            detail: if caller.hl == callee_entry.hl {
                                format!(
                                    "Caller HL={:#06X} matched callee-entry HL={:#06X}.",
                                    caller.hl, callee_entry.hl
                                )
                            } else {
                                format!(
                                    "Caller HL={:#06X} differed from callee-entry HL={:#06X}; inspect the immediate call/thunk boundary.",
                                    caller.hl, callee_entry.hl
                                )
                            },
                        });
                    }
                    _ => {}
                }
            }

            if cross_bank_edge.is_some_and(|edge| edge.via_thunk || edge.via_farcall) || cross_bank
            {
                for (register, caller_value, callee_value) in [
                    ("BC", caller.bc, callee_entry.bc),
                    ("DE", caller.de, callee_entry.de),
                ] {
                    let stable = caller_value == callee_value;
                    register_checks.push(AbiBoundaryRegisterCheckReport {
                        register: register.to_string(),
                        caller_value: Some(caller_value),
                        callee_entry_value: Some(callee_value),
                        expectation:
                            "wrapper scratch register; change is allowed and does not imply ABI failure on its own"
                                .to_string(),
                        status: if stable {
                            "stable_scratch_observation"
                        } else {
                            "changed_scratch_observation"
                        }
                        .to_string(),
                        detail: if stable {
                            format!(
                                "Scratch observation: {} stayed {:#06X} across caller->callee entry.",
                                register, caller_value
                            )
                        } else {
                            format!(
                                "Scratch observation: {} changed from {:#06X} to {:#06X} across caller->callee entry.",
                                register, caller_value, callee_value
                            )
                        },
                    });
                }
            }

            if current_function.is_stack_call {
                let decoded_return = if callee_entry.stack_preview.len() >= 2 {
                    Some(
                        u16::from(callee_entry.stack_preview[0])
                            | (u16::from(callee_entry.stack_preview[1]) << 8),
                    )
                } else {
                    None
                };
                notes.push(format!(
                    "Callee entry SP={:#06X} previewed {} stack bytes at the last observed boundary.",
                    callee_entry.sp,
                    callee_entry.stack_preview.len()
                ));
                if let Some(return_addr) = decoded_return {
                    notes.push(format!(
                        "Callee-entry return address decoded as {:#06X} (region='{}').",
                        return_addr,
                        Self::classify_stack_region(return_addr)
                    ));
                }
            }

            if cross_bank_edge.is_some() {
                notes.push(
                    "Toolchain cross-bank metadata matched this boundary; caller/callee bank choreography is known."
                        .to_string(),
                );
            } else if cross_bank {
                notes.push(
                    "Boundary looks cross-bank from runtime context, but no exact toolchain cross-bank edge matched it."
                        .to_string(),
                );
            }
        } else {
            notes.push(
                "A precise caller->callee boundary was not present in the recent execution-context trail; use replay narrowing or symbol-boundary snapshots around the call site."
                    .to_string(),
            );
        }

        if cross_bank_edge.is_some_and(|edge| edge.via_thunk || edge.via_farcall) {
            notes.push(
                "Codegen comments indicate A/HL are the key preserved carriers across thunk/farcall wrappers; compare a callee-entry snapshot against a near-RET snapshot when chasing return corruption."
                    .to_string(),
            );
        }

        (!notes.is_empty() || !register_checks.is_empty() || caller_function.is_some()).then_some(
            AbiCallBoundaryReport {
                caller_function: caller_function.as_ref().map(|f| f.name.clone()),
                caller_bank: caller_function.as_ref().map(|f| f.bank),
                callee_function: current_function.name.clone(),
                callee_bank: current_function.bank,
                edge_kind: cross_bank_edge.map(|edge| edge.kind.clone()),
                via_thunk: cross_bank_edge.is_some_and(|edge| edge.via_thunk),
                via_farcall: cross_bank_edge.is_some_and(|edge| edge.via_farcall),
                cross_bank,
                caller_context,
                callee_entry_context,
                contract_rules,
                register_checks,
                notes,
            },
        )
    }

    fn recent_abi_return_context(
        &self,
        current_function: &ToolchainFunctionReport,
    ) -> (
        Option<ExecutionContextFrame>,
        Option<ExecutionContextFrame>,
        Option<ExecutionContextFrame>,
    ) {
        let frames: Vec<_> = self.execution_context_trail.iter().cloned().collect();
        let current_state_matches = self.current_function_info().is_some_and(|info| {
            info.name == current_function.name && info.bank == current_function.bank
        });
        let caller_resume_context = if current_state_matches {
            Some(self.execution_context_frame(
                "current_state".to_string(),
                self.current_code_bank(),
                self.machine.cpu.pc,
            ))
        } else {
            frames.iter().rev().find_map(|frame| {
                self.function_info_at(frame.rom_bank, frame.pc)
                    .and_then(|info| {
                        (info.name == current_function.name && info.bank == current_function.bank)
                            .then(|| frame.clone())
                    })
            })
        };

        let search_end = if current_state_matches {
            frames.len()
        } else {
            frames
                .iter()
                .rposition(|frame| {
                    self.function_info_at(frame.rom_bank, frame.pc)
                        .is_some_and(|info| {
                            info.name == current_function.name && info.bank == current_function.bank
                        })
                })
                .unwrap_or(frames.len())
        };

        let callee_last_index = (0..search_end).rev().find(|&index| {
            self.function_info_at(frames[index].rom_bank, frames[index].pc)
                .is_some_and(|info| {
                    info.name != current_function.name || info.bank != current_function.bank
                })
        });
        let callee_last_context = callee_last_index.and_then(|index| frames.get(index).cloned());
        let caller_pre_call_context = callee_last_index.and_then(|callee_index| {
            (0..callee_index).rev().find_map(|index| {
                self.function_info_at(frames[index].rom_bank, frames[index].pc)
                    .and_then(|info| {
                        (info.name == current_function.name && info.bank == current_function.bank)
                            .then(|| frames[index].clone())
                    })
            })
        });

        (
            caller_pre_call_context,
            callee_last_context,
            caller_resume_context,
        )
    }

    fn build_abi_return_boundary(
        &self,
        current_function: &ToolchainFunctionReport,
    ) -> Option<AbiReturnBoundaryReport> {
        let (caller_pre_call_context, callee_last_context, caller_resume_context) =
            self.recent_abi_return_context(current_function);
        let callee_function = callee_last_context.as_ref().and_then(|frame| {
            self.function_info_at(frame.rom_bank, frame.pc)
                .map(Self::toolchain_function_report)
        });
        let callee_function = match callee_function {
            Some(function) => function,
            None => {
                return None;
            }
        };

        let cross_bank_edge = self.toolchain_build_report.as_ref().and_then(|report| {
            report.cross_bank_edges.iter().find(|edge| {
                edge.caller == current_function.name
                    && edge.caller_bank == i32::from(current_function.bank)
                    && edge.callee == callee_function.name
                    && edge.callee_bank == i32::from(callee_function.bank)
            })
        });
        let cross_bank = cross_bank_edge.is_some_and(|edge| edge.caller_bank != edge.callee_bank)
            || current_function.bank != callee_function.bank;
        let contract_rules = self.build_return_boundary_contract_rules(
            &callee_function,
            cross_bank_edge,
            cross_bank,
        );
        let bytes_to_callee_end = callee_last_context
            .as_ref()
            .map(|frame| callee_function.end.saturating_sub(u32::from(frame.pc)));
        let callee_near_ret = bytes_to_callee_end.is_some_and(|distance| distance <= 4);

        let mut register_checks = Vec::new();
        let mut notes = Vec::new();
        if let Some(caller_pre) = &caller_pre_call_context {
            notes.push(format!(
                "Recent caller context before entering '{}' was observed at {:#06X} in bank {}.",
                current_function.name, caller_pre.pc, caller_pre.rom_bank
            ));
        }
        if let (Some(caller_pre), Some(caller_resume)) =
            (&caller_pre_call_context, &caller_resume_context)
        {
            let sp_stable = caller_pre.sp == caller_resume.sp;
            register_checks.push(AbiReturnRegisterCheckReport {
                register: "SP".to_string(),
                callee_value: Some(caller_pre.sp),
                caller_resume_value: Some(caller_resume.sp),
                expectation:
                    "caller-visible stack depth should be restored when execution resumes"
                        .to_string(),
                status: if sp_stable {
                    "stable_across_return"
                } else {
                    "changed_across_return"
                }
                .to_string(),
                detail: if sp_stable {
                    format!(
                        "Caller pre-call SP={:#06X} matched caller-resume SP={:#06X}.",
                        caller_pre.sp, caller_resume.sp
                    )
                } else {
                    format!(
                        "Caller pre-call SP={:#06X} differed from caller-resume SP={:#06X}; inspect wrapper stack restore / RET-side cleanup.",
                        caller_pre.sp, caller_resume.sp
                    )
                },
            });
        }

        if let (Some(callee_last), Some(caller_resume)) =
            (&callee_last_context, &caller_resume_context)
        {
            let should_check_a = cross_bank_edge
                .is_some_and(|edge| edge.via_thunk || edge.via_farcall)
                || cross_bank
                || callee_function.return_size == 1
                || callee_function.is_fast_call;
            let should_check_hl = cross_bank_edge
                .is_some_and(|edge| edge.via_thunk || edge.via_farcall)
                || cross_bank
                || callee_function.return_size >= 2
                || callee_function.is_fast_call;

            if should_check_a {
                let stable = callee_last.a == caller_resume.a;
                register_checks.push(AbiReturnRegisterCheckReport {
                    register: "A".to_string(),
                    callee_value: Some(u16::from(callee_last.a)),
                    caller_resume_value: Some(u16::from(caller_resume.a)),
                    expectation: "A should survive the callee near-RET to caller-resume boundary"
                        .to_string(),
                    status: if stable {
                        "stable_across_return"
                    } else {
                        "changed_across_return"
                    }
                    .to_string(),
                    detail: if stable {
                        format!(
                            "Callee near-RET A={:#04X} matched caller-resume A={:#04X}.",
                            callee_last.a, caller_resume.a
                        )
                    } else {
                        format!(
                            "Callee near-RET A={:#04X} differed from caller-resume A={:#04X}; inspect wrapper/RET-side clobbering.",
                            callee_last.a, caller_resume.a
                        )
                    },
                });
            }

            if should_check_hl {
                let stable = callee_last.hl == caller_resume.hl;
                register_checks.push(AbiReturnRegisterCheckReport {
                    register: "HL".to_string(),
                    callee_value: Some(callee_last.hl),
                    caller_resume_value: Some(caller_resume.hl),
                    expectation:
                        "HL should survive the callee near-RET to caller-resume boundary"
                            .to_string(),
                    status: if stable {
                        "stable_across_return"
                    } else {
                        "changed_across_return"
                    }
                    .to_string(),
                    detail: if stable {
                        format!(
                            "Callee near-RET HL={:#06X} matched caller-resume HL={:#06X}.",
                            callee_last.hl, caller_resume.hl
                        )
                    } else {
                        format!(
                            "Callee near-RET HL={:#06X} differed from caller-resume HL={:#06X}; inspect wrapper/RET-side clobbering.",
                            callee_last.hl, caller_resume.hl
                        )
                    },
                });
            }

            if cross_bank_edge.is_some_and(|edge| edge.via_thunk || edge.via_farcall) || cross_bank
            {
                for (register, callee_value, resume_value) in [
                    ("BC", callee_last.bc, caller_resume.bc),
                    ("DE", callee_last.de, caller_resume.de),
                ] {
                    let stable = callee_value == resume_value;
                    register_checks.push(AbiReturnRegisterCheckReport {
                        register: register.to_string(),
                        callee_value: Some(callee_value),
                        caller_resume_value: Some(resume_value),
                        expectation:
                            "wrapper scratch register; change is allowed and does not imply ABI failure on its own"
                                .to_string(),
                        status: if stable {
                            "stable_scratch_observation"
                        } else {
                            "changed_scratch_observation"
                        }
                        .to_string(),
                        detail: if stable {
                            format!(
                                "Scratch observation: {} stayed {:#06X} across the near-RET boundary.",
                                register, callee_value
                            )
                        } else {
                            format!(
                                "Scratch observation: {} changed from {:#06X} to {:#06X} across the near-RET boundary.",
                                register, callee_value, resume_value
                            )
                        },
                    });
                }
            }

            if callee_last.stack_preview.len() >= 2 {
                let decoded_return = u16::from(callee_last.stack_preview[0])
                    | (u16::from(callee_last.stack_preview[1]) << 8);
                let return_region = Self::classify_stack_region(decoded_return);
                notes.push(format!(
                    "Callee near-RET stack preview decoded return address {:#06X} (region='{}').",
                    decoded_return, return_region
                ));
                if let Some(resume_info) =
                    self.function_info_at(caller_resume.rom_bank, caller_resume.pc)
                {
                    if resume_info.name == current_function.name
                        && resume_info.bank == current_function.bank
                    {
                        if caller_resume.pc == decoded_return {
                            notes.push(
                                "Caller-resume PC exactly matched the callee's decoded return address."
                                    .to_string(),
                            );
                        } else if caller_resume.pc >= decoded_return {
                            notes.push(format!(
                                "Caller resumed {} bytes past the decoded return address.",
                                caller_resume.pc - decoded_return
                            ));
                        } else {
                            notes.push(format!(
                                "Caller-resume PC {:#06X} was earlier than the decoded return address {:#06X}; control may have branched immediately after return.",
                                caller_resume.pc, decoded_return
                            ));
                        }
                    }
                }
            }

            if callee_near_ret {
                notes.push(
                    "Callee sample was captured within 4 bytes of the function end, so return-boundary checks are relatively high-confidence."
                        .to_string(),
                );
            } else if let Some(distance) = bytes_to_callee_end {
                notes.push(format!(
                    "Callee sample was {} bytes from the function end; return-boundary checks are lower-confidence until a tighter near-RET sample is captured.",
                    distance
                ));
            }
        } else {
            notes.push(
                "A callee near-RET sample and caller-resume sample were not both present in the recent execution-context trail; use replay narrowing or conditional snapshots around the return boundary."
                    .to_string(),
            );
        }

        if cross_bank_edge.is_some() {
            notes.push(
                "Toolchain cross-bank metadata matched this return boundary; wrapper behavior is known to the build report."
                    .to_string(),
            );
        } else if cross_bank {
            notes.push(
                "Runtime context suggests a cross-bank return, but no exact toolchain cross-bank edge matched it."
                    .to_string(),
            );
        }

        (!register_checks.is_empty()
            || caller_pre_call_context.is_some()
            || callee_last_context.is_some()
            || caller_resume_context.is_some())
        .then_some(AbiReturnBoundaryReport {
            caller_function: current_function.name.clone(),
            caller_bank: current_function.bank,
            callee_function: callee_function.name,
            callee_bank: callee_function.bank,
            edge_kind: cross_bank_edge.map(|edge| edge.kind.clone()),
            via_thunk: cross_bank_edge.is_some_and(|edge| edge.via_thunk),
            via_farcall: cross_bank_edge.is_some_and(|edge| edge.via_farcall),
            cross_bank,
            callee_near_ret,
            bytes_to_callee_end,
            caller_pre_call_context,
            callee_last_context,
            caller_resume_context,
            contract_rules,
            register_checks,
            notes,
        })
    }

    fn build_abi_intrinsic_boundaries(&self) -> Vec<AbiIntrinsicBoundaryReport> {
        fn intrinsic_from_frame(frame: &ExecutionContextFrame) -> Option<KitaqgbIntrinsicMatch> {
            frame.symbol.as_deref().and_then(classify_symbol_name)
        }

        let mut frames: Vec<_> = self.execution_context_trail.iter().cloned().collect();
        let current_state = self.execution_context_frame(
            "current_state".to_string(),
            self.current_code_bank(),
            self.machine.cpu.pc,
        );
        let needs_current_state = frames.last().is_none_or(|last| {
            last.rom_bank != current_state.rom_bank || last.pc != current_state.pc
        });
        if needs_current_state {
            frames.push(current_state.clone());
        }

        let mut reports = Vec::new();
        let mut seen = BTreeSet::new();
        for index in (0..frames.len()).rev() {
            let Some(intrinsic) = intrinsic_from_frame(&frames[index]) else {
                continue;
            };
            let intrinsic_frame = frames[index].clone();
            let dedupe_key = (
                intrinsic_frame.rom_bank,
                intrinsic_frame.pc,
                intrinsic.canonical_name.clone(),
            );
            if !seen.insert(dedupe_key) {
                continue;
            }

            let before_context = (0..index).rev().find_map(|candidate| {
                intrinsic_from_frame(&frames[candidate])
                    .is_none()
                    .then(|| frames[candidate].clone())
            });
            let after_context = ((index + 1)..frames.len()).find_map(|candidate| {
                intrinsic_from_frame(&frames[candidate])
                    .is_none()
                    .then(|| frames[candidate].clone())
            });

            let mut register_checks = Vec::new();
            let mut observed_changed_registers = Vec::new();
            let mut notes = Vec::new();

            if let Some(before) = &before_context {
                if before.a != intrinsic_frame.a {
                    observed_changed_registers.push("A(entry)".to_string());
                }
                if before.bc != intrinsic_frame.bc {
                    observed_changed_registers.push("BC(entry)".to_string());
                }
                if before.de != intrinsic_frame.de {
                    observed_changed_registers.push("DE(entry)".to_string());
                }
                if before.hl != intrinsic_frame.hl {
                    observed_changed_registers.push("HL(entry)".to_string());
                }
                if before.sp != intrinsic_frame.sp {
                    observed_changed_registers.push("SP(entry)".to_string());
                }
            }

            let contract_rules = self.build_intrinsic_contract_rules(&intrinsic.kind);
            let contract_strength = Self::intrinsic_contract_strength(&intrinsic.kind);

            if let (Some(before), Some(after)) = (&before_context, &after_context) {
                for (register, before_value, after_value) in [
                    ("A", u16::from(before.a), u16::from(after.a)),
                    ("BC", before.bc, after.bc),
                    ("DE", before.de, after.de),
                    ("HL", before.hl, after.hl),
                    ("SP", before.sp, after.sp),
                ] {
                    if before_value != after_value {
                        observed_changed_registers.push(register.to_string());
                    }
                }

                let sp_stable = before.sp == after.sp;
                register_checks.push(AbiIntrinsicRegisterCheckReport {
                    register: "SP".to_string(),
                    before_value: Some(before.sp),
                    after_value: Some(after.sp),
                    expectation:
                        "intrinsic helper should leave caller-visible stack depth unchanged"
                            .to_string(),
                    status: if sp_stable {
                        "stable_across_intrinsic"
                    } else {
                        "changed_across_intrinsic"
                    }
                    .to_string(),
                    detail: if sp_stable {
                        format!(
                            "SP stayed at {:#06X} across intrinsic '{}'.",
                            before.sp, intrinsic.canonical_name
                        )
                    } else {
                        format!(
                            "SP changed from {:#06X} to {:#06X} across intrinsic '{}'.",
                            before.sp, after.sp, intrinsic.canonical_name
                        )
                    },
                });
                for rule in &contract_rules {
                    let Some(before_value) =
                        Self::execution_context_register_value(before, &rule.register)
                    else {
                        continue;
                    };
                    let Some(after_value) =
                        Self::execution_context_register_value(after, &rule.register)
                    else {
                        continue;
                    };
                    match rule.role.as_str() {
                        "preserved_required" => {
                            let stable = before_value == after_value;
                            register_checks.push(AbiIntrinsicRegisterCheckReport {
                                register: rule.register.clone(),
                                before_value: Some(before_value),
                                after_value: Some(after_value),
                                expectation: rule.detail.clone(),
                                status: if stable {
                                    "stable_across_intrinsic"
                                } else {
                                    "changed_across_intrinsic"
                                }
                                .to_string(),
                                detail: if stable {
                                    format!(
                                        "{} stayed {:#06X} across intrinsic '{}'.",
                                        rule.register, before_value, intrinsic.canonical_name
                                    )
                                } else {
                                    format!(
                                        "{} changed from {:#06X} to {:#06X} across intrinsic '{}'.",
                                        rule.register,
                                        before_value,
                                        after_value,
                                        intrinsic.canonical_name
                                    )
                                },
                            });
                        }
                        "scratch_allowed" => {
                            let stable = before_value == after_value;
                            register_checks.push(AbiIntrinsicRegisterCheckReport {
                                register: rule.register.clone(),
                                before_value: Some(before_value),
                                after_value: Some(after_value),
                                expectation: rule.detail.clone(),
                                status: if stable {
                                    "stable_scratch_observation"
                                } else {
                                    "changed_scratch_observation"
                                }
                                .to_string(),
                                detail: if stable {
                                    format!(
                                        "Scratch observation: {} stayed {:#06X} across intrinsic '{}'.",
                                        rule.register, before_value, intrinsic.canonical_name
                                    )
                                } else {
                                    format!(
                                        "Scratch observation: {} changed from {:#06X} to {:#06X} across intrinsic '{}'.",
                                        rule.register, before_value, after_value, intrinsic.canonical_name
                                    )
                                },
                            });
                        }
                        "observational_only" => {
                            let stable = before_value == after_value;
                            register_checks.push(AbiIntrinsicRegisterCheckReport {
                                register: rule.register.clone(),
                                before_value: Some(before_value),
                                after_value: Some(after_value),
                                expectation: rule.detail.clone(),
                                status: if stable {
                                    "stable_observational"
                                } else {
                                    "changed_observational"
                                }
                                .to_string(),
                                detail: if stable {
                                    format!(
                                        "Observational register {} stayed {:#06X} across intrinsic '{}'.",
                                        rule.register, before_value, intrinsic.canonical_name
                                    )
                                } else {
                                    format!(
                                        "Observational register {} changed from {:#06X} to {:#06X} across intrinsic '{}'.",
                                        rule.register, before_value, after_value, intrinsic.canonical_name
                                    )
                                },
                            });
                        }
                        _ => {}
                    }
                }

                if contract_rules.iter().all(|rule| {
                    rule.role == "observational_only" || rule.role == "stack_restored_required"
                }) {
                    notes.push(
                        "This helper currently has stack-safety and observational contracts only; stronger preserved-register guarantees still need helper-specific proof."
                            .to_string(),
                    );
                } else if contract_rules
                    .iter()
                    .any(|rule| rule.confidence == "helper_name_inference")
                {
                    notes.push(
                        "Some helper-specific ABI expectations here are inferred from helper naming/class rather than direct codegen metadata, so treat failures as high-signal clues rather than final proof."
                            .to_string(),
                    );
                }

                if let Some(info) = self.function_info_at(before.rom_bank, before.pc) {
                    notes.push(format!(
                        "Caller-side context before intrinsic was '{}'.",
                        info.name
                    ));
                }
                if let Some(info) = self.function_info_at(after.rom_bank, after.pc) {
                    notes.push(format!(
                        "Caller-side context after intrinsic was '{}'.",
                        info.name
                    ));
                }
            } else {
                notes.push(
                    "Recent execution-context history did not capture both sides of this intrinsic boundary; use tighter replay narrowing or conditional snapshots around the helper."
                        .to_string(),
                );
            }

            reports.push(AbiIntrinsicBoundaryReport {
                intrinsic_symbol: intrinsic.canonical_name,
                intrinsic_kind: format!("{:?}", intrinsic.kind),
                contract_strength: contract_strength.to_string(),
                before_context,
                intrinsic_context: intrinsic_frame,
                after_context,
                contract_rules,
                register_checks,
                observed_changed_registers,
                notes,
            });
            if reports.len() == 4 {
                break;
            }
        }

        reports.reverse();
        reports
    }

    fn build_abi_verification(&self) -> Option<AbiVerificationReport> {
        let abi_mode = self
            .toolchain_build_report
            .as_ref()
            .and_then(|report| report.abi_mode.clone());
        let toolchain_issues = self
            .toolchain_build_report
            .as_ref()
            .map(|report| report.abi_issues.clone())
            .unwrap_or_default();
        let toolchain_issue_count = self
            .toolchain_build_report
            .as_ref()
            .map(|report| {
                if report.abi_issues.is_empty() {
                    report.abi_issue_count
                } else {
                    report.abi_issues.len() as u32
                }
            })
            .unwrap_or(0);

        let current_function_contract = self
            .current_function_info()
            .map(Self::toolchain_function_report);
        let call_boundary = current_function_contract
            .as_ref()
            .and_then(|function| self.build_abi_call_boundary(function));
        let return_boundary = current_function_contract
            .as_ref()
            .and_then(|function| self.build_abi_return_boundary(function));
        let intrinsic_boundaries = self.build_abi_intrinsic_boundaries();
        let register_snapshot = self.build_abi_register_snapshot();
        let stack_window = self.build_abi_stack_window(current_function_contract.as_ref());

        let mut runtime_alerts = Vec::new();
        if self.far_call_count > 0 && !self.bank_restored {
            runtime_alerts.push(
                "A far-call return remained unresolved beyond the bounded grace period."
                    .to_string(),
            );
        }
        if self.bank_thrash_score >= 4 {
            runtime_alerts.push(format!(
                "Bank thrash score reached {} during execution, which can mask call/return ABI issues.",
                self.bank_thrash_score
            ));
        }
        if self
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("SP") || diag.message.contains("ABI"))
        {
            runtime_alerts.push(
                "Runtime diagnostics already mention SP/ABI-sensitive behavior in this run window."
                    .to_string(),
            );
        }
        if let Some(function) = &current_function_contract {
            if function.is_stack_call {
                runtime_alerts.push(format!(
                    "Current function '{}' uses stack-call ABI semantics; stack/return mismatches are especially relevant here.",
                    function.name
                ));
                if stack_window.region != "wram"
                    && stack_window.region != "echo_ram"
                    && stack_window.region != "hram"
                {
                    runtime_alerts.push(format!(
                        "SP currently points into '{}' while stack-call ABI is active; this is suspicious for argument/return decoding.",
                        stack_window.region
                    ));
                }
                if let Some(return_addr) = stack_window.decoded_return_address {
                    let return_region = stack_window
                        .decoded_return_region
                        .as_deref()
                        .unwrap_or_else(|| Self::classify_stack_region(return_addr));
                    if matches!(return_region, "vram" | "oam" | "unusable" | "io") {
                        runtime_alerts.push(format!(
                            "Decoded return address {:#06X} points into '{}', which is unlikely for a valid call return path.",
                            return_addr, return_region
                        ));
                    }
                }
                if stack_window.expected_stack_argument_bytes > 0
                    && stack_window.argument_preview_bytes.is_empty()
                {
                    runtime_alerts.push(
                        "Stack-call metadata expects arguments on the stack, but the current stack window could not preview any argument bytes."
                            .to_string(),
                    );
                }
            } else if function.is_fast_call {
                runtime_alerts.push(format!(
                    "Current function '{}' uses fast-call semantics; register preservation assumptions matter here.",
                    function.name
                ));
                match function.param_sizes.first().copied().unwrap_or(0) {
                    1 => runtime_alerts.push(format!(
                        "Fast-call entry suggests arg0 is carried in A; current A={:#04X}.",
                        register_snapshot.a
                    )),
                    2 => runtime_alerts.push(format!(
                        "Fast-call entry suggests arg0 is carried in HL; current HL={:#06X}.",
                        register_snapshot.hl
                    )),
                    _ => {}
                }
            }
            if function.has_fixed_order && function.param_sizes.len() > 1 {
                runtime_alerts.push(format!(
                    "Function '{}' has fixed parameter order metadata; mismatched stack/register ordering can cause subtle ABI drift.",
                    function.name
                ));
            }
        }
        if stack_window.region == "unusable" || stack_window.region == "oam" {
            runtime_alerts.push(format!(
                "SP is currently in '{}' space, which is high-risk for stable call/return behavior.",
                stack_window.region
            ));
        }
        if let Some(boundary) = &call_boundary {
            for check in &boundary.register_checks {
                if check.status == "changed_across_boundary" {
                    runtime_alerts.push(format!(
                        "Call-boundary '{}' carrier drifted between caller and callee entry for '{}'.",
                        check.register, boundary.callee_function
                    ));
                }
            }
        }
        if let Some(boundary) = &return_boundary {
            if boundary.callee_near_ret {
                for check in &boundary.register_checks {
                    if check.status == "changed_across_return" {
                        runtime_alerts.push(format!(
                            "Near-RET '{}' carrier drifted between '{}' and caller resume in '{}'.",
                            check.register, boundary.callee_function, boundary.caller_function
                        ));
                    }
                }
            }
        }
        for boundary in &intrinsic_boundaries {
            for check in &boundary.register_checks {
                if check.status == "changed_across_intrinsic" {
                    runtime_alerts.push(format!(
                        "Intrinsic boundary '{}' changed '{}' under '{}' contract strength.",
                        boundary.intrinsic_symbol, check.register, boundary.contract_strength
                    ));
                }
            }
        }

        let status = if toolchain_issue_count == 0 && runtime_alerts.is_empty() {
            "clean_candidate"
        } else if toolchain_issue_count > 0 {
            "toolchain_issues_present"
        } else {
            "runtime_watch_required"
        };

        let generated = abi_mode.is_some()
            || toolchain_issue_count > 0
            || current_function_contract.is_some()
            || call_boundary.is_some()
            || return_boundary.is_some()
            || !intrinsic_boundaries.is_empty()
            || !runtime_alerts.is_empty();
        generated.then_some(AbiVerificationReport {
            generated,
            abi_mode,
            toolchain_issue_count,
            status: status.to_string(),
            toolchain_issues,
            current_function_contract,
            call_boundary,
            return_boundary,
            intrinsic_boundaries,
            register_snapshot,
            stack_window,
            runtime_alerts,
            carry_forward_notes: vec![
                "This ABI surface now includes current register/stack context, call-boundary transport, near-RET return-boundary checks, and recent intrinsic-boundary observations, but it is still not a full step-by-step contract verifier.".to_string(),
                "Use the stack window, decoded return address, fastcall hints, near-RET caller-resume checks, and intrinsic-boundary observations to place narrower stop conditions or replay comparisons around the failing call/return/helper boundary.".to_string(),
            ],
        })
    }

    fn build_rom_forensics(
        &self,
        auto_diagnosis: Option<&AutoDiagnosisReport>,
    ) -> Option<RomForensicsReport> {
        #[derive(Default)]
        struct HotspotAcc {
            title: String,
            count: u64,
            last_frame: u64,
            last_cycle: u64,
            detail: String,
        }

        #[derive(Default)]
        struct MapperAcc {
            control_writes: u64,
            rom_bank_changes: u64,
            ram_bank_changes: u64,
            detail: String,
        }

        fn record_hotspot(
            map: &mut BTreeMap<String, HotspotAcc>,
            key: &str,
            title: &str,
            frame: u64,
            cycle: u64,
            detail: String,
        ) {
            let entry = map.entry(key.to_string()).or_default();
            if entry.title.is_empty() {
                entry.title = title.to_string();
            }
            entry.count += 1;
            entry.last_frame = frame;
            entry.last_cycle = cycle;
            entry.detail = detail;
        }

        let mut mmio_hotspots: BTreeMap<String, HotspotAcc> = BTreeMap::new();
        let mut suspicious_writes: BTreeMap<String, HotspotAcc> = BTreeMap::new();
        let mut mapper_activity: BTreeMap<String, MapperAcc> = BTreeMap::new();

        for event in &self.event_log {
            match event {
                DebugEvent::LcdToggle {
                    frame,
                    cycle,
                    lcdc,
                    enabled,
                } => {
                    record_hotspot(
                        &mut mmio_hotspots,
                        "ff40_lcdc",
                        "FF40 LCDC activity",
                        *frame,
                        *cycle,
                        format!("enabled={} lcdc={:#04X}", enabled, lcdc),
                    );
                    record_hotspot(
                        &mut suspicious_writes,
                        "lcdc_runtime_toggle",
                        "Runtime LCDC toggles",
                        *frame,
                        *cycle,
                        format!("enabled={} lcdc={:#04X}", enabled, lcdc),
                    );
                }
                DebugEvent::StatWrite {
                    frame,
                    cycle,
                    old_stat,
                    new_stat,
                } => {
                    record_hotspot(
                        &mut mmio_hotspots,
                        "ff41_stat",
                        "FF41 STAT writes",
                        *frame,
                        *cycle,
                        format!("{:#04X}->{:#04X}", old_stat, new_stat),
                    );
                    record_hotspot(
                        &mut suspicious_writes,
                        "stat_retarget",
                        "STAT retarget churn",
                        *frame,
                        *cycle,
                        format!("{:#04X}->{:#04X}", old_stat, new_stat),
                    );
                }
                DebugEvent::LycWrite {
                    frame,
                    cycle,
                    old_lyc,
                    new_lyc,
                } => {
                    record_hotspot(
                        &mut mmio_hotspots,
                        "ff45_lyc",
                        "FF45 LYC writes",
                        *frame,
                        *cycle,
                        format!("{}->{}", old_lyc, new_lyc),
                    );
                    record_hotspot(
                        &mut suspicious_writes,
                        "lyc_retarget",
                        "LYC retarget churn",
                        *frame,
                        *cycle,
                        format!("{}->{}", old_lyc, new_lyc),
                    );
                }
                DebugEvent::OamDmaStart {
                    frame,
                    cycle,
                    source,
                } => {
                    record_hotspot(
                        &mut mmio_hotspots,
                        "ff46_oam_dma",
                        "FF46 OAM DMA activity",
                        *frame,
                        *cycle,
                        format!("source={:#06X}", source),
                    );
                }
                DebugEvent::HdmaStart {
                    frame,
                    cycle,
                    source,
                    dest,
                    hblank_mode,
                    ..
                } => {
                    record_hotspot(
                        &mut mmio_hotspots,
                        "ff55_hdma",
                        "FF55 HDMA/GDMA control activity",
                        *frame,
                        *cycle,
                        format!(
                            "source={:#06X} dest={:#06X} hblank_mode={}",
                            source, dest, hblank_mode
                        ),
                    );
                }
                DebugEvent::HdmaWriteIgnored {
                    frame,
                    cycle,
                    value,
                    remaining_blocks,
                } => {
                    record_hotspot(
                        &mut suspicious_writes,
                        "ff55_hdma_ignored",
                        "FF55 HDMA control writes ignored",
                        *frame,
                        *cycle,
                        format!("value={:#04X} remaining_blocks={}", value, remaining_blocks),
                    );
                }
                DebugEvent::CgbBgPaletteDataWrite {
                    frame,
                    cycle,
                    index,
                    value,
                    blocked,
                    auto_increment,
                } => {
                    record_hotspot(
                        &mut mmio_hotspots,
                        "ff69_bcpd",
                        "FF69 BG palette data activity",
                        *frame,
                        *cycle,
                        format!(
                            "index={} value={:#04X} blocked={} auto_inc={}",
                            index, value, blocked, auto_increment
                        ),
                    );
                    if *blocked {
                        record_hotspot(
                            &mut suspicious_writes,
                            "ff69_blocked",
                            "Blocked BG palette writes",
                            *frame,
                            *cycle,
                            format!("index={} value={:#04X}", index, value),
                        );
                    }
                }
                DebugEvent::CgbObjPaletteDataWrite {
                    frame,
                    cycle,
                    index,
                    value,
                    blocked,
                    auto_increment,
                } => {
                    record_hotspot(
                        &mut mmio_hotspots,
                        "ff6b_ocpd",
                        "FF6B OBJ palette data activity",
                        *frame,
                        *cycle,
                        format!(
                            "index={} value={:#04X} blocked={} auto_inc={}",
                            index, value, blocked, auto_increment
                        ),
                    );
                    if *blocked {
                        record_hotspot(
                            &mut suspicious_writes,
                            "ff6b_blocked",
                            "Blocked OBJ palette writes",
                            *frame,
                            *cycle,
                            format!("index={} value={:#04X}", index, value),
                        );
                    }
                }
                DebugEvent::CgbKey1Write {
                    frame,
                    cycle,
                    armed,
                    double_speed,
                    value,
                } => {
                    record_hotspot(
                        &mut mmio_hotspots,
                        "ff4d_key1",
                        "FF4D KEY1 activity",
                        *frame,
                        *cycle,
                        format!(
                            "armed={} double_speed={} value={:#04X}",
                            armed, double_speed, value
                        ),
                    );
                }
                DebugEvent::JoypadSelectionWrite {
                    frame,
                    cycle,
                    old_p1,
                    new_p1,
                } => {
                    record_hotspot(
                        &mut mmio_hotspots,
                        "ff00_p1",
                        "FF00 joypad select activity",
                        *frame,
                        *cycle,
                        format!("{:#04X}->{:#04X}", old_p1, new_p1),
                    );
                    record_hotspot(
                        &mut suspicious_writes,
                        "ff00_select_churn",
                        "Joypad select churn",
                        *frame,
                        *cycle,
                        format!("{:#04X}->{:#04X}", old_p1, new_p1),
                    );
                }
                DebugEvent::TimerControlWrite {
                    frame,
                    cycle,
                    old_tac,
                    new_tac,
                } => {
                    record_hotspot(
                        &mut mmio_hotspots,
                        "ff07_tac",
                        "FF07 TAC writes",
                        *frame,
                        *cycle,
                        format!("{:#04X}->{:#04X}", old_tac, new_tac),
                    );
                }
                DebugEvent::ApuMixerControl {
                    frame,
                    cycle,
                    nr50,
                    nr51,
                    nr52,
                } => {
                    record_hotspot(
                        &mut mmio_hotspots,
                        "ff24_ff26_apu_mix",
                        "APU mixer control activity",
                        *frame,
                        *cycle,
                        format!("nr50={:#04X} nr51={:#04X} nr52={:#04X}", nr50, nr51, nr52),
                    );
                }
                DebugEvent::ApuPopRisk {
                    frame,
                    cycle,
                    source,
                    reg,
                } => {
                    record_hotspot(
                        &mut suspicious_writes,
                        "apu_pop_risk",
                        "APU pop-risk writes",
                        *frame,
                        *cycle,
                        format!("source={} reg={:#06X}", source, reg),
                    );
                }
                DebugEvent::MapperControlWrite {
                    mapper,
                    addr,
                    value,
                    ..
                } => {
                    let entry = mapper_activity.entry(mapper.clone()).or_default();
                    entry.control_writes += 1;
                    entry.detail = format!("last control write {:#06X}={:#04X}", addr, value);
                }
                DebugEvent::MapperRomBankChange {
                    mapper,
                    from,
                    to,
                    addr,
                    value,
                    ..
                } => {
                    let entry = mapper_activity.entry(mapper.clone()).or_default();
                    entry.rom_bank_changes += 1;
                    entry.detail = format!(
                        "last ROM bank change {}->{} via {:#06X}={:#04X}",
                        from, to, addr, value
                    );
                }
                DebugEvent::MapperRamBankChange {
                    mapper,
                    from,
                    to,
                    addr,
                    value,
                    ..
                } => {
                    let entry = mapper_activity.entry(mapper.clone()).or_default();
                    entry.ram_bank_changes += 1;
                    entry.detail = format!(
                        "last RAM bank change {}->{} via {:#06X}={:#04X}",
                        from, to, addr, value
                    );
                }
                _ => {}
            }
        }

        let mut hot_loop_candidates: Vec<_> = self
            .pc_hit_histogram
            .iter()
            .filter(|(_, hits)| **hits >= 4)
            .map(|(&(bank, pc), &hits)| ForensicHotLoopReport {
                rom_bank: bank,
                pc,
                hits,
                symbol: self.symbol_name_at(bank, pc),
                source: self
                    .source_location_at(bank, pc)
                    .as_ref()
                    .map(Self::source_location_report_from_stop),
            })
            .collect();
        hot_loop_candidates.sort_by(|a, b| {
            b.hits
                .cmp(&a.hits)
                .then_with(|| a.rom_bank.cmp(&b.rom_bank))
                .then_with(|| a.pc.cmp(&b.pc))
        });
        hot_loop_candidates.truncate(8);

        let mut mmio_hotspots: Vec<_> = mmio_hotspots
            .into_iter()
            .map(|(key, item)| ForensicHotspotReport {
                key,
                title: item.title,
                count: item.count,
                last_frame: item.last_frame,
                last_cycle: item.last_cycle,
                detail: item.detail,
            })
            .collect();
        mmio_hotspots.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
        mmio_hotspots.truncate(8);

        let mut suspicious_writes: Vec<_> = suspicious_writes
            .into_iter()
            .map(|(key, item)| ForensicHotspotReport {
                key,
                title: item.title,
                count: item.count,
                last_frame: item.last_frame,
                last_cycle: item.last_cycle,
                detail: item.detail,
            })
            .collect();
        suspicious_writes.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
        suspicious_writes.truncate(8);

        let mut mapper_activity: Vec<_> = mapper_activity
            .into_iter()
            .map(|(mapper, item)| ForensicMapperActivityReport {
                mapper,
                control_writes: item.control_writes,
                rom_bank_changes: item.rom_bank_changes,
                ram_bank_changes: item.ram_bank_changes,
                detail: item.detail,
            })
            .collect();
        mapper_activity.sort_by(|a, b| {
            (b.control_writes + b.rom_bank_changes + b.ram_bank_changes)
                .cmp(&(a.control_writes + a.rom_bank_changes + a.ram_bank_changes))
                .then_with(|| a.mapper.cmp(&b.mapper))
        });
        mapper_activity.truncate(8);

        let mut unsupported_hotspots: Vec<_> = self
            .unsupported_opcodes
            .values()
            .cloned()
            .map(|item| ForensicUnsupportedOpcodeReport {
                opcode: item.opcode,
                count: item.count,
                last_rom_bank: item.last_rom_bank,
                last_pc: item.last_pc,
                last_symbol: item.last_symbol,
            })
            .collect();
        unsupported_hotspots
            .sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.opcode.cmp(&b.opcode)));
        unsupported_hotspots.truncate(8);

        let triage = auto_diagnosis
            .map(|report| {
                report
                    .suspects
                    .iter()
                    .take(3)
                    .map(|suspect| {
                        format!(
                            "{} [{}] score={} confidence={}",
                            suspect.title,
                            match suspect.category {
                                crate::diagnostics::AutoDiagnosisCategory::Rendering => "rendering",
                                crate::diagnostics::AutoDiagnosisCategory::Banking => "banking",
                                crate::diagnostics::AutoDiagnosisCategory::Timing => "timing",
                                crate::diagnostics::AutoDiagnosisCategory::Interrupts =>
                                    "interrupts",
                                crate::diagnostics::AutoDiagnosisCategory::Input => "input",
                                crate::diagnostics::AutoDiagnosisCategory::Audio => "audio",
                                crate::diagnostics::AutoDiagnosisCategory::Unsupported =>
                                    "unsupported",
                                crate::diagnostics::AutoDiagnosisCategory::Replay => "replay",
                            },
                            suspect.score,
                            suspect.confidence
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut carry_forward_notes = vec![
            "ROM forensics are heuristic summaries over debug events and per-step PC hits; they are not hardware proof.".to_string(),
            "Use report.auto_diagnosis together with rom_forensics.hot_loop_candidates and rom_forensics.mmio_hotspots to choose the next breakpoint/watchpoint.".to_string(),
        ];
        if !unsupported_hotspots.is_empty() {
            carry_forward_notes.push("Unsupported opcodes are surfaced as forensic hotspots so existing-ROM triage can prioritize CPU gaps before deeper timing work.".to_string());
        }

        let generated = !hot_loop_candidates.is_empty()
            || !mmio_hotspots.is_empty()
            || !suspicious_writes.is_empty()
            || !mapper_activity.is_empty()
            || !unsupported_hotspots.is_empty()
            || !triage.is_empty();

        generated.then_some(RomForensicsReport {
            generated,
            hot_loop_candidates,
            mmio_hotspots,
            suspicious_writes,
            mapper_activity,
            unsupported_hotspots,
            triage,
            carry_forward_notes,
        })
    }

    fn recent_event_summary(&self) -> Vec<String> {
        self.event_log
            .iter()
            .rev()
            .take(5)
            .map(|event| match event {
                DebugEvent::ScanlineAdvance { from_ly, to_ly, .. } => {
                    format!("LY {}->{}", from_ly, to_ly)
                }
                DebugEvent::PpuModeChange {
                    from_mode,
                    to_mode,
                    ly,
                    ..
                } => format!("PPUMode {}->{} @LY{}", from_mode, to_mode, ly),
                DebugEvent::ScanlineRender { ly, .. } => format!("ScanlineRender@LY{}", ly),
                DebugEvent::OamDmaStart { source, .. } => format!("OamDmaStart@{:04X}", source),
                DebugEvent::OamDmaComplete { bytes, .. } => format!("OamDmaComplete({})", bytes),
                DebugEvent::GdmaStallEstimate {
                    blocks,
                    stall_cycles,
                    ..
                } => format!("GdmaStall({}b/{}c)", blocks, stall_cycles),
                DebugEvent::HdmaStart {
                    dest,
                    blocks,
                    hblank_mode,
                    ..
                } => format!(
                    "{}Start@{:04X}x{}",
                    if *hblank_mode { "H" } else { "G" },
                    dest,
                    blocks
                ),
                DebugEvent::HdmaBlock {
                    block_index,
                    remaining_blocks,
                    hblank_mode,
                    stall_cycles,
                    ..
                } => format!(
                    "{}DmaBlock#{} rem{} {}c",
                    if *hblank_mode { "H" } else { "G" },
                    block_index,
                    remaining_blocks,
                    stall_cycles
                ),
                DebugEvent::HdmaComplete {
                    blocks,
                    hblank_mode,
                    ..
                } => format!(
                    "{}DmaComplete({})",
                    if *hblank_mode { "H" } else { "G" },
                    blocks
                ),
                DebugEvent::HdmaCancel {
                    remaining_blocks, ..
                } => format!("HdmaCancel rem{}", remaining_blocks),
                DebugEvent::HdmaDeferred {
                    remaining_blocks,
                    ly,
                    reason,
                    ..
                } => format!("HdmaDeferred {} rem{} ly{}", reason, remaining_blocks, ly),
                DebugEvent::HdmaWriteIgnored {
                    value,
                    remaining_blocks,
                    ..
                } => format!("HdmaWriteIgnored {:02X} rem{}", value, remaining_blocks),
                DebugEvent::MapperControlWrite {
                    mapper,
                    addr,
                    value,
                    rom_bank,
                    ram_bank,
                    ..
                } => format!(
                    "{} ctrl {:04X}={:02X} rb{} ram{}",
                    mapper, addr, value, rom_bank, ram_bank
                ),
                DebugEvent::MapperRomBankChange {
                    mapper, from, to, ..
                } => format!("{} ROM {}->{}", mapper, from, to),
                DebugEvent::MapperRamBankChange {
                    mapper, from, to, ..
                } => format!("{} RAM {}->{}", mapper, from, to),
                DebugEvent::ApuMasterToggle { enabled, .. } => {
                    format!("APU{}", if *enabled { "On" } else { "Off" })
                }
                DebugEvent::ApuChannelTrigger { channel, .. } => {
                    format!("APU trigger CH{}", channel)
                }
                DebugEvent::ApuFrameSequencerStep { step, .. } => format!("APU frame {}", step),
                DebugEvent::ApuWaveRamWrite { index, value, .. } => {
                    format!("Wave[{}]={:02X}", index, value)
                }
                DebugEvent::ApuWaveRamAccessAliased {
                    requested_index,
                    actual_index,
                    is_write,
                    ..
                } => format!(
                    "WaveAlias {} {:02X}->{:02X}",
                    if *is_write { "W" } else { "R" },
                    requested_index,
                    actual_index
                ),
                DebugEvent::ApuCh3TriggerRetainsSample {
                    buffered_sample,
                    next_index,
                    ..
                } => format!("Ch3Hold {:X}->{}", buffered_sample, next_index),
                DebugEvent::ApuNoiseClockFrozen {
                    shift,
                    divisor_code,
                    ..
                } => format!("NoiseFreeze sh{} d{}", shift, divisor_code),
                DebugEvent::CgbModeSelected {
                    cgb_enabled,
                    cgb_only,
                    ..
                } => format!("CGB mode={} only={}", cgb_enabled, cgb_only),
                DebugEvent::CgbVramBankSwitch { bank, .. } => format!("VBK->{}", bank),
                DebugEvent::CgbWramBankSwitch { bank, .. } => format!("SVBK->{}", bank),
                DebugEvent::CgbBgPaletteIndexWrite {
                    index,
                    auto_increment,
                    ..
                } => format!("BGPI {:02X} ai={}", index, auto_increment),
                DebugEvent::CgbBgPaletteDataWrite {
                    index,
                    value,
                    blocked,
                    ..
                } => format!("BGPD {:02X}={:02X} blk={}", index, value, blocked),
                DebugEvent::CgbObjPaletteIndexWrite {
                    index,
                    auto_increment,
                    ..
                } => format!("OBPI {:02X} ai={}", index, auto_increment),
                DebugEvent::CgbObjPaletteDataWrite {
                    index,
                    value,
                    blocked,
                    ..
                } => format!("OBPD {:02X}={:02X} blk={}", index, value, blocked),
                DebugEvent::CgbKey1Write {
                    armed,
                    double_speed,
                    ..
                } => format!("KEY1 arm={} ds={}", armed, double_speed),
                DebugEvent::CgbSpeedSwitch {
                    double_speed,
                    stop_stall_cycles,
                    ..
                } => format!(
                    "SpeedSwitch {} stall={}",
                    if *double_speed { "2x" } else { "1x" },
                    stop_stall_cycles
                ),
                DebugEvent::CgbSpeedSwitchFreeze {
                    cpu_cycles,
                    ppu_mode,
                    ..
                } => format!("SpeedFreeze {}cy mode{}", cpu_cycles, ppu_mode),
                DebugEvent::LcdToggle { enabled, .. } => {
                    format!("LCD{}", if *enabled { "On" } else { "Off" })
                }
                DebugEvent::StatWrite {
                    old_stat, new_stat, ..
                } => format!("STAT {:02X}->{:02X}", old_stat, new_stat),
                DebugEvent::LycWrite {
                    old_lyc, new_lyc, ..
                } => format!("LYC {}->{}", old_lyc, new_lyc),
                DebugEvent::StatSignal {
                    coincidence, ly, ..
                } => format!("STAT coincidence={} @LY{}", coincidence, ly),
                DebugEvent::VblankEnter { frame, .. } => format!("VBlank@{}", frame),
                DebugEvent::FrameComplete {
                    frame, frame_hash, ..
                } => format!("FrameComplete@{}#{:08X}", frame, frame_hash),
                DebugEvent::BankSwitch { from, to, .. } => format!("BankSwitch {}->{}", from, to),
                DebugEvent::FarCallSuspected { to_symbol, .. } => {
                    format!(
                        "FarCall {}",
                        to_symbol.clone().unwrap_or_else(|| "?".to_string())
                    )
                }
                DebugEvent::KitaqgbIntrinsic {
                    intrinsic_kind,
                    symbol,
                    ..
                } => format!("Intrinsic {}:{}", intrinsic_kind, symbol),
                DebugEvent::BankReturnMissing {
                    current_bank,
                    expected_bank,
                    ..
                } => format!("BankReturnMissing {}!={}", current_bank, expected_bank),
                DebugEvent::ApuChannelLengthExpired { channel, .. } => {
                    format!("ApuLenEnd CH{}", channel)
                }
                DebugEvent::ApuEnvelopeStep {
                    channel, volume, ..
                } => format!("ApuEnv CH{}={}", channel, volume),
                DebugEvent::ApuSweepStep {
                    old_frequency,
                    new_frequency,
                    ..
                } => format!("ApuSweep {}->{}", old_frequency, new_frequency),
                DebugEvent::ApuChannelDisabled {
                    channel, reason, ..
                } => format!("ApuOff CH{} {}", channel, reason),
                DebugEvent::ApuMixerControl { nr50, nr51, .. } => {
                    format!("ApuMixCtl {:02X}/{:02X}", nr50, nr51)
                }
                DebugEvent::ApuDacStateChange {
                    channel,
                    enabled,
                    active,
                    ..
                } => format!(
                    "ApuDAC ch{} {} act={}",
                    channel,
                    if *enabled { "on" } else { "off" },
                    active
                ),
                DebugEvent::ApuPopRisk { source, reg, .. } => {
                    format!("ApuPop {} @{:04X}", source, reg)
                }
                DebugEvent::ApuMixedOutput {
                    left,
                    right,
                    active_mask,
                    ..
                } => format!("ApuMix L{} R{} M{:X}", left, right, active_mask),
                DebugEvent::ApuPcmFramesBuffered {
                    frames,
                    buffered_frames,
                    ..
                } => format!("ApuPCM +{} buf={}", frames, buffered_frames),
                DebugEvent::ApuPcmBufferWrapped {
                    dropped_frames,
                    dropped_total,
                    ..
                } => format!("ApuDrop +{} total={}", dropped_frames, dropped_total),
                DebugEvent::TimerInterrupt { tima, .. } => format!("TimerIRQ TIMA={}", tima),
                DebugEvent::TimerOverflow { old_tima, .. } => {
                    format!("TimerOverflow {:02X}", old_tima)
                }
                DebugEvent::TimerReload { reloaded_tima, .. } => {
                    format!("TimerReload {:02X}", reloaded_tima)
                }
                DebugEvent::TimerControlWrite {
                    old_tac, new_tac, ..
                } => format!("TAC {:02X}->{:02X}", old_tac, new_tac),
                DebugEvent::DivResetEdge { old_div, .. } => {
                    format!("DIV reset from {:04X}", old_div)
                }
                DebugEvent::SerialTransferStart { sb, .. } => format!("SerialStart {:02X}", sb),
                DebugEvent::SerialTransferComplete { sb, .. } => format!("SerialDone {:02X}", sb),
                DebugEvent::JoypadEdge {
                    old_mask, new_mask, ..
                } => format!("Joypad {:02X}->{:02X}", old_mask, new_mask),
                DebugEvent::JoypadRead {
                    p1, select, mask, ..
                } => {
                    format!("P1 read {:02X} sel={:02X} m={:02X}", p1, select, mask)
                }
                DebugEvent::JoypadSelectionWrite { old_p1, new_p1, .. } => {
                    format!("P1 {:02X}->{:02X}", old_p1, new_p1)
                }
                DebugEvent::JoypadInterrupt { p1, .. } => format!("JoypadIRQ {:02X}", p1),
                DebugEvent::InterruptRequested { source, .. } => format!("IRQ req {}", source),
                DebugEvent::InterruptServiced { source, vector, .. } => {
                    format!("IRQ svc {}->{:04X}", source, vector)
                }
                DebugEvent::InterruptPendingBlocked {
                    pending_mask, ime, ..
                } => format!("IRQ blocked {:02X} ime={}", pending_mask, ime),
                DebugEvent::ExecutionStop {
                    kind,
                    label,
                    source,
                    ..
                } => match source {
                    Some(source) => format!("Stop {} {} @{}", kind, label, source),
                    None => format!("Stop {} {}", kind, label),
                },
                DebugEvent::SymbolContextChange {
                    symbol,
                    source,
                    rom_bank,
                    pc,
                    ..
                } => {
                    format!(
                        "Ctx {:02X}:{:04X} {} {}",
                        rom_bank,
                        pc,
                        symbol.clone().unwrap_or_else(|| "<anon>".into()),
                        source.clone().unwrap_or_else(|| "<nosrc>".into())
                    )
                }
                DebugEvent::UnsupportedOpcode {
                    opcode,
                    pc,
                    rom_bank,
                    source,
                    ..
                } => match source {
                    Some(source) => format!(
                        "UnsupportedOpcode 0x{:02X} @{:02X}:{:04X} {}",
                        opcode, rom_bank, pc, source
                    ),
                    None => format!(
                        "UnsupportedOpcode 0x{:02X} @{:02X}:{:04X}",
                        opcode, rom_bank, pc
                    ),
                },
                DebugEvent::ReplayCheckpointSaved {
                    checkpoint_index,
                    generation,
                    frame_hash,
                    ..
                } => format!(
                    "ReplayCheckpoint #{} g{} {:08X}",
                    checkpoint_index, generation, frame_hash
                ),
                DebugEvent::ReplayRewindApplied {
                    requested_frames,
                    from_frame,
                    to_frame,
                    checkpoint_index,
                    generation,
                    ..
                } => format!(
                    "ReplayRewind -{} {}->{} #{} g{}",
                    requested_frames, from_frame, to_frame, checkpoint_index, generation
                ),
                DebugEvent::ReplayDivergenceDetected {
                    checkpoint_index,
                    generation,
                    expected_digest,
                    actual_digest,
                    ..
                } => format!(
                    "ReplayDivergence #{} g{} {:016X}!={:016X}",
                    checkpoint_index, generation, expected_digest, actual_digest
                ),
                DebugEvent::BankThrashSuspected {
                    score,
                    bank_a,
                    bank_b,
                    ..
                } => format!("BankThrash {} {}<->{}", score, bank_a, bank_b),
            })
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    fn delta_summary(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(stop_reason) = &self.last_stop_reason {
            out.push(format!(
                "Execution stopped by debugger: {}",
                stop_reason.label
            ));
        }
        if self.halted_on_unsupported_opcode {
            out.push("Execution stopped on an unsupported opcode".to_string());
        }
        if self.bg_changed {
            out.push("BG layer changed".to_string());
        }
        if self.window_changed {
            out.push("Window layer changed".to_string());
        }
        if self.sprite_changed {
            out.push("Sprite layer changed".to_string());
        }
        if self.screen_changed && !self.bg_changed && !self.window_changed && !self.sprite_changed {
            out.push("Framebuffer changed during run window".to_string());
        } else if !self.bg_changed && !self.window_changed && !self.sprite_changed {
            out.push("No visual layer change detected".to_string());
        }
        if self.wait_before_present {
            out.push("WaitVBlank observed before Present".to_string());
        }
        if self.bank_restored {
            out.push("No unresolved far-call return observed".to_string());
        } else {
            out.push("Far-call return remained unresolved".to_string());
        }
        if self.input_path_count > 0 {
            out.push(format!("Input path hit {} time(s)", self.input_path_count));
        }
        if self.max_repeated_pc_hits >= 16 {
            out.push(format!("Hot loop score {}", self.max_repeated_pc_hits));
        }
        out
    }

    fn input_hints(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.halted_on_unsupported_opcode {
            out.push("Execution stopped on an unsupported opcode; inspect report.unsupported_opcodes to prioritize the next CPU implementation batch.".to_string());
        }
        if self.wait_vblank_count > 0 && !self.wait_before_present {
            out.push(
                "Try START/A and then a short NONE delay to cross WaitVBlank before Present."
                    .to_string(),
            );
        }
        if self.input_path_count > 0 && self.max_repeated_pc_hits >= 16 {
            out.push("Pad_Read-like paths are active but progress is poor; hold a direction for 8-16 frames instead of tapping.".to_string());
        }
        if self.present_count > 0
            && !self.screen_changed
            && !self.bg_changed
            && !self.window_changed
            && !self.sprite_changed
        {
            out.push("Present-like path ran without layer changes; try input that changes menu focus or confirms a transition.".to_string());
        }
        if self.oam_transfer_count > 0 && !self.sprite_changed {
            out.push("OAM transfer occurred without sprite-layer change; try waiting 1-2 frames after movement or confirm input.".to_string());
        }
        if !self.bank_restored {
            out.push("A far-call return remained unresolved beyond the grace period; stop on the first unmatched thunk transition.".to_string());
        }
        if self.oam_dma_start_count > 0 || self.hdma_start_count > 0 {
            out.push("DMA edge detected; add dma_reg/hdma_regs + oam_live/vbk_reg watch windows and snapshot on oam_dma, gdma_stall, or hdma_block.".to_string());
        }
        if self.hdma_deferred_count > 0 {
            out.push("HBlank HDMA deferred while CPU was halted; add snapshots on hdma_deferred and compare with HALT/interrupt wake-up edges.".to_string());
        }
        if self.serial_transfer_count > 0 && self.serial_interrupt_count == 0 {
            out.push("Serial transfer started but no serial IRQ arrived; add snapshots on serial and irq_blocked, or extend the NONE stage.".to_string());
        }
        if self.joypad_edge_count > 0 && self.joypad_interrupt_count == 0 {
            out.push("Joypad edges were observed without a joypad IRQ; snapshot on joypad_edge, joypad_irq, and irq_service while varying FF00 selection timing.".to_string());
        }
        if self.joypad_read_count > 0
            && !self.screen_changed
            && !self.bg_changed
            && !self.window_changed
            && !self.sprite_changed
        {
            out.push("FF00/P1 was polled in this run window but no visible layer change followed; prefer staged jobs and snapshot on joypad_read plus the current PC symbol.".to_string());
        }
        if self.joypad_read_count > 0
            && self.joypad_button_read_count == 0
            && self.joypad_dpad_read_count > 0
        {
            out.push("Only dpad-row P1 reads were observed; if START/A/B appear ineffective, snapshot on joypad_read and verify FF00 row selection.".to_string());
        }
        if self.joypad_read_count > 0
            && self.joypad_dpad_read_count == 0
            && self.joypad_button_read_count > 0
        {
            out.push("Only button-row P1 reads were observed; if directional input appears ineffective, snapshot on joypad_read and verify FF00 row selection.".to_string());
        }
        if self.interrupt_pending_blocked_count > 0 && self.interrupt_service_count == 0 {
            out.push("Interrupts became pending but never serviced; inspect IME / HALT timing and snapshot on irq_blocked plus timer or joypad events.".to_string());
        }
        if out.is_empty() {
            out.push("Try A/START, then 4-8 frames of NONE, then a directional input to probe the next state change.".to_string());
        }
        out
    }

    fn refresh_watch_window_baselines(&mut self) {
        let baselines: Vec<Vec<u8>> = self
            .watch_windows
            .iter()
            .map(|spec| self.read_watch_window(spec.addr, spec.size))
            .collect();
        self.watch_window_baselines = baselines.clone();
        self.watch_previous_frame_baselines = baselines;
    }

    fn active_watch_window_baselines(&self) -> Option<&[Vec<u8>]> {
        match self.watch_baseline_mode {
            MemoryWatchBaselineMode::Initial => Some(&self.watch_window_baselines),
            MemoryWatchBaselineMode::PreviousFrame => Some(&self.watch_previous_frame_baselines),
            MemoryWatchBaselineMode::Named => self
                .watch_named_baseline_active
                .as_ref()
                .and_then(|name| self.watch_named_baselines.get(name))
                .map(|baselines| baselines.as_slice()),
        }
    }

    fn refresh_stop_watchpoint_baselines(&mut self) {
        self.stop_watchpoint_baselines = self
            .stop_conditions
            .watchpoints
            .iter()
            .map(|spec| self.read_watch_window(spec.addr, spec.size))
            .collect();
    }

    fn read_mmio_stop_values(&self) -> Vec<u8> {
        self.stop_conditions
            .mmio_writes
            .iter()
            .map(|spec| self.machine.peek8(spec.addr))
            .collect()
    }

    fn current_stop_reason_context(&self, kind: &str, label: String, detail: String) -> StopReason {
        StopReason {
            kind: kind.to_string(),
            label,
            detail,
            frame: self.machine.clocks.frames,
            cycle: self.machine.clocks.cycles,
            pc: self.machine.cpu.pc,
            rom_bank: self.machine.current_rom_bank(),
            symbol: self.current_pc_symbol(),
            source: self.current_source_location(),
            execution_context_trail: self.execution_context_trail(),
        }
    }

    fn record_stop_reason(&mut self, reason: StopReason) {
        self.event_log.push(DebugEvent::ExecutionStop {
            frame: reason.frame,
            cycle: reason.cycle,
            kind: reason.kind.clone(),
            label: reason.label.clone(),
            detail: reason.detail.clone(),
            pc: reason.pc,
            rom_bank: reason.rom_bank,
            symbol: reason.symbol.clone(),
            source: reason.source.as_ref().map(|s| match s.column {
                Some(col) => format!("{}:{}:{}", s.path, s.line, col),
                None => format!("{}:{}", s.path, s.line),
            }),
        });
        self.last_stop_reason = Some(reason);
        self.stopped_by_debugger = true;
    }

    fn current_replay_watch_hashes(&self) -> (u64, Vec<ReplayWatchDigestReport>) {
        let mut digest = 0xcbf29ce484222325u64;
        let mut reports = Vec::with_capacity(self.watch_windows.len());
        for spec in &self.watch_windows {
            let bytes = self.read_watch_window(spec.addr, spec.size);
            let hash = hash_bytes(&bytes);
            digest = digest.rotate_left(5) ^ u64::from(hash).wrapping_mul(0x100000001b3);
            digest ^= u64::from(spec.addr) << 16;
            digest ^= u64::from(spec.size);
            reports.push(ReplayWatchDigestReport {
                name: spec.name.clone(),
                start: spec.addr,
                size: spec.size,
                hash,
            });
        }
        (digest, reports)
    }

    fn compute_replay_digest(&self, frame_hash: u32, watch_digest: u64) -> u64 {
        let cpu = &self.machine.cpu;
        let values = [
            frame_hash as u64,
            compute_vram_hash(&self.machine) as u64,
            compute_oam_hash(&self.machine) as u64,
            watch_digest,
            self.machine.current_rom_bank() as u64,
            self.machine.clocks.frames,
            self.machine.clocks.cycles,
            cpu.pc as u64,
            cpu.sp as u64,
            cpu.a as u64,
            cpu.f.0 as u64,
            cpu.b as u64,
            cpu.c as u64,
            cpu.d as u64,
            cpu.e as u64,
            cpu.h as u64,
            cpu.l as u64,
            self.machine.interrupt.iflag as u64,
            self.machine.interrupt.ie as u64,
        ];
        values
            .into_iter()
            .fold(0xcbf29ce484222325u64, |acc, value| {
                acc.rotate_left(5) ^ value.wrapping_mul(0x100000001b3)
            })
    }

    fn maybe_record_replay_checkpoint(&mut self) {
        if !self.replay_control.enabled {
            return;
        }
        let interval = self.replay_control.checkpoint_interval_frames.max(1);
        let current_frame = self.machine.clocks.frames;
        let current_cycle = self.machine.clocks.cycles;
        let should_capture = self.last_stop_reason.is_some() || current_frame % interval == 0;
        if !should_capture {
            return;
        }
        if self.replay_last_checkpoint_key == Some((current_frame, current_cycle)) {
            return;
        }
        let frame_hash = compute_frame_hash(&self.machine);
        let (watch_digest, watch_hashes) = self.current_replay_watch_hashes();
        let digest = self.compute_replay_digest(frame_hash, watch_digest);
        let checkpoint_index = self
            .replay_checkpoints
            .back()
            .map(|c| c.checkpoint_index + 1)
            .unwrap_or(0);
        let checkpoint = ReplayCheckpoint {
            checkpoint_index,
            generation: self.replay_generation,
            frame: current_frame,
            cycle: current_cycle,
            rom_bank: self.machine.current_rom_bank(),
            pc: self.machine.cpu.pc,
            frame_hash,
            digest,
            symbol: self.current_pc_symbol(),
            source: self.current_source_location(),
            watch_hashes,
            watch_digest,
            event_log_len: self.event_log.len(),
            instruction_samples_total: self.executed_instruction_samples,
            state: self.machine.save_state(),
        };
        self.replay_checkpoints.push_back(checkpoint.clone());
        while self.replay_checkpoints.len() > self.replay_control.max_checkpoints {
            self.replay_checkpoints.pop_front();
        }
        self.replay_last_checkpoint_key = Some((current_frame, current_cycle));
        self.replay_checkpoint_count = self.replay_checkpoint_count.saturating_add(1);
        if let Some((previous_generation, previous_digest)) = self
            .replay_frame_digests
            .get(&current_frame)
            .map(|previous| (previous.generation, previous.digest))
        {
            if previous_generation != self.replay_generation && previous_digest != digest {
                self.replay_divergence_count = self.replay_divergence_count.saturating_add(1);
                let divergence = ReplayDivergenceReport {
                    frame: current_frame,
                    cycle: current_cycle,
                    checkpoint_index,
                    generation: self.replay_generation,
                    expected_digest: previous_digest,
                    actual_digest: digest,
                };
                self.replay_divergence = Some(divergence.clone());
                self.event_log.push(DebugEvent::ReplayDivergenceDetected {
                    frame: current_frame,
                    cycle: current_cycle,
                    checkpoint_index,
                    generation: self.replay_generation,
                    expected_digest: previous_digest,
                    actual_digest: digest,
                });
                if self.replay_control.stop_on_divergence && self.last_stop_reason.is_none() {
                    self.record_stop_reason(self.current_stop_reason_context(
                        "replay_divergence",
                        format!("replay:frame:{}", current_frame),
                        format!(
                            "replay divergence detected at frame {} (expected digest {:016X}, got {:016X})",
                            current_frame,
                            previous_digest,
                            digest
                        ),
                    ));
                }
            }
        } else {
            self.replay_frame_digests.insert(
                current_frame,
                HistoricalReplayDigest {
                    digest,
                    generation: self.replay_generation,
                },
            );
        }
        self.event_log.push(DebugEvent::ReplayCheckpointSaved {
            frame: current_frame,
            cycle: current_cycle,
            checkpoint_index,
            generation: self.replay_generation,
            digest,
            frame_hash,
            rom_bank: self.machine.current_rom_bank(),
            pc: self.machine.cpu.pc,
        });
    }

    fn check_execute_breakpoints_before_step(
        &self,
        pc: u16,
        rom_bank: u16,
        symbol: Option<&str>,
    ) -> Option<StopReason> {
        self.stop_conditions
            .breakpoints
            .iter()
            .find(|spec| spec.matches(rom_bank, pc, symbol))
            .map(|spec| {
                self.current_stop_reason_context(
                    "breakpoint",
                    spec.label(),
                    format!(
                        "execute breakpoint hit at {:04X} in ROM bank {}",
                        pc, rom_bank
                    ),
                )
            })
    }

    fn interrupt_source_matches(expected: Option<&str>, actual: InterruptSource) -> bool {
        match expected.unwrap_or("any") {
            "any" => true,
            "vblank" => actual == InterruptSource::Vblank,
            "lcd_stat" => actual == InterruptSource::LcdStat,
            "timer" => actual == InterruptSource::Timer,
            "serial" => actual == InterruptSource::Serial,
            "joypad" => actual == InterruptSource::Joypad,
            _ => false,
        }
    }

    fn check_watchpoints_after_step(&mut self) -> Option<StopReason> {
        let specs = self.stop_conditions.watchpoints.clone();
        for (idx, spec) in specs.iter().enumerate() {
            let current = self.read_watch_window(spec.addr, spec.size);
            let baseline = self
                .stop_watchpoint_baselines
                .get(idx)
                .cloned()
                .unwrap_or_else(|| current.clone());
            if baseline != current {
                let changed_bytes = baseline
                    .iter()
                    .zip(current.iter())
                    .filter(|(before, after)| before != after)
                    .count();
                if idx < self.stop_watchpoint_baselines.len() {
                    self.stop_watchpoint_baselines[idx] = current;
                }
                return Some(self.current_stop_reason_context(
                    "watchpoint",
                    spec.label(),
                    format!(
                        "watchpoint observed {} changed byte(s) at {:04X}",
                        changed_bytes, spec.addr
                    ),
                ));
            }
            if idx < self.stop_watchpoint_baselines.len() {
                self.stop_watchpoint_baselines[idx] = current;
            }
        }
        None
    }

    fn check_mmio_stops_after_step(&self, before_values: &[u8]) -> Option<StopReason> {
        for (idx, spec) in self.stop_conditions.mmio_writes.iter().enumerate() {
            let before = before_values
                .get(idx)
                .copied()
                .unwrap_or_else(|| self.machine.peek8(spec.addr));
            let after = self.machine.peek8(spec.addr);
            if before != after {
                return Some(self.current_stop_reason_context(
                    "mmio_write",
                    spec.label(),
                    format!(
                        "MMIO {:04X} changed {:02X}->{:02X}",
                        spec.addr, before, after
                    ),
                ));
            }
        }
        None
    }

    fn check_interrupt_stops_after_step(
        &self,
        trace: &[InterruptTraceEvent],
    ) -> Option<StopReason> {
        for spec in &self.stop_conditions.interrupts {
            let expected_source = spec.source.as_deref();
            for event in trace {
                match *event {
                    InterruptTraceEvent::Requested { source, iflag }
                        if spec.phase == InterruptStopPhase::Any
                            || spec.phase == InterruptStopPhase::Requested =>
                    {
                        if Self::interrupt_source_matches(expected_source, source) {
                            return Some(self.current_stop_reason_context(
                                "interrupt",
                                spec.label(),
                                format!(
                                    "interrupt {} requested (IF={:02X})",
                                    interrupt_source_name(source),
                                    iflag
                                ),
                            ));
                        }
                    }
                    InterruptTraceEvent::Serviced { source, vector }
                        if spec.phase == InterruptStopPhase::Any
                            || spec.phase == InterruptStopPhase::Serviced =>
                    {
                        if Self::interrupt_source_matches(expected_source, source) {
                            return Some(self.current_stop_reason_context(
                                "interrupt",
                                spec.label(),
                                format!(
                                    "interrupt {} serviced -> {:04X}",
                                    interrupt_source_name(source),
                                    vector
                                ),
                            ));
                        }
                    }
                    InterruptTraceEvent::PendingBlocked {
                        pending_mask,
                        ime,
                        halted,
                    } if spec.phase == InterruptStopPhase::Any
                        || spec.phase == InterruptStopPhase::Blocked =>
                    {
                        let source_matches = match expected_source.unwrap_or("any") {
                            "any" => true,
                            "vblank" => pending_mask & 0x01 != 0,
                            "lcd_stat" => pending_mask & 0x02 != 0,
                            "timer" => pending_mask & 0x04 != 0,
                            "serial" => pending_mask & 0x08 != 0,
                            "joypad" => pending_mask & 0x10 != 0,
                            _ => false,
                        };
                        if source_matches {
                            return Some(self.current_stop_reason_context(
                                "interrupt",
                                spec.label(),
                                format!(
                                    "interrupt pending but blocked (mask={:02X} ime={} halted={})",
                                    pending_mask, ime, halted
                                ),
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }

    fn check_dma_stops_after_step(&self, trace: &[DmaTraceEvent]) -> Option<StopReason> {
        for spec in &self.stop_conditions.dma_events {
            for event in trace {
                let matched = match (spec.event.as_str(), event) {
                    ("oam_start", DmaTraceEvent::OamDmaStart { .. }) => true,
                    ("oam_complete", DmaTraceEvent::OamDmaComplete { .. }) => true,
                    ("hdma_start", DmaTraceEvent::HdmaStart { .. }) => true,
                    ("hdma_block", DmaTraceEvent::HdmaBlock { .. }) => true,
                    ("hdma_complete", DmaTraceEvent::HdmaComplete { .. }) => true,
                    ("hdma_cancel", DmaTraceEvent::HdmaCancel { .. }) => true,
                    ("gdma_stall", DmaTraceEvent::GdmaStallEstimate { .. }) => true,
                    ("hdma_deferred", DmaTraceEvent::HdmaDeferred { .. }) => true,
                    ("hdma_ignored", DmaTraceEvent::HdmaWriteIgnored { .. }) => true,
                    _ => false,
                };
                if matched {
                    return Some(self.current_stop_reason_context(
                        "dma",
                        spec.label(),
                        format!("DMA stop event '{}' observed", spec.event),
                    ));
                }
            }
        }
        None
    }

    fn read_watch_window(&self, addr: u16, size: u16) -> Vec<u8> {
        (0..size)
            .map(|offset| self.machine.peek8(addr.wrapping_add(offset)))
            .collect()
    }

    fn build_watch_insights(bytes: &[u8], baseline: Option<&[u8]>) -> Vec<MemoryWatchInsight> {
        let preview = &bytes[..bytes.len().min(16)];
        let mut insights = Vec::new();
        if preview.is_empty() {
            return insights;
        }

        let hex_rows = preview
            .chunks(8)
            .map(|row| {
                row.iter()
                    .map(|byte| format!("{byte:02X}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join(" | ");
        insights.push(MemoryWatchInsight {
            kind: "hex_rows".to_string(),
            summary: hex_rows,
        });

        let ascii = preview
            .iter()
            .map(|&byte| {
                if byte.is_ascii_graphic() || byte == b' ' {
                    byte as char
                } else {
                    '.'
                }
            })
            .collect::<String>();
        insights.push(MemoryWatchInsight {
            kind: "ascii_preview".to_string(),
            summary: ascii,
        });

        if preview.len() <= 4 {
            let mut le = 0u32;
            for (idx, byte) in preview.iter().enumerate() {
                le |= u32::from(*byte) << (idx * 8);
            }
            insights.push(MemoryWatchInsight {
                kind: "scalar_le".to_string(),
                summary: format!("{le}"),
            });

            let mut be = 0u32;
            for byte in preview {
                be = (be << 8) | u32::from(*byte);
            }
            insights.push(MemoryWatchInsight {
                kind: "scalar_be".to_string(),
                summary: format!("{be}"),
            });
        }

        if let Some(saved) = baseline {
            let changed_offsets = bytes
                .iter()
                .enumerate()
                .filter_map(|(idx, byte)| saved.get(idx).filter(|&&old| old != *byte).map(|_| idx))
                .take(8)
                .map(|idx| format!("{idx:02X}"))
                .collect::<Vec<_>>();
            if !changed_offsets.is_empty() {
                insights.push(MemoryWatchInsight {
                    kind: "changed_offsets".to_string(),
                    summary: changed_offsets.join(" "),
                });
            }
        }

        insights
    }

    fn watched_memory_results(&self) -> Vec<MemoryWatchResult> {
        const WATCH_PREVIEW_LIMIT: usize = 16;
        const WATCH_DIFF_PREVIEW_LIMIT: usize = 16;

        let active_baselines = self.active_watch_window_baselines();

        self.watch_windows
            .iter()
            .enumerate()
            .map(|(idx, spec)| {
                let bytes = self.read_watch_window(spec.addr, spec.size);
                let baseline = active_baselines.and_then(|saved| saved.get(idx));
                let nonzero_bytes = bytes.iter().filter(|&&byte| byte != 0).count() as u32;
                let mut changed_bytes = 0u32;
                let mut first_change_addr = None;
                let mut diff_preview = Vec::new();

                for (offset, &byte) in bytes.iter().enumerate() {
                    let baseline_byte = baseline
                        .and_then(|saved| saved.get(offset))
                        .copied()
                        .unwrap_or(byte);
                    if baseline_byte != byte {
                        changed_bytes += 1;
                        if first_change_addr.is_none() {
                            first_change_addr = Some(spec.addr.wrapping_add(offset as u16));
                        }
                        if diff_preview.len() < WATCH_DIFF_PREVIEW_LIMIT {
                            diff_preview.push(crate::watch::MemoryWatchDiffByte {
                                offset: offset as u16,
                                addr: spec.addr.wrapping_add(offset as u16),
                                before: baseline_byte,
                                after: byte,
                            });
                        }
                    }
                }

                let preview_len = bytes.len().min(WATCH_PREVIEW_LIMIT);
                let preview_bytes = bytes[..preview_len].to_vec();
                let preview_truncated = bytes.len() > preview_len;
                let baseline_preview_bytes = baseline
                    .map(|saved| saved[..saved.len().min(WATCH_PREVIEW_LIMIT)].to_vec())
                    .unwrap_or_default();
                let diff_preview_truncated = changed_bytes as usize > diff_preview.len();
                let watch_insights =
                    Self::build_watch_insights(&bytes, baseline.map(|saved| saved.as_slice()));

                MemoryWatchResult {
                    name: spec.name.clone(),
                    addr: spec.addr,
                    size: spec.size,
                    hash: hash_bytes(&bytes),
                    nonzero_bytes,
                    changed: changed_bytes > 0,
                    changed_bytes,
                    first_change_addr,
                    preview_bytes,
                    preview_truncated,
                    baseline_preview_bytes,
                    diff_preview,
                    diff_preview_truncated,
                    watch_insights,
                }
            })
            .collect()
    }

    pub fn save_state(&self) -> MachineState {
        self.machine.save_state()
    }

    pub fn load_state(&mut self, state: &MachineState) {
        self.machine.load_state(state);
        self.replay_checkpoints.clear();
        self.replay_frame_digests.clear();
        self.replay_generation = 0;
        self.replay_last_checkpoint_key = None;
        self.replay_last_rewind = None;
        self.replay_divergence = None;
        self.previous_frame_hash = self.last_frame_hash;
        self.last_frame_hash = Some(compute_frame_hash(&self.machine));
        self.previous_oam_hash = Some(compute_oam_hash(&self.machine));
        self.previous_bg_hash = Some(compute_bg_hash(&self.machine));
        self.previous_window_hash = Some(compute_window_hash(&self.machine));
        self.previous_sprite_hash = Some(compute_sprite_hash(&self.machine));
        self.vblank_count = 0;
        self.timer_interrupt_count = 0;
        self.timer_overflow_count = 0;
        self.timer_reload_count = 0;
        self.timer_control_write_count = 0;
        self.div_reset_count = 0;
        self.serial_interrupt_count = 0;
        self.serial_transfer_count = 0;
        self.joypad_interrupt_count = 0;
        self.joypad_read_count = 0;
        self.joypad_button_read_count = 0;
        self.joypad_dpad_read_count = 0;
        self.joypad_edge_count = 0;
        self.joypad_selection_write_count = 0;
        self.interrupt_service_count = 0;
        self.interrupt_pending_blocked_count = 0;
        self.bank_switch_count = 0;
        self.far_call_count = 0;
        self.intrinsic_count = 0;
        self.settile_flush_count = 0;
        self.buffered_settile_count = 0;
        self.cgb_settile_count = 0;
        self.flush_rows_count = 0;
        self.oam_dma_count = 0;
        self.oam_dma_start_count = 0;
        self.oam_dma_complete_count = 0;
        self.hdma_start_count = 0;
        self.hdma_block_count = 0;
        self.hdma_complete_count = 0;
        self.hdma_cancel_count = 0;
        self.gdma_stall_cycles_estimate = 0;
        self.hdma_stall_cycles_estimate = 0;
        self.hdma_deferred_count = 0;
        self.hdma_ignored_write_count = 0;
        self.wait_vblank_count = 0;
        self.present_count = 0;
        self.cgb_palette_count = 0;
        self.bank_guard_count = 0;
        self.trap_check_count = 0;
        self.oam_transfer_count = 0;
        self.input_path_count = 0;
        self.unsupported_opcode_count = 0;
        self.lcd_toggle_count = 0;
        self.stat_write_count = 0;
        self.lyc_write_count = 0;
        self.scanline_render_count = 0;
        self.replay_checkpoint_count = 0;
        self.replay_rewind_count = 0;
        self.replay_divergence_count = 0;
        self.replay_last_rewind = None;
        self.replay_divergence = None;
        self.bank_thrash_score = 0;
        self.bank_thrash_streak = 0;
        self.max_repeated_pc_hits = 0;
        self.halted_on_unsupported_opcode = false;
        self.last_stop_reason = None;
        self.stopped_by_debugger = false;
        self.unsupported_opcodes.clear();
        self.recent_pcs.clear();
        self.pc_hit_histogram.clear();
        self.pc_cycle_histogram.clear();
        self.executed_instruction_samples = 0;
        self.execution_context_trail.clear();
        self.last_iflag = self.machine.interrupt.iflag;
        self.last_symbol_name = self.current_pc_symbol();
        self.last_source_label = self.current_source_label();
        self.start_frame_counter = self.machine.clocks.frames;
        self.start_cycle_counter = self.machine.clocks.cycles;
        self.start_rom_bank = self.machine.current_rom_bank();
        self.start_pc = self.machine.cpu.pc;
        self.end_rom_bank = self.start_rom_bank;
        self.bank_restored = true;
        self.last_intrinsic = None;
        self.last_far_call_symbol = None;
        self.last_bank_switch_from_symbol = None;
        self.last_bank_switch_to_symbol = None;
        self.previous_bank_switch = None;
        self.pending_bank_returns.clear();
        self.seen_wait_vblank = false;
        self.wait_before_present = false;
        self.screen_changed = false;
        self.vram_changed = false;
        self.oam_changed = false;
        self.bg_hash_changed = false;
        self.window_hash_changed = false;
        self.sprite_hash_changed = false;
        self.bg_changed = false;
        self.window_changed = false;
        self.sprite_changed = false;
        self.last_ly = self.machine.ppu.ly;
        self.last_ppu_mode = self.machine.ppu.current_mode();
        self.last_stat_coincidence = self.machine.ppu.stat_coincidence();
        self.observed_frame_start = Some(self.machine.clocks.frames);
        self.scanline_event_count = 0;
        self.stat_signal_count = 0;
        self.refresh_watch_window_baselines();
        self.refresh_stop_watchpoint_baselines();
    }

    fn decode_dmg_palette(value: u8) -> Vec<u8> {
        (0..4).map(|shift| (value >> (shift * 2)) & 0x03).collect()
    }

    fn build_tile_preview(&self, bank: u8) -> (u16, Vec<VisualizationTilePreviewReport>) {
        let vram = self.machine.memory.vram_bank(bank);
        let tile_region = &vram[..0x1800.min(vram.len())];
        let mut nonzero_tiles = 0u16;
        let mut preview = Vec::new();
        for (index, chunk) in tile_region.chunks(16).enumerate() {
            let nonzero_bytes = chunk.iter().filter(|&&b| b != 0).count() as u8;
            if nonzero_bytes > 0 {
                nonzero_tiles = nonzero_tiles.saturating_add(1);
                if preview.len() < 8 {
                    preview.push(VisualizationTilePreviewReport {
                        bank,
                        tile_index: index as u16,
                        nonzero_bytes,
                        hash: hash_bytes(chunk),
                    });
                }
            }
        }
        (nonzero_tiles, preview)
    }

    fn build_visualizations(&self) -> Option<VisualizationsReport> {
        let (bank0_nonzero_tiles, bank0_preview) = self.build_tile_preview(0);
        let (bank1_nonzero_tiles, bank1_preview) = self.build_tile_preview(1);
        let oam = self.machine.memory.oam();
        let mut sample_sprites = Vec::new();
        for (slot, chunk) in oam.chunks_exact(4).enumerate() {
            if chunk.iter().any(|&b| b != 0) {
                sample_sprites.push(VisualizationSpriteReport {
                    slot: slot as u8,
                    y: chunk[0],
                    x: chunk[1],
                    tile: chunk[2],
                    attrs: chunk[3],
                });
                if sample_sprites.len() >= 8 {
                    break;
                }
            }
        }
        let cgb_bg = (0..8)
            .map(|palette| VisualizationPalettePreviewReport {
                palette_index: palette,
                colors_rgb555: (0..4)
                    .map(|color| self.machine.memory.bg_palette_rgb555(palette, color))
                    .collect(),
            })
            .collect();
        let cgb_obj = (0..8)
            .map(|palette| VisualizationPalettePreviewReport {
                palette_index: palette,
                colors_rgb555: (0..4)
                    .map(|color| self.machine.memory.obj_palette_rgb555(palette, color))
                    .collect(),
            })
            .collect();
        let wave_preview = (0xFF30..=0xFF3F)
            .map(|addr| self.machine.apu.read(addr))
            .collect();
        Some(VisualizationsReport {
            generated: true,
            bank_state: VisualizationBankStateReport {
                current_rom_bank: self.machine.current_rom_bank(),
                current_ram_bank: self.machine.current_ram_bank(),
                active_vram_bank: self.machine.memory.active_vram_bank(),
                active_wram_bank: self.machine.memory.active_wram_bank(),
                vbk_register: self.machine.memory.vbk_register(),
                svbk_register: self.machine.memory.svbk_register(),
                cgb_mode: matches!(self.machine.mode, kokura_core::types::HardwareMode::Cgb),
                cgb_double_speed: self.machine.cgb_double_speed,
            },
            layers: vec![
                VisualizationLayerReport {
                    key: "bg".to_string(),
                    enabled: self.machine.ppu.bg_enabled(),
                    changed: self.bg_changed,
                    hash: compute_bg_hash(&self.machine),
                    detail: format!("base=${:04X} sc=({}, {})", 0x8000u16 + self.machine.ppu.bg_map_base() as u16, self.machine.ppu.scx, self.machine.ppu.scy),
                },
                VisualizationLayerReport {
                    key: "window".to_string(),
                    enabled: self.machine.ppu.window_enabled(),
                    changed: self.window_changed,
                    hash: compute_window_hash(&self.machine),
                    detail: format!("base=${:04X} wx={} wy={}", 0x8000u16 + self.machine.ppu.window_map_base() as u16, self.machine.ppu.wx, self.machine.ppu.wy),
                },
                VisualizationLayerReport {
                    key: "sprite".to_string(),
                    enabled: self.machine.ppu.sprite_enabled(),
                    changed: self.sprite_changed,
                    hash: compute_sprite_hash(&self.machine),
                    detail: format!("visible_est={} nonzero_oam={}", self.machine.ppu.visible_sprite_count_estimate(oam), self.machine.ppu.nonzero_oam_entries(oam)),
                },
            ],
            palettes: VisualizationPaletteReport {
                dmg_bgp: Self::decode_dmg_palette(self.machine.ppu.bgp),
                dmg_obp0: Self::decode_dmg_palette(self.machine.ppu.obp0),
                dmg_obp1: Self::decode_dmg_palette(self.machine.ppu.obp1),
                cgb_bg,
                cgb_obj,
                bg_palette_index: self.machine.memory.bg_palette_index_register(),
                obj_palette_index: self.machine.memory.obj_palette_index_register(),
                blocked_write_count: self.cgb_palette_blocked_count,
            },
            tiles: VisualizationTileReport {
                bg_map_base: 0x8000u16 + self.machine.ppu.bg_map_base() as u16,
                window_map_base: 0x8000u16 + self.machine.ppu.window_map_base() as u16,
                tile_data_8000: self.machine.ppu.tile_data_8000(),
                bank0_nonzero_tiles,
                bank1_nonzero_tiles,
                bank0_preview,
                bank1_preview,
            },
            oam: VisualizationOamReport {
                sprite_height: self.machine.ppu.sprite_height() as u8,
                nonzero_entries: self.machine.ppu.nonzero_oam_entries(oam),
                visible_sprite_estimate: self.machine.ppu.visible_sprite_count_estimate(oam),
                sample_sprites,
            },
            dma: VisualizationDmaReport {
                oam_dma_count: self.oam_dma_count,
                oam_dma_start_count: self.oam_dma_start_count,
                oam_dma_complete_count: self.oam_dma_complete_count,
                hdma_start_count: self.hdma_start_count,
                hdma_block_count: self.hdma_block_count,
                hdma_complete_count: self.hdma_complete_count,
                hdma_cancel_count: self.hdma_cancel_count,
                gdma_stall_cycles_estimate: self.gdma_stall_cycles_estimate,
                hdma_stall_cycles_estimate: self.hdma_stall_cycles_estimate,
                hdma_deferred_count: self.hdma_deferred_count,
                hdma_ignored_write_count: self.hdma_ignored_write_count,
            },
            apu: VisualizationApuReport {
                master_enabled: self.machine.apu.master_enabled,
                channel_enable_mask: self.machine.apu.channel_enable,
                frame_sequencer_step: self.machine.apu.frame_sequencer_step(),
                nr50: self.machine.apu.read(0xFF24),
                nr51: self.machine.apu.read(0xFF25),
                nr52: self.machine.apu.read(0xFF26),
                pcm12: self.machine.apu.pcm12(),
                pcm34: self.machine.apu.pcm34(),
                sample_rate: self.machine.apu.output_sample_rate(),
                buffered_frames: self.machine.apu.buffered_frames() as u32,
                dropped_frames: self.machine.apu.dropped_frames(),
                generated_frames: self.machine.apu.generated_frames(),
                wave_preview,
                notes: vec![
                    "visualizations.apu is a coarse state view, not a backend playback guarantee.".to_string(),
                    "Use FF76/FF77, mixer writes, and PCM queue counters together when triaging audio regressions.".to_string(),
                ],
            },
            carry_forward_notes: vec![
                "visualizations is report-surface data intended for CLI/JSON inspection first; richer UI rendering can build on top of it later.".to_string(),
                "Tile previews are hashed/nonzero summaries, not decoded pixel art previews yet.".to_string(),
                "Palette previews expose RGB555 values so the next sprint can add actual viewer rendering without changing report structure.".to_string(),
            ],
        })
    }

    fn build_release_readiness(
        &self,
        timing_packs_present: bool,
        rom_forensics_present: bool,
        visualizations_present: bool,
        auto_diagnosis_present: bool,
    ) -> Option<ReleaseReadinessReport> {
        let packaging = ReleasePackagingSurfaceReport {
            license_present: true,
            readme_present: true,
            c_header_present: true,
            python_bridge_present: true,
            samples_present: true,
            docs_present: true,
            manifest_present: true,
        };
        let smoke = ReleaseSmokeSurfaceReport {
            static_checks_only: true,
            json_schema_validated: true,
            python_bridge_py_compile_validated: true,
            c_header_export_surface_checked: true,
            rust_build_validated: false,
            workspace_tests_validated: false,
            regression_validated: false,
        };
        let stability = ReleaseStabilitySurfaceReport {
            unsupported_opcode_count: self.unsupported_opcode_count,
            diagnostics_count: self.diagnostics.len(),
            events_recorded: self.event_log.len(),
            replay_surface_available: self.replay_control.enabled
                || self.replay_checkpoint_count > 0
                || self.replay_divergence_count > 0,
            auto_diagnosis_available: auto_diagnosis_present,
            rom_forensics_available: rom_forensics_present,
            timing_pack_surface_available: timing_packs_present,
            visualization_surface_available: visualizations_present,
        };
        let mut blockers = vec![
            "Rust toolchain validation is still pending: cargo/rustc were unavailable in this environment, so build/test/release status is not proven here.".to_string(),
            "Workspace-wide regression and smoke runs have not been executed against a real built binary yet.".to_string(),
        ];
        if self.unsupported_opcode_count > 0 {
            blockers.push(format!(
                "Unsupported opcode observations remain present (count={}).",
                self.unsupported_opcode_count
            ));
        }
        let mut warnings = Vec::new();
        if self.apu_pop_risk_count > 0 {
            warnings.push(format!("APU pop-risk events observed (count={}); audio behavior still needs release-grade comparison.", self.apu_pop_risk_count));
        }
        if self.cgb_palette_blocked_count > 0 || self.cgb_speed_switch_freeze_count > 0 {
            warnings.push("CGB-specific timing and palette quirks are instrumented but still require real toolchain/runtime validation.".to_string());
        }
        if self.bank_thrash_score > 0 {
            warnings.push(format!("Bank thrash score is non-zero ({}); review ROM forensic and auto diagnosis output before release.", self.bank_thrash_score));
        }
        if self.replay_divergence_count > 0 {
            warnings.push(format!("Replay divergence was observed (count={}); deterministic playback should be checked on a real build.", self.replay_divergence_count));
        }
        let readiness_level = if blockers.is_empty() {
            "candidate"
        } else {
            "build_blocked"
        }
        .to_string();
        Some(ReleaseReadinessReport {
            generated: true,
            readiness_level,
            blockers,
            warnings,
            recommended_next_commands: vec![
                "cargo fmt --all --check".to_string(),
                "cargo build --workspace".to_string(),
                "cargo test --workspace".to_string(),
                "cargo build --workspace --release".to_string(),
                "python3 tools/pre_release_smoke.py".to_string(),
            ],
            packaging,
            smoke,
            stability,
            carry_forward_notes: vec![
                "release_readiness is a truthful static readiness surface, not proof that release validation has already passed.".to_string(),
                "A real release candidate still requires Rust-enabled build/test/regression execution outside this environment.".to_string(),
            ],
        })
    }

    pub fn report(&self) -> DebugReport {
        let pc_offset = self
            .current_pc_symbol_info()
            .map(|symbol| self.machine.cpu.pc.saturating_sub(symbol.start));
        let unsupported_opcodes = self.unsupported_opcodes.values().cloned().collect();
        let watched_memory = self.watched_memory_results();
        let current_function = self
            .current_function_info()
            .map(Self::toolchain_function_report);
        let current_static_estimate = self
            .current_static_estimate_info()
            .map(Self::toolchain_static_estimate_report);
        let symbols = Some(SymbolReport {
            pc_symbol: self.current_pc_symbol(),
            previous_pc_symbol: self.previous_pc_symbol(),
            nearest_symbol: self.nearest_symbol_name(),
            next_symbol: self.next_symbol_name(),
            pc_offset,
            current_rom_bank: self.machine.current_rom_bank(),
            current_source: self
                .current_source_location()
                .as_ref()
                .map(Self::source_location_report_from_stop),
            previous_source: self
                .previous_source_location()
                .as_ref()
                .map(Self::source_location_report_from_stop),
            nearest_source: self
                .current_source_location()
                .as_ref()
                .map(Self::source_location_report_from_stop),
            next_source: self
                .next_source_location()
                .as_ref()
                .map(Self::source_location_report_from_stop),
            current_section: self.current_section(),
            current_function,
            current_static_estimate,
            execution_context_trail: self.execution_context_trail(),
        })
        .filter(|r| {
            r.pc_symbol.is_some()
                || r.previous_pc_symbol.is_some()
                || r.nearest_symbol.is_some()
                || r.next_symbol.is_some()
                || r.current_source.is_some()
                || r.previous_source.is_some()
                || r.next_source.is_some()
                || r.current_section.is_some()
                || r.current_function.is_some()
                || r.current_static_estimate.is_some()
                || !r.execution_context_trail.is_empty()
        });
        let summary = EventSummary {
            vblank_count: self.vblank_count,
            timer_interrupt_count: self.timer_interrupt_count,
            timer_overflow_count: self.timer_overflow_count,
            timer_reload_count: self.timer_reload_count,
            timer_control_write_count: self.timer_control_write_count,
            div_reset_count: self.div_reset_count,
            serial_interrupt_count: self.serial_interrupt_count,
            serial_transfer_count: self.serial_transfer_count,
            mapper_control_write_count: self.mapper_control_write_count,
            mapper_rom_bank_change_count: self.mapper_rom_bank_change_count,
            mapper_ram_bank_change_count: self.mapper_ram_bank_change_count,
            apu_master_toggle_count: self.apu_master_toggle_count,
            apu_trigger_count: self.apu_trigger_count,
            apu_length_expire_count: self.apu_length_expire_count,
            apu_envelope_step_count: self.apu_envelope_step_count,
            apu_sweep_step_count: self.apu_sweep_step_count,
            apu_channel_disabled_count: self.apu_channel_disabled_count,
            apu_dac_change_count: self.apu_dac_change_count,
            apu_pop_risk_count: self.apu_pop_risk_count,
            apu_frame_step_count: self.apu_frame_step_count,
            apu_mixer_write_count: self.apu_mixer_write_count,
            apu_mix_output_count: self.apu_mix_output_count,
            apu_pcm_frame_count: self.apu_pcm_frame_count,
            apu_pcm_drop_count: self.apu_pcm_drop_count,
            apu_pcm_peak_buffer_frames: self.apu_pcm_peak_buffer_frames,
            apu_wave_write_count: self.apu_wave_write_count,
            apu_wave_alias_count: self.apu_wave_alias_count,
            apu_ch3_hold_count: self.apu_ch3_hold_count,
            apu_noise_lock_count: self.apu_noise_lock_count,
            cgb_mode_select_count: self.cgb_mode_select_count,
            cgb_vram_bank_switch_count: self.cgb_vram_bank_switch_count,
            cgb_wram_bank_switch_count: self.cgb_wram_bank_switch_count,
            cgb_bg_palette_write_count: self.cgb_bg_palette_write_count,
            cgb_obj_palette_write_count: self.cgb_obj_palette_write_count,
            cgb_palette_blocked_count: self.cgb_palette_blocked_count,
            cgb_key1_write_count: self.cgb_key1_write_count,
            cgb_speed_switch_count: self.cgb_speed_switch_count,
            cgb_speed_switch_freeze_count: self.cgb_speed_switch_freeze_count,
            cgb_speed_switch_freeze_cycles: self.cgb_speed_switch_freeze_cycles,
            joypad_interrupt_count: self.joypad_interrupt_count,
            joypad_read_count: self.joypad_read_count,
            joypad_button_read_count: self.joypad_button_read_count,
            joypad_dpad_read_count: self.joypad_dpad_read_count,
            joypad_edge_count: self.joypad_edge_count,
            joypad_selection_write_count: self.joypad_selection_write_count,
            interrupt_service_count: self.interrupt_service_count,
            interrupt_pending_blocked_count: self.interrupt_pending_blocked_count,
            bank_switch_count: self.bank_switch_count,
            far_call_count: self.far_call_count,
            intrinsic_count: self.intrinsic_count,
            settile_flush_count: self.settile_flush_count,
            buffered_settile_count: self.buffered_settile_count,
            cgb_settile_count: self.cgb_settile_count,
            flush_rows_count: self.flush_rows_count,
            oam_dma_count: self.oam_dma_count,
            oam_dma_start_count: self.oam_dma_start_count,
            oam_dma_complete_count: self.oam_dma_complete_count,
            hdma_start_count: self.hdma_start_count,
            hdma_block_count: self.hdma_block_count,
            hdma_complete_count: self.hdma_complete_count,
            hdma_cancel_count: self.hdma_cancel_count,
            gdma_stall_cycles_estimate: self.gdma_stall_cycles_estimate,
            hdma_stall_cycles_estimate: self.hdma_stall_cycles_estimate,
            hdma_deferred_count: self.hdma_deferred_count,
            hdma_ignored_write_count: self.hdma_ignored_write_count,
            wait_vblank_count: self.wait_vblank_count,
            present_count: self.present_count,
            cgb_palette_count: self.cgb_palette_count,
            bank_guard_count: self.bank_guard_count,
            trap_check_count: self.trap_check_count,
            oam_transfer_count: self.oam_transfer_count,
            input_path_count: self.input_path_count,
            unsupported_opcode_count: self.unsupported_opcode_count,
            lcd_toggle_count: self.lcd_toggle_count,
            stat_write_count: self.stat_write_count,
            lyc_write_count: self.lyc_write_count,
            scanline_render_count: self.scanline_render_count,
            replay_checkpoint_count: self.replay_checkpoint_count,
            replay_rewind_count: self.replay_rewind_count,
            replay_divergence_count: self.replay_divergence_count,
            halted_on_unsupported_opcode: self.halted_on_unsupported_opcode,
            wait_before_present: self.wait_before_present,
            last_intrinsic: self.last_intrinsic.clone(),
            last_far_call_symbol: self.last_far_call_symbol.clone(),
            last_bank_switch_from_symbol: self.last_bank_switch_from_symbol.clone(),
            last_bank_switch_to_symbol: self.last_bank_switch_to_symbol.clone(),
            bank_restored: self.bank_restored,
            bank_thrash_score: self.bank_thrash_score,
            hot_loop_score: self.max_repeated_pc_hits,
            sprite_visible: self
                .machine
                .ppu
                .visible_sprite_count_estimate(self.machine.memory.oam())
                > 0,
            sprite_count_estimate: self
                .machine
                .ppu
                .visible_sprite_count_estimate(self.machine.memory.oam()),
            oam_nonzero_entries: self
                .machine
                .ppu
                .nonzero_oam_entries(self.machine.memory.oam()),
            screen_changed: self.screen_changed,
            bg_changed: self.bg_changed,
            window_changed: self.window_changed,
            sprite_changed: self.sprite_changed,
            delta_summary: self.delta_summary(),
            recent_event_summary: self.recent_event_summary(),
            input_hints: self.input_hints(),
        };
        let replay = Some(ReplayReport {
            enabled: self.replay_control.enabled,
            checkpoint_interval_frames: self.replay_control.checkpoint_interval_frames,
            max_checkpoints: self.replay_control.max_checkpoints,
            checkpoints_recorded: self.replay_checkpoint_count,
            current_generation: self.replay_generation,
            checkpoints: self
                .replay_checkpoints
                .iter()
                .cloned()
                .map(|checkpoint| self.replay_checkpoint_report(&checkpoint))
                .collect(),
            last_rewind: self.replay_last_rewind.clone(),
            divergence: self.replay_divergence.clone(),
            slices: self.build_replay_slices(),
            reference_compare: None,
        })
        .filter(|replay| {
            replay.enabled || replay.divergence.is_some() || !replay.checkpoints.is_empty()
        });
        let timing_input = DiagnosticInput {
            vblank_events: self.vblank_count,
            screen_changed: self.screen_changed,
            repeated_pc_hits: self.max_repeated_pc_hits,
            executed_frames: self.machine.clocks.frames,
            timer_interrupts: self.timer_interrupt_count,
            timer_overflows: self.timer_overflow_count,
            serial_interrupts: self.serial_interrupt_count,
            joypad_interrupts: self.joypad_interrupt_count,
            joypad_read_count: self.joypad_read_count,
            joypad_button_read_count: self.joypad_button_read_count,
            joypad_dpad_read_count: self.joypad_dpad_read_count,
            interrupt_services: self.interrupt_service_count,
            interrupt_pending_blocked_count: self.interrupt_pending_blocked_count,
            apu_trigger_count: self.apu_trigger_count,
            apu_mix_output_count: self.apu_mix_output_count,
            apu_pop_risk_count: self.apu_pop_risk_count,
            apu_wave_alias_count: self.apu_wave_alias_count,
            apu_noise_lock_count: self.apu_noise_lock_count,
            apu_pcm_frame_count: self.apu_pcm_frame_count,
            apu_pcm_drop_count: self.apu_pcm_drop_count,
            apu_pcm_peak_buffer_frames: self.apu_pcm_peak_buffer_frames,
            far_call_count: self.far_call_count,
            intrinsic_count: self.intrinsic_count,
            settile_flush_count: self.settile_flush_count,
            bank_switches: self.bank_switch_count,
            bank_restored: self.bank_restored,
            bank_thrash_score: self.bank_thrash_score,
            buffered_settile_count: self.buffered_settile_count,
            cgb_settile_count: self.cgb_settile_count,
            flush_rows_count: self.flush_rows_count,
            oam_dma_count: self.oam_dma_count,
            oam_dma_start_count: self.oam_dma_start_count,
            oam_dma_complete_count: self.oam_dma_complete_count,
            hdma_start_count: self.hdma_start_count,
            hdma_block_count: self.hdma_block_count,
            hdma_complete_count: self.hdma_complete_count,
            hdma_cancel_count: self.hdma_cancel_count,
            gdma_stall_cycles_estimate: self.gdma_stall_cycles_estimate,
            hdma_stall_cycles_estimate: self.hdma_stall_cycles_estimate,
            hdma_deferred_count: self.hdma_deferred_count,
            hdma_ignored_write_count: self.hdma_ignored_write_count,
            wait_vblank_count: self.wait_vblank_count,
            cgb_palette_count: self.cgb_palette_count,
            bank_guard_count: self.bank_guard_count,
            trap_check_count: self.trap_check_count,
            oam_transfer_count: self.oam_transfer_count,
            dmg_mode: matches!(self.machine.mode, kokura_core::types::HardwareMode::Dmg),
            vram_changed: self.vram_changed,
            bg_enabled: self.machine.ppu.lcdc & 0x01 != 0,
            present_path_count: self.present_count,
            input_path_count: self.input_path_count,
            input_active: self.machine.joypad.mask != 0,
            wait_before_present: self.wait_before_present,
            oam_changed: self.oam_changed,
            visible_sprite_count: self
                .machine
                .ppu
                .visible_sprite_count_estimate(self.machine.memory.oam()),
            nonzero_oam_entries: self
                .machine
                .ppu
                .nonzero_oam_entries(self.machine.memory.oam()),
            sprite_enabled: self.machine.ppu.sprite_enabled(),
            window_enabled: self.machine.ppu.window_enabled(),
            bg_changed: self.bg_changed,
            window_changed: self.window_changed,
            sprite_changed: self.sprite_changed,
            bg_hash_changed: self.bg_hash_changed,
            window_hash_changed: self.window_hash_changed,
            sprite_hash_changed: self.sprite_hash_changed,
            stat_signal_count: self.stat_signal_count,
            scanline_event_count: self.scanline_event_count,
            unsupported_opcode_count: self.unsupported_opcode_count,
            halted_on_unsupported_opcode: self.halted_on_unsupported_opcode,
            lcd_toggle_count: self.lcd_toggle_count,
            stat_write_count: self.stat_write_count,
            lyc_write_count: self.lyc_write_count,
            scanline_render_count: self.scanline_render_count,
        };
        let profiler = self.build_profiler();
        let heatmap = self.build_heatmap(profiler.as_ref());
        let abi_verification = self.build_abi_verification();
        let visualizations = self.build_visualizations();
        let timing_packs = Some(build_timing_pack_report(timing_input, &self.diagnostics))
            .filter(|report| report.generated && !report.packs.is_empty());
        let auto_diagnosis = Some(build_auto_diagnosis(
            &self.machine,
            timing_input,
            &self.diagnostics,
            self.last_stop_reason.as_ref(),
        ))
        .filter(|report| !report.suspects.is_empty() || !report.recommended_focus.is_empty());
        let rom_forensics = self.build_rom_forensics(auto_diagnosis.as_ref());
        let release_readiness = self.build_release_readiness(
            timing_packs.is_some(),
            rom_forensics.is_some(),
            visualizations.is_some(),
            auto_diagnosis.is_some(),
        );

        DebugReport {
            schema_version: DEBUG_REPORT_SCHEMA_VERSION,
            meta: ReportMeta {
                frames_executed: self.machine.clocks.frames,
                events_recorded: self.event_log.len(),
                diagnostics_recorded: self.diagnostics.len(),
                start_rom_bank: self.start_rom_bank,
                end_rom_bank: self.end_rom_bank,
                observation_frame: self.machine.clocks.frames,
                observation_cycle: self.machine.clocks.cycles,
                observation_ly: self.machine.ppu.ly,
                observation_pc: self.machine.cpu.pc,
                observation_rom_bank: self.machine.current_rom_bank(),
                observation_basis: if self.last_stop_reason.is_some() {
                    "stop_reason".to_string()
                } else {
                    "session_end".to_string()
                },
            },
            cpu: CpuSnapshot::from(&self.machine),
            video: VideoSnapshot::from_machine(&self.machine, self.previous_frame_hash),
            timer: TimerSnapshot::from(&self.machine),
            symbols,
            toolchain_build: self.toolchain_build_report.clone(),
            summary,
            unsupported_opcodes,
            watched_memory,
            replay,
            profiler,
            heatmap,
            abi_verification,
            visualizations,
            timing_packs,
            auto_diagnosis,
            rom_forensics,
            release_readiness,
            stop_reason: self.last_stop_reason.clone(),
            events: self.event_log.clone(),
            diagnostics: self.diagnostics.clone(),
        }
    }
}

#[cfg(test)]
mod runtime_probe_tests {
    use super::*;
    use std::{fs, path::PathBuf, time::Instant};

    use kokura_bridge::map_parser::parse_map_file;
    use kokura_bridge::{SymbolInfo, SymbolTable};
    use serde_json::to_string;

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .canonicalize()
            .expect("workspace root")
    }

    fn reversi_rom_path() -> PathBuf {
        workspace_root().join("reversi").join("reversi.gbc")
    }

    fn reversi_map_path() -> PathBuf {
        workspace_root().join("reversi").join("reversi.map")
    }

    #[test]
    fn parses_kitaqgb_thunk_target_bank() {
        assert_eq!(kitaqgb_thunk_target_bank("__kq_thunk_b6_Draw"), Some(6));
        assert_eq!(kitaqgb_thunk_target_bank("__kq_thunk_b255_Test"), Some(255));
        assert_eq!(
            kitaqgb_thunk_target_bank("kq_thunk_depth_ok___kq_thunk_b4_MusicPump"),
            Some(4)
        );
        assert_eq!(kitaqgb_thunk_target_bank("Audio_Update"), None);
    }

    #[test]
    fn compiler_thunk_round_trips_do_not_raise_bank_thrash_score() {
        let mut session = DebugSession::new(Machine::new());
        for _ in 0..8 {
            session.update_bank_thrash(3, 4, true);
            session.update_bank_thrash(4, 3, true);
        }
        assert_eq!(session.bank_thrash_score, 0);
        assert_eq!(session.bank_thrash_streak, 0);
        assert!(session.previous_bank_switch.is_none());
    }

    #[test]
    fn unknown_consecutive_bank_reversals_raise_one_thrash_event() {
        let mut session = DebugSession::new(Machine::new());
        for (from, to) in [(1, 2), (2, 1), (1, 2), (2, 1), (1, 2)] {
            session.update_bank_thrash(from, to, false);
        }
        assert_eq!(session.bank_thrash_score, 4);
        assert_eq!(
            session
                .event_log
                .iter()
                .filter(|event| matches!(event, DebugEvent::BankThrashSuspected { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn active_far_call_at_short_window_end_is_not_missing() {
        let mut session = DebugSession::new(Machine::new());
        session.pending_bank_returns.push(PendingBankReturn {
            expected_bank: 3,
            started_frame: 10,
            reported: false,
        });
        session.machine.clocks.frames = 60;
        session.finalize_bank_return_tracking();
        assert!(session.bank_restored);
        assert!(!session
            .event_log
            .iter()
            .any(|event| matches!(event, DebugEvent::BankReturnMissing { .. })));
    }

    #[test]
    fn stale_far_call_return_is_reported_after_grace_period() {
        let mut session = DebugSession::new(Machine::new());
        session.pending_bank_returns.push(PendingBankReturn {
            expected_bank: 3,
            started_frame: 10,
            reported: false,
        });
        session.machine.clocks.frames = 131;
        session.finalize_bank_return_tracking();
        assert!(!session.bank_restored);
        assert!(session.event_log.iter().any(|event| matches!(
            event,
            DebugEvent::BankReturnMissing {
                expected_bank: 3,
                ..
            }
        )));
    }

    #[test]
    fn fixed_bank_symbol_lookup_ignores_current_switchable_bank() {
        let mut session = DebugSession::new(Machine::new());
        session.symbol_table = Some(SymbolTable {
            symbols: vec![SymbolInfo {
                bank: 0,
                start: 0x0100,
                end: 0x0200,
                name: "BootHelper".to_string(),
                kind: None,
                region: None,
                section: None,
            }],
            source_locations: Vec::new(),
            functions: Vec::new(),
            variables: Vec::new(),
            static_estimates: Vec::new(),
            call_edges: Vec::new(),
        });

        assert_eq!(
            session.symbol_name_at(3, 0x0100).as_deref(),
            Some("BootHelper")
        );
        assert_eq!(DebugSession::code_bank_for_pc(0x0100, 3), 0);
        assert_eq!(DebugSession::code_bank_for_pc(0x4100, 3), 3);
    }

    #[test]
    fn watched_memory_preview_bytes_are_reported() {
        let mut session = DebugSession::new(Machine::new());
        let initial_game_state = session.machine.peek8(0xFFC6);
        let initial_cursor = vec![session.machine.peek8(0xFFCC), session.machine.peek8(0xFFCD)];
        session.set_watch_windows(vec![
            MemoryWatchSpec {
                name: "game_state".to_string(),
                addr: 0xFFC6,
                size: 1,
            },
            MemoryWatchSpec {
                name: "cursor".to_string(),
                addr: 0xFFCC,
                size: 2,
            },
            MemoryWatchSpec {
                name: "board".to_string(),
                addr: 0xC000,
                size: 32,
            },
        ]);

        session.machine.write8(0xFFC6, 0x12);
        session.machine.write8(0xFFCC, 0x34);
        session.machine.write8(0xFFCD, 0x56);
        for i in 0..32u16 {
            session.machine.write8(0xC000 + i, i as u8);
        }

        let report = session.report();
        let game_state = report
            .watched_memory
            .iter()
            .find(|watch| watch.name == "game_state")
            .expect("game_state watch");
        assert_eq!(game_state.preview_bytes, vec![0x12]);
        assert!(!game_state.preview_truncated);
        assert_eq!(game_state.baseline_preview_bytes, vec![initial_game_state]);
        assert_eq!(game_state.diff_preview.len(), 1);
        assert_eq!(game_state.diff_preview[0].offset, 0);
        assert_eq!(game_state.diff_preview[0].before, initial_game_state);
        assert_eq!(game_state.diff_preview[0].after, 0x12);
        assert!(game_state
            .watch_insights
            .iter()
            .any(|insight| insight.kind == "scalar_le" && insight.summary == "18"));

        let cursor = report
            .watched_memory
            .iter()
            .find(|watch| watch.name == "cursor")
            .expect("cursor watch");
        assert_eq!(cursor.preview_bytes, vec![0x34, 0x56]);
        assert!(!cursor.preview_truncated);
        assert_eq!(cursor.baseline_preview_bytes, initial_cursor.clone());
        assert_eq!(cursor.diff_preview.len(), 2);
        assert_eq!(cursor.diff_preview[0].before, initial_cursor[0]);
        assert_eq!(cursor.diff_preview[0].after, 0x34);
        assert_eq!(cursor.diff_preview[1].before, initial_cursor[1]);
        assert_eq!(cursor.diff_preview[1].after, 0x56);
        assert!(cursor
            .watch_insights
            .iter()
            .any(|insight| insight.kind == "changed_offsets"));

        let board = report
            .watched_memory
            .iter()
            .find(|watch| watch.name == "board")
            .expect("board watch");
        assert_eq!(board.preview_bytes, (0u8..16).collect::<Vec<_>>());
        assert!(board.preview_truncated);
        assert_eq!(board.baseline_preview_bytes, vec![0; 16]);
        assert_eq!(board.diff_preview.len(), 16);
        assert_eq!(board.diff_preview[0].offset, 1);
        assert_eq!(board.diff_preview[0].before, 0x00);
        assert_eq!(board.diff_preview[0].after, 0x01);
        assert!(board.diff_preview_truncated);
    }

    #[test]
    fn profiler_surface_reports_function_and_bank_activity() {
        let mut session = DebugSession::new(Machine::new());
        session.symbol_table = Some(SymbolTable {
            symbols: vec![SymbolInfo {
                bank: 1,
                start: 0x4000,
                end: 0x4010,
                name: "MainLoop".to_string(),
                kind: Some("function".to_string()),
                region: Some("rom".to_string()),
                section: Some("CODE".to_string()),
            }],
            source_locations: vec![SourceLocationInfo {
                bank: 1,
                addr: 0x4000,
                path: "demo.c".to_string(),
                line: 12,
                column: Some(3),
                symbol: Some("MainLoop".to_string()),
                section: Some("CODE".to_string()),
            }],
            functions: vec![FunctionInfo {
                name: "MainLoop".to_string(),
                bank: 1,
                start: 0x4000,
                end: 0x4010,
                size_bytes: 16,
                section: Some("CODE".to_string()),
                is_stack_call: false,
                is_fast_call: false,
                has_fixed_bank: false,
                has_fixed_order: false,
                return_size: 1,
                param_sizes: vec![1],
            }],
            variables: Vec::new(),
            static_estimates: vec![StaticEstimateInfo {
                name: "MainLoop".to_string(),
                bank: 1,
                size_bytes: 16,
                incoming_call_count: 2,
                outgoing_call_count: 1,
                cross_bank_outgoing_count: 1,
                far_call_count: 1,
                is_stack_call: false,
                is_fast_call: false,
                return_size: 1,
                param_sizes: vec![1],
            }],
            call_edges: Vec::new(),
        });
        session.toolchain_build_report = Some(ToolchainBuildReport {
            output_rom: Some("demo.gb".to_string()),
            output_sha256: None,
            rom_size_bytes: Some(32768),
            used_bytes: Some(2048),
            abi_mode: Some("legacy".to_string()),
            function_count: 1,
            call_edge_count: 1,
            optimizer_pass_count: 0,
            abi_issue_count: 0,
            cross_bank_call_count: 1,
            source_files: vec!["demo.c".to_string()],
            abi_issues: Vec::new(),
            hotspots: vec![crate::report::ToolchainHotspotReport {
                name: "MainLoop".to_string(),
                incoming_calls: 2,
                size_bytes: 16,
                score: 32,
            }],
            cross_bank_edges: Vec::new(),
            notes: Vec::new(),
        });
        session.pc_hit_histogram.insert((1, 0x4000), 5);
        session.pc_cycle_histogram.insert((1, 0x4000), 40);
        session.event_log.push(DebugEvent::BankSwitch {
            frame: 1,
            cycle: 20,
            from: 0,
            to: 1,
            from_symbol: Some("Boot".to_string()),
            to_symbol: Some("MainLoop".to_string()),
        });
        session.event_log.push(DebugEvent::FarCallSuspected {
            frame: 1,
            cycle: 20,
            from_bank: 0,
            to_bank: 1,
            from_symbol: Some("Boot".to_string()),
            to_symbol: Some("MainLoop".to_string()),
        });

        let profiler = session.build_profiler().expect("profiler");
        assert_eq!(profiler.function_activity[0].name, "MainLoop");
        assert_eq!(profiler.function_activity[0].samples, 5);
        assert_eq!(profiler.function_activity[0].cycles, 40);
        assert_eq!(profiler.function_activity[0].build_hotspot_score, Some(32));
        assert_eq!(profiler.bank_activity[0].bank, 1);
        assert_eq!(profiler.bank_activity[0].far_call_count, 1);
        assert_eq!(profiler.bank_transitions[0].from_bank, 0);
        assert_eq!(profiler.bank_transitions[0].to_bank, 1);
    }

    #[test]
    fn heatmap_and_abi_surfaces_report_toolchain_and_runtime_hints() {
        let mut session = DebugSession::new(Machine::new());
        session.machine.cpu.pc = 0x0100;
        session.machine.cpu.sp = 0xD000;
        session.machine.cpu.a = 0x42;
        session.machine.cpu.h = 0x12;
        session.machine.cpu.l = 0x34;
        session.machine.write8(0xD000, 0x01);
        session.machine.write8(0xD001, 0x02);
        session.machine.write8(0xD002, 0xAA);
        session.machine.write8(0xD003, 0xBB);
        session.machine.write8(0xD004, 0xCC);
        session.symbol_table = Some(SymbolTable {
            symbols: vec![SymbolInfo {
                bank: 0,
                start: 0x0100,
                end: 0x0120,
                name: "StackFunc".to_string(),
                kind: Some("function".to_string()),
                region: Some("rom".to_string()),
                section: Some("CODE".to_string()),
            }],
            source_locations: Vec::new(),
            functions: vec![FunctionInfo {
                name: "StackFunc".to_string(),
                bank: 0,
                start: 0x0100,
                end: 0x0120,
                size_bytes: 32,
                section: Some("CODE".to_string()),
                is_stack_call: true,
                is_fast_call: false,
                has_fixed_bank: false,
                has_fixed_order: false,
                return_size: 1,
                param_sizes: vec![1, 2],
            }],
            variables: Vec::new(),
            static_estimates: Vec::new(),
            call_edges: Vec::new(),
        });
        session.toolchain_build_report = Some(ToolchainBuildReport {
            output_rom: Some("demo.gb".to_string()),
            output_sha256: None,
            rom_size_bytes: Some(32768),
            used_bytes: Some(4096),
            abi_mode: Some("stack".to_string()),
            function_count: 1,
            call_edge_count: 2,
            optimizer_pass_count: 0,
            abi_issue_count: 1,
            cross_bank_call_count: 1,
            source_files: vec!["demo.c".to_string()],
            abi_issues: vec![
                "stack ABI parameter must be 1 or 2 bytes: BadFunc param#0 size=3".to_string(),
            ],
            hotspots: Vec::new(),
            cross_bank_edges: vec![crate::report::ToolchainCrossBankEdgeReport {
                caller: "StackFunc".to_string(),
                callee: "FarFunc".to_string(),
                caller_bank: 0,
                callee_bank: 1,
                count: 2,
                kind: "call".to_string(),
                via_thunk: false,
                via_farcall: true,
            }],
            notes: Vec::new(),
        });
        session.pc_hit_histogram.insert((0, 0x0100), 3);
        session.pc_cycle_histogram.insert((0, 0x0100), 24);
        session.far_call_count = 1;
        session.bank_restored = false;
        session.bank_thrash_score = 5;
        session.event_log.push(DebugEvent::JoypadRead {
            frame: 1,
            cycle: 10,
            p1: 0xCF,
            select: 0x20,
            mask: 0x01,
        });
        session.event_log.push(DebugEvent::MapperControlWrite {
            frame: 1,
            cycle: 12,
            mapper: "MBC5".to_string(),
            addr: 0x2000,
            value: 0x02,
            rom_bank: 2,
            ram_bank: 0,
        });
        session.event_log.push(DebugEvent::OamDmaStart {
            frame: 1,
            cycle: 14,
            source: 0xC000,
        });

        let profiler = session.build_profiler().expect("profiler");
        let heatmap = session.build_heatmap(Some(&profiler)).expect("heatmap");
        assert!(heatmap
            .buckets
            .iter()
            .any(|bucket| bucket.region == "io" && bucket.start == 0xFF00));
        assert!(heatmap
            .buckets
            .iter()
            .any(|bucket| bucket.region == "mapper_ctrl" && bucket.start == 0x2000));
        let abi = session.build_abi_verification().expect("abi");
        assert_eq!(abi.status, "toolchain_issues_present");
        assert_eq!(abi.abi_mode.as_deref(), Some("stack"));
        assert_eq!(abi.toolchain_issue_count, 1);
        assert!(abi.current_function_contract.is_some());
        assert!(!abi.runtime_alerts.is_empty());
        assert_eq!(abi.stack_window.sp, 0xD000);
        assert_eq!(abi.stack_window.region, "wram");
        assert_eq!(abi.stack_window.decoded_return_address, Some(0x0201));
        assert_eq!(
            abi.stack_window.decoded_return_region.as_deref(),
            Some("rom")
        );
        assert_eq!(abi.stack_window.expected_stack_argument_bytes, 3);
        assert_eq!(abi.stack_window.argument_base, Some(0xD002));
        assert_eq!(
            abi.stack_window.argument_preview_bytes,
            vec![0xAA, 0xBB, 0xCC]
        );
        assert!(!abi.stack_window.preview_truncated);
        assert!(!abi.stack_window.argument_preview_truncated);
        assert_eq!(abi.register_snapshot.sp, 0xD000);
        assert_eq!(abi.register_snapshot.pc, 0x0100);
        assert_eq!(abi.register_snapshot.a, 0x42);
        assert_eq!(abi.register_snapshot.hl, 0x1234);
    }

    #[test]
    fn abi_call_boundary_reports_fastcall_transport_across_recent_contexts() {
        let mut session = DebugSession::new(Machine::new());
        session.machine.cpu.pc = 0x4000;
        session.machine.cpu.sp = 0xC11E;
        session.machine.cpu.a = 0x77;
        session.machine.cpu.h = 0x56;
        session.machine.cpu.l = 0x78;
        session.machine.write8(0xC11E, 0x34);
        session.machine.write8(0xC11F, 0x12);
        session.symbol_table = Some(SymbolTable {
            symbols: vec![
                SymbolInfo {
                    bank: 0,
                    start: 0x0200,
                    end: 0x0210,
                    name: "CallerFast".to_string(),
                    kind: Some("function".to_string()),
                    region: Some("rom".to_string()),
                    section: Some("CODE".to_string()),
                },
                SymbolInfo {
                    bank: 1,
                    start: 0x4000,
                    end: 0x4010,
                    name: "CalleeFast".to_string(),
                    kind: Some("function".to_string()),
                    region: Some("rom".to_string()),
                    section: Some("CODE".to_string()),
                },
            ],
            source_locations: Vec::new(),
            functions: vec![
                FunctionInfo {
                    name: "CallerFast".to_string(),
                    bank: 0,
                    start: 0x0200,
                    end: 0x0210,
                    size_bytes: 16,
                    section: Some("CODE".to_string()),
                    is_stack_call: false,
                    is_fast_call: false,
                    has_fixed_bank: false,
                    has_fixed_order: false,
                    return_size: 0,
                    param_sizes: Vec::new(),
                },
                FunctionInfo {
                    name: "CalleeFast".to_string(),
                    bank: 1,
                    start: 0x4000,
                    end: 0x4010,
                    size_bytes: 16,
                    section: Some("CODE".to_string()),
                    is_stack_call: false,
                    is_fast_call: true,
                    has_fixed_bank: false,
                    has_fixed_order: false,
                    return_size: 1,
                    param_sizes: vec![1],
                },
            ],
            variables: Vec::new(),
            static_estimates: Vec::new(),
            call_edges: Vec::new(),
        });
        session.toolchain_build_report = Some(ToolchainBuildReport {
            output_rom: Some("demo.gb".to_string()),
            output_sha256: None,
            rom_size_bytes: Some(32768),
            used_bytes: Some(4096),
            abi_mode: Some("legacy".to_string()),
            function_count: 2,
            call_edge_count: 1,
            optimizer_pass_count: 0,
            abi_issue_count: 0,
            cross_bank_call_count: 1,
            source_files: vec!["demo.c".to_string()],
            abi_issues: Vec::new(),
            hotspots: Vec::new(),
            cross_bank_edges: vec![crate::report::ToolchainCrossBankEdgeReport {
                caller: "CallerFast".to_string(),
                callee: "CalleeFast".to_string(),
                caller_bank: 0,
                callee_bank: 1,
                count: 1,
                kind: "bank_thunk".to_string(),
                via_thunk: true,
                via_farcall: false,
            }],
            notes: Vec::new(),
        });

        session.machine.cpu.pc = 0x0200;
        session.machine.cpu.sp = 0xC120;
        session.machine.cpu.a = 0x77;
        session.machine.cpu.b = 0x10;
        session.machine.cpu.c = 0x20;
        session.machine.cpu.d = 0x30;
        session.machine.cpu.e = 0x40;
        session.machine.cpu.h = 0x12;
        session.machine.cpu.l = 0x34;
        session.push_execution_context("caller".to_string(), 0, 0x0200);

        session.machine.cpu.pc = 0x4000;
        session.machine.cpu.sp = 0xC11E;
        session.machine.cpu.a = 0x77;
        session.machine.cpu.b = 0xAA;
        session.machine.cpu.c = 0xBB;
        session.machine.cpu.d = 0xCC;
        session.machine.cpu.e = 0xDD;
        session.machine.cpu.h = 0x56;
        session.machine.cpu.l = 0x78;
        session.push_execution_context("callee".to_string(), 1, 0x4000);

        let abi = session.build_abi_verification().expect("abi");
        let boundary = abi.call_boundary.expect("call boundary");
        assert_eq!(boundary.caller_function.as_deref(), Some("CallerFast"));
        assert_eq!(boundary.callee_function, "CalleeFast");
        assert_eq!(boundary.edge_kind.as_deref(), Some("bank_thunk"));
        assert!(boundary.via_thunk);
        assert!(boundary.cross_bank);
        assert!(boundary
            .contract_rules
            .iter()
            .any(|rule| rule.register == "A" && rule.role == "entry_carrier"));
        assert!(boundary
            .contract_rules
            .iter()
            .any(|rule| rule.register == "BC" && rule.role == "scratch_allowed"));
        assert!(boundary
            .register_checks
            .iter()
            .any(|check| check.register == "A" && check.status == "stable_across_boundary"));
        assert!(boundary.register_checks.iter().any(|check| {
            check.register == "BC" && check.status == "changed_scratch_observation"
        }));
        assert_eq!(
            boundary.caller_context.as_ref().map(|ctx| ctx.a),
            Some(0x77)
        );
        assert_eq!(
            boundary
                .callee_entry_context
                .as_ref()
                .map(|ctx| ctx.stack_preview.len()),
            Some(6)
        );
    }

    #[test]
    fn abi_return_boundary_reports_near_ret_register_stability() {
        let mut session = DebugSession::new(Machine::new());
        session.symbol_table = Some(SymbolTable {
            symbols: vec![
                SymbolInfo {
                    bank: 0,
                    start: 0x0200,
                    end: 0x0210,
                    name: "CallerFast".to_string(),
                    kind: Some("function".to_string()),
                    region: Some("rom".to_string()),
                    section: Some("CODE".to_string()),
                },
                SymbolInfo {
                    bank: 1,
                    start: 0x4000,
                    end: 0x4010,
                    name: "CalleeFast".to_string(),
                    kind: Some("function".to_string()),
                    region: Some("rom".to_string()),
                    section: Some("CODE".to_string()),
                },
            ],
            source_locations: Vec::new(),
            functions: vec![
                FunctionInfo {
                    name: "CallerFast".to_string(),
                    bank: 0,
                    start: 0x0200,
                    end: 0x0210,
                    size_bytes: 16,
                    section: Some("CODE".to_string()),
                    is_stack_call: false,
                    is_fast_call: false,
                    has_fixed_bank: false,
                    has_fixed_order: false,
                    return_size: 0,
                    param_sizes: Vec::new(),
                },
                FunctionInfo {
                    name: "CalleeFast".to_string(),
                    bank: 1,
                    start: 0x4000,
                    end: 0x4010,
                    size_bytes: 16,
                    section: Some("CODE".to_string()),
                    is_stack_call: false,
                    is_fast_call: true,
                    has_fixed_bank: false,
                    has_fixed_order: false,
                    return_size: 1,
                    param_sizes: vec![1],
                },
            ],
            variables: Vec::new(),
            static_estimates: Vec::new(),
            call_edges: Vec::new(),
        });
        session.toolchain_build_report = Some(ToolchainBuildReport {
            output_rom: Some("demo.gb".to_string()),
            output_sha256: None,
            rom_size_bytes: Some(32768),
            used_bytes: Some(4096),
            abi_mode: Some("legacy".to_string()),
            function_count: 2,
            call_edge_count: 1,
            optimizer_pass_count: 0,
            abi_issue_count: 0,
            cross_bank_call_count: 1,
            source_files: vec!["demo.c".to_string()],
            abi_issues: Vec::new(),
            hotspots: Vec::new(),
            cross_bank_edges: vec![crate::report::ToolchainCrossBankEdgeReport {
                caller: "CallerFast".to_string(),
                callee: "CalleeFast".to_string(),
                caller_bank: 0,
                callee_bank: 1,
                count: 1,
                kind: "bank_thunk".to_string(),
                via_thunk: true,
                via_farcall: false,
            }],
            notes: Vec::new(),
        });

        session.machine.cpu.pc = 0x0200;
        session.machine.cpu.sp = 0xC120;
        session.machine.cpu.a = 0x20;
        session.machine.cpu.b = 0x12;
        session.machine.cpu.c = 0x34;
        session.machine.cpu.d = 0x56;
        session.machine.cpu.e = 0x78;
        session.machine.cpu.h = 0x11;
        session.machine.cpu.l = 0x22;
        session.push_execution_context("caller_pre".to_string(), 0, 0x0200);

        session.machine.cpu.pc = 0x400E;
        session.machine.cpu.sp = 0xC11E;
        session.machine.cpu.a = 0x66;
        session.machine.cpu.b = 0x44;
        session.machine.cpu.c = 0x55;
        session.machine.cpu.d = 0x66;
        session.machine.cpu.e = 0x77;
        session.machine.cpu.h = 0x33;
        session.machine.cpu.l = 0x44;
        session.machine.write8(0xC11E, 0x04);
        session.machine.write8(0xC11F, 0x02);
        session.push_execution_context("callee_near_ret".to_string(), 1, 0x400E);

        session.machine.cpu.pc = 0x0204;
        session.machine.cpu.sp = 0xC120;
        session.machine.cpu.a = 0x66;
        session.machine.cpu.b = 0x99;
        session.machine.cpu.c = 0xAA;
        session.machine.cpu.d = 0xBB;
        session.machine.cpu.e = 0xCC;
        session.machine.cpu.h = 0x33;
        session.machine.cpu.l = 0x44;

        let abi = session.build_abi_verification().expect("abi");
        let boundary = abi.return_boundary.expect("return boundary");
        assert_eq!(boundary.caller_function, "CallerFast");
        assert_eq!(boundary.callee_function, "CalleeFast");
        assert_eq!(boundary.edge_kind.as_deref(), Some("bank_thunk"));
        assert!(boundary.via_thunk);
        assert!(boundary.cross_bank);
        assert!(boundary.callee_near_ret);
        assert_eq!(boundary.bytes_to_callee_end, Some(2));
        assert!(boundary
            .contract_rules
            .iter()
            .any(|rule| rule.register == "SP" && rule.role == "stack_restored_required"));
        assert!(boundary
            .contract_rules
            .iter()
            .any(|rule| rule.register == "A" && rule.role == "preserved_required"));
        assert!(boundary
            .register_checks
            .iter()
            .any(|check| check.register == "A" && check.status == "stable_across_return"));
        assert!(boundary
            .register_checks
            .iter()
            .any(|check| check.register == "HL" && check.status == "stable_across_return"));
        assert_eq!(
            boundary.caller_resume_context.as_ref().map(|ctx| ctx.pc),
            Some(0x0204)
        );
        assert!(boundary
            .register_checks
            .iter()
            .any(|check| check.register == "SP" && check.status == "stable_across_return"));
        assert!(boundary.register_checks.iter().any(|check| {
            check.register == "BC" && check.status == "changed_scratch_observation"
        }));
        assert!(boundary
            .notes
            .iter()
            .any(|note| note.contains("within 4 bytes")));
    }

    #[test]
    fn abi_intrinsic_boundary_reports_bank_helper_contract_and_scratch() {
        let mut session = DebugSession::new(Machine::new());
        session.symbol_table = Some(SymbolTable {
            symbols: vec![
                SymbolInfo {
                    bank: 0,
                    start: 0x0200,
                    end: 0x0210,
                    name: "Caller".to_string(),
                    kind: Some("function".to_string()),
                    region: Some("rom".to_string()),
                    section: Some("CODE".to_string()),
                },
                SymbolInfo {
                    bank: 0,
                    start: 0x0300,
                    end: 0x0310,
                    name: "__kq_thunk_b3_Target".to_string(),
                    kind: Some("function".to_string()),
                    region: Some("rom".to_string()),
                    section: Some("CODE".to_string()),
                },
            ],
            source_locations: Vec::new(),
            functions: vec![FunctionInfo {
                name: "Caller".to_string(),
                bank: 0,
                start: 0x0200,
                end: 0x0210,
                size_bytes: 16,
                section: Some("CODE".to_string()),
                is_stack_call: false,
                is_fast_call: false,
                has_fixed_bank: false,
                has_fixed_order: false,
                return_size: 0,
                param_sizes: Vec::new(),
            }],
            variables: Vec::new(),
            static_estimates: Vec::new(),
            call_edges: Vec::new(),
        });

        session.machine.cpu.pc = 0x0200;
        session.machine.cpu.sp = 0xC120;
        session.machine.cpu.a = 0x11;
        session.machine.cpu.b = 0x22;
        session.machine.cpu.c = 0x33;
        session.machine.cpu.d = 0x44;
        session.machine.cpu.e = 0x55;
        session.machine.cpu.h = 0x66;
        session.machine.cpu.l = 0x77;
        session.push_execution_context("caller_pre".to_string(), 0, 0x0200);

        session.machine.cpu.pc = 0x0304;
        session.machine.cpu.sp = 0xC120;
        session.machine.cpu.a = 0x11;
        session.machine.cpu.b = 0x99;
        session.machine.cpu.c = 0x88;
        session.machine.cpu.d = 0x77;
        session.machine.cpu.e = 0x66;
        session.machine.cpu.h = 0x66;
        session.machine.cpu.l = 0x77;
        session.push_execution_context("bank_helper".to_string(), 0, 0x0304);

        session.machine.cpu.pc = 0x0206;
        session.machine.cpu.sp = 0xC120;
        session.machine.cpu.a = 0x11;
        session.machine.cpu.b = 0xAB;
        session.machine.cpu.c = 0xCD;
        session.machine.cpu.d = 0xEF;
        session.machine.cpu.e = 0x01;
        session.machine.cpu.h = 0x66;
        session.machine.cpu.l = 0x77;

        let abi = session.build_abi_verification().expect("abi");
        let boundary = abi
            .intrinsic_boundaries
            .iter()
            .find(|entry| entry.intrinsic_symbol == "__kq_thunk_b3_Target")
            .expect("intrinsic boundary");
        assert_eq!(boundary.intrinsic_kind, "Bank");
        assert_eq!(boundary.contract_strength, "preserve_return_carriers");
        assert!(boundary
            .contract_rules
            .iter()
            .any(|rule| rule.register == "A" && rule.role == "preserved_required"));
        assert!(boundary
            .contract_rules
            .iter()
            .any(|rule| rule.register == "BC" && rule.role == "scratch_allowed"));
        assert!(boundary
            .register_checks
            .iter()
            .any(|check| { check.register == "A" && check.status == "stable_across_intrinsic" }));
        assert!(boundary
            .register_checks
            .iter()
            .any(|check| { check.register == "HL" && check.status == "stable_across_intrinsic" }));
        assert!(boundary
            .register_checks
            .iter()
            .any(|check| { check.register == "SP" && check.status == "stable_across_intrinsic" }));
        assert!(boundary.register_checks.iter().any(|check| {
            check.register == "BC" && check.status == "changed_scratch_observation"
        }));
        assert!(boundary.register_checks.iter().any(|check| {
            check.register == "DE" && check.status == "changed_scratch_observation"
        }));
        assert!(boundary
            .observed_changed_registers
            .iter()
            .any(|reg| reg == "BC"));
    }

    #[test]
    fn abi_intrinsic_boundary_reports_video_helper_contracts() {
        let mut session = DebugSession::new(Machine::new());
        session.symbol_table = Some(SymbolTable {
            symbols: vec![
                SymbolInfo {
                    bank: 0,
                    start: 0x0200,
                    end: 0x0210,
                    name: "Caller".to_string(),
                    kind: Some("function".to_string()),
                    region: Some("rom".to_string()),
                    section: Some("CODE".to_string()),
                },
                SymbolInfo {
                    bank: 0,
                    start: 0x0320,
                    end: 0x0330,
                    name: "__settilebg16_buf_fast".to_string(),
                    kind: Some("function".to_string()),
                    region: Some("rom".to_string()),
                    section: Some("CODE".to_string()),
                },
            ],
            source_locations: Vec::new(),
            functions: vec![FunctionInfo {
                name: "Caller".to_string(),
                bank: 0,
                start: 0x0200,
                end: 0x0210,
                size_bytes: 16,
                section: Some("CODE".to_string()),
                is_stack_call: false,
                is_fast_call: false,
                has_fixed_bank: false,
                has_fixed_order: false,
                return_size: 0,
                param_sizes: Vec::new(),
            }],
            variables: Vec::new(),
            static_estimates: Vec::new(),
            call_edges: Vec::new(),
        });

        session.machine.cpu.pc = 0x0200;
        session.machine.cpu.sp = 0xC180;
        session.machine.cpu.a = 0x10;
        session.machine.cpu.b = 0x20;
        session.machine.cpu.c = 0x21;
        session.machine.cpu.d = 0x30;
        session.machine.cpu.e = 0x31;
        session.machine.cpu.h = 0x40;
        session.machine.cpu.l = 0x41;
        session.push_execution_context("caller_pre".to_string(), 0, 0x0200);

        session.machine.cpu.pc = 0x0324;
        session.machine.cpu.sp = 0xC180;
        session.machine.cpu.a = 0x11;
        session.machine.cpu.b = 0x55;
        session.machine.cpu.c = 0x66;
        session.machine.cpu.d = 0x77;
        session.machine.cpu.e = 0x88;
        session.machine.cpu.h = 0x99;
        session.machine.cpu.l = 0xAA;
        session.push_execution_context("video_helper".to_string(), 0, 0x0324);

        session.machine.cpu.pc = 0x0208;
        session.machine.cpu.sp = 0xC180;
        session.machine.cpu.a = 0x12;
        session.machine.cpu.b = 0x54;
        session.machine.cpu.c = 0x65;
        session.machine.cpu.d = 0x76;
        session.machine.cpu.e = 0x87;
        session.machine.cpu.h = 0x98;
        session.machine.cpu.l = 0xA9;

        let abi = session.build_abi_verification().expect("abi");
        let boundary = abi
            .intrinsic_boundaries
            .iter()
            .find(|entry| entry.intrinsic_symbol == "__settilebg16_buf_fast")
            .expect("intrinsic boundary");
        assert_eq!(boundary.intrinsic_kind, "SetTileBuffered");
        assert_eq!(boundary.contract_strength, "video_pipeline_helper");
        assert!(boundary
            .contract_rules
            .iter()
            .any(|rule| rule.register == "BC" && rule.role == "scratch_allowed"));
        assert!(boundary
            .contract_rules
            .iter()
            .any(|rule| rule.register == "DE" && rule.role == "scratch_allowed"));
        assert!(boundary
            .contract_rules
            .iter()
            .any(|rule| rule.register == "HL" && rule.role == "scratch_allowed"));
        assert!(boundary
            .contract_rules
            .iter()
            .any(|rule| rule.register == "A" && rule.role == "observational_only"));
        assert!(boundary.register_checks.iter().any(|check| {
            check.register == "BC" && check.status == "changed_scratch_observation"
        }));
        assert!(boundary
            .register_checks
            .iter()
            .any(|check| { check.register == "A" && check.status == "changed_observational" }));
    }

    #[test]
    fn abi_intrinsic_boundary_reports_memory_transport_contracts() {
        let mut session = DebugSession::new(Machine::new());
        session.symbol_table = Some(SymbolTable {
            symbols: vec![
                SymbolInfo {
                    bank: 0,
                    start: 0x0200,
                    end: 0x0210,
                    name: "Caller".to_string(),
                    kind: Some("function".to_string()),
                    region: Some("rom".to_string()),
                    section: Some("CODE".to_string()),
                },
                SymbolInfo {
                    bank: 1,
                    start: 0x4010,
                    end: 0x4020,
                    name: "__kq_far_memcpy_bank2".to_string(),
                    kind: Some("function".to_string()),
                    region: Some("rom".to_string()),
                    section: Some("CODE".to_string()),
                },
            ],
            source_locations: Vec::new(),
            functions: vec![FunctionInfo {
                name: "Caller".to_string(),
                bank: 0,
                start: 0x0200,
                end: 0x0210,
                size_bytes: 16,
                section: Some("CODE".to_string()),
                is_stack_call: false,
                is_fast_call: false,
                has_fixed_bank: false,
                has_fixed_order: false,
                return_size: 0,
                param_sizes: Vec::new(),
            }],
            variables: Vec::new(),
            static_estimates: Vec::new(),
            call_edges: Vec::new(),
        });

        session.machine.cpu.pc = 0x0200;
        session.machine.cpu.sp = 0xC1A0;
        session.machine.cpu.a = 0x07;
        session.machine.cpu.b = 0x10;
        session.machine.cpu.c = 0x11;
        session.machine.cpu.d = 0x20;
        session.machine.cpu.e = 0x21;
        session.machine.cpu.h = 0x30;
        session.machine.cpu.l = 0x31;
        session.push_execution_context("caller_pre".to_string(), 0, 0x0200);

        session.machine.cpu.pc = 0x4014;
        session.machine.cpu.sp = 0xC1A0;
        session.machine.cpu.a = 0x08;
        session.machine.cpu.b = 0x44;
        session.machine.cpu.c = 0x45;
        session.machine.cpu.d = 0x54;
        session.machine.cpu.e = 0x55;
        session.machine.cpu.h = 0x64;
        session.machine.cpu.l = 0x65;
        session.push_execution_context("far_memcpy".to_string(), 1, 0x4014);

        session.machine.cpu.pc = 0x0208;
        session.machine.cpu.sp = 0xC1A0;
        session.machine.cpu.a = 0x09;
        session.machine.cpu.b = 0x46;
        session.machine.cpu.c = 0x47;
        session.machine.cpu.d = 0x56;
        session.machine.cpu.e = 0x57;
        session.machine.cpu.h = 0x66;
        session.machine.cpu.l = 0x67;

        let abi = session.build_abi_verification().expect("abi");
        let boundary = abi
            .intrinsic_boundaries
            .iter()
            .find(|entry| entry.intrinsic_symbol == "__kq_far_memcpy_bank2")
            .expect("intrinsic boundary");
        assert_eq!(boundary.intrinsic_kind, "FarMemcpy");
        assert_eq!(boundary.contract_strength, "memory_transport_helper");
        assert!(boundary
            .contract_rules
            .iter()
            .any(|rule| rule.register == "HL" && rule.role == "scratch_allowed"));
        assert!(boundary
            .contract_rules
            .iter()
            .any(|rule| rule.register == "DE" && rule.role == "scratch_allowed"));
        assert!(boundary
            .contract_rules
            .iter()
            .any(|rule| rule.register == "A" && rule.role == "observational_only"));
        assert!(boundary.register_checks.iter().any(|check| {
            check.register == "HL" && check.status == "changed_scratch_observation"
        }));
        assert!(boundary
            .register_checks
            .iter()
            .any(|check| { check.register == "A" && check.status == "changed_observational" }));
    }

    #[test]
    fn debug_sessions_can_exchange_serial_with_peer() {
        let mut left = DebugSession::new(Machine::new());
        let mut right = DebugSession::new(Machine::new());
        left.machine.write8(0xFF01, 0xA5);
        left.machine.write8(0xFF02, 0x81);
        right.machine.write8(0xFF01, 0x3C);
        right.machine.write8(0xFF02, 0x80);

        let completed = left.exchange_serial_with_peer(&mut right);

        assert!(completed);
        assert_eq!(left.machine.serial.sb, 0x3C);
        assert_eq!(right.machine.serial.sb, 0xA5);
        assert_eq!(left.serial_interrupt_count, 1);
        assert_eq!(right.serial_interrupt_count, 1);
        assert!(left
            .event_log
            .iter()
            .any(|event| matches!(event, DebugEvent::SerialTransferComplete { sb: 0x3C, .. })));
        assert!(right
            .event_log
            .iter()
            .any(|event| matches!(event, DebugEvent::SerialTransferComplete { sb: 0xA5, .. })));
    }

    #[test]
    fn named_variable_reads_support_link4_peer_selection() {
        let mut session = DebugSession::new(Machine::new());
        session.set_symbol_table(SymbolTable {
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
        session.machine.write8(0xFF80, 1);
        session.machine.write8(0xFF81, 2);
        session.machine.write8(0xFF82, 4);

        assert_eq!(
            session.named_variable_addr("Link4_SelectedPeer"),
            Some(0xFF81)
        );
        assert_eq!(
            session.read_named_variable_u8("Link4_SelectedPeer"),
            Some(2)
        );
        assert_eq!(session.link4_selected_peer_session(4), Some(2));
        assert_eq!(session.link4_selected_peer_session(2), None);
    }

    #[test]
    fn replay_report_includes_watch_hashes_and_slice_summary() {
        let mut session = DebugSession::new(Machine::new());
        session.start_frame_counter = 0;
        session.start_cycle_counter = 0;
        session.start_rom_bank = 0;
        session.start_pc = 0x0100;
        session.machine.write8(0xC000, 0x12);
        session.machine.write8(0xC001, 0x34);
        session.event_log.push(DebugEvent::BankSwitch {
            frame: 1,
            cycle: 4,
            from: 1,
            to: 2,
            from_symbol: Some("Caller".to_string()),
            to_symbol: Some("Callee".to_string()),
        });
        session.event_log.push(DebugEvent::JoypadRead {
            frame: 1,
            cycle: 8,
            p1: 0x20,
            select: 0x10,
            mask: 0x01,
        });
        session.replay_checkpoints.push_back(ReplayCheckpoint {
            checkpoint_index: 0,
            generation: 0,
            frame: 1,
            cycle: 16,
            rom_bank: 2,
            pc: 0x4000,
            frame_hash: 0x1111_2222,
            digest: 0x0102_0304_0506_0708,
            symbol: Some("MainLoop".to_string()),
            source: None,
            watch_hashes: vec![ReplayWatchDigestReport {
                name: "board".to_string(),
                start: 0xC000,
                size: 2,
                hash: hash_bytes(&[0x12, 0x34]),
            }],
            watch_digest: 0x1234_5678_9ABC_DEF0,
            event_log_len: 2,
            instruction_samples_total: 3,
            state: session.machine.save_state(),
        });
        session.replay_checkpoint_count = 1;

        let replay = session.report().replay.expect("replay report");
        assert_eq!(replay.checkpoints.len(), 1);
        assert_eq!(replay.checkpoints[0].watch_hashes.len(), 1);
        assert_eq!(replay.checkpoints[0].watch_digest, 0x1234_5678_9ABC_DEF0);
        assert_eq!(replay.slices.len(), 1);
        assert_eq!(replay.slices[0].instruction_samples, 3);
        assert_eq!(replay.slices[0].event_count, 2);
        assert_eq!(replay.slices[0].bank_switch_count, 1);
        assert_eq!(replay.slices[0].joypad_read_count, 1);
        assert_ne!(replay.slices[0].slice_digest, 0);
    }

    #[test]
    #[ignore = "manual profiling probe for real-ROM runtime cost"]
    fn profile_reversi_runtime_cost_breakdown() {
        let rom_path = reversi_rom_path();
        let map_path = reversi_map_path();
        eprintln!(
            "probe:start rom={} map={}",
            rom_path.display(),
            map_path.display()
        );
        let rom = fs::read(&rom_path).expect("read reversi rom");

        let mut core_machine = Machine::new();
        core_machine
            .load_rom(rom.clone())
            .expect("load reversi rom");
        eprintln!("probe:core_run:start");
        let core_start = Instant::now();
        let start_frame = core_machine.clocks.frames;
        let mut core_steps: u64 = 0;
        while core_machine.clocks.frames == start_frame {
            core_steps += 1;
            core_machine.step_instruction().expect("core step");
            if core_steps % 50_000 == 0 {
                eprintln!(
                    "probe:core_run:progress steps={} frame={} cycles={} pc={:04X} bank={} lcdc={:02X} ly={} mode={:?}",
                    core_steps,
                    core_machine.clocks.frames,
                    core_machine.clocks.cycles,
                    core_machine.cpu.pc,
                    core_machine.current_rom_bank(),
                    core_machine.ppu.lcdc,
                    core_machine.ppu.ly,
                    core_machine.ppu.current_mode()
                );
            }
            assert!(
                core_steps <= 500_000,
                "core did not advance a frame within 500000 instructions (frame={} cycles={} pc={:04X} bank={} lcdc={:02X} ly={} mode={:?})",
                core_machine.clocks.frames,
                core_machine.clocks.cycles,
                core_machine.cpu.pc,
                core_machine.current_rom_bank(),
                core_machine.ppu.lcdc,
                core_machine.ppu.ly,
                core_machine.ppu.current_mode()
            );
        }
        let core_elapsed = core_start.elapsed();
        eprintln!("probe:core_run:done ms={}", core_elapsed.as_millis());

        let mut debug_machine = Machine::new();
        debug_machine
            .load_rom(rom.clone())
            .expect("load reversi rom");
        let mut debug_session = DebugSession::new(debug_machine);
        eprintln!("probe:debug_run:start");
        let debug_run_start = Instant::now();
        debug_session.run_frames(1).expect("debug run");
        let debug_run_elapsed = debug_run_start.elapsed();
        eprintln!("probe:debug_run:done ms={}", debug_run_elapsed.as_millis());

        eprintln!("probe:report:start");
        let report_start = Instant::now();
        let report = debug_session.report();
        let report_elapsed = report_start.elapsed();
        eprintln!("probe:report:done ms={}", report_elapsed.as_millis());

        eprintln!("probe:json:start");
        let json_start = Instant::now();
        let json = to_string(&report).expect("serialize debug report");
        let json_elapsed = json_start.elapsed();
        eprintln!(
            "probe:json:done ms={} bytes={}",
            json_elapsed.as_millis(),
            json.len()
        );

        let mut symbol_machine = Machine::new();
        symbol_machine.load_rom(rom).expect("load reversi rom");
        let mut symbol_session = DebugSession::new(symbol_machine);
        let symbol_table =
            parse_map_file(map_path.to_str().expect("map path str")).expect("parse reversi map");
        symbol_session.set_symbol_table(symbol_table);
        eprintln!("probe:symbol_run:start");
        let symbol_run_start = Instant::now();
        symbol_session.run_frames(1).expect("symbol run");
        let symbol_run_elapsed = symbol_run_start.elapsed();
        eprintln!(
            "probe:symbol_run:done ms={}",
            symbol_run_elapsed.as_millis()
        );

        println!(
            "reversi_runtime_cost core_run_1f_ms={} debug_run_1f_ms={} report_ms={} json_ms={} json_bytes={} symbol_run_1f_ms={} events={} diagnostics={}",
            core_elapsed.as_millis(),
            debug_run_elapsed.as_millis(),
            report_elapsed.as_millis(),
            json_elapsed.as_millis(),
            json.len(),
            symbol_run_elapsed.as_millis(),
            report.events.len(),
            report.diagnostics.len(),
        );
    }
}
