//! Typed command-line options.
//!
//! Use clap enums to represent hardware modes, stopping conditions and output
//! formats before passing the selected settings to execution logic.

use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum)]
// Typed hardware selection; Auto leaves mode detection to the ROM-loading path.
pub enum HardwareArg {
    Auto,
    Dmg,
    Cgb,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
// Select the timeline serializer after execution has collected its events.
pub enum TimelineFormatArg {
    Jsonl,
    Csv,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
// Select initial, previous-frame or explicitly named reference data for memory-watch comparisons.
pub enum WatchBaselineModeArg {
    Initial,
    PreviousFrame,
    Named,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
// Share output-format choices between decompilation and disassembly.
pub enum DecompileFormatArg {
    Json,
    Markdown,
    Text,
}

#[derive(Debug, Parser)]
#[command(name = "kokuradbg")]
// Clap stores raw option values here. The execution layer parses structured strings
// and resolves jobs, defaults and combinations; this type alone does not validate their semantics.
pub struct Args {
    // Keep the positional ROM optional so a job, link job or regression matrix can provide its own inputs.
    pub rom: Option<String>,

    #[arg(long, value_enum, default_value_t = HardwareArg::Auto)]
    pub hardware: HardwareArg,

    #[arg(long)]
    pub job: Option<String>,

    #[arg(long = "link-job")]
    pub link_job: Option<String>,

    #[arg(long = "link-topology")]
    pub link_topology: Option<String>,

    #[arg(long = "link-initial-peer-slot")]
    pub link_initial_peer_slot: Option<u8>,

    #[arg(long = "link-session")]
    // Collect repeated link-session specifications for parsing by the link execution path.
    pub link_sessions: Vec<String>,

    #[arg(long)]
    pub regression_matrix: Option<String>,

    #[arg(long, default_value_t = 1)]
    // Default direct execution to a one-frame budget; unsigned parsing rejects negative command-line values.
    pub run_frames: u64,

    #[arg(long)]
    pub input: Option<String>,

    #[arg(long)]
    pub input_seq: Option<String>,

    #[arg(long)]
    // Retain the requested state input path; loading and resume precedence are handled during execution.
    pub load_state: Option<String>,

    #[arg(long = "resume-state")]
    pub resume_state: Option<String>,

    #[arg(long)]
    pub save_state: Option<String>,

    #[arg(long)]
    pub dump_report: Option<String>,

    #[arg(long = "dump-replay-tape")]
    // Replay export, comparison and mismatch snapshots are separate optional artifact requests.
    pub dump_replay_tape: Option<String>,

    #[arg(long = "compare-replay-tape")]
    pub compare_replay_tape: Option<String>,

    #[arg(long = "snapshot-on-replay-mismatch")]
    pub snapshot_on_replay_mismatch: Option<String>,

    #[arg(long)]
    pub screenshot: Option<String>,

    #[arg(long = "png")]
    pub png: Option<String>,

    #[arg(long = "snapshot")]
    pub snapshot: Option<String>,

    #[arg(long = "trace-jsonl")]
    pub trace_jsonl: Option<String>,

    #[arg(long = "diagnostics-jsonl")]
    pub diagnostics_jsonl: Option<String>,

    #[arg(long = "repro-bundle")]
    pub repro_bundle: Option<String>,

    #[arg(long = "png-on-diagnostic")]
    pub png_on_diagnostic: Option<String>,

    #[arg(long = "snapshot-on-diagnostic")]
    pub snapshot_on_diagnostic: Option<String>,

    #[arg(long = "break-on-diagnostic")]
    // Collect repeated diagnostic stop filters rather than interpreting event names in clap.
    pub break_on_diagnostic: Vec<String>,

    #[arg(long = "input-script")]
    pub input_script: Option<String>,

    #[arg(long = "screenshot-frames")]
    pub screenshot_frames: Option<String>,

    #[arg(long = "record-wav")]
    pub record_wav: Option<String>,

    #[arg(long = "record-wav-frames")]
    // Defer audio frame-range syntax to the recorder setup; this is not a sample count.
    pub record_wav_frames: Option<String>,

    #[arg(long = "record-video")]
    pub record_video: Option<String>,

    #[arg(long = "record-video-frames")]
    // Defer video frame-range syntax to the recorder setup.
    pub record_video_frames: Option<String>,

    #[arg(long = "audio-buffer-frames")]
    // Measure audio queue capacity in stereo frames, not individual channel samples.
    pub audio_buffer_frames: Option<usize>,

    #[arg(long)]
    pub symbols: Option<String>,

    #[arg(long = "source-map")]
    pub source_map: Option<String>,

