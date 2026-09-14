pub const DEBUG_REPORT_SCHEMA_VERSION: &str = "1";

use serde::{Deserialize, Serialize};

use crate::diagnostics::{AutoDiagnosisReport, TimingPackReport};
use crate::snapshot::{CpuSnapshot, TimerSnapshot, VideoSnapshot};
use crate::stop::{ExecutionContextFrame, StopReason};
use crate::watch::MemoryWatchResult;
use crate::{DebugEvent, Diagnostic};

#[derive(Debug, Clone, Serialize)]
// Record the run totals and exact observation coordinates alongside their stated basis.
pub struct ReportMeta {
    pub frames_executed: u64,
    pub events_recorded: usize,
    pub diagnostics_recorded: usize,
    pub start_rom_bank: u16,
    pub end_rom_bank: u16,
    pub observation_frame: u64,
    pub observation_cycle: u64,
    pub observation_ly: u8,
    pub observation_pc: u16,
    pub observation_rom_bank: u16,
    pub observation_basis: String,
}

#[derive(Debug, Clone, Serialize)]
// Carry counters, estimates and recent observations computed by the session.
// These fields describe retained evidence; this payload type does not itself detect events.
pub struct EventSummary {
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
    pub halted_on_unsupported_opcode: bool,
    pub wait_before_present: bool,
    pub last_intrinsic: Option<String>,
    pub last_far_call_symbol: Option<String>,
    pub last_bank_switch_from_symbol: Option<String>,
    pub last_bank_switch_to_symbol: Option<String>,
    pub bank_restored: bool,
    pub bank_thrash_score: u32,
    pub hot_loop_score: u32,
    pub sprite_visible: bool,
    pub sprite_count_estimate: u32,
    pub oam_nonzero_entries: u32,
    pub screen_changed: bool,
    pub bg_changed: bool,
    pub window_changed: bool,
    pub sprite_changed: bool,
    pub delta_summary: Vec<String>,
    pub recent_event_summary: Vec<String>,
    pub input_hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
// Aggregate one opcode value with its count and most recent observed code location.
pub struct UnsupportedOpcodeSummary {
    pub opcode: u8,
    pub count: u64,
    pub last_pc: u16,
    pub last_rom_bank: u16,
    pub last_symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Keep source coordinates together with bank/address identity and optional symbol/section labels.
pub struct SourceLocationReport {
    pub path: String,
    pub line: u32,
    pub column: Option<u32>,
    pub bank: u16,
    pub addr: u16,
    pub symbol: Option<String>,
    pub section: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
// Expose compiler-provided function extent and calling-convention metadata.
pub struct ToolchainFunctionReport {
    pub name: String,
    pub bank: u16,
    pub start: u16,
    pub end: u32,
    pub size_bytes: u32,
    pub section: Option<String>,
    pub is_stack_call: bool,
    pub is_fast_call: bool,
    pub has_fixed_bank: bool,
    pub has_fixed_order: bool,
    pub return_size: u8,
    pub param_sizes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
// Keep static call-graph estimates distinct from runtime instruction samples.
pub struct ToolchainStaticEstimateReport {
    pub name: String,
    pub bank: u16,
    pub size_bytes: u32,
    pub incoming_call_count: u32,
    pub outgoing_call_count: u32,
    pub cross_bank_outgoing_count: u32,
    pub far_call_count: u32,
    pub is_stack_call: bool,
    pub is_fast_call: bool,
    pub return_size: u8,
    pub param_sizes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Deserialize an optional build hotspot with zero defaults for missing numeric estimates.
pub struct ToolchainHotspotReport {
    pub name: String,
    #[serde(default)]
    pub incoming_calls: u32,
    #[serde(default)]
    pub size_bytes: u32,
    #[serde(default)]
    pub score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Preserve compiler edge labels and thunk/far-call flags; signed bank values are retained.
pub struct ToolchainCrossBankEdgeReport {
    pub caller: String,
    pub callee: String,
    pub caller_bank: i32,
    pub callee_bank: i32,
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub via_thunk: bool,
    #[serde(default)]
    pub via_farcall: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Carry optional build identity, sizes, call-graph data and ABI findings.
// Missing metadata defaults to empty values; these fields alone do not authenticate a ROM.
pub struct ToolchainBuildReport {
    #[serde(default)]
    pub output_rom: Option<String>,
    #[serde(default)]
    pub output_sha256: Option<String>,
    #[serde(default)]
    pub rom_size_bytes: Option<u32>,
    #[serde(default)]
    pub used_bytes: Option<u32>,
    #[serde(default)]
    pub abi_mode: Option<String>,
    #[serde(default)]
    pub function_count: u32,
    #[serde(default)]
    pub call_edge_count: u32,
    #[serde(default)]
    pub optimizer_pass_count: u32,
    #[serde(default)]
    pub abi_issue_count: u32,
    #[serde(default)]
    pub cross_bank_call_count: u32,
    #[serde(default)]
    pub source_files: Vec<String>,
    #[serde(default)]
    pub abi_issues: Vec<String>,
    #[serde(default)]
    pub hotspots: Vec<ToolchainHotspotReport>,
    #[serde(default)]
    pub cross_bank_edges: Vec<ToolchainCrossBankEdgeReport>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
// Group current and neighboring symbol/source matches with optional function contracts
// and the captured execution-context trail.
pub struct SymbolReport {
    pub pc_symbol: Option<String>,
    pub previous_pc_symbol: Option<String>,
    pub nearest_symbol: Option<String>,
    pub next_symbol: Option<String>,
    pub pc_offset: Option<u16>,
    pub current_rom_bank: u16,
    pub current_source: Option<SourceLocationReport>,
    pub previous_source: Option<SourceLocationReport>,
    pub nearest_source: Option<SourceLocationReport>,
    pub next_source: Option<SourceLocationReport>,
    pub current_section: Option<String>,
    pub current_function: Option<ToolchainFunctionReport>,
    pub current_static_estimate: Option<ToolchainStaticEstimateReport>,
    pub execution_context_trail: Vec<ExecutionContextFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Identify a watched address window and its recorded digest without embedding its bytes.
pub struct ReplayWatchDigestReport {
    pub name: String,
    pub start: u16,
    pub size: u16,
    pub hash: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Describe activity between replay checkpoints, including endpoints, counters and a slice digest.
pub struct ReplaySliceReport {
    pub from_checkpoint_index: Option<u64>,
    pub to_checkpoint_index: u64,
    pub generation: u32,
    pub start_frame: u64,
    pub end_frame: u64,
    pub start_cycle: u64,
    pub end_cycle: u64,
    pub start_rom_bank: u16,
    pub start_pc: u16,
    pub start_symbol: Option<String>,
    pub start_source: Option<SourceLocationReport>,
    pub end_rom_bank: u16,
    pub end_pc: u16,
    pub end_symbol: Option<String>,
    pub end_source: Option<SourceLocationReport>,
    pub instruction_samples: u64,
    pub cpu_cycles: u64,
    pub event_count: u64,
    pub bank_switch_count: u64,
    pub far_call_count: u64,
    pub joypad_read_count: u64,
    pub dma_event_count: u64,
    pub interrupt_event_count: u64,
    pub render_event_count: u64,
    pub mmio_event_count: u64,
    pub slice_digest: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Keep checkpoint/generation identity, observation coordinates and frame/state/watch digests.
pub struct ReplayCheckpointReport {
    pub checkpoint_index: u64,
    pub generation: u32,
    pub frame: u64,
    pub cycle: u64,
    pub rom_bank: u16,
    pub pc: u16,
    pub frame_hash: u32,
    pub digest: u64,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub source: Option<SourceLocationReport>,
    #[serde(default)]
    pub watch_hashes: Vec<ReplayWatchDigestReport>,
    #[serde(default)]
    pub watch_digest: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Distinguish the requested rewind distance from the checkpoint and frame actually selected.
pub struct ReplayRewindReport {
    pub requested_frames: u64,
    pub from_frame: u64,
    pub to_frame: u64,
    pub checkpoint_index: u64,
    pub generation: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Record expected and actual digests at the observed checkpoint/generation.
pub struct ReplayDivergenceReport {
    pub frame: u64,
    pub cycle: u64,
    pub checkpoint_index: u64,
    pub generation: u32,
    pub expected_digest: u64,
    pub actual_digest: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Describe a reference comparison mismatch with optional checkpoint/frame coordinates.
pub struct ReplayReferenceMismatchReport {
    pub kind: String,
    pub checkpoint_index: Option<u64>,
    pub frame: Option<u64>,
    pub detail: String,
    pub expected: String,
    pub actual: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Provide neighboring replay slices and suggested investigation settings.
// Recommendations are report data, not commands executed by this type.
pub struct ReplayDivergenceHelperReport {
    #[serde(default)]
    pub mismatch_slice_index: Option<usize>,
    #[serde(default)]
    pub mismatch_checkpoint_index: Option<u64>,
    #[serde(default)]
    pub frame_window_start: Option<u64>,
    #[serde(default)]
    pub frame_window_end: Option<u64>,
    #[serde(default)]
    pub previous_reference_slice: Option<ReplaySliceReport>,
    #[serde(default)]
    pub previous_actual_slice: Option<ReplaySliceReport>,
    #[serde(default)]
    pub reference_slice: Option<ReplaySliceReport>,
    #[serde(default)]
    pub actual_slice: Option<ReplaySliceReport>,
    #[serde(default)]
    pub next_reference_slice: Option<ReplaySliceReport>,
    #[serde(default)]
    pub next_actual_slice: Option<ReplaySliceReport>,
    #[serde(default)]
    pub recommended_cli_stop_specs: Vec<String>,
    #[serde(default)]
    pub recommended_watch_windows: Vec<String>,
    #[serde(default)]
    pub recommended_focus: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Describe one proposed conditional capture and its optional state-file destination.
pub struct ReplayConditionalSnapshotTriggerReport {
    pub kind: String,
    pub value: String,
    #[serde(default)]
    pub save_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Keep a proposed replay window, run budget, stops and captures separate from execution results.
pub struct ReplayConditionalSnapshotPlanReport {
    pub frame_window_start: u64,
    pub frame_window_end: u64,
    pub run_frames: u64,
    #[serde(default)]
    pub stop_specs: Vec<String>,
    #[serde(default)]
    pub watch_windows: Vec<String>,
    #[serde(default)]
    pub snapshot_triggers: Vec<ReplayConditionalSnapshotTriggerReport>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Record whether a capture was attempted, whether it rewound and which state paths were generated.
pub struct ReplayConditionalSnapshotCaptureReport {
    pub attempted: bool,
    pub rewound: bool,
    pub start_frame: u64,
    pub end_frame: u64,
    pub run_frames: u64,
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub generated_states: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Combine comparison counts and first mismatch with optional investigation plans and capture outcomes.
pub struct ReplayReferenceComparisonReport {
    #[serde(default)]
    pub reference_path: Option<String>,
    pub matched: bool,
    pub reference_checkpoint_count: usize,
    pub actual_checkpoint_count: usize,
    pub reference_slice_count: usize,
    pub actual_slice_count: usize,
    #[serde(default)]
    pub first_mismatch: Option<ReplayReferenceMismatchReport>,
    #[serde(default)]
    pub divergence_helper: Option<ReplayDivergenceHelperReport>,
    #[serde(default)]
    pub conditional_snapshot_plan: Option<ReplayConditionalSnapshotPlanReport>,
    #[serde(default)]
    pub conditional_snapshot_capture: Option<ReplayConditionalSnapshotCaptureReport>,
    #[serde(default)]
    pub carry_forward_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Expose replay configuration, retained checkpoints and slices, latest rewind and optional divergence.
pub struct ReplayReport {
    pub enabled: bool,
    pub checkpoint_interval_frames: u64,
    pub max_checkpoints: usize,
    pub checkpoints_recorded: u64,
    pub current_generation: u32,
    pub checkpoints: Vec<ReplayCheckpointReport>,
    pub last_rewind: Option<ReplayRewindReport>,
    pub divergence: Option<ReplayDivergenceReport>,
    #[serde(default)]
    pub slices: Vec<ReplaySliceReport>,
    #[serde(default)]
    pub reference_compare: Option<ReplayReferenceComparisonReport>,
}

#[derive(Debug, Clone, Serialize)]
// Carry a sampled hot-loop candidate and its optional source match, without asserting program intent.
pub struct ForensicHotLoopReport {
    pub rom_bank: u16,
    pub pc: u16,
    pub hits: u64,
    pub symbol: Option<String>,
    pub source: Option<SourceLocationReport>,
}

#[derive(Debug, Clone, Serialize)]
// Describe a grouped observation with its count, last coordinates and explanatory detail.
pub struct ForensicHotspotReport {
    pub key: String,
    pub title: String,
    pub count: u64,
    pub last_frame: u64,
    pub last_cycle: u64,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
// Summarize observed mapper controls and bank transitions for investigation.
pub struct ForensicMapperActivityReport {
    pub mapper: String,
    pub control_writes: u64,
    pub rom_bank_changes: u64,
    pub ram_bank_changes: u64,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
// Retain unsupported-opcode counts and the latest associated bank, PC and symbol.
pub struct ForensicUnsupportedOpcodeReport {
    pub opcode: u8,
    pub count: u64,
    pub last_rom_bank: u16,
    pub last_pc: u16,
    pub last_symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
// Group candidate findings and follow-up notes; generated indicates report availability,
// not proof that a candidate is an actual program defect.
pub struct RomForensicsReport {
    pub generated: bool,
    pub hot_loop_candidates: Vec<ForensicHotLoopReport>,
    pub mmio_hotspots: Vec<ForensicHotspotReport>,
    pub suspicious_writes: Vec<ForensicHotspotReport>,
    pub mapper_activity: Vec<ForensicMapperActivityReport>,
    pub unsupported_hotspots: Vec<ForensicUnsupportedOpcodeReport>,
    pub triage: Vec<String>,
    pub carry_forward_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
// Combine dynamic samples/cycles with optional static function estimates and source metadata.
pub struct ProfilerFunctionActivityReport {
    pub name: String,
    pub bank: u16,
    pub section: Option<String>,
    pub samples: u64,
    pub cycles: u64,
    pub unique_pcs: u32,
    pub hottest_pc: u16,
    pub last_pc: u16,
    pub source: Option<SourceLocationReport>,
    pub static_estimate: Option<ToolchainStaticEstimateReport>,
    pub build_hotspot_score: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
// Aggregate per-bank activity and transitions, retaining the leading function labels.
pub struct ProfilerBankActivityReport {
    pub bank: u16,
    pub samples: u64,
    pub cycles: u64,
    pub unique_pcs: u32,
    pub switch_in_count: u64,
    pub switch_out_count: u64,
    pub far_call_count: u64,
    pub top_functions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
// Count a directed bank transition and associated far calls with optional endpoint symbols.
pub struct ProfilerBankTransitionReport {
    pub from_bank: u16,
    pub to_bank: u16,
    pub count: u64,
    pub far_call_count: u64,
    pub from_symbol: Option<String>,
    pub to_symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
// State the sampling unit explicitly alongside function, bank and transition activity.
pub struct ProfilerReport {
    pub generated: bool,
    pub sample_unit: String,
    pub function_activity: Vec<ProfilerFunctionActivityReport>,
    pub bank_activity: Vec<ProfilerBankActivityReport>,
    pub bank_transitions: Vec<ProfilerBankTransitionReport>,
    pub carry_forward_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
// Describe an address bucket, its region label and the accumulated observation count.
pub struct HeatmapBucketReport {
    pub region: String,
    pub start: u16,
    pub end: u16,
    pub count: u64,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
// Keep a region-wide count separately from individual heatmap buckets.
pub struct HeatmapRegionTotalReport {
    pub region: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize)]
// Associate sampled activity with a candidate function for heatmap investigation.
pub struct HeatmapCulpritFunctionReport {
    pub name: String,
    pub bank: u16,
    pub samples: u64,
    pub cycles: u64,
}

#[derive(Debug, Clone, Serialize)]
// Associate sampled activity with a candidate bank for heatmap investigation.
pub struct HeatmapCulpritBankReport {
    pub bank: u16,
    pub samples: u64,
    pub cycles: u64,
}

#[derive(Debug, Clone, Serialize)]
// Group memory buckets, region totals and candidate activity correlations with their notes.
pub struct HeatmapReport {
    pub generated: bool,
    pub buckets: Vec<HeatmapBucketReport>,
    pub region_totals: Vec<HeatmapRegionTotalReport>,
    pub culprit_functions: Vec<HeatmapCulpritFunctionReport>,
    pub culprit_banks: Vec<HeatmapCulpritBankReport>,
    pub carry_forward_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
// Capture the register subset used by ABI checks, including the combined HL pair.
pub struct AbiRegisterSnapshotReport {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub hl: u16,
    pub sp: u16,
    pub pc: u16,
}

#[derive(Debug, Clone, Serialize)]
// Keep bounded stack/argument previews, truncation flags and optional decoded return data
// together with the expected argument and result sizes.
pub struct AbiStackWindowReport {
    pub sp: u16,
    pub region: String,
    pub preview_bytes: Vec<u8>,
    pub preview_truncated: bool,
    pub decoded_return_address: Option<u16>,
    pub decoded_return_region: Option<String>,
    pub expected_stack_argument_bytes: u16,
    pub expected_return_size: u8,
    pub argument_base: Option<u16>,
    pub argument_preview_bytes: Vec<u8>,
    pub argument_preview_truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
// Compare optional caller/callee-entry values against a stated register expectation.
pub struct AbiBoundaryRegisterCheckReport {
    pub register: String,
    #[serde(default)]
    pub caller_value: Option<u16>,
    #[serde(default)]
    pub callee_entry_value: Option<u16>,
    pub expectation: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
// Carry the register role, confidence and explanation used to interpret boundary observations.
pub struct AbiRegisterContractRuleReport {
    pub register: String,
    pub role: String,
    pub confidence: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
// Combine call-edge metadata, optional contexts and register checks without inventing missing observations.
pub struct AbiCallBoundaryReport {
    #[serde(default)]
    pub caller_function: Option<String>,
    #[serde(default)]
    pub caller_bank: Option<u16>,
    pub callee_function: String,
    pub callee_bank: u16,
    #[serde(default)]
    pub edge_kind: Option<String>,
    pub via_thunk: bool,
    pub via_farcall: bool,
    pub cross_bank: bool,
    #[serde(default)]
    pub caller_context: Option<ExecutionContextFrame>,
    #[serde(default)]
    pub callee_entry_context: Option<ExecutionContextFrame>,
    #[serde(default)]
    pub contract_rules: Vec<AbiRegisterContractRuleReport>,
    #[serde(default)]
    pub register_checks: Vec<AbiBoundaryRegisterCheckReport>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
// Compare optional callee and resumed-caller values against a stated return-boundary expectation.
pub struct AbiReturnRegisterCheckReport {
    pub register: String,
    #[serde(default)]
    pub callee_value: Option<u16>,
    #[serde(default)]
    pub caller_resume_value: Option<u16>,
    pub expectation: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
// Retain pre-call, near-return and resumed contexts alongside edge information and contract checks.
pub struct AbiReturnBoundaryReport {
    pub caller_function: String,
    pub caller_bank: u16,
    pub callee_function: String,
    pub callee_bank: u16,
    #[serde(default)]
    pub edge_kind: Option<String>,
    pub via_thunk: bool,
    pub via_farcall: bool,
    pub cross_bank: bool,
    pub callee_near_ret: bool,
    #[serde(default)]
    pub bytes_to_callee_end: Option<u32>,
    #[serde(default)]
    pub caller_pre_call_context: Option<ExecutionContextFrame>,
    #[serde(default)]
    pub callee_last_context: Option<ExecutionContextFrame>,
    #[serde(default)]
    pub caller_resume_context: Option<ExecutionContextFrame>,
    #[serde(default)]
    pub contract_rules: Vec<AbiRegisterContractRuleReport>,
    #[serde(default)]
    pub register_checks: Vec<AbiReturnRegisterCheckReport>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
// Describe the observed before/after values and expectation for one intrinsic register.
pub struct AbiIntrinsicRegisterCheckReport {
    pub register: String,
    #[serde(default)]
    pub before_value: Option<u16>,
    #[serde(default)]
    pub after_value: Option<u16>,
    pub expectation: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
// State intrinsic contract strength and available contexts separately from observed register changes.
pub struct AbiIntrinsicBoundaryReport {
    pub intrinsic_symbol: String,
    pub intrinsic_kind: String,
    pub contract_strength: String,
    #[serde(default)]
    pub before_context: Option<ExecutionContextFrame>,
    pub intrinsic_context: ExecutionContextFrame,
    #[serde(default)]
    pub after_context: Option<ExecutionContextFrame>,
    #[serde(default)]
    pub contract_rules: Vec<AbiRegisterContractRuleReport>,
    #[serde(default)]
    pub register_checks: Vec<AbiIntrinsicRegisterCheckReport>,
    #[serde(default)]
    pub observed_changed_registers: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
// Combine compiler issues, runtime boundary evidence, register/stack snapshots and explicit status.
// Availability of this payload is separate from a successful ABI verification result.
pub struct AbiVerificationReport {
    pub generated: bool,
    pub abi_mode: Option<String>,
    pub toolchain_issue_count: u32,
    pub status: String,
    pub toolchain_issues: Vec<String>,
    pub current_function_contract: Option<ToolchainFunctionReport>,
    #[serde(default)]
    pub call_boundary: Option<AbiCallBoundaryReport>,
    #[serde(default)]
    pub return_boundary: Option<AbiReturnBoundaryReport>,
    #[serde(default)]
    pub intrinsic_boundaries: Vec<AbiIntrinsicBoundaryReport>,
    pub register_snapshot: AbiRegisterSnapshotReport,
    pub stack_window: AbiStackWindowReport,
    pub runtime_alerts: Vec<String>,
    pub carry_forward_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
// Show active memory banks together with their raw CGB control registers and speed/mode flags.
pub struct VisualizationBankStateReport {
    pub current_rom_bank: u16,
    pub current_ram_bank: u16,
    pub active_vram_bank: u8,
    pub active_wram_bank: u8,
    pub vbk_register: u8,
    pub svbk_register: u8,
    pub cgb_mode: bool,
    pub cgb_double_speed: bool,
}

#[derive(Debug, Clone, Serialize)]
// Describe one layer's enable/change flags, digest and explanatory detail.
pub struct VisualizationLayerReport {
    pub key: String,
    pub enabled: bool,
    pub changed: bool,
    pub hash: u32,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
// Preserve palette index and packed RGB555 colors for display consumers.
pub struct VisualizationPalettePreviewReport {
    pub palette_index: u8,
    pub colors_rgb555: Vec<u16>,
}

#[derive(Debug, Clone, Serialize)]
// Keep DMG palette mappings separate from CGB palette previews and blocked-write counts.
pub struct VisualizationPaletteReport {
    pub dmg_bgp: Vec<u8>,
    pub dmg_obp0: Vec<u8>,
    pub dmg_obp1: Vec<u8>,
    pub cgb_bg: Vec<VisualizationPalettePreviewReport>,
    pub cgb_obj: Vec<VisualizationPalettePreviewReport>,
    pub bg_palette_index: u8,
    pub obj_palette_index: u8,
    pub blocked_write_count: u64,
}

#[derive(Debug, Clone, Serialize)]
// Summarize a tile by bank/index, nonzero byte count and digest rather than embedding image pixels.
pub struct VisualizationTilePreviewReport {
    pub bank: u8,
    pub tile_index: u16,
    pub nonzero_bytes: u8,
    pub hash: u32,
}

#[derive(Debug, Clone, Serialize)]
// Describe selected maps/addressing and bounded tile previews for both VRAM banks.
pub struct VisualizationTileReport {
    pub bg_map_base: u16,
    pub window_map_base: u16,
    pub tile_data_8000: bool,
    pub bank0_nonzero_tiles: u16,
    pub bank1_nonzero_tiles: u16,
    pub bank0_preview: Vec<VisualizationTilePreviewReport>,
    pub bank1_preview: Vec<VisualizationTilePreviewReport>,
}

#[derive(Debug, Clone, Serialize)]
// Carry an OAM slot's raw coordinate, tile and attribute bytes for inspection.
pub struct VisualizationSpriteReport {
    pub slot: u8,
    pub y: u8,
    pub x: u8,
    pub tile: u8,
    pub attrs: u8,
}

#[derive(Debug, Clone, Serialize)]
// Keep sprite-height settings, populated OAM counts and estimated visibility distinct.
pub struct VisualizationOamReport {
    pub sprite_height: u8,
    pub nonzero_entries: u32,
    pub visible_sprite_estimate: u32,
    pub sample_sprites: Vec<VisualizationSpriteReport>,
}

#[derive(Debug, Clone, Serialize)]
// Group DMA lifecycle counts and estimated stall cycles for display, without issuing DMA transfers.
pub struct VisualizationDmaReport {
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
}

#[derive(Debug, Clone, Serialize)]
// Expose APU control state, queue counters and wave preview; this is not an audio recording.
pub struct VisualizationApuReport {
    pub master_enabled: bool,
    pub channel_enable_mask: u8,
    pub frame_sequencer_step: u8,
    pub nr50: u8,
    pub nr51: u8,
    pub nr52: u8,
    pub pcm12: u8,
    pub pcm34: u8,
    pub sample_rate: u32,
    pub buffered_frames: u32,
    pub dropped_frames: u64,
    pub generated_frames: u64,
    pub wave_preview: Vec<u8>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
// Group optional-report availability and display data for banks, layers, palettes, tiles, OAM, DMA and APU.
pub struct VisualizationsReport {
    pub generated: bool,
    pub bank_state: VisualizationBankStateReport,
    pub layers: Vec<VisualizationLayerReport>,
    pub palettes: VisualizationPaletteReport,
    pub tiles: VisualizationTileReport,
    pub oam: VisualizationOamReport,
    pub dma: VisualizationDmaReport,
    pub apu: VisualizationApuReport,
    pub carry_forward_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
// Record packaging presence flags; file presence alone says nothing about its contents or license validity.
pub struct ReleasePackagingSurfaceReport {
    pub license_present: bool,
    pub readme_present: bool,
    pub c_header_present: bool,
    pub python_bridge_present: bool,
    pub samples_present: bool,
    pub docs_present: bool,
    pub manifest_present: bool,
}

#[derive(Debug, Clone, Serialize)]
// Keep individual validation flags explicit, especially whether only static checks were performed.
pub struct ReleaseSmokeSurfaceReport {
    pub static_checks_only: bool,
    pub json_schema_validated: bool,
    pub python_bridge_py_compile_validated: bool,
    pub c_header_export_surface_checked: bool,
    pub rust_build_validated: bool,
    pub workspace_tests_validated: bool,
    pub regression_validated: bool,
}

#[derive(Debug, Clone, Serialize)]
// Describe observed diagnostics and report availability without implying general hardware compatibility.
pub struct ReleaseStabilitySurfaceReport {
    pub unsupported_opcode_count: u64,
    pub diagnostics_count: usize,
    pub events_recorded: usize,
    pub replay_surface_available: bool,
    pub auto_diagnosis_available: bool,
    pub rom_forensics_available: bool,
    pub timing_pack_surface_available: bool,
    pub visualization_surface_available: bool,
}

#[derive(Debug, Clone, Serialize)]
// Carry readiness assessment, blockers and recommended commands as data.
// This model neither executes the commands nor performs publication.
pub struct ReleaseReadinessReport {
    pub generated: bool,
    pub readiness_level: String,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub recommended_next_commands: Vec<String>,
    pub packaging: ReleasePackagingSurfaceReport,
    pub smoke: ReleaseSmokeSurfaceReport,
    pub stability: ReleaseStabilitySurfaceReport,
    pub carry_forward_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
// Serialize a complete debug observation with optional analysis sections. An absent
// section denotes unavailable output rather than a successful check with no findings.
pub struct DebugReport {
    pub schema_version: &'static str,
    pub meta: ReportMeta,
    pub cpu: CpuSnapshot,
    pub video: VideoSnapshot,
    pub timer: TimerSnapshot,
    pub symbols: Option<SymbolReport>,
    pub toolchain_build: Option<ToolchainBuildReport>,
    pub summary: EventSummary,
    pub unsupported_opcodes: Vec<UnsupportedOpcodeSummary>,
    pub watched_memory: Vec<MemoryWatchResult>,
    pub replay: Option<ReplayReport>,
    pub profiler: Option<ProfilerReport>,
    pub heatmap: Option<HeatmapReport>,
    pub abi_verification: Option<AbiVerificationReport>,
    pub visualizations: Option<VisualizationsReport>,
    pub timing_packs: Option<TimingPackReport>,
    pub auto_diagnosis: Option<AutoDiagnosisReport>,
    pub rom_forensics: Option<RomForensicsReport>,
    pub release_readiness: Option<ReleaseReadinessReport>,
    pub stop_reason: Option<StopReason>,
    pub events: Vec<DebugEvent>,
    pub diagnostics: Vec<Diagnostic>,
}
