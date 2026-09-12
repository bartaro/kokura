//! CLIオプションの型定義。
//!
//! ここでは文字列を実行ロジックへ直接渡さず、ハードウェアモード、停止条件、
//! 出力形式などを `clap` の列挙型へ変換します。

use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum HardwareArg {
    Auto,
    Dmg,
    Cgb,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum TimelineFormatArg {
    Jsonl,
    Csv,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum WatchBaselineModeArg {
    Initial,
    PreviousFrame,
    Named,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DecompileFormatArg {
    Json,
    Markdown,
    Text,
}

#[derive(Debug, Parser)]
#[command(name = "kokuradbg")]
pub struct Args {
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
    pub link_sessions: Vec<String>,

    #[arg(long)]
    pub regression_matrix: Option<String>,

    #[arg(long, default_value_t = 1)]
    pub run_frames: u64,

    #[arg(long)]
    pub input: Option<String>,

    #[arg(long)]
    pub input_seq: Option<String>,

    #[arg(long)]
    pub load_state: Option<String>,

    #[arg(long = "resume-state")]
    pub resume_state: Option<String>,

    #[arg(long)]
    pub save_state: Option<String>,

    #[arg(long)]
    pub dump_report: Option<String>,

    #[arg(long = "dump-replay-tape")]
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
    pub break_on_diagnostic: Vec<String>,

    #[arg(long = "input-script")]
    pub input_script: Option<String>,

    #[arg(long = "screenshot-frames")]
    pub screenshot_frames: Option<String>,

    #[arg(long = "record-wav")]
    pub record_wav: Option<String>,

    #[arg(long = "record-wav-frames")]
    pub record_wav_frames: Option<String>,

    #[arg(long = "record-video")]
    pub record_video: Option<String>,

    #[arg(long = "record-video-frames")]
    pub record_video_frames: Option<String>,

    #[arg(long = "audio-buffer-frames")]
    pub audio_buffer_frames: Option<usize>,

    #[arg(long)]
    pub symbols: Option<String>,

    #[arg(long = "source-map")]
    pub source_map: Option<String>,

    #[arg(long = "toolchain-metadata")]
    pub toolchain_metadata: Option<String>,

    #[arg(long = "watch-window")]
    pub watch_windows: Vec<String>,

    #[arg(long = "watch-baseline-mode", value_enum, default_value_t = WatchBaselineModeArg::Initial)]
    pub watch_baseline_mode: WatchBaselineModeArg,

    #[arg(long = "watch-baseline-tag")]
    pub watch_baseline_tag: Option<String>,

    #[arg(long = "capture-watch-baseline")]
    pub capture_watch_baseline: Vec<String>,

    #[arg(long = "watch-fields")]
    pub watch_fields: Option<String>,

    #[arg(long = "report-sections")]
    pub report_sections: Option<String>,

    #[arg(long = "report-minimal")]
    pub report_minimal: Option<String>,

    #[arg(long = "snapshot-at")]
    pub snapshot_at: Vec<String>,

    #[arg(long = "run-until")]
    pub run_until: Vec<String>,

    #[arg(long = "trace-point")]
    pub trace_points: Vec<String>,

    #[arg(long = "timeline-out")]
    pub timeline_out: Option<String>,

    #[arg(long = "timeline-format", value_enum, default_value_t = TimelineFormatArg::Jsonl)]
    pub timeline_format: TimelineFormatArg,

    #[arg(long = "breakpoint")]
    pub breakpoints: Vec<String>,

    #[arg(long = "watchpoint")]
    pub watchpoints: Vec<String>,

    #[arg(long = "stop-on-mmio")]
    pub stop_on_mmio: Vec<String>,

    #[arg(long = "stop-on-irq")]
    pub stop_on_irq: Vec<String>,

    #[arg(long = "stop-on-dma")]
    pub stop_on_dma: Vec<String>,

    #[arg(long = "replay-interval")]
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
    pub decompile_functions: Vec<String>,

    #[arg(long = "decompile-all", default_value_t = false)]
    pub decompile_all: bool,

    #[arg(long = "decompile-annotations")]
    pub decompile_annotations: Option<String>,

    #[arg(long = "decompile-trace")]
    pub decompile_trace: Option<String>,

    #[arg(long = "disassemble-out")]
    pub disassemble_out: Option<String>,

    #[arg(long = "disassemble-format", value_enum, default_value_t = DecompileFormatArg::Text)]
    pub disassemble_format: DecompileFormatArg,

    #[arg(long = "disassemble-range")]
    pub disassemble_ranges: Vec<String>,

    #[arg(long = "emit-diagnostics")]
    pub emit_diagnostics: Option<String>,

    #[arg(long = "diagnostic-pack")]
    pub diagnostic_pack: Option<String>,

    #[arg(long = "diagnostic-rule")]
    pub diagnostic_rules: Vec<String>,

    #[arg(long = "diagnostic-summary-limit")]
    pub diagnostic_summary_limit: Option<usize>,
}