    #[arg(long = "toolchain-metadata")]
    pub toolchain_metadata: Option<String>,

    #[arg(long = "watch-window")]
    // Preserve repeated raw memory-window specifications for execution-layer parsing.
    pub watch_windows: Vec<String>,

    #[arg(long = "watch-baseline-mode", value_enum, default_value_t = WatchBaselineModeArg::Initial)]
    pub watch_baseline_mode: WatchBaselineModeArg,

    #[arg(long = "watch-baseline-tag")]
    // Carry the named-baseline selector separately from the chosen baseline mode.
    pub watch_baseline_tag: Option<String>,

    #[arg(long = "capture-watch-baseline")]
    // Collect explicit baseline capture requests without changing memory during argument parsing.
    pub capture_watch_baseline: Vec<String>,

    #[arg(long = "watch-fields")]
    pub watch_fields: Option<String>,

    #[arg(long = "report-sections")]
    // Retain the report-section selection string for filtering the generated report.
    pub report_sections: Option<String>,

    #[arg(long = "report-minimal")]
    pub report_minimal: Option<String>,

    #[arg(long = "snapshot-at")]
    pub snapshot_at: Vec<String>,

    #[arg(long = "run-until")]
    // Collect condition strings; parsing and condition checks occur in the execution path.
    pub run_until: Vec<String>,

    #[arg(long = "trace-point")]
    pub trace_points: Vec<String>,

    #[arg(long = "timeline-out")]
    // Separate the output destination from the selected timeline encoding.
    pub timeline_out: Option<String>,

    #[arg(long = "timeline-format", value_enum, default_value_t = TimelineFormatArg::Jsonl)]
    pub timeline_format: TimelineFormatArg,

    #[arg(long = "breakpoint")]
    // Accept multiple instruction breakpoint specifications.
    pub breakpoints: Vec<String>,

    #[arg(long = "watchpoint")]
    // Accept multiple memory-access watchpoint specifications.
    pub watchpoints: Vec<String>,

    #[arg(long = "stop-on-mmio")]
    // Retain repeated MMIO, IRQ and DMA stop selectors for their respective parsers.
    pub stop_on_mmio: Vec<String>,

    #[arg(long = "stop-on-irq")]
    pub stop_on_irq: Vec<String>,

    #[arg(long = "stop-on-dma")]
    pub stop_on_dma: Vec<String>,

    #[arg(long = "replay-interval")]
    // Keep replay checkpoint interval, history capacity and rewind distance independently optional.
    pub replay_interval: Option<u64>,

    #[arg(long = "replay-max-checkpoints")]
    pub replay_max_checkpoints: Option<usize>,

    #[arg(long = "rewind-on-stop-frames")]
    pub rewind_on_stop_frames: Option<u64>,

    #[arg(long = "stop-on-divergence", default_value_t = false)]
    pub stop_on_divergence: bool,

    #[arg(long = "compare-replay-watch-only", default_value_t = false)]
    pub compare_replay_watch_only: bool,

    #[arg(long = "decompile-out")]
    pub decompile_out: Option<String>,

    #[arg(long = "decompile-format", value_enum, default_value_t = DecompileFormatArg::Json)]
    pub decompile_format: DecompileFormatArg,

    #[arg(long = "decompile-function")]
    // Collect requested function selectors; decompile_all is a separate opt-in flag.
    pub decompile_functions: Vec<String>,

    #[arg(long = "decompile-all", default_value_t = false)]
    pub decompile_all: bool,

    #[arg(long = "decompile-annotations")]
    // Carry optional annotation and execution-trace inputs for the decompiler.
    pub decompile_annotations: Option<String>,

    #[arg(long = "decompile-trace")]
    pub decompile_trace: Option<String>,

    #[arg(long = "disassemble-out")]
    pub disassemble_out: Option<String>,

    #[arg(long = "disassemble-format", value_enum, default_value_t = DecompileFormatArg::Text)]
    pub disassemble_format: DecompileFormatArg,

    #[arg(long = "disassemble-range")]
    // Collect bank/address range strings for disassembly rather than treating them as file paths.
    pub disassemble_ranges: Vec<String>,

    #[arg(long = "emit-diagnostics")]
    pub emit_diagnostics: Option<String>,

    #[arg(long = "diagnostic-pack")]
    pub diagnostic_pack: Option<String>,

    #[arg(long = "diagnostic-rule")]
    // Collect repeated diagnostic rules; the optional summary limit bounds displayed findings.
    pub diagnostic_rules: Vec<String>,

    #[arg(long = "diagnostic-summary-limit")]
    pub diagnostic_summary_limit: Option<usize>,
}
