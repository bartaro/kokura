//! Coordinate ROM execution, observation and image/audio/report output.
//!
//! Keep CLI-specific I/O here; delegate emulation state transitions to
//! kokura-core and diagnostic/report data types to kokura-debug.

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use gif::{Encoder as GifEncoder, Frame as GifFrame, Repeat};
use image::{ImageBuffer, Rgb};
use kokura_bridge::{
    analyze_rom, apply_annotations, apply_trace_observations, disassemble_range,
    map_parser::parse_map_file, parse_source_map_file, DecodedInstruction, DecompileAnnotationFile,
    DecompileOptions, DecompileReport, DecompileTraceObservation, SymbolTable,
};
use kokura_core::{
    diagnostic_events::DiagnosticEvent, state::MachineState, types::HardwareMode, Machine,
};
use kokura_debug::report::{
    ReplayConditionalSnapshotCaptureReport, ReplayConditionalSnapshotPlanReport,
    ReplayConditionalSnapshotTriggerReport, ReplayDivergenceHelperReport,
    ReplayReferenceComparisonReport, ReplayReferenceMismatchReport, ReplayReport,
};
use kokura_debug::DebugEvent;
use kokura_debug::{
    watch::MemoryWatchBaselineMode, DebugReport, DebugSession, DmaStopSpec, ExecuteBreakpointSpec,
    InterruptStopPhase, InterruptStopSpec, LinkRunSummary, LinkTopology, MemoryWatchResult,
    MemoryWatchSpec, MemoryWatchpointSpec, MmioWriteStopSpec, ReplayControlSet, StopConditionSet,
    TimingAwareLinkRunner, ToolchainBuildReport,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue};
use zip::{write::FileOptions, ZipWriter};

use crate::{
    args::{Args, DecompileFormatArg, HardwareArg, TimelineFormatArg, WatchBaselineModeArg},
    json_io::{
        load_job_file, load_link_job_file, load_regression_matrix, ExpectedSuggestion, JobSpec,
        JobStage, LinkJobSpec, RegressionCase, SnapshotTrigger, JOB_SPEC_SCHEMA_VERSION,
        LINK_JOB_SPEC_SCHEMA_VERSION, REGRESSION_MATRIX_SCHEMA_VERSION,
    },
};

pub const CLI_OUTPUT_SCHEMA_VERSION: &str = "1";

#[derive(Debug, Clone, Serialize)]
// Keep an actionable capture proposal separate from any snapshot actually taken.
struct SnapshotSuggestion {
    kind: String,
    value: String,
    reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
// Identify which observation point produced a timeline row or condition match.
enum ObservationBasis {
    FrameStart,
    Step,
    FrameEnd,
    Event,
    TracePoint,
    Snapshot,
    Stop,
}

#[derive(Debug, Clone, Serialize)]
// Combine sampled execution coordinates with bounded watch/event summaries and optional detailed values.
struct TimelineRow {
    basis: ObservationBasis,
    label: String,
    completed_frames: u64,
    active_frame: u64,
    cycle: u64,
    ly: u8,
    pc: u16,
    rom_bank: u16,
    ppu_mode: String,
    symbol: Option<String>,
    source: Option<String>,
    #[serde(default)]
    registers: BTreeMap<String, u64>,
    #[serde(default)]
    flags: BTreeMap<String, bool>,
    #[serde(default)]
    stack_slot_values: BTreeMap<String, u64>,
    watch_summary: String,
    event_summary: String,
    stop_reason: Option<String>,
    watched_memory: Vec<MemoryWatchResult>,
}

#[derive(Debug, Clone)]
enum ObservationTerm {
    Frame(u64),
    Ly(u8),
    Pc(u16),
    Bank(u16),
    BankPc(u16, u16),
    SymbolContains(String),
    SourceContains(String),
    EventContains(String),
    PpuMode(String),
    Basis(ObservationBasis),
}

#[derive(Debug, Clone)]
// Store terms that must all match one observation; evaluation happens in condition_matches.
struct ObservationCondition {
    terms: Vec<ObservationTerm>,
}

#[derive(Debug, Clone)]
struct SnapshotRequest {
    label: String,
    condition: ObservationCondition,
    save_path: Option<String>,
}

#[derive(Debug, Clone)]
struct BaselineCaptureRequest {
    name: String,
    condition: ObservationCondition,
}

#[derive(Debug, Clone)]
struct TracePointRequest {
    label: String,
    condition: ObservationCondition,
}

#[derive(Debug, Clone, Default)]
// Limit serialized report/watch fields without changing the underlying execution or report data.
struct OutputFilterOptions {
    report_sections: Option<BTreeSet<String>>,
    watch_fields: Option<BTreeSet<String>>,
}

struct TimelineWriter {
    format: TimelineFormatArg,
    writer: BufWriter<fs::File>,
    wrote_csv_header: bool,
}

impl TimelineWriter {
    // Create or truncate a buffered timeline file; parent-directory creation is the caller's responsibility.
    fn create(path: &str, format: TimelineFormatArg) -> Result<Self> {
        let file = fs::File::create(path)
            .with_context(|| format!("failed to create timeline output: {path}"))?;
        Ok(Self {
            format,
            writer: BufWriter::new(file),
            wrote_csv_header: false,
        })
    }

    // Write full JSONL rows or a reduced CSV-shaped field list with a single header.
    // The current CSV branch uses JSON string escaping; embedded quotes are not RFC-style CSV escaping.
    fn write_row(&mut self, row: &TimelineRow) -> Result<()> {
        match self.format {
            TimelineFormatArg::Jsonl => {
                serde_json::to_writer(&mut self.writer, row)?;
                self.writer.write_all(b"\n")?;
            }
            TimelineFormatArg::Csv => {
                if !self.wrote_csv_header {
                    self.writer.write_all(
                        b"basis,label,completed_frames,active_frame,cycle,ly,pc,rom_bank,ppu_mode,symbol,source,watch_summary,event_summary,stop_reason\n",
                    )?;
                    self.wrote_csv_header = true;
                }
                let fields = [
                    serde_json::to_string(&format!("{:?}", row.basis).to_ascii_lowercase())?,
                    serde_json::to_string(&row.label)?,
                    row.completed_frames.to_string(),
                    row.active_frame.to_string(),
                    row.cycle.to_string(),
                    row.ly.to_string(),
                    format!("{:04X}", row.pc),
                    row.rom_bank.to_string(),
                    serde_json::to_string(&row.ppu_mode)?,
                    serde_json::to_string(row.symbol.as_deref().unwrap_or(""))?,
                    serde_json::to_string(row.source.as_deref().unwrap_or(""))?,
                    serde_json::to_string(&row.watch_summary)?,
                    serde_json::to_string(&row.event_summary)?,
                    serde_json::to_string(row.stop_reason.as_deref().unwrap_or(""))?,
                ];
                self.writer.write_all(fields.join(",").as_bytes())?;
                self.writer.write_all(b"\n")?;
            }
        }
        Ok(())
    }

    // Flush buffered timeline bytes so output errors are returned before the writer is dropped.
    fn finish(&mut self) -> Result<()> {
        self.writer.flush().map_err(anyhow::Error::from)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FrameRange {
    start: u64,
    end: u64,
}

impl FrameRange {
    // Test inclusive frame bounds for capture selection.
    fn contains(self, frame: u64) -> bool {
        frame >= self.start && frame <= self.end
    }
}

#[derive(Debug, Clone)]
// Buffer selected screenshots, RGB555 companions, video frames and audio until output is flushed.
// Executed-frame counting is relative to this capture run, independent of restored machine frame totals.
struct OutputCaptureState {
    screenshot_path: Option<String>,
    screenshot_range: Option<FrameRange>,
    captured_screenshots: Vec<(u64, Vec<u8>)>,
    captured_screenshot_rgb555: Vec<(u64, Vec<u16>)>,
    screenshot_cgb_compat_mode: bool,
    record_video_path: Option<String>,
    record_video_range: FrameRange,
    captured_video_frames: Vec<(u64, Vec<u8>)>,
    record_wav_path: Option<String>,
    record_wav_range: FrameRange,
    recorded_audio: Vec<i16>,
    executed_frames: u64,
}

#[derive(Debug, Clone, Serialize)]
// Attach one stage report, optional saved state and heuristic difference/capture suggestions.
struct StageOutput {
    schema_version: &'static str,
    stage_index: usize,
    input: Option<String>,
    frames: u64,
    report: DebugReport,
    saved_state: Option<String>,
    diff_summary: Vec<String>,
    diff_score: u32,
    diff_severity: String,
    snapshot_suggestions: Vec<SnapshotSuggestion>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
// Serialize either a single report or a staged object without an extra enum tag.
enum OutputEnvelope {
    Single(DebugReport),
    Staged {
        schema_version: &'static str,
        job_schema_version: &'static str,
        final_report: DebugReport,
        stage_reports: Vec<StageOutput>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Wrap replay observations with ROM identity metadata; loading this structure alone does not validate identity.
struct ReplayTapeEnvelope {
    schema_version: String,
    rom_path: String,
    #[serde(default)]
    rom_sha256: Option<String>,
    replay: ReplayReport,
}

#[derive(Debug, Clone, Serialize)]
struct RegressionMatrixOutput {
    schema_version: &'static str,
    matrix_schema_version: &'static str,
    matrix_path: String,
    total_cases: usize,
    executed_cases: usize,
    passed_cases: usize,
    failed_cases: usize,
    skipped_cases: usize,
    cases: Vec<RegressionCaseResult>,
}

#[derive(Debug, Clone, Serialize)]
// Retain actual metrics and per-expectation outcomes so skipped, executed and failed cases remain distinguishable.
struct RegressionCaseResult {
    name: String,
    job: String,
    stage_index: usize,
    passed: bool,
    skipped: bool,
    skip_reason: Option<String>,
    actual_diff_severity: Option<String>,
    actual_bank_switch_count: u64,
    bank_switch_floor_passed: bool,
    actual_far_call_count: u64,
    far_call_floor_passed: bool,
    actual_intrinsic_count: u64,
    intrinsic_floor_passed: bool,
    actual_oam_dma_count: u64,
    oam_dma_floor_passed: bool,
    actual_hdma_block_count: u64,
    hdma_block_floor_passed: bool,
    actual_dma_complete_count: u64,
    dma_complete_floor_passed: bool,
    actual_dma_stall_cycles: u64,
    dma_stall_cycle_floor_passed: bool,
    actual_bank_thrash_score: u32,
    bank_thrash_floor_passed: bool,
    actual_timer_interrupt_count: u64,
    timer_interrupt_floor_passed: bool,
    actual_vblank_count: u64,
    vblank_floor_passed: bool,
    actual_event_type_counts: BTreeMap<String, u64>,
    event_count_floor_passed: bool,
    matched_event_count_floors: Vec<String>,
    missing_event_count_floors: Vec<String>,
    matched_suggestions: Vec<String>,
    missing_suggestions: Vec<String>,
    matched_diagnostics: Vec<String>,
    missing_diagnostics: Vec<String>,
    extra_diagnostics: Vec<String>,
    matched_unsupported_opcodes: Vec<String>,
    missing_unsupported_opcodes: Vec<String>,
    extra_unsupported_opcodes: Vec<String>,
    matched_event_types: Vec<String>,
    missing_event_types: Vec<String>,
    extra_event_types: Vec<String>,
    matched_watch_changes: Vec<String>,
    missing_watch_changes: Vec<String>,
    extra_watch_changes: Vec<String>,
    stage_count: usize,
}

#[derive(Debug, Clone, Serialize)]
// Report requested link frames alongside actual runner/session results; requested duration alone is not completion proof.
struct LinkOutputEnvelope {
    schema_version: &'static str,
    link_job_schema_version: &'static str,
    topology: String,
    run_frames: u64,
    runner_summary: LinkRunSummary,
    session_order_note: String,
    sessions: Vec<LinkSessionOutput>,
}

#[derive(Debug, Clone, Serialize)]
struct LinkSessionOutput {
    runner_index: usize,
    name: Option<String>,
    slot: Option<u8>,
    rom_path: String,
    input: Option<String>,
    input_sequence: Option<String>,
    report: DebugReport,
    saved_state: Option<String>,
}

// Dispatch by output mode in priority order: decompile, disassemble, regression matrix,
// link job, inline link, then ordinary execution. Write the selected report or print JSON;
// a completed regression command returns success even if its result contains failed cases.
pub fn run(args: Args) -> Result<()> {
    if let Some(path) = &args.decompile_out {
        let rendered = run_decompile(&args)?;
        fs::write(path, rendered)
            .with_context(|| format!("failed to write decompile output: {}", path))?;
        return Ok(());
    }

    if let Some(path) = &args.disassemble_out {
        let rendered = run_disassemble(&args)?;
        fs::write(path, rendered)
            .with_context(|| format!("failed to write disassembly output: {}", path))?;
        return Ok(());
    }

    if let Some(matrix_path) = &args.regression_matrix {
        let output = run_regression_matrix(matrix_path)?;
        let json = serde_json::to_string_pretty(&output)?;
        if let Some(path) = args.dump_report.clone() {
            fs::write(&path, json).with_context(|| format!("failed to write report: {}", path))?;
        } else {
            println!("{json}");
        }
        return Ok(());
    }

    if let Some(link_job_path) = &args.link_job {
        let (output, job_dump_report) = execute_link_job_path(link_job_path)?;
        let json = render_link_output_json(&output, &args)?;
        if let Some(path) = args.dump_report.clone().or(job_dump_report) {
            fs::write(&path, json).with_context(|| format!("failed to write report: {}", path))?;
        } else {
            println!("{json}");
        }
        return Ok(());
    }

    if args.link_topology.is_some()
        || args.link_initial_peer_slot.is_some()
        || !args.link_sessions.is_empty()
    {
        let output = execute_inline_link_args(&args)?;
        let json = render_link_output_json(&output, &args)?;
        if let Some(path) = args.dump_report.clone() {
            fs::write(&path, json).with_context(|| format!("failed to write report: {}", path))?;
        } else {
            println!("{json}");
        }
        return Ok(());
    }

    let output = execute_args(&args)?;
    let json = render_output_json(&output, &args)?;
    if let Some(path) = args.dump_report.clone() {
        fs::write(&path, json).with_context(|| format!("failed to write report: {}", path))?;
    } else {
        println!("{json}");
    }
    Ok(())
}

// Create parent directories and write native diagnostic aggregates, or synthesize rows
// from report diagnostics only when the native event list is empty. Flush before returning.
fn write_sarakura_diagnostics_jsonl(
    path: &str,
    events: &[DiagnosticEvent],
    report: &DebugReport,
) -> Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create diagnostic events dir: {}",
                    parent.display()
                )
            })?;
        }
    }
    let file = fs::File::create(path)
        .with_context(|| format!("failed to create diagnostic events JSONL: {}", path))?;
    let mut writer = BufWriter::new(file);
    if events.is_empty() {
        write_report_diagnostics_as_events(&mut writer, report)?;
    } else {
        for event in events {
            serde_json::to_writer(&mut writer, event)?;
            writer.write_all(b"\n")?;
        }
    }
    writer.flush()?;
    Ok(())
}

// Serialize the same native-or-report diagnostic choice into an in-memory UTF-8 JSONL string.
fn diagnostic_events_jsonl_string(
    events: &[DiagnosticEvent],
    report: &DebugReport,
) -> Result<String> {
    let mut buf = Vec::new();
    if events.is_empty() {
        write_report_diagnostics_as_events(&mut buf, report)?;
    } else {
        for event in events {
            serde_json::to_writer(&mut buf, event)?;
            buf.write_all(b"\n")?;
        }
    }
    String::from_utf8(buf).map_err(anyhow::Error::from)
}

// Adapt report diagnostics to SARAKURA event rows using current report PC/frame and
// a count of one. These synthesized coordinates are not original per-access observations.
fn write_report_diagnostics_as_events<W: Write>(
    writer: &mut W,
    report: &DebugReport,
) -> Result<()> {
    for (idx, diag) in report.diagnostics.iter().enumerate() {
        let code = format!("{:?}", diag.code);
        let event_type = map_kokura_diagnostic_code_to_sarakura(&code);
        let severity = match format!("{:?}", diag.severity).to_ascii_lowercase().as_str() {
            "warning" | "warn" => "warn",
            "error" => "error",
            _ => "info",
        };
        let row = serde_json::json!({
            "schema": "kokura-diagnostic-event",
            "schema_version": 3,
            "event_id": format!("kokura_diag_{:06}", idx + 1),
            "event_type": event_type,
            "severity": severity,
            "frame": report.meta.observation_frame,
            "pc": format!("0x{:04X}", report.cpu.pc),
            "bank": report.cpu.current_rom_bank,
            "symbol_hint": report.symbols.as_ref().and_then(|s| s.pc_symbol.clone()),
            "summary_key": format!("{}:{}:0x{:04X}", event_type, code, report.cpu.pc),
            "count": 1,
            "first_seen": report.meta.observation_frame,
            "last_seen": report.meta.observation_frame,
            "message": diag.message.clone(),
        });
        serde_json::to_writer(&mut *writer, &row)?;
        writer.write_all(b"\n")?;
    }
    Ok(())
}

// Map code-name substrings to broad SARAKURA categories in priority order; this is
// a compatibility classification and does not prove the specific fault named by that category.
fn map_kokura_diagnostic_code_to_sarakura(code: &str) -> &'static str {
    let upper = code.to_ascii_uppercase();
    if upper.contains("VRAM")
        || upper.contains("PPU")
        || upper.contains("LCDC")
        || upper.contains("SCREEN")
    {
        "VRAM_WRITE_OUTSIDE_SAFE_PERIOD"
    } else if upper.contains("OAM") || upper.contains("SPRITE") {
        "OAM_ACCESS_DURING_FORBIDDEN_PERIOD"
    } else if upper.contains("INPUT") || upper.contains("JOYPAD") {
        "INPUT_POLL_JITTER"
    } else if upper.contains("DMA") || upper.contains("HDMA") {
        "DMA_OR_HDMA_CONFLICT"
    } else if upper.contains("APU") || upper.contains("AUDIO") || upper.contains("PCM") {
        "APU_UPDATE_JITTER"
    } else if upper.contains("BANK") || upper.contains("MBC") || upper.contains("FAR") {
        "BANK_SWITCH_MISMATCH"
    } else if upper.contains("ABI") || upper.contains("STACK") {
        "ABI_SP_OR_REGISTER_MISMATCH"
    } else {
        "KOKURA_DEBUG_DIAGNOSTIC"
    }
}

// Match any report diagnostic by all, case-insensitive code/category equality or message substring.
fn diagnostic_break_matches(report: &DebugReport, filters: &[String]) -> bool {
    if filters
        .iter()
        .any(|value| value.eq_ignore_ascii_case("all"))
    {
        return !report.diagnostics.is_empty();
    }
    report.diagnostics.iter().any(|diag| {
        let code = format!("{:?}", diag.code);
        filters.iter().any(|filter| {
            code.eq_ignore_ascii_case(filter)
                || map_kokura_diagnostic_code_to_sarakura(&code).eq_ignore_ascii_case(filter)
                || diag
                    .message
                    .to_ascii_lowercase()
                    .contains(&filter.to_ascii_lowercase())
        })
    })
}

// When report diagnostics exist, save the current screen/state under fixed diagnostic_000001
// filenames. Repeated calls can overwrite them; capture reflects current session state, not each event time.
fn capture_diagnostic_artifacts(
    job: &LoadedJob,
    session: &DebugSession,
    report: &DebugReport,
) -> Result<()> {
    if report.diagnostics.is_empty() {
        return Ok(());
    }
    if let Some(dir) = &job.png_on_diagnostic {
        fs::create_dir_all(dir)
            .with_context(|| format!("failed to create diagnostic PNG dir: {}", dir))?;
        let path = Path::new(dir).join("diagnostic_000001.png");
        write_screenshot(
            &path.to_string_lossy(),
            session.machine.framebuffer(),
            Some(session.machine.framebuffer_rgb555()),
            session.machine.is_cgb_compat_mode(),
        )?;
    }
    if let Some(dir) = &job.snapshot_on_diagnostic {
        fs::create_dir_all(dir)
            .with_context(|| format!("failed to create diagnostic snapshot dir: {}", dir))?;
        let path = Path::new(dir).join("diagnostic_000001.kqs");
        session
            .save_state()
            .save_to_path(&path)
            .with_context(|| format!("failed to save diagnostic snapshot: {}", path.display()))?;
    }
    Ok(())
}

// Create a ZIP containing the report, diagnostics and manifest. ROM/metadata paths
// are references; this bundle does not embed their files, screenshots, snapshots or traces.
fn write_kokura_repro_bundle(
    path: &str,
    job: &LoadedJob,
    events: &[DiagnosticEvent],
    report: &DebugReport,
) -> Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create repro bundle dir: {}", parent.display())
            })?;
        }
    }
    let file = fs::File::create(path).with_context(|| format!("failed to create {path}"))?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let report_json = serde_json::to_string_pretty(report)?;
    zip.start_file("report/kokura_debug_report.json", options)?;
    zip.write_all(report_json.as_bytes())?;

    let diagnostic_events = diagnostic_events_jsonl_string(events, report)?;
    zip.start_file("diagnostic_events.jsonl", options)?;
    zip.write_all(diagnostic_events.as_bytes())?;

    let manifest = serde_json::json!({
        "schema": "kitaq-repro-bundle",
        "schema_version": 1,
        "producer": "kokura",
        "platform": "gb",
        "rom": {
            "path": job.rom_path,
            // This hash comes from attached build metadata, not a fresh hash of the ROM bytes read by this function.
            "hash": report.toolchain_build.as_ref().and_then(|build| build.output_sha256.clone())
        },
        "metadata": job.toolchain_metadata_path,
        "diagnostics": "diagnostic_events.jsonl",
        "screenshots": [],
        "snapshots": [],
        "traces": [],
        "input_script": job.input_sequence,
        "report": "report/kokura_debug_report.json"
    });
    zip.start_file("manifest.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;
    zip.finish()?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
struct DisassembleRangeOutput {
    range: String,
    bank: u16,
    start_addr: u16,
    end_addr: u16,
    instructions: Vec<DecodedInstruction>,
}

#[derive(Debug, Clone, Serialize)]
struct DisassembleOutput {
    schema_version: &'static str,
    rom_path: String,
    rom_size_bytes: usize,
    ranges: Vec<DisassembleRangeOutput>,
}

// Load the selected ROM, require explicit ranges and render decoded ranges as JSON, Markdown or text.
fn run_disassemble(args: &Args) -> Result<String> {
    let (rom_path, _, _, _, _, _) = resolve_decompile_inputs(args)?;
    let rom = fs::read(&rom_path)
        .with_context(|| format!("failed to read ROM for disassembly: {}", rom_path))?;
    if args.disassemble_ranges.is_empty() {
        bail!("--disassemble-out requires at least one --disassemble-range BANK:START-END");
    }
    let mut ranges = Vec::new();
    for spec in &args.disassemble_ranges {
        let (bank, start_addr, end_addr) = parse_disassemble_range(spec)?;
        let instructions = disassemble_range(&rom, bank, start_addr, end_addr);
        ranges.push(DisassembleRangeOutput {
            range: spec.clone(),
            bank,
            start_addr,
            end_addr,
            instructions,
        });
    }
    let output = DisassembleOutput {
        schema_version: CLI_OUTPUT_SCHEMA_VERSION,
        rom_path,
        rom_size_bytes: rom.len(),
        ranges,
    };
    match args.disassemble_format {
        DecompileFormatArg::Json => {
            serde_json::to_string_pretty(&output).map_err(anyhow::Error::from)
        }
        DecompileFormatArg::Markdown => Ok(render_disassembly_markdown(&output)),
        DecompileFormatArg::Text => Ok(render_disassembly_text(&output)),
    }
}

// Parse hexadecimal BANK:START-END, reject reversed bounds and leave ROM-window validation to the decoder.
fn parse_disassemble_range(spec: &str) -> Result<(u16, u16, u16)> {
    let (bank_text, addr_text) = spec
        .split_once(':')
        .ok_or_else(|| anyhow!("bad disassemble range, expected BANK:START-END: {spec}"))?;
    let (start_text, end_text) = addr_text
        .split_once('-')
        .ok_or_else(|| anyhow!("bad disassemble range, expected BANK:START-END: {spec}"))?;
    let bank = parse_hex_u16(bank_text)
        .with_context(|| format!("bad disassemble range bank in {spec}"))?;
    let start = parse_hex_u16(start_text)
        .with_context(|| format!("bad disassemble range start in {spec}"))?;
    let end =
        parse_hex_u16(end_text).with_context(|| format!("bad disassemble range end in {spec}"))?;
    if start > end {
        bail!("bad disassemble range start after end: {spec}");
    }
    Ok((bank, start, end))
}

// Parse a trimmed hexadecimal u16 with an optional 0x/0X prefix.
fn parse_hex_u16(text: &str) -> Result<u16> {
    let trimmed = text.trim();
    let raw = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    u16::from_str_radix(raw, 16).with_context(|| format!("expected hex u16: {text}"))
}

// Render ROM details and each range inside an assembly fence; supplied path/text is inserted literally.
fn render_disassembly_markdown(output: &DisassembleOutput) -> String {
    let mut out = String::new();
    out.push_str("# KOKURA disassembly report\n\n");
    out.push_str(&format!("- ROM: {}\n", output.rom_path));
    out.push_str(&format!("- ROM size: {} bytes\n\n", output.rom_size_bytes));
    for range in &output.ranges {
        out.push_str(&format!(
            "## {:02X}:{:04X}-{:04X}\n\n",
            range.bank, range.start_addr, range.end_addr
        ));
        out.push_str("```asm\n");
        push_disassembly_lines(&mut out, &range.instructions);
        out.push_str("```\n\n");
    }
    out
}

// Render the same banked instruction listings with plain-text range headings.
fn render_disassembly_text(output: &DisassembleOutput) -> String {
    let mut out = String::new();
    out.push_str("KOKURA disassembly report\n");
    out.push_str(&format!("ROM: {}\n", output.rom_path));
    out.push_str(&format!("ROM size: {} bytes\n", output.rom_size_bytes));
    for range in &output.ranges {
        out.push_str(&format!(
            "\nrange {:02X}:{:04X}-{:04X}\n",
            range.bank, range.start_addr, range.end_addr
        ));
        push_disassembly_lines(&mut out, &range.instructions);
    }
    out
}

// Append aligned bank/address, available hex bytes and instruction text without interpreting flow.
fn push_disassembly_lines(out: &mut String, instructions: &[DecodedInstruction]) {
    for ins in instructions {
        let bytes = ins
            .bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!(
            "{:02X}:{:04X}  {:<10}  {}\n",
            ins.bank, ins.addr, bytes, ins.text
        ));
    }
}

// Merge toolchain metadata, map symbols and source locations, then perform static analysis.
// Apply optional user annotations and supplied trace rows afterward before choosing the output renderer.
fn run_decompile(args: &Args) -> Result<String> {
    let (
        rom_path,
        symbols_path,
        source_map_path,
        toolchain_metadata_path,
        decompile_annotations_path,
        decompile_trace_path,
    ) = resolve_decompile_inputs(args)?;
    let rom = fs::read(&rom_path)
        .with_context(|| format!("failed to read ROM for decompilation: {}", rom_path))?;

    let mut loaded_symbol_table: Option<SymbolTable> = None;
    if let Some(toolchain_metadata_path) = &toolchain_metadata_path {
        let table = load_symbol_table_json_file(toolchain_metadata_path).with_context(|| {
            format!(
                "failed to load toolchain metadata JSON: {}",
                toolchain_metadata_path
            )
        })?;
        let mut merged = loaded_symbol_table.take().unwrap_or_default();
        merged.merge_from(table);
        loaded_symbol_table = Some(merged);
    }
    if let Some(symbols_path) = &symbols_path {
        let table = parse_map_file(symbols_path)
            .with_context(|| format!("failed to load symbols map: {}", symbols_path))?;
        let mut merged = loaded_symbol_table.take().unwrap_or_default();
        merged.merge_from(table);
        loaded_symbol_table = Some(merged);
    }
    if let Some(source_map_path) = &source_map_path {
        let locations = parse_source_map_file(source_map_path)
            .with_context(|| format!("failed to load source map: {}", source_map_path))?;
        let mut table = loaded_symbol_table.take().unwrap_or_default();
        table.merge_sources(locations);
        loaded_symbol_table = Some(table);
    }

    let mut report = analyze_rom(
        &rom,
        loaded_symbol_table.as_ref(),
        &DecompileOptions {
            selected_functions: args.decompile_functions.clone(),
            include_all_named_functions: args.decompile_all,
        },
    );
    if let Some(annotations_path) = &decompile_annotations_path {
        let annotations = load_decompile_annotation_file(annotations_path).with_context(|| {
            format!(
                "failed to load decompile annotations JSON: {}",
                annotations_path
            )
        })?;
        apply_annotations(&mut report, &annotations);
    }
    if let Some(trace_path) = &decompile_trace_path {
        let trace_rows = load_decompile_trace_rows(trace_path).with_context(|| {
            format!(
                "failed to load decompile trace observations: {}",
                trace_path
            )
        })?;
        apply_trace_observations(&mut report, &trace_rows);
    }
    match args.decompile_format {
        DecompileFormatArg::Json => {
            serde_json::to_string_pretty(&report).map_err(anyhow::Error::from)
        }
        DecompileFormatArg::Markdown => Ok(render_decompile_markdown(&report)),
        DecompileFormatArg::Text => Ok(render_decompile_text(&report)),
    }
}

// With a job file, rebase its declared ROM/metadata paths and omit annotation/trace overlays.
// Without a job, use CLI paths or existing ROM sidecars for all five optional inputs.
fn resolve_decompile_inputs(
    args: &Args,
) -> Result<(
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
)> {
    if let Some(job_path) = &args.job {
        let mut job = load_job_file(job_path)?;
        let base = Path::new(job_path)
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        resolve_job_paths(&mut job, &base);
        Ok((
            job.rom,
            job.symbols,
            job.source_map,
            job.toolchain_metadata,
            None,
            None,
        ))
    } else {
        let rom_path = args
            .rom
            .clone()
            .ok_or_else(|| anyhow!("ROM path is required for decompilation"))?;
        let symbols_path = args
            .symbols
            .clone()
            .or_else(|| detect_sidecar_path(&rom_path, "map"));
        let source_map_path = args
            .source_map
            .clone()
            .or_else(|| detect_sidecar_path(&rom_path, "source_map.txt"));
        let toolchain_metadata_path = args
            .toolchain_metadata
            .clone()
            .or_else(|| detect_sidecar_path(&rom_path, "dbg2.json"));
        let decompile_annotations_path = args
            .decompile_annotations
            .clone()
            .or_else(|| detect_sidecar_path(&rom_path, "decompile.json"));
        let decompile_trace_path = args
            .decompile_trace
            .clone()
            .or_else(|| detect_sidecar_path(&rom_path, "timeline.jsonl"));
        Ok((
            rom_path,
            symbols_path,
            source_map_path,
            toolchain_metadata_path,
            decompile_annotations_path,
            decompile_trace_path,
        ))
    }
}

// Present report notes, ranges and per-function candidate/trace detail with pseudocode
// and disassembly fences. This is a textual view, not the complete serialized report.
fn render_decompile_markdown(report: &DecompileReport) -> String {
    let mut out = String::new();
    out.push_str("# KOKURA decompile report\n\n");
    out.push_str(&format!("- ROM size: {} bytes\n", report.rom_size_bytes));
    out.push_str(&format!("- ROM banks: {}\n", report.rom_bank_count));
    if let Some(title) = &report.title {
        out.push_str(&format!("- Title: {}\n", title));
    }
    if let Some(mapper) = &report.mapper {
        out.push_str(&format!("- Mapper: {}\n", mapper));
    }
    out.push_str(&format!("- Functions: {}\n\n", report.functions.len()));
    if !report.notes.is_empty() {
        out.push_str("## Notes\n\n");
        for note in &report.notes {
            out.push_str(&format!("- {}\n", note));
        }
        out.push('\n');
    }
    if !report.data_ranges.is_empty() {
        out.push_str("## Data ranges\n\n");
        for range in &report.data_ranges {
            out.push_str(&format!(
                "- {:02X}:{:04X}-{:04X} {}\n",
                range.bank, range.start, range.end, range.classification
            ));
        }
        out.push('\n');
    }
    for function in &report.functions {
        out.push_str(&format!(
            "## {} ({:02X}:{:04X})\n\n",
            function.name, function.bank, function.start_address
        ));
        out.push_str(&format!("- source: {}\n", function.source_kind));
        out.push_str(&format!("- confidence: {:.2}\n", function.confidence_score));
        if function.name != function.canonical_name {
            out.push_str(&format!("- canonical name: {}\n", function.canonical_name));
        }
        if let Some(cc) = &function.calling_convention_guess {
            out.push_str(&format!("- calling convention: {}\n", cc));
        }
        if !function.user_notes.is_empty() {
            for note in &function.user_notes {
                out.push_str(&format!("- note: {}\n", note));
            }
        }
        if let Some(trace_summary) = &function.artifact.trace_summary {
            out.push_str(&format!("- trace hits: {}\n", trace_summary.hit_count));
            if let (Some(first), Some(last)) = (trace_summary.first_frame, trace_summary.last_frame)
            {
                out.push_str(&format!("- trace frames: {}..{}\n", first, last));
            }
            if let Some(pc) = trace_summary.hot_pcs.first() {
                out.push_str(&format!(
                    "- hottest trace pc: {:02X}:{:04X} ({})\n",
                    pc.rom_bank, pc.pc, pc.hit_count
                ));
            }
        }
        if !function.artifact.stack_slots.is_empty() {
            let joined = function
                .artifact
                .stack_slots
                .iter()
                .map(|slot| {
                    format!(
                        "{}:{}:{}@SP{:+}{}:r{}:w{}:{}{}{}",
                        slot.name,
                        slot.confidence,
                        slot.inferred_type.as_deref().unwrap_or("unknown"),
                        slot.offset,
                        slot.runtime_role
                            .as_ref()
                            .map(|role| format!(":{}", role))
                            .unwrap_or_default(),
                        slot.read_count,
                        slot.write_count,
                        slot.access_patterns.join("+"),
                        slot.lifetime_hint
                            .as_ref()
                            .map(|hint| format!(":life={hint}"))
                            .unwrap_or_default(),
                        if slot.trace_value_hints.is_empty() {
                            String::new()
                        } else {
                            format!(":trace={}", slot.trace_value_hints.join("|"))
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("- stack slots: {}\n", joined));
        }
        if !function.artifact.temp_slots.is_empty() {
            let joined = function
                .artifact
                .temp_slots
                .iter()
                .map(|slot| {
                    format!(
                        "{}:{}:{}{}:r{}:w{}:{}",
                        slot.name,
                        slot.confidence,
                        slot.inferred_type.as_deref().unwrap_or("unknown"),
                        slot.runtime_role
                            .as_ref()
                            .map(|role| format!(":{}", role))
                            .unwrap_or_default(),
                        slot.read_count,
                        slot.write_count,
                        slot.access_patterns.join("+")
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("- temp slots: {}\n", joined));
        }
        if !function.artifact.call_sites.is_empty() {
            out.push_str("- call sites:\n");
            for site in &function.artifact.call_sites {
                out.push_str(&format!(
                    "  - {:02X}:{:04X} -> {}\n",
                    site.call_bank,
                    site.call_addr,
                    site.target_name
                        .clone()
                        .or_else(|| site.target_addr.map(|addr| format!(
                            "{:02X}:{:04X}",
                            site.target_bank.unwrap_or(function.bank),
                            addr
                        )))
                        .unwrap_or_else(|| "unknown".to_string())
                ));
                if !site.static_argument_hints.is_empty() {
                    out.push_str(&format!(
                        "    static args: {}\n",
                        site.static_argument_hints.join(", ")
                    ));
                }
                if !site.runtime_argument_hints.is_empty() {
                    out.push_str(&format!(
                        "    runtime args (hits {}): {}\n",
                        site.hit_count,
                        site.runtime_argument_hints.join(", ")
                    ));
                }
                if let Some(cc) = &site.calling_convention_hint {
                    out.push_str(&format!("    cc hint: {}\n", cc));
                }
            }
        }
        if !function.artifact.fingerprints.is_empty() {
            out.push_str("- fingerprints:\n");
            for fingerprint in &function.artifact.fingerprints {
                out.push_str(&format!("  - {}\n", fingerprint));
            }
        }
        if !function.artifact.suggestions.is_empty() {
            out.push_str("- suggestions:\n");
            for suggestion in &function.artifact.suggestions {
                out.push_str(&format!("  - {}\n", suggestion));
            }
        }
        if !function.intrinsic_match.is_empty() {
            let joined = function
                .intrinsic_match
                .iter()
                .map(|m| m.canonical_name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("- intrinsic matches: {}\n", joined));
        }
        if !function.switch_candidates.is_empty() {
            out.push_str("- switch candidates:\n");
            for switch in &function.switch_candidates {
                out.push_str(&format!(
                    "  - table {:02X}:{:04X}, jump {:02X}:{:04X}, cases {}\n",
                    switch.table_bank,
                    switch.table_addr,
                    switch.jump_bank,
                    switch.jump_addr,
                    switch.cases.len()
                ));
            }
        }
        out.push_str("\n### Pseudocode\n\n```c\n");
        out.push_str(&function.artifact.pseudocode_text);
        out.push_str("\n```\n\n### Disassembly\n\n```asm\n");
        out.push_str(&function.artifact.disassembly_text);
        out.push_str("\n```\n\n");
    }
    out
}

// Present compact per-function names, inferred hints and listings; global notes/data
// ranges and several detailed JSON fields are not included in this format.
fn render_decompile_text(report: &DecompileReport) -> String {
    let mut out = String::new();
    out.push_str("KOKURA decompile report\n");
    out.push_str(&format!("ROM size: {} bytes\n", report.rom_size_bytes));
    out.push_str(&format!("ROM banks: {}\n", report.rom_bank_count));
    if let Some(title) = &report.title {
        out.push_str(&format!("Title: {}\n", title));
    }
    if let Some(mapper) = &report.mapper {
        out.push_str(&format!("Mapper: {}\n", mapper));
    }
    out.push('\n');
    for function in &report.functions {
        out.push_str(&format!(
            "FUNCTION {} {:02X}:{:04X}\n",
            function.name, function.bank, function.start_address
        ));
        out.push_str(&format!(
            "confidence={:.2} source={}\n",
            function.confidence_score, function.source_kind
        ));
        if function.name != function.canonical_name {
            out.push_str(&format!("canonical_name={}\n", function.canonical_name));
        }
        if let Some(cc) = &function.calling_convention_guess {
            out.push_str(&format!("calling_convention={}\n", cc));
        }
        if let Some(trace_summary) = &function.artifact.trace_summary {
            out.push_str(&format!("trace_hits={}\n", trace_summary.hit_count));
            if let (Some(first), Some(last)) = (trace_summary.first_frame, trace_summary.last_frame)
            {
                out.push_str(&format!("trace_frames={}..{}\n", first, last));
            }
        }
        if !function.artifact.stack_slots.is_empty() {
            let joined = function
                .artifact
                .stack_slots
                .iter()
                .map(|slot| {
                    format!(
                        "{}:{}:{}@SP{:+}{}:r{}:w{}:{}{}{}",
                        slot.name,
                        slot.confidence,
                        slot.inferred_type.as_deref().unwrap_or("unknown"),
                        slot.offset,
                        slot.runtime_role
                            .as_ref()
                            .map(|role| format!(":{}", role))
                            .unwrap_or_default(),
                        slot.read_count,
                        slot.write_count,
                        slot.access_patterns.join("+"),
                        slot.lifetime_hint
                            .as_ref()
                            .map(|hint| format!(":life={hint}"))
                            .unwrap_or_default(),
                        if slot.trace_value_hints.is_empty() {
                            String::new()
                        } else {
                            format!(":trace={}", slot.trace_value_hints.join("|"))
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("stack_slots={}\n", joined));
        }
        if !function.artifact.temp_slots.is_empty() {
            let joined = function
                .artifact
                .temp_slots
                .iter()
                .map(|slot| {
                    format!(
                        "{}:{}:{}{}:r{}:w{}:{}",
                        slot.name,
                        slot.confidence,
                        slot.inferred_type.as_deref().unwrap_or("unknown"),
                        slot.runtime_role
                            .as_ref()
                            .map(|role| format!(":{}", role))
                            .unwrap_or_default(),
                        slot.read_count,
                        slot.write_count,
                        slot.access_patterns.join("+")
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("temp_slots={}\n", joined));
        }
        if !function.artifact.call_sites.is_empty() {
            let joined = function
                .artifact
                .call_sites
                .iter()
                .map(|site| {
                    format!(
                        "{:02X}:{:04X}->{}:hits={}:cc={}:static={}:runtime={}",
                        site.call_bank,
                        site.call_addr,
                        site.target_name
                            .clone()
                            .or_else(|| site.target_addr.map(|addr| format!(
                                "{:02X}:{:04X}",
                                site.target_bank.unwrap_or(function.bank),
                                addr
                            )))
                            .unwrap_or_else(|| "unknown".to_string()),
                        site.hit_count,
                        site.calling_convention_hint
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                        site.static_argument_hints.join("+"),
                        site.runtime_argument_hints.join("+")
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("call_sites={}\n", joined));
        }
        if !function.artifact.fingerprints.is_empty() {
            out.push_str(&format!(
                "fingerprints={}\n",
                function.artifact.fingerprints.join(", ")
            ));
        }
        if !function.artifact.suggestions.is_empty() {
            out.push_str(&format!(
                "suggestions={}\n",
                function.artifact.suggestions.join(" | ")
            ));
        }
        for note in &function.user_notes {
            out.push_str(&format!("note={}\n", note));
        }
        out.push_str("-- pseudocode --\n");
        out.push_str(&function.artifact.pseudocode_text);
        out.push_str("\n-- disassembly --\n");
        out.push_str(&function.artifact.disassembly_text);
        out.push_str("\n\n");
    }
    out
}

// Combine CLI options with a loaded job, validate debugger/watch/replay settings and
// fill missing metadata paths from ROM sidecars. Most execution fields come from the job
// when present; hardware, observation and selected alias overrides still come from CLI options.
fn execute_args(args: &Args) -> Result<OutputEnvelope> {
    let forced_mode = match args.hardware {
        HardwareArg::Auto => None,
        HardwareArg::Dmg => Some(HardwareMode::Dmg),
        HardwareArg::Cgb => Some(HardwareMode::Cgb),
    };
    let (
        rom_path,
        symbols_path,
        source_map_path,
        toolchain_metadata_path,
        input,
        input_sequence,
        stages,
        load_state,
        save_state,
        dump_replay_tape,
        compare_replay_tape,
        snapshot_on_replay_mismatch,
        screenshot_path,
        screenshot_frames,
        record_video_path,
        record_video_frames,
        record_wav_path,
        record_wav_frames,
        audio_buffer_frames,
        run_frames,
        autosave_prefix,
        watch_windows,
        debugger,
        replay,
        job_base,
        emit_diagnostics,
        diagnostic_pack,
        diagnostic_rules,
        diagnostic_summary_limit,
        repro_bundle,
        png_on_diagnostic,
        snapshot_on_diagnostic,
        break_on_diagnostic,
    ) = if let Some(job_path) = &args.job {
        let mut job = load_job_file(job_path)?;
        let base = Path::new(job_path)
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        resolve_job_paths(&mut job, &base);
        let watch_windows = normalize_watch_windows(job.watch_windows)?;
        let debugger = job
            .debugger
            .validate()
            .map_err(|err| anyhow!("invalid job debugger config: {err}"))?;
        (
            job.rom,
            job.symbols,
            job.source_map,
            job.toolchain_metadata,
            job.input,
            job.input_sequence,
            job.stages,
            job.load_state,
            job.save_state,
            job.dump_replay_tape,
            job.compare_replay_tape,
            job.snapshot_on_replay_mismatch,
            job.screenshot,
            job.screenshot_frames,
            job.record_video,
            job.record_video_frames,
            job.record_wav,
            job.record_wav_frames,
            job.audio_buffer_frames,
            job.run.frames,
            job.autosave_prefix,
            watch_windows,
            debugger,
            job.replay
                .clone()
                .validate()
                .map_err(|err| anyhow!("invalid job replay config: {err}"))?,
            Some(base),
            args.emit_diagnostics.clone(),
            args.diagnostic_pack.clone(),
            args.diagnostic_rules.clone(),
            args.diagnostic_summary_limit,
            args.repro_bundle.clone(),
            args.png_on_diagnostic.clone(),
            args.snapshot_on_diagnostic.clone(),
            args.break_on_diagnostic.clone(),
        )
    } else {
        let rom = args
            .rom
            .clone()
            .ok_or_else(|| anyhow!("ROM path is required unless --job is used"))?;
        let watch_windows = args
            .watch_windows
            .iter()
            .map(|spec| parse_watch_window_spec(spec))
            .collect::<Result<Vec<_>>>()?;
        let debugger = parse_cli_debugger_args(args)?;
        (
            rom,
            args.symbols.clone(),
            args.source_map.clone(),
            args.toolchain_metadata.clone(),
            args.input.clone(),
            args.input_script.clone().or(args.input_seq.clone()),
            Vec::new(),
            args.resume_state.clone().or(args.load_state.clone()),
            args.snapshot.clone().or(args.save_state.clone()),
            args.dump_replay_tape.clone(),
            args.compare_replay_tape.clone(),
            args.snapshot_on_replay_mismatch.clone(),
            args.png.clone().or(args.screenshot.clone()),
            args.screenshot_frames.clone(),
            args.record_video.clone(),
            args.record_video_frames.clone(),
            args.record_wav.clone(),
            args.record_wav_frames.clone(),
            args.audio_buffer_frames,
            args.run_frames,
            None,
            watch_windows,
            debugger,
            parse_cli_replay_args(args)?,
            None,
            args.emit_diagnostics
                .clone()
                .or(args.diagnostics_jsonl.clone()),
            args.diagnostic_pack.clone(),
            args.diagnostic_rules.clone(),
            args.diagnostic_summary_limit,
            args.repro_bundle.clone(),
            args.png_on_diagnostic.clone(),
            args.snapshot_on_diagnostic.clone(),
            args.break_on_diagnostic.clone(),
        )
    };
    // The resume-state CLI alias overrides even a job-specified load state; it is not rebased to the job directory here.
    let load_state = args.resume_state.clone().or(load_state);

    let symbols_path = symbols_path.or_else(|| detect_sidecar_path(&rom_path, "map"));
    let source_map_path =
        source_map_path.or_else(|| detect_sidecar_path(&rom_path, "source_map.txt"));
    let toolchain_metadata_path =
        toolchain_metadata_path.or_else(|| detect_sidecar_path(&rom_path, "dbg2.json"));

    execute_loaded_job(LoadedJob {
        rom_path,
        forced_mode,
        symbols_path,
        source_map_path,
        toolchain_metadata_path,
        input,
        input_sequence,
        stages,
        load_state,
        save_state,
        dump_replay_tape,
        compare_replay_tape,
        snapshot_on_replay_mismatch,
        screenshot_path,
        screenshot_frames,
        record_video_path,
        record_video_frames,
        record_wav_path,
        record_wav_frames,
        audio_buffer_frames,
        run_frames,
        autosave_prefix,
        watch_windows,
        watch_baseline_mode: args.watch_baseline_mode,
        watch_baseline_tag: args.watch_baseline_tag.clone(),
        capture_watch_baseline: args.capture_watch_baseline.clone(),
        snapshot_at: args.snapshot_at.clone(),
        run_until: args.run_until.clone(),
        trace_points: args.trace_points.clone(),
        timeline_out: args.trace_jsonl.clone().or(args.timeline_out.clone()),
        timeline_format: args.timeline_format,
        watch_fields: args.watch_fields.clone(),
        report_sections: args.report_sections.clone(),
        report_minimal: args.report_minimal.clone(),
        debugger,
        replay,
        compare_replay_watch_only: args.compare_replay_watch_only,
        job_base,
        emit_diagnostics,
        diagnostic_pack,
        diagnostic_rules,
        diagnostic_summary_limit,
        repro_bundle,
        png_on_diagnostic,
        snapshot_on_diagnostic,
        break_on_diagnostic,
    })
}

#[derive(Debug, Clone)]
// Keep normalized per-machine inputs and output destinations separate from topology ordering.
struct LoadedLinkSession {
    name: Option<String>,
    slot: Option<u8>,
    rom_path: String,
    symbols_path: Option<String>,
    source_map_path: Option<String>,
    toolchain_metadata_path: Option<String>,
    load_state: Option<String>,
    save_state: Option<String>,
    input: Option<String>,
    input_sequence: Option<String>,
    audio_buffer_frames: Option<usize>,
    watch_windows: Vec<MemoryWatchSpec>,
    debugger: StopConditionSet,
    replay: ReplayControlSet,
}

#[derive(Debug, Clone)]
struct LoadedLinkJob {
    topology: String,
    initial_peer_slot: Option<u8>,
    dump_report: Option<String>,
    sessions: Vec<LoadedLinkSession>,
    run_frames: u64,
}

#[derive(Debug, Clone)]
struct SessionInputProgram {
    fallback_mask: u8,
    sequence: Vec<(u8, u64)>,
    cursor: usize,
    remaining_in_step: u64,
}

impl SessionInputProgram {
    // Parse the fallback button mask and optional frame-counted sequence, starting at its first entry.
    fn from_session(session: &LoadedLinkSession) -> Result<Self> {
        let fallback_mask = parse_input_mask(session.input.as_deref().unwrap_or(""))?;
        let sequence = session
            .input_sequence
            .as_deref()
            .map(parse_input_sequence)
            .transpose()?
            .unwrap_or_default();
        let remaining_in_step = sequence.first().map(|(_, frames)| *frames).unwrap_or(0);
        Ok(Self {
            fallback_mask,
            sequence,
            cursor: 0,
            remaining_in_step,
        })
    }

    // Return one frame's mask and advance a finished sequence entry, then use fallback
    // input once exhausted. A zero-duration entry still returns its mask for one call.
    fn next_mask(&mut self) -> u8 {
        if let Some((mask, _)) = self.sequence.get(self.cursor).copied() {
            let out = mask;
            if self.remaining_in_step > 0 {
                self.remaining_in_step -= 1;
            }
            if self.remaining_in_step == 0 {
                self.cursor += 1;
                self.remaining_in_step = self
                    .sequence
                    .get(self.cursor)
                    .map(|(_, frames)| *frames)
                    .unwrap_or(0);
            }
            out
        } else {
            self.fallback_mask
        }
    }
}

// Load a link job, rebase its declared paths against the job directory and return both
// the executed result and its optional report destination.
fn execute_link_job_path(path: &str) -> Result<(LinkOutputEnvelope, Option<String>)> {
    let mut spec = load_link_job_file(path)?;
    let base = Path::new(path)
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    resolve_link_job_paths(&mut spec, &base);
    let job = normalize_link_job_spec(spec)?;
    let dump_report = job.dump_report.clone();
    let output = execute_loaded_link_job(job)?;
    Ok((output, dump_report))
}

// Require a topology and at least two inline sessions, resolving their paths against
// the current directory before running the link job.
fn execute_inline_link_args(args: &Args) -> Result<LinkOutputEnvelope> {
    let topology_label = args
        .link_topology
        .as_deref()
        .ok_or_else(|| anyhow!("--link-topology is required when using --link-session"))?;
    if args.link_sessions.len() < 2 {
        bail!("at least two --link-session values are required for inline link mode");
    }

    let base = env::current_dir().context("failed to resolve current working directory")?;
    let sessions = args
        .link_sessions
        .iter()
        .map(|spec| parse_inline_link_session_spec(spec, &base))
        .collect::<Result<Vec<_>>>()?;

    execute_loaded_link_job(LoadedLinkJob {
        topology: normalize_link_topology_label(topology_label)?,
        initial_peer_slot: args.link_initial_peer_slot,
        dump_report: None,
        sessions,
        run_frames: args.run_frames,
    })
}

// Parse pipe-separated key=value entries, rebasing file paths and accumulating watch
// windows. Repeated scalar keys use the last value; literal pipes are not escaped by this parser.
fn parse_inline_link_session_spec(spec: &str, base: &Path) -> Result<LoadedLinkSession> {
    let mut name = None;
    let mut slot = None;
    let mut rom_path = None;
    let mut symbols_path = None;
    let mut source_map_path = None;
    let mut toolchain_metadata_path = None;
    let mut load_state = None;
    let mut save_state = None;
    let mut input = None;
    let mut input_sequence = None;
    let mut audio_buffer_frames = None;
    let mut watch_windows = Vec::new();

    for raw_entry in spec.split('|') {
        let entry = raw_entry.trim();
        if entry.is_empty() {
            continue;
        }
        let (raw_key, raw_value) = entry
            .split_once('=')
            .ok_or_else(|| anyhow!("inline link session entries must use key=value: {entry}"))?;
        let key = normalize_inline_link_session_key(raw_key);
        let value = raw_value.trim();
        match key.as_str() {
            "name" => name = Some(value.to_string()),
            "slot" => {
                let parsed = value
                    .parse::<u8>()
                    .with_context(|| format!("invalid link session slot: {value}"))?;
                slot = Some(parsed);
            }
            "rom" => rom_path = Some(normalize_path(base, value).display().to_string()),
            "symbols" => symbols_path = Some(normalize_path(base, value).display().to_string()),
            "source_map" => {
                source_map_path = Some(normalize_path(base, value).display().to_string())
            }
            "toolchain_metadata" => {
                toolchain_metadata_path = Some(normalize_path(base, value).display().to_string())
            }
            "load_state" => load_state = Some(normalize_path(base, value).display().to_string()),
            "save_state" => save_state = Some(normalize_path(base, value).display().to_string()),
            "input" => input = Some(value.to_string()),
            "input_sequence" | "input_seq" => input_sequence = Some(value.to_string()),
            "audio_buffer_frames" => {
                let parsed = value
                    .parse::<usize>()
                    .with_context(|| format!("invalid audio buffer frame count: {value}"))?;
                audio_buffer_frames = Some(parsed);
            }
            "watch_window" => {
                for raw_watch in value.split(',') {
                    let trimmed = raw_watch.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    watch_windows.push(parse_watch_window_spec(trimmed)?);
                }
            }
            _ => bail!("unsupported inline link session key: {raw_key}"),
        }
    }

    Ok(LoadedLinkSession {
        name,
        slot,
        rom_path: rom_path.ok_or_else(|| anyhow!("inline link session requires rom=<path>"))?,
        symbols_path,
        source_map_path,
        toolchain_metadata_path,
        load_state,
        save_state,
        input,
        input_sequence,
        audio_buffer_frames,
        watch_windows,
        debugger: StopConditionSet::default(),
        replay: ReplayControlSet::default(),
    })
}

// Normalize trimmed ASCII case and hyphens so supported inline keys can use either spelling.
fn normalize_inline_link_session_key(key: &str) -> String {
    key.trim().to_ascii_lowercase().replace('-', "_")
}

// Require at least two sessions and validate each watch/debugger/replay configuration.
// Topology-specific counts and slot continuity are checked later when ordering the runner.
fn normalize_link_job_spec(mut spec: LinkJobSpec) -> Result<LoadedLinkJob> {
    if spec.sessions.len() < 2 {
        bail!("link job requires at least 2 sessions");
    }

    let topology = normalize_link_topology_label(&spec.topology)?;
    let mut sessions = Vec::with_capacity(spec.sessions.len());
    for session in spec.sessions.drain(..) {
        let watch_windows = normalize_watch_windows(session.watch_windows)?;
        let debugger = session
            .debugger
            .validate()
            .map_err(|err| anyhow!("invalid link session debugger config: {err}"))?;
        let replay = session
            .replay
            .validate()
            .map_err(|err| anyhow!("invalid link session replay config: {err}"))?;
        sessions.push(LoadedLinkSession {
            name: session.name,
            slot: session.slot,
            rom_path: session.rom,
            symbols_path: session.symbols,
            source_map_path: session.source_map,
            toolchain_metadata_path: session.toolchain_metadata,
            load_state: session.load_state,
            save_state: session.save_state,
            input: session.input,
            input_sequence: session.input_sequence,
            audio_buffer_frames: session.audio_buffer_frames,
            watch_windows,
            debugger,
            replay,
        });
    }

    Ok(LoadedLinkJob {
        topology,
        initial_peer_slot: spec.initial_peer_slot,
        dump_report: spec.dump_report,
        sessions,
        run_frames: spec.run.frames,
    })
}

// Build ordered machines and input programs, validate/attach the runner with a zero-frame
// call, then advance one requested frame at a time. Stop on the runner stop/unsupported result
// and save per-session state/report data even when the requested duration was not completed.
fn execute_loaded_link_job(job: LoadedLinkJob) -> Result<LinkOutputEnvelope> {
    let (ordered_sessions, topology, session_order_note) = build_link_runner_layout(&job)?;
    let mut sessions = ordered_sessions
        .iter()
        .map(build_link_debug_session)
        .collect::<Result<Vec<_>>>()?;
    let mut input_programs = ordered_sessions
        .iter()
        .map(SessionInputProgram::from_session)
        .collect::<Result<Vec<_>>>()?;
    let runner = TimingAwareLinkRunner::new(topology);

    // Zero-frame execution still validates topology and attaches serial handling before the frame loop.
    let mut summary = {
        let mut refs: Vec<&mut DebugSession> = sessions.iter_mut().collect();
        runner.run_frames(&mut refs, 0)?
    };

    for _frame in 0..job.run_frames {
        for (session, input_program) in sessions.iter_mut().zip(input_programs.iter_mut()) {
            session.machine.set_joypad_mask(input_program.next_mask());
        }
        let frame_summary = {
            let mut refs: Vec<&mut DebugSession> = sessions.iter_mut().collect();
            runner.run_frames(&mut refs, 1)?
        };
        summary.absorb(frame_summary);
        if summary.stopped_session.is_some() || summary.halted_on_unsupported_opcode {
            break;
        }
    }

    let mut session_outputs = Vec::with_capacity(sessions.len());
    for (runner_index, (session, config)) in
        sessions.iter_mut().zip(ordered_sessions.iter()).enumerate()
    {
        let save_path = if let Some(path) = &config.save_state {
            let state = session.save_state();
            state
                .save_to_path(path)
                .with_context(|| format!("failed to save link session state file: {}", path))?;
            Some(path.clone())
        } else {
            None
        };
        session_outputs.push(LinkSessionOutput {
            runner_index,
            name: config.name.clone(),
            slot: config.slot,
            rom_path: config.rom_path.clone(),
            input: config.input.clone(),
            input_sequence: config.input_sequence.clone(),
            report: session.report(),
            saved_state: save_path,
        });
    }

    Ok(LinkOutputEnvelope {
        schema_version: CLI_OUTPUT_SCHEMA_VERSION,
        link_job_schema_version: LINK_JOB_SPEC_SCHEMA_VERSION,
        topology: job.topology,
        run_frames: job.run_frames,
        runner_summary: summary,
        session_order_note,
        sessions: session_outputs,
    })
}

// Load a ROM in automatic hardware mode and validate any saved-state header before
// restoring it. Apply audio capacity, merged metadata, watch, stop and replay configuration.
fn build_link_debug_session(config: &LoadedLinkSession) -> Result<DebugSession> {
    let rom = fs::read(Path::new(&config.rom_path))
        .with_context(|| format!("failed to read ROM: {}", config.rom_path))?;
    let symbols_path = config
        .symbols_path
        .clone()
        .or_else(|| detect_sidecar_path(&config.rom_path, "map"));
    let source_map_path = config
        .source_map_path
        .clone()
        .or_else(|| detect_sidecar_path(&config.rom_path, "source_map.txt"));
    let toolchain_metadata_path = config
        .toolchain_metadata_path
        .clone()
        .or_else(|| detect_sidecar_path(&config.rom_path, "dbg2.json"));
    let build_report_path = detect_sidecar_path(&config.rom_path, "build_report.json");

    let mut machine = Machine::new();
    machine.load_rom(rom)?;
    if let Some(state_path) = &config.load_state {
        let state = MachineState::load_boxed_from_path(state_path)
            .with_context(|| format!("failed to load state file: {}", state_path))?;
        state
            .validate_header_against_machine(&machine)
            .with_context(|| format!("state file does not match loaded ROM: {}", state_path))?;
        machine.load_state(&state);
    }
    if let Some(audio_buffer_frames) = config.audio_buffer_frames {
        machine
            .set_audio_buffer_capacity_frames(audio_buffer_frames)
            .map_err(|err| anyhow!("invalid audio buffer capacity: {err}"))?;
    }

    let mut session = DebugSession::new(machine);
    let mut loaded_symbol_table: Option<SymbolTable> = None;
    if let Some(toolchain_metadata_path) = &toolchain_metadata_path {
        let table = load_symbol_table_json_file(toolchain_metadata_path).with_context(|| {
            format!(
                "failed to load toolchain metadata JSON: {}",
                toolchain_metadata_path
            )
        })?;
        let mut merged = loaded_symbol_table.take().unwrap_or_default();
        merged.merge_from(table);
        loaded_symbol_table = Some(merged);
    }
    if let Some(symbols_path) = &symbols_path {
        let table = parse_map_file(symbols_path)
            .with_context(|| format!("failed to load symbols map: {}", symbols_path))?;
        let mut merged = loaded_symbol_table.take().unwrap_or_default();
        merged.merge_from(table);
        loaded_symbol_table = Some(merged);
    }
    if let Some(source_map_path) = &source_map_path {
        let locations = parse_source_map_file(source_map_path)
            .with_context(|| format!("failed to load source map: {}", source_map_path))?;
        let mut table = loaded_symbol_table.take().unwrap_or_default();
        table.merge_sources(locations);
        loaded_symbol_table = Some(table);
    }
    if let Some(table) = loaded_symbol_table.filter(|t| !t.is_empty()) {
        session.set_symbol_table(table);
    }
    if let Some(build_report_path) = &build_report_path {
        let report = load_toolchain_build_report_file(build_report_path).with_context(|| {
            format!("failed to load toolchain build report JSON: {build_report_path}")
        })?;
        session.set_toolchain_build_report(report);
    }
    session.set_watch_windows(config.watch_windows.clone());
    session
        .set_stop_conditions(config.debugger.clone())
        .map_err(|err| anyhow!("invalid link session stop config: {err}"))?;
    session
        .set_replay_control(config.replay.clone())
        .map_err(|err| anyhow!("invalid link session replay config: {err}"))?;
    Ok(session)
}

// Keep pair wire order or sort adapter sessions into contiguous slots. The selected-peer
// adapter clamps its initial peer into the available range; DMG-07 uses all ordered ports.
fn build_link_runner_layout(
    job: &LoadedLinkJob,
) -> Result<(Vec<LoadedLinkSession>, LinkTopology, String)> {
    match job.topology.as_str() {
        "pair" => {
            if job.sessions.len() != 2 {
                bail!("pair link job requires exactly 2 sessions");
            }
            Ok((
                job.sessions.clone(),
                LinkTopology::Pair,
                "session order is wire order: runner session 0 <-> runner session 1".to_string(),
            ))
        }
        "four_player_adapter" => {
            if !(2..=4).contains(&job.sessions.len()) {
                bail!("four_player_adapter link job requires 2-4 sessions");
            }
            let ordered_sessions =
                order_contiguous_link_sessions(&job.sessions, "four_player_adapter")?;
            if ordered_sessions.first().and_then(|session| session.slot) != Some(0) {
                bail!("four_player_adapter requires host slot 0");
            }
            let initial_peer_slot = job
                .initial_peer_slot
                .unwrap_or(1)
                .clamp(1, (ordered_sessions.len() - 1) as u8);
            Ok((
                ordered_sessions,
                LinkTopology::FourPlayerAdapter {
                    host_session: 0,
                    active_peer: usize::from(initial_peer_slot),
                },
                "sessions are reordered into adapter slot order; slot 0 is the host and host-selected peer changes are followed from Link4_SelectedPeer when metadata is present".to_string(),
            ))
        }
        "dmg07" => {
            if !(2..=4).contains(&job.sessions.len()) {
                bail!("dmg07 link job requires 2-4 sessions");
            }
            let ordered_sessions = order_contiguous_link_sessions(&job.sessions, "dmg07")?;
            Ok((
                ordered_sessions,
                LinkTopology::Dmg07,
                "sessions are reordered into physical DMG-07 port order; slot 0 is Player 1, all slots use adapter-driven external-clock transfers, and absent ports up to Player 4 broadcast zero-filled payloads".to_string(),
            ))
        }
        other => bail!("unsupported link topology: {other}"),
    }
}

// Default missing slots to input positions, sort, then reject duplicates or gaps
// by requiring exact slots zero through session-count minus one.
fn order_contiguous_link_sessions(
    sessions: &[LoadedLinkSession],
    topology: &str,
) -> Result<Vec<LoadedLinkSession>> {
    let mut ordered = sessions
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, mut session)| {
            let slot = session.slot.unwrap_or(index as u8);
            session.slot = Some(slot);
            (slot, session)
        })
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(slot, _)| *slot);
    for (expected_slot, (actual_slot, _)) in ordered.iter().enumerate() {
        if usize::from(*actual_slot) != expected_slot {
            bail!(
                "{} sessions must cover contiguous slots starting at 0; expected slot {}, found {}",
                topology,
                expected_slot,
                actual_slot
            );
        }
    }
    Ok(ordered.into_iter().map(|(_, session)| session).collect())
}

// Accept supported topology aliases ignoring surrounding whitespace and ASCII case.
fn normalize_link_topology_label(label: &str) -> Result<String> {
    let normalized = label.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "pair" | "p2p" | "two_player" | "two-player" => Ok("pair".to_string()),
        "four_player_adapter" | "four-player-adapter" | "link4" | "4p_adapter" => {
            Ok("four_player_adapter".to_string())
        }
        "dmg07" | "dmg-07" | "dmg_07" | "four_player_adapter_dmg07" => Ok("dmg07".to_string()),
        _ => bail!("unsupported link topology: {label}"),
    }
}

#[derive(Debug, Clone)]
// Carry execution configuration after CLI/job precedence and path resolution; some diagnostic
// configuration fields are retained but not applied by the current executor.
struct LoadedJob {
    rom_path: String,
    forced_mode: Option<HardwareMode>,
    symbols_path: Option<String>,
    source_map_path: Option<String>,
    toolchain_metadata_path: Option<String>,
    input: Option<String>,
    input_sequence: Option<String>,
    stages: Vec<JobStage>,
    load_state: Option<String>,
    save_state: Option<String>,
    dump_replay_tape: Option<String>,
    compare_replay_tape: Option<String>,
    snapshot_on_replay_mismatch: Option<String>,
    screenshot_path: Option<String>,
    screenshot_frames: Option<String>,
    record_video_path: Option<String>,
    record_video_frames: Option<String>,
    record_wav_path: Option<String>,
    record_wav_frames: Option<String>,
    audio_buffer_frames: Option<usize>,
    run_frames: u64,
    autosave_prefix: Option<String>,
    watch_windows: Vec<MemoryWatchSpec>,
    watch_baseline_mode: WatchBaselineModeArg,
    watch_baseline_tag: Option<String>,
    capture_watch_baseline: Vec<String>,
    snapshot_at: Vec<String>,
    run_until: Vec<String>,
    trace_points: Vec<String>,
    timeline_out: Option<String>,
    timeline_format: TimelineFormatArg,
    watch_fields: Option<String>,
    report_sections: Option<String>,
    report_minimal: Option<String>,
    debugger: StopConditionSet,
    replay: ReplayControlSet,
    compare_replay_watch_only: bool,
    job_base: Option<PathBuf>,
    emit_diagnostics: Option<String>,
    diagnostic_pack: Option<String>,
    diagnostic_rules: Vec<String>,
    diagnostic_summary_limit: Option<usize>,
    repro_bundle: Option<String>,
    png_on_diagnostic: Option<String>,
    snapshot_on_diagnostic: Option<String>,
    break_on_diagnostic: Vec<String>,
}

// Build the session and execute explicit stages, an input sequence or a simple/observed
// run in that priority order. Save requested state/media, compare or export replay data,
// and assemble diagnostics and the single/staged report after execution.
fn execute_loaded_job(job: LoadedJob) -> Result<OutputEnvelope> {
    let mut session = build_job_debug_session(&job)?;
    if job.emit_diagnostics.is_some()
        || job.repro_bundle.is_some()
        || !job.break_on_diagnostic.is_empty()
    {
        session.machine.set_diagnostic_events_enabled(true);
    }
    // Native diagnostic aggregation is enabled for JSONL, bundles or diagnostic filters,
    // not solely by diagnostic PNG/snapshot destinations.
    let requested_watch_baseline_mode = parse_watch_baseline_mode_arg(job.watch_baseline_mode);

    if job.screenshot_frames.is_some() && job.screenshot_path.is_none() {
        bail!("--screenshot-frames requires --screenshot");
    }
    if job.record_video_frames.is_some() && job.record_video_path.is_none() {
        bail!("--record-video-frames requires --record-video");
    }
    if job.record_wav_frames.is_some() && job.record_wav_path.is_none() {
        bail!("--record-wav-frames requires --record-wav");
    }

    let mut capture_state = if job.screenshot_path.is_some()
        || job.record_video_path.is_some()
        || job.record_wav_path.is_some()
    {
        Some(OutputCaptureState {
            screenshot_path: job.screenshot_path.clone(),
            screenshot_range: job
                .screenshot_frames
                .as_deref()
                .map(parse_frame_range_spec)
                .transpose()?,
            captured_screenshots: Vec::new(),
            captured_screenshot_rgb555: Vec::new(),
            screenshot_cgb_compat_mode: session.machine.is_cgb_compat_mode(),
            record_video_path: job.record_video_path.clone(),
            record_video_range: job
                .record_video_frames
                .as_deref()
                .map(parse_frame_range_spec)
                .transpose()?
                .unwrap_or(FrameRange {
                    start: 1,
                    end: u64::MAX,
                }),
            captured_video_frames: Vec::new(),
            record_wav_path: job.record_wav_path.clone(),
            record_wav_range: job
                .record_wav_frames
                .as_deref()
                .map(parse_frame_range_spec)
                .transpose()?
                .unwrap_or(FrameRange {
                    start: 1,
                    end: u64::MAX,
                }),
            recorded_audio: Vec::new(),
            executed_frames: 0,
        })
    } else {
        None
    };
    if requested_watch_baseline_mode == MemoryWatchBaselineMode::Named
        && !observation_mode_enabled(&job)
    {
        bail!("named watch baseline mode requires observation triggers so the named baseline can be captured during the run");
    }
    if requested_watch_baseline_mode != MemoryWatchBaselineMode::Named {
        session
            .set_watch_baseline_mode(requested_watch_baseline_mode, None)
            .map_err(anyhow::Error::msg)?;
    }
    // Explicit stages take precedence over input sequences; only the simple-run branch
    // below invokes the observation-trigger engine.
    let mut stage_reports = Vec::new();
    let mut previous_report: Option<DebugReport> = None;
    if !job.stages.is_empty() {
        for (idx, stage) in job.stages.clone().into_iter().enumerate() {
            let stage_debugger = job
                .debugger
                .merged(&stage.debugger)
                .validate()
                .map_err(|err| anyhow!("invalid debugger config for stage {}: {err}", idx))?;
            let stage_replay = job
                .replay
                .merged(&stage.replay)
                .validate()
                .map_err(|err| anyhow!("invalid replay config for stage {}: {err}", idx))?;
            session.set_stop_conditions(stage_debugger).map_err(|err| {
                anyhow!("failed to apply debugger config for stage {}: {err}", idx)
            })?;
            session
                .set_replay_control(stage_replay)
                .map_err(|err| anyhow!("failed to apply replay config for stage {}: {err}", idx))?;
            apply_stage_input(&mut session, stage.input.as_deref())?;
            let conditional_save = if stage.snapshots.is_empty() {
                run_session_frames(&mut session, stage.frames, capture_state.as_mut())?;
                None
            } else {
                run_stage_with_snapshots(
                    &mut session,
                    idx,
                    &stage,
                    job.autosave_prefix.as_deref(),
                    capture_state.as_mut(),
                )?
            };
            let report = session.report();
            let diff_summary = build_stage_diff(previous_report.as_ref(), &report);
            let diff_score = compute_diff_score(&report, &diff_summary);
            let diff_severity = classify_diff_severity(diff_score).to_string();
            let snapshot_suggestions = build_snapshot_suggestions(&report, &diff_summary);
            previous_report = Some(report.clone());
            // A conditional snapshot path can be selected here again and overwritten with the final stage state.
            let save_path = if let Some(path) =
                stage.save_state.clone().or(conditional_save).or_else(|| {
                    job.autosave_prefix
                        .as_ref()
                        .map(|p| format!("{}_stage{:02}.kqs", p, idx + 1))
                }) {
                let state = session.save_state();
                state
                    .save_to_path(&path)
                    .with_context(|| format!("failed to save stage state file: {}", path))?;
                Some(path)
            } else {
                None
            };
            if let Some(path) = &stage.dump_report {
                fs::write(path, serde_json::to_string_pretty(&report)?)
                    .with_context(|| format!("failed to write stage report: {}", path))?;
            }
            stage_reports.push(StageOutput {
                schema_version: CLI_OUTPUT_SCHEMA_VERSION,
                stage_index: idx,
                input: stage.input.clone(),
                frames: stage.frames,
                report,
                saved_state: save_path,
                diff_summary,
                diff_score,
                diff_severity,
                snapshot_suggestions,
            });
            if stage_reports.last().is_some_and(|stage| {
                stage.report.summary.halted_on_unsupported_opcode
                    || stage.report.stop_reason.is_some()
            }) {
                break;
            }
        }
    } else if let Some(seq) = &job.input_sequence {
        session
            .set_stop_conditions(job.debugger.clone())
            .map_err(|err| anyhow!("invalid debugger stop config: {err}"))?;
        session
            .set_replay_control(job.replay.clone())
            .map_err(|err| anyhow!("invalid replay config: {err}"))?;
        for (idx, (mask, frames)) in parse_input_sequence(seq)?.into_iter().enumerate() {
            session.machine.set_joypad_mask(mask);
            run_session_frames(&mut session, frames, capture_state.as_mut())?;
            if let Some(prefix) = &job.autosave_prefix {
                let path = format!("{}_seq{:02}.kqs", prefix, idx + 1);
                session
                    .save_state()
                    .save_to_path(&path)
                    .with_context(|| format!("failed to save auto state file: {}", path))?;
            }
            if session.halted_on_unsupported_opcode() || session.stop_reason().is_some() {
                break;
            }
        }
    } else {
        session
            .set_stop_conditions(job.debugger.clone())
            .map_err(|err| anyhow!("invalid debugger stop config: {err}"))?;
        session
            .set_replay_control(job.replay.clone())
            .map_err(|err| anyhow!("invalid replay config: {err}"))?;
        if let Some(input) = &job.input {
            let mask = parse_input_mask(input)?;
            session.machine.set_joypad_mask(mask);
        }
        if observation_mode_enabled(&job) {
            run_observation_session(&mut session, &job, capture_state.as_mut())?;
        } else {
            run_session_frames(&mut session, job.run_frames, capture_state.as_mut())?;
        }
    }

    if let Some(state_path) = &job.save_state {
        let state = session.save_state();
        state
            .save_to_path(state_path)
            .with_context(|| format!("failed to save state file: {}", state_path))?;
    }
    if let Some(capture) = capture_state {
        flush_output_captures(&capture, &session)?;
    }

    let mut report = session.report();
    if let Some(reference_path) = &job.compare_replay_tape {
        let reference = load_replay_tape_file(reference_path)?;
        let replay = report.replay.as_mut().ok_or_else(|| {
            anyhow!(
                "replay comparison requested but no replay data was recorded; enable replay checkpoints first"
            )
        })?;
        let mut comparison = compare_replay_reports(
            &reference.replay,
            replay,
            Some(reference_path.clone()),
            job.compare_replay_watch_only,
        );
        if let (Some(mismatch), Some(helper)) = (
            comparison.first_mismatch.as_ref(),
            comparison.divergence_helper.as_ref(),
        ) {
            let plan = build_replay_mismatch_snapshot_plan(
                mismatch,
                helper,
                job.snapshot_on_replay_mismatch.as_deref(),
            );
            if let Some(prefix) = &job.snapshot_on_replay_mismatch {
                comparison.conditional_snapshot_capture = Some(
                    run_replay_mismatch_snapshot_followup(&mut session, &plan, prefix)?,
                );
            }
            comparison.conditional_snapshot_plan = Some(plan);
        }
        replay.reference_compare = Some(comparison);
    }
    if let Some(path) = &job.dump_replay_tape {
        let tape = build_replay_tape(&report, &job.rom_path).ok_or_else(|| {
            anyhow!("replay tape export requested but no replay data was recorded")
        })?;
        fs::write(path, serde_json::to_string_pretty(&tape)?)
            .with_context(|| format!("failed to write replay tape: {}", path))?;
    }
    // This is a final-report match for artifact capture, not an instruction-level stop.
    // With nonempty filters, any final diagnostic also enters the capture path below.
    let diagnostic_break_matched = !job.break_on_diagnostic.is_empty()
        && diagnostic_break_matches(&report, &job.break_on_diagnostic);
    if diagnostic_break_matched
        || (!job.break_on_diagnostic.is_empty() && !report.diagnostics.is_empty())
    {
        capture_diagnostic_artifacts(&job, &session, &report)?;
    }
    if let Some(path) = &job.emit_diagnostics {
        write_sarakura_diagnostics_jsonl(path, session.machine.diagnostic_events(), &report)?;
    }
    if let Some(path) = &job.repro_bundle {
        write_kokura_repro_bundle(path, &job, session.machine.diagnostic_events(), &report)?;
    }
    let output = if stage_reports.is_empty() {
        OutputEnvelope::Single(report)
    } else {
        OutputEnvelope::Staged {
            schema_version: CLI_OUTPUT_SCHEMA_VERSION,
            job_schema_version: JOB_SPEC_SCHEMA_VERSION,
            final_report: report,
            stage_reports,
        }
    };
    // These diagnostic pack/rule/limit fields currently have no execution effect in this function.
    let _ = (
        &job.job_base,
        &job.diagnostic_pack,
        &job.diagnostic_rules,
        job.diagnostic_summary_limit,
    ); // keep fields available for future reporting and avoid unused drift.
    Ok(output)
}

// Load the selected hardware mode, validate and restore an optional state, then apply
// audio capacity and merged symbol/build metadata before watch/stop/replay setup.
fn build_job_debug_session(job: &LoadedJob) -> Result<Box<DebugSession>> {
    let rom = fs::read(Path::new(&job.rom_path))
        .with_context(|| format!("failed to read ROM: {}", job.rom_path))?;
    let build_report_path = detect_sidecar_path(&job.rom_path, "build_report.json");

    let mut machine = Machine::new();
    machine.load_rom_with_mode(rom, job.forced_mode)?;

    if let Some(state_path) = &job.load_state {
        let state = MachineState::load_boxed_from_path(state_path)
            .with_context(|| format!("failed to load state file: {}", state_path))?;
        state
            .validate_header_against_machine(&machine)
            .with_context(|| format!("state file does not match loaded ROM: {}", state_path))?;
        machine.load_state(&state);
    }
    if let Some(audio_buffer_frames) = job.audio_buffer_frames {
        machine
            .set_audio_buffer_capacity_frames(audio_buffer_frames)
            .map_err(|err| anyhow!("invalid audio buffer capacity: {err}"))?;
    }

    let mut session = Box::new(DebugSession::new(machine));
    let mut loaded_symbol_table: Option<SymbolTable> = None;
    if let Some(toolchain_metadata_path) = &job.toolchain_metadata_path {
        let table = load_symbol_table_json_file(toolchain_metadata_path).with_context(|| {
            format!(
                "failed to load toolchain metadata JSON: {}",
                toolchain_metadata_path
            )
        })?;
        let mut merged = loaded_symbol_table.take().unwrap_or_default();
        merged.merge_from(table);
        loaded_symbol_table = Some(merged);
    }
    if let Some(symbols_path) = &job.symbols_path {
        let table = parse_map_file(symbols_path)
            .with_context(|| format!("failed to load symbols map: {}", symbols_path))?;
        let mut merged = loaded_symbol_table.take().unwrap_or_default();
        merged.merge_from(table);
        loaded_symbol_table = Some(merged);
    }
    if let Some(source_map_path) = &job.source_map_path {
        let locations = parse_source_map_file(source_map_path)
            .with_context(|| format!("failed to load source map: {}", source_map_path))?;
        let mut table = loaded_symbol_table.take().unwrap_or_default();
        table.merge_sources(locations);
        loaded_symbol_table = Some(table);
    }
    if let Some(table) = loaded_symbol_table.filter(|t| !t.is_empty()) {
        session.set_symbol_table(table);
    }
    if let Some(build_report_path) = &build_report_path {
        let report = load_toolchain_build_report_file(build_report_path).with_context(|| {
            format!("failed to load toolchain build report JSON: {build_report_path}")
        })?;
        session.set_toolchain_build_report(report);
    }
    session.set_watch_windows(job.watch_windows.clone());
    session
        .set_stop_conditions(job.debugger.clone())
        .map_err(|err| anyhow!("invalid debugger stop config: {err}"))?;
    session
        .set_replay_control(job.replay.clone())
        .map_err(|err| anyhow!("invalid replay config: {err}"))?;
    Ok(session)
}

// Run enabled job cases sequentially with job-relative paths. Disabled or missing-job
// cases are skipped; execution errors become failed rows, while evaluation errors propagate
// and abort the matrix. The result carries pass/fail counts independently of CLI exit status.
fn run_regression_matrix(matrix_path: &str) -> Result<RegressionMatrixOutput> {
    let matrix = load_regression_matrix(matrix_path)?;
    let matrix_file = Path::new(matrix_path);
    let base_dir = matrix_file.parent().unwrap_or_else(|| Path::new("."));
    let mut results = Vec::new();
    let mut executed_cases = 0usize;
    let mut passed_cases = 0usize;
    let mut failed_cases = 0usize;
    let mut skipped_cases = 0usize;

    for case in matrix.cases {
        let enabled = case.enabled.unwrap_or(true);
        // Skip without executing; placeholder metric booleans in this row are not passing-test evidence.
        if !enabled {
            skipped_cases += 1;
            results.push(RegressionCaseResult {
                name: case.name.clone(),
                job: case.job.clone(),
                stage_index: case.stage_index.unwrap_or(0),
                passed: false,
                skipped: true,
                skip_reason: Some("case disabled".to_string()),
                actual_diff_severity: None,
                actual_bank_switch_count: 0,
                bank_switch_floor_passed: true,
                actual_far_call_count: 0,
                far_call_floor_passed: true,
                actual_intrinsic_count: 0,
                intrinsic_floor_passed: true,
                actual_oam_dma_count: 0,
                oam_dma_floor_passed: true,
                actual_hdma_block_count: 0,
                hdma_block_floor_passed: true,
                actual_dma_complete_count: 0,
                dma_complete_floor_passed: true,
                actual_dma_stall_cycles: 0,
                dma_stall_cycle_floor_passed: true,
                actual_bank_thrash_score: 0,
                bank_thrash_floor_passed: true,
                actual_timer_interrupt_count: 0,
                timer_interrupt_floor_passed: true,
                actual_vblank_count: 0,
                vblank_floor_passed: true,
                actual_event_type_counts: BTreeMap::new(),
                event_count_floor_passed: true,
                matched_event_count_floors: Vec::new(),
                missing_event_count_floors: Vec::new(),
                matched_suggestions: Vec::new(),
                missing_suggestions: Vec::new(),
                matched_diagnostics: Vec::new(),
                missing_diagnostics: Vec::new(),
                extra_diagnostics: Vec::new(),
                matched_unsupported_opcodes: Vec::new(),
                missing_unsupported_opcodes: Vec::new(),
                extra_unsupported_opcodes: Vec::new(),
                matched_event_types: Vec::new(),
                missing_event_types: Vec::new(),
                extra_event_types: Vec::new(),
                matched_watch_changes: Vec::new(),
                missing_watch_changes: Vec::new(),
                extra_watch_changes: Vec::new(),
                stage_count: 0,
            });
            continue;
        }

        let job_path = normalize_path(base_dir, &case.job);
        if !job_path.exists() {
            skipped_cases += 1;
            results.push(RegressionCaseResult {
                name: case.name.clone(),
                job: job_path.display().to_string(),
                stage_index: case.stage_index.unwrap_or(0),
                passed: false,
                skipped: true,
                skip_reason: Some("job file missing".to_string()),
                actual_diff_severity: None,
                actual_bank_switch_count: 0,
                bank_switch_floor_passed: case.expected_bank_switch_floor.is_none(),
                actual_far_call_count: 0,
                far_call_floor_passed: case.expected_far_call_floor.is_none(),
                actual_intrinsic_count: 0,
                intrinsic_floor_passed: case.expected_intrinsic_floor.is_none(),
                actual_oam_dma_count: 0,
                oam_dma_floor_passed: case.expected_oam_dma_floor.is_none(),
                actual_hdma_block_count: 0,
                hdma_block_floor_passed: case.expected_hdma_block_floor.is_none(),
                actual_dma_complete_count: 0,
                dma_complete_floor_passed: case.expected_dma_complete_floor.is_none(),
                actual_dma_stall_cycles: 0,
                dma_stall_cycle_floor_passed: case.expected_dma_stall_cycle_floor.is_none(),
                actual_bank_thrash_score: 0,
                bank_thrash_floor_passed: case.expected_bank_thrash_floor.is_none(),
                actual_timer_interrupt_count: 0,
                timer_interrupt_floor_passed: case.expected_timer_interrupt_floor.is_none(),
                actual_vblank_count: 0,
                vblank_floor_passed: case.expected_vblank_floor.is_none(),
                actual_event_type_counts: BTreeMap::new(),
                event_count_floor_passed: case.expected_event_count_floor.is_empty(),
                matched_event_count_floors: Vec::new(),
                missing_event_count_floors: expected_event_count_floor_to_labels(
                    &case.expected_event_count_floor,
                ),
                matched_suggestions: Vec::new(),
                missing_suggestions: expected_suggestions_to_labels(&case.expected_suggestions),
                matched_diagnostics: Vec::new(),
                missing_diagnostics: case.expected_diagnostics.clone(),
                extra_diagnostics: Vec::new(),
                matched_unsupported_opcodes: Vec::new(),
                missing_unsupported_opcodes: normalize_opcode_expectations(
                    &case.expected_unsupported_opcodes,
                ),
                extra_unsupported_opcodes: Vec::new(),
                matched_event_types: Vec::new(),
                missing_event_types: normalize_event_expectations(&case.expected_event_types),
                extra_event_types: Vec::new(),
                matched_watch_changes: Vec::new(),
                missing_watch_changes: normalize_watch_name_expectations(
                    &case.expected_watch_changes,
                ),
                extra_watch_changes: Vec::new(),
                stage_count: 0,
            });
            continue;
        }

        // Construct a job-only CLI invocation with observation, media and replay override fields unset.
        // The referenced job can still request its own outputs.
        let args = Args {
            rom: None,
            hardware: HardwareArg::Auto,
            job: Some(job_path.display().to_string()),
            link_job: None,
            link_topology: None,
            link_initial_peer_slot: None,
            link_sessions: Vec::new(),
            regression_matrix: None,
            run_frames: 1,
            input: None,
            input_seq: None,
            load_state: None,
            resume_state: None,
            save_state: None,
            dump_report: None,
            dump_replay_tape: None,
            compare_replay_tape: None,
            snapshot_on_replay_mismatch: None,
            screenshot: None,
            png: None,
            snapshot: None,
            trace_jsonl: None,
            diagnostics_jsonl: None,
            repro_bundle: None,
            png_on_diagnostic: None,
            snapshot_on_diagnostic: None,
            break_on_diagnostic: Vec::new(),
            input_script: None,
            screenshot_frames: None,
            record_video: None,
            record_video_frames: None,
            record_wav: None,
            record_wav_frames: None,
            audio_buffer_frames: None,
            decompile_out: None,
            decompile_format: DecompileFormatArg::Json,
            decompile_functions: Vec::new(),
            decompile_all: false,
            decompile_annotations: None,
            decompile_trace: None,
            disassemble_out: None,
            disassemble_format: DecompileFormatArg::Text,
            disassemble_ranges: Vec::new(),
            symbols: None,
            source_map: None,
            toolchain_metadata: None,
            watch_windows: Vec::new(),
            watch_baseline_mode: WatchBaselineModeArg::Initial,
            watch_baseline_tag: None,
            capture_watch_baseline: Vec::new(),
            watch_fields: None,
            report_sections: None,
            report_minimal: None,
            snapshot_at: Vec::new(),
            run_until: Vec::new(),
            trace_points: Vec::new(),
            timeline_out: None,
            timeline_format: TimelineFormatArg::Jsonl,
            breakpoints: Vec::new(),
            watchpoints: Vec::new(),
            stop_on_mmio: Vec::new(),
            stop_on_irq: Vec::new(),
            stop_on_dma: Vec::new(),
            replay_interval: None,
            replay_max_checkpoints: None,
            rewind_on_stop_frames: None,
            stop_on_divergence: false,
            compare_replay_watch_only: false,
            emit_diagnostics: None,
            diagnostic_pack: None,
            diagnostic_rules: Vec::new(),
            diagnostic_summary_limit: None,
        };
        executed_cases += 1;
        let outcome = execute_args(&args);
        match outcome {
            Ok(output) => {
                // An invalid selected stage propagates here, rather than becoming the execution-error row below.
                let result = evaluate_regression_case(case, job_path, output)?;
                if result.passed {
                    passed_cases += 1;
                } else {
                    failed_cases += 1;
                }
                results.push(result);
            }
            Err(err) => {
                failed_cases += 1;
                results.push(RegressionCaseResult {
                    name: case.name.clone(),
                    job: job_path.display().to_string(),
                    stage_index: case.stage_index.unwrap_or(0),
                    passed: false,
                    skipped: false,
                    skip_reason: Some(err.to_string()),
                    actual_diff_severity: None,
                    actual_bank_switch_count: 0,
                    bank_switch_floor_passed: case.expected_bank_switch_floor.is_none(),
                    actual_far_call_count: 0,
                    far_call_floor_passed: case.expected_far_call_floor.is_none(),
                    actual_intrinsic_count: 0,
                    intrinsic_floor_passed: case.expected_intrinsic_floor.is_none(),
                    actual_oam_dma_count: 0,
                    oam_dma_floor_passed: case.expected_oam_dma_floor.is_none(),
                    actual_hdma_block_count: 0,
                    hdma_block_floor_passed: case.expected_hdma_block_floor.is_none(),
                    actual_dma_complete_count: 0,
                    dma_complete_floor_passed: case.expected_dma_complete_floor.is_none(),
                    actual_dma_stall_cycles: 0,
                    dma_stall_cycle_floor_passed: case.expected_dma_stall_cycle_floor.is_none(),
                    actual_bank_thrash_score: 0,
                    bank_thrash_floor_passed: case.expected_bank_thrash_floor.is_none(),
                    actual_timer_interrupt_count: 0,
                    timer_interrupt_floor_passed: case.expected_timer_interrupt_floor.is_none(),
                    actual_vblank_count: 0,
                    vblank_floor_passed: case.expected_vblank_floor.is_none(),
                    actual_event_type_counts: BTreeMap::new(),
                    event_count_floor_passed: case.expected_event_count_floor.is_empty(),
                    matched_event_count_floors: Vec::new(),
                    missing_event_count_floors: expected_event_count_floor_to_labels(
                        &case.expected_event_count_floor,
                    ),
                    matched_suggestions: Vec::new(),
                    missing_suggestions: expected_suggestions_to_labels(&case.expected_suggestions),
                    matched_diagnostics: Vec::new(),
                    missing_diagnostics: case.expected_diagnostics.clone(),
                    extra_diagnostics: Vec::new(),
                    matched_unsupported_opcodes: Vec::new(),
                    missing_unsupported_opcodes: normalize_opcode_expectations(
                        &case.expected_unsupported_opcodes,
                    ),
                    extra_unsupported_opcodes: Vec::new(),
                    matched_event_types: Vec::new(),
                    missing_event_types: normalize_event_expectations(&case.expected_event_types),
                    extra_event_types: Vec::new(),
                    matched_watch_changes: Vec::new(),
                    missing_watch_changes: normalize_watch_name_expectations(
                        &case.expected_watch_changes,
                    ),
                    extra_watch_changes: Vec::new(),
                    stage_count: 0,
                });
            }
        }
    }

    Ok(RegressionMatrixOutput {
        schema_version: CLI_OUTPUT_SCHEMA_VERSION,
        matrix_schema_version: REGRESSION_MATRIX_SCHEMA_VERSION,
        matrix_path: matrix_file.display().to_string(),
        total_cases: results.len(),
        executed_cases,
        passed_cases,
        failed_cases,
        skipped_cases,
        cases: results,
    })
}

// Choose the requested or last stage and check expected presence plus numeric floors.
// Extra diagnostics/opcodes/events/watch changes are reported but do not fail the case.
// Suggestion expectations for another stage are omitted from both matched and missing lists.
fn evaluate_regression_case(
    case: RegressionCase,
    job_path: PathBuf,
    output: OutputEnvelope,
) -> Result<RegressionCaseResult> {
    let RegressionCase {
        name,
        job: _,
        enabled: _,
        stage_index: requested_stage_index,
        expected_suggestions,
        expected_diff_severity,
        expected_diagnostics,
        expected_unsupported_opcodes,
        expected_event_types,
        expected_event_count_floor,
        expected_bank_switch_floor,
        expected_far_call_floor,
        expected_intrinsic_floor,
        expected_oam_dma_floor,
        expected_hdma_block_floor,
        expected_dma_complete_floor,
        expected_dma_stall_cycle_floor,
        expected_bank_thrash_floor,
        expected_timer_interrupt_floor,
        expected_vblank_floor,
        expected_watch_changes,
        notes: _,
    } = case;

    let (stage_count, stage) = match &output {
        OutputEnvelope::Staged { stage_reports, .. } => {
            let idx =
                requested_stage_index.unwrap_or_else(|| stage_reports.len().saturating_sub(1));
            let stage = stage_reports.get(idx).ok_or_else(|| {
                anyhow!(
                    "stage_index {} out of range for {} stage(s)",
                    idx,
                    stage_reports.len()
                )
            })?;
            (stage_reports.len(), StageView::Staged(stage))
        }
        // A requested stage index is not validated for a single report; its effective index is zero.
        OutputEnvelope::Single(report) => (1, StageView::Single(report)),
    };

    let actual_severity = stage.diff_severity().to_string();
    let actual_suggestions = stage.suggestion_labels();
    let matched_suggestions = expected_suggestions
        .iter()
        .filter(|expected| {
            expected
                .stage_index
                .map(|wanted| wanted == stage.stage_index())
                .unwrap_or(true)
                && actual_suggestions.contains(&suggestion_label(&expected.kind, &expected.value))
        })
        .map(|expected| suggestion_label(&expected.kind, &expected.value))
        .collect::<Vec<_>>();
    let missing_suggestions = expected_suggestions
        .iter()
        .filter(|expected| {
            expected
                .stage_index
                .map(|wanted| wanted == stage.stage_index())
                .unwrap_or(true)
                && !actual_suggestions.contains(&suggestion_label(&expected.kind, &expected.value))
        })
        .map(|expected| suggestion_label(&expected.kind, &expected.value))
        .collect::<Vec<_>>();

    let actual_diagnostics = stage.diagnostic_codes();
    let expected_diagnostics = normalize_diagnostic_expectations(&expected_diagnostics);
    let matched_diagnostics = expected_diagnostics
        .iter()
        .filter(|expected| actual_diagnostics.contains(*expected))
        .cloned()
        .collect::<Vec<_>>();
    let missing_diagnostics = expected_diagnostics
        .iter()
        .filter(|expected| !actual_diagnostics.contains(*expected))
        .cloned()
        .collect::<Vec<_>>();
    let extra_diagnostics = actual_diagnostics
        .iter()
        .filter(|actual| !expected_diagnostics.contains(*actual))
        .cloned()
        .collect::<Vec<_>>();

    let actual_unsupported = stage.unsupported_opcode_hexes();
    let expected_unsupported = normalize_opcode_expectations(&expected_unsupported_opcodes);
    let matched_unsupported = expected_unsupported
        .iter()
        .filter(|expected| actual_unsupported.contains(*expected))
        .cloned()
        .collect::<Vec<_>>();
    let missing_unsupported = expected_unsupported
        .iter()
        .filter(|expected| !actual_unsupported.contains(*expected))
        .cloned()
        .collect::<Vec<_>>();
    let extra_unsupported = actual_unsupported
        .iter()
        .filter(|actual| !expected_unsupported.contains(*actual))
        .cloned()
        .collect::<Vec<_>>();

    let actual_event_types = stage.event_types();
    let expected_event_types = normalize_event_expectations(&expected_event_types);
    let matched_event_types = expected_event_types
        .iter()
        .filter(|expected| actual_event_types.contains(*expected))
        .cloned()
        .collect::<Vec<_>>();
    let missing_event_types = expected_event_types
        .iter()
        .filter(|expected| !actual_event_types.contains(*expected))
        .cloned()
        .collect::<Vec<_>>();
    let extra_event_types = actual_event_types
        .iter()
        .filter(|actual| !expected_event_types.contains(*actual))
        .cloned()
        .collect::<Vec<_>>();

    let actual_event_type_counts = stage.event_type_counts();
    let expected_event_count_floor =
        normalize_event_count_floor_expectations(&expected_event_count_floor);
    let matched_event_count_floors = expected_event_count_floor
        .iter()
        .filter(|(event_type, floor)| {
            actual_event_type_counts
                .get(event_type)
                .copied()
                .unwrap_or(0)
                >= *floor
        })
        .map(|(event_type, floor)| format_event_count_floor_label(event_type, *floor))
        .collect::<Vec<_>>();
    let missing_event_count_floors = expected_event_count_floor
        .iter()
        .filter(|(event_type, floor)| {
            actual_event_type_counts
                .get(event_type)
                .copied()
                .unwrap_or(0)
                < *floor
        })
        .map(|(event_type, floor)| format_event_count_floor_label(event_type, *floor))
        .collect::<Vec<_>>();

    let actual_watch_changes = stage.changed_watch_names();
    let expected_watch_changes = normalize_watch_name_expectations(&expected_watch_changes);
    let matched_watch_changes = expected_watch_changes
        .iter()
        .filter(|expected| actual_watch_changes.contains(*expected))
        .cloned()
        .collect::<Vec<_>>();
    let missing_watch_changes = expected_watch_changes
        .iter()
        .filter(|expected| !actual_watch_changes.contains(*expected))
        .cloned()
        .collect::<Vec<_>>();
    let extra_watch_changes = actual_watch_changes
        .iter()
        .filter(|actual| !expected_watch_changes.contains(*actual))
        .cloned()
        .collect::<Vec<_>>();

    let actual_bank_switch_count = stage.bank_switch_count();
    let bank_switch_floor_passed = expected_bank_switch_floor
        .map(|floor| actual_bank_switch_count >= floor)
        .unwrap_or(true);
    let actual_far_call_count = stage.far_call_count();
    let far_call_floor_passed = expected_far_call_floor
        .map(|floor| actual_far_call_count >= floor)
        .unwrap_or(true);
    let actual_intrinsic_count = stage.intrinsic_count();
    let intrinsic_floor_passed = expected_intrinsic_floor
        .map(|floor| actual_intrinsic_count >= floor)
        .unwrap_or(true);
    let actual_oam_dma_count = stage.oam_dma_count();
    let oam_dma_floor_passed = expected_oam_dma_floor
        .map(|floor| actual_oam_dma_count >= floor)
        .unwrap_or(true);
    let actual_hdma_block_count = stage.hdma_block_count();
    let hdma_block_floor_passed = expected_hdma_block_floor
        .map(|floor| actual_hdma_block_count >= floor)
        .unwrap_or(true);
    let actual_dma_complete_count = stage.dma_complete_count();
    let dma_complete_floor_passed = expected_dma_complete_floor
        .map(|floor| actual_dma_complete_count >= floor)
        .unwrap_or(true);
    let actual_dma_stall_cycles = stage.dma_stall_cycles();
    let dma_stall_cycle_floor_passed = expected_dma_stall_cycle_floor
        .map(|floor| actual_dma_stall_cycles >= floor)
        .unwrap_or(true);
    let actual_bank_thrash_score = stage.bank_thrash_score();
    let bank_thrash_floor_passed = expected_bank_thrash_floor
        .map(|floor| actual_bank_thrash_score >= floor)
        .unwrap_or(true);
    let actual_timer_interrupt_count = stage.timer_interrupt_count();
    let timer_interrupt_floor_passed = expected_timer_interrupt_floor
        .map(|floor| actual_timer_interrupt_count >= floor)
        .unwrap_or(true);
    let actual_vblank_count = stage.vblank_count();
    let vblank_floor_passed = expected_vblank_floor
        .map(|floor| actual_vblank_count >= floor)
        .unwrap_or(true);
    let event_count_floor_passed = missing_event_count_floors.is_empty();

    let severity_ok = expected_diff_severity
        .as_ref()
        .map(|expected| expected.eq_ignore_ascii_case(&actual_severity))
        .unwrap_or(true);
    // Only missing expectations and failed floors affect this conjunction; extra observed items do not.
    let passed = severity_ok
        && bank_switch_floor_passed
        && far_call_floor_passed
        && intrinsic_floor_passed
        && oam_dma_floor_passed
        && hdma_block_floor_passed
        && dma_complete_floor_passed
        && dma_stall_cycle_floor_passed
        && bank_thrash_floor_passed
        && timer_interrupt_floor_passed
        && vblank_floor_passed
        && event_count_floor_passed
        && missing_suggestions.is_empty()
        && missing_diagnostics.is_empty()
        && missing_unsupported.is_empty()
        && missing_event_types.is_empty()
        && missing_watch_changes.is_empty();

    Ok(RegressionCaseResult {
        name,
        job: job_path.display().to_string(),
        stage_index: stage.stage_index(),
        passed,
        skipped: false,
        skip_reason: None,
        actual_diff_severity: Some(actual_severity),
        actual_bank_switch_count,
        bank_switch_floor_passed,
        actual_far_call_count,
        far_call_floor_passed,
        actual_intrinsic_count,
        intrinsic_floor_passed,
        actual_oam_dma_count,
        oam_dma_floor_passed,
        actual_hdma_block_count,
        hdma_block_floor_passed,
        actual_dma_complete_count,
        dma_complete_floor_passed,
        actual_dma_stall_cycles,
        dma_stall_cycle_floor_passed,
        actual_bank_thrash_score,
        bank_thrash_floor_passed,
        actual_timer_interrupt_count,
        timer_interrupt_floor_passed,
        actual_vblank_count,
        vblank_floor_passed,
        actual_event_type_counts,
        event_count_floor_passed,
        matched_event_count_floors,
        missing_event_count_floors,
        matched_suggestions,
        missing_suggestions,
        matched_diagnostics,
        missing_diagnostics,
        extra_diagnostics,
        matched_unsupported_opcodes: matched_unsupported,
        missing_unsupported_opcodes: missing_unsupported,
        extra_unsupported_opcodes: extra_unsupported,
        matched_event_types,
        missing_event_types,
        extra_event_types,
        matched_watch_changes,
        missing_watch_changes,
        extra_watch_changes,
        stage_count,
    })
}

enum StageView<'a> {
    Staged(&'a StageOutput),
    Single(&'a DebugReport),
}

impl<'a> StageView<'a> {
    // Borrow the underlying report from either a stage wrapper or a single-run result.
    fn report(&self) -> &'a DebugReport {
        match self {
            StageView::Staged(stage) => &stage.report,
            StageView::Single(report) => report,
        }
    }

    // Use the recorded stage index, or zero for a single-run report.
    fn stage_index(&self) -> usize {
        match self {
            StageView::Staged(stage) => stage.stage_index,
            StageView::Single(_) => 0,
        }
    }

    // Use the stored stage severity; for a single report, score its current delta summary on demand.
    fn diff_severity(&self) -> &str {
        match self {
            StageView::Staged(stage) => &stage.diff_severity,
            StageView::Single(report) => {
                let score = compute_diff_score(report, &report.summary.delta_summary);
                classify_diff_severity(score)
            }
        }
    }

    // Build unique kind/value labels from stored stage suggestions or suggestions inferred for a single report.
    fn suggestion_labels(&self) -> BTreeSet<String> {
        match self {
            StageView::Staged(stage) => stage
                .snapshot_suggestions
                .iter()
                .map(|s| suggestion_label(&s.kind, &s.value))
                .collect(),
            StageView::Single(report) => {
                build_snapshot_suggestions(report, &report.summary.delta_summary)
                    .into_iter()
                    .map(|s| suggestion_label(&s.kind, &s.value))
                    .collect()
            }
        }
    }

    // Collect exact debug-formatted diagnostic code names into a presence set.
    fn diagnostic_codes(&self) -> BTreeSet<String> {
        self.report()
            .diagnostics
            .iter()
            .map(|d| format!("{:?}", d.code))
            .collect()
    }

    // Normalize recorded unsupported bytes as unique two-digit lowercase hexadecimal values.
    fn unsupported_opcode_hexes(&self) -> BTreeSet<String> {
        self.report()
            .unsupported_opcodes
            .iter()
            .map(|entry| format!("{:02x}", entry.opcode))
            .collect()
    }

    // Collect normalized types from retained report events, not a separate lifetime event counter.
    fn event_types(&self) -> BTreeSet<String> {
        self.report()
            .events
            .iter()
            .map(normalize_event_type_from_debug_event)
            .collect()
    }

    // Read the report summary's bank-switch count.
    fn bank_switch_count(&self) -> u64 {
        self.report().summary.bank_switch_count
    }

    // Read the report summary's far-call observation count.
    fn far_call_count(&self) -> u64 {
        self.report().summary.far_call_count
    }

    // Read the report summary's recognized-intrinsic count.
    fn intrinsic_count(&self) -> u64 {
        self.report().summary.intrinsic_count
    }

    // Use OAM DMA starts as the OAM-transfer metric.
    fn oam_dma_count(&self) -> u64 {
        self.report().summary.oam_dma_start_count
    }

    // Use the summary's transferred HDMA block count.
    fn hdma_block_count(&self) -> u64 {
        self.report().summary.hdma_block_count
    }

    // Combine OAM and HDMA completion counts for the matrix floor check.
    fn dma_complete_count(&self) -> u64 {
        self.report().summary.oam_dma_complete_count + self.report().summary.hdma_complete_count
    }

    // Combine reported GDMA and HDMA stall estimates; this is not a measured host-time metric.
    fn dma_stall_cycles(&self) -> u64 {
        self.report().summary.gdma_stall_cycles_estimate
            + self.report().summary.hdma_stall_cycles_estimate
    }

    // Read the summary's heuristic bank-thrashing score.
    fn bank_thrash_score(&self) -> u32 {
        self.report().summary.bank_thrash_score
    }

    // Read the summary's timer-interrupt observation count.
    fn timer_interrupt_count(&self) -> u64 {
        self.report().summary.timer_interrupt_count
    }

    // Read the summary's VBlank count.
    fn vblank_count(&self) -> u64 {
        self.report().summary.vblank_count
    }

    // Count retained event objects by normalized type; trace retention limits can affect these totals.
    fn event_type_counts(&self) -> BTreeMap<String, u64> {
        let mut counts = BTreeMap::new();
        for event in &self.report().events {
            let key = normalize_event_type_from_debug_event(event);
            *counts.entry(key).or_insert(0) += 1;
        }
        counts
    }

    // Select watches currently marked changed and normalize their names for expectation matching.
    fn changed_watch_names(&self) -> BTreeSet<String> {
        self.report()
            .watched_memory
            .iter()
            .filter(|watch| watch.changed)
            .map(|watch| watch.name.trim().to_ascii_lowercase())
            .filter(|name| !name.is_empty())
            .collect()
    }
}

// Apply a parsed stage button mask, releasing all buttons when the stage omits input.
fn apply_stage_input(session: &mut DebugSession, input: Option<&str>) -> Result<()> {
    let mask = match input {
        Some(spec) => parse_input_mask(spec)?,
        None => 0,
    };
    session.machine.set_joypad_mask(mask);
    Ok(())
}

// Format every supplied preview byte as two-digit uppercase hexadecimal.
fn format_watch_preview(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

// Prefer up to four explicit changed-byte entries with a truncation marker, otherwise
// compare available current/baseline previews. Changes outside a preview can have no detail here.
fn summarize_watch_diff_preview(watch: &kokura_debug::MemoryWatchResult) -> Option<String> {
    if !watch.diff_preview.is_empty() {
        let mut parts = watch
            .diff_preview
            .iter()
            .take(4)
            .map(|diff| format!("{:04X}:{:02X}->{:02X}", diff.addr, diff.before, diff.after))
            .collect::<Vec<_>>();
        if watch.diff_preview_truncated || watch.diff_preview.len() > 4 {
            parts.push("...".to_string());
        }
        return Some(parts.join(", "));
    }
    if !watch.preview_bytes.is_empty()
        && !watch.baseline_preview_bytes.is_empty()
        && watch.preview_bytes != watch.baseline_preview_bytes
    {
        return Some(format!(
            "{} -> {}",
            format_watch_preview(&watch.baseline_preview_bytes),
            format_watch_preview(&watch.preview_bytes)
        ));
    }
    None
}

// Describe differing nonempty previews between two reports; equality does not prove whole-window equality.
fn summarize_watch_transition(
    previous_watch: &kokura_debug::MemoryWatchResult,
    current_watch: &kokura_debug::MemoryWatchResult,
) -> Option<String> {
    if previous_watch.preview_bytes.is_empty() || current_watch.preview_bytes.is_empty() {
        return None;
    }
    if previous_watch.preview_bytes == current_watch.preview_bytes {
        return None;
    }
    Some(format!(
        "{} -> {}",
        format_watch_preview(&previous_watch.preview_bytes),
        format_watch_preview(&current_watch.preview_bytes)
    ))
}

// Compare selected hashes/locations with the previous report, then append current
// watch/activity observations. Stage wording uses the supplied report counters without
// subtracting previous totals, so its time scope depends on how the session produced them.
fn build_stage_diff(previous: Option<&DebugReport>, current: &DebugReport) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(prev) = previous {
        if prev.video.bg_hash != current.video.bg_hash {
            out.push("BG hash changed vs previous stage".to_string());
        }
        if prev.video.window_hash != current.video.window_hash {
            out.push("Window hash changed vs previous stage".to_string());
        }
        if prev.video.sprite_hash != current.video.sprite_hash {
            out.push("Sprite hash changed vs previous stage".to_string());
        }
        if prev.cpu.current_rom_bank != current.cpu.current_rom_bank {
            out.push(format!(
                "ROM bank {} -> {}",
                prev.cpu.current_rom_bank, current.cpu.current_rom_bank
            ));
        }
        let prev_sym = prev.symbols.as_ref().and_then(|s| s.pc_symbol.clone());
        let cur_sym = current.symbols.as_ref().and_then(|s| s.pc_symbol.clone());
        if prev_sym != cur_sym {
            out.push("PC symbol changed vs previous stage".to_string());
        }
        let prev_src = prev
            .symbols
            .as_ref()
            .and_then(|s| s.current_source.as_ref())
            .map(|s| format!("{}:{}", s.path, s.line));
        let cur_src = current
            .symbols
            .as_ref()
            .and_then(|s| s.current_source.as_ref())
            .map(|s| format!("{}:{}", s.path, s.line));
        if prev_src != cur_src {
            out.push("PC source location changed vs previous stage".to_string());
        }
        for watch in &current.watched_memory {
            if let Some(previous_watch) = prev.watched_memory.iter().find(|candidate| {
                candidate.name == watch.name
                    && candidate.addr == watch.addr
                    && candidate.size == watch.size
            }) {
                if previous_watch.hash != watch.hash {
                    if let Some(detail) = summarize_watch_transition(previous_watch, watch) {
                        out.push(format!(
                            "Watch '{}' changed vs previous stage ({detail})",
                            watch.name
                        ));
                    } else {
                        out.push(format!("Watch '{}' changed vs previous stage", watch.name));
                    }
                }
            }
        }
    } else {
        out.push("Initial stage baseline established".to_string());
    }
    for watch in &current.watched_memory {
        if watch.changed {
            if let Some(detail) = summarize_watch_diff_preview(watch) {
                out.push(format!(
                    "Watch '{}' changed inside stage ({} byte(s); {detail})",
                    watch.name, watch.changed_bytes
                ));
            } else {
                out.push(format!(
                    "Watch '{}' changed inside stage ({} byte(s))",
                    watch.name, watch.changed_bytes
                ));
            }
        }
    }
    if current.summary.screen_changed {
        out.push("Framebuffer changed during stage".to_string());
    }
    if current.summary.bg_changed {
        out.push("BG layer changed during stage".to_string());
    }
    if current.summary.window_changed {
        out.push("Window layer changed during stage".to_string());
    }
    if current.summary.sprite_changed {
        out.push("Sprite layer changed during stage".to_string());
    }
    if current.summary.joypad_read_count > 0 {
        out.push(format!(
            "FF00/P1 read {} time(s) in this stage (buttons {}, dpad {})",
            current.summary.joypad_read_count,
            current.summary.joypad_button_read_count,
            current.summary.joypad_dpad_read_count
        ));
        if !current.summary.screen_changed
            && !current.summary.bg_changed
            && !current.summary.window_changed
            && !current.summary.sprite_changed
        {
            out.push(
                "Input polling was observed, but no visible layer change was detected in this stage"
                    .to_string(),
            );
        }
    }
    // This fallback is inserted before later LCD/DMA activity notes, so both can appear in one result.
    if out.is_empty() {
        out.push("No major stage-to-stage delta detected".to_string());
    }
    if current.summary.lcd_toggle_count > 0 {
        out.push(format!(
            "LCDC toggled {} time(s) in this stage",
            current.summary.lcd_toggle_count
        ));
    }
    if current.summary.stat_write_count > 0 {
        out.push(format!(
            "STAT control changed {} time(s) in this stage",
            current.summary.stat_write_count
        ));
    }
    if current.summary.lyc_write_count > 0 {
        out.push(format!(
            "LYC target changed {} time(s) in this stage",
            current.summary.lyc_write_count
        ));
    }
    if current.summary.scanline_render_count > 0 {
        out.push(format!(
            "Scanline render boundary observed {} time(s)",
            current.summary.scanline_render_count
        ));
    }
    if current.summary.oam_dma_start_count > 0 {
        out.push(format!(
            "Core OAM DMA started {} time(s)",
            current.summary.oam_dma_start_count
        ));
    }
    if current.summary.hdma_block_count > 0 {
        out.push(format!(
            "HDMA/GDMA block transfer observed {} time(s)",
            current.summary.hdma_block_count
        ));
    }
    if current.summary.gdma_stall_cycles_estimate > 0
        || current.summary.hdma_stall_cycles_estimate > 0
    {
        out.push(format!(
            "DMA stall estimate observed (GDMA={} cycles, HDMA={} cycles)",
            current.summary.gdma_stall_cycles_estimate, current.summary.hdma_stall_cycles_estimate
        ));
    }
    if current.summary.hdma_deferred_count > 0 {
        out.push(format!(
            "HBlank HDMA deferred {} time(s)",
            current.summary.hdma_deferred_count
        ));
    }
    if current.summary.halted_on_unsupported_opcode {
        out.push("Execution halted on an unsupported opcode in this stage".to_string());
    }
    out
}

// Weight summary length and selected activity/diagnostic counters into a heuristic
// attention score. Normal rendering or bank activity can raise it without establishing a fault.
fn compute_diff_score(report: &DebugReport, diff_summary: &[String]) -> u32 {
    let mut score = (diff_summary.len() as u32).saturating_mul(4);
    score += report.diagnostics.len() as u32;
    if report.summary.halted_on_unsupported_opcode {
        score += 24;
    }
    score += (report.summary.unsupported_opcode_count.min(8) as u32) * 3;
    score += (report.summary.bank_switch_count.min(8) as u32) * 2;
    score += (report.summary.hot_loop_score / 8).min(8);
    if report.summary.screen_changed
        || report.summary.bg_changed
        || report.summary.window_changed
        || report.summary.sprite_changed
    {
        score += 3;
    }
    score += (report.summary.lcd_toggle_count.min(4) as u32) * 4;
    score += (report.summary.stat_write_count.min(8) as u32) * 2;
    score += (report.summary.lyc_write_count.min(8) as u32) * 2;
    score += report.summary.scanline_render_count.min(8) as u32;
    score += (report.summary.oam_dma_start_count.min(4) as u32) * 3;
    score += (report.summary.hdma_block_count.min(8) as u32) * 2;
    score += ((report.summary.gdma_stall_cycles_estimate
        + report.summary.hdma_stall_cycles_estimate)
        / 32)
        .min(8) as u32;
    score += (report.summary.hdma_deferred_count.min(4) as u32) * 2;
    score
}

// Map the attention score to five fixed labels; these are not calibrated error probabilities.
fn classify_diff_severity(score: u32) -> &'static str {
    match score {
        0..=4 => "none",
        5..=11 => "low",
        12..=23 => "medium",
        24..=39 => "high",
        _ => "critical",
    }
}

// Propose event/symbol/source/frame capture anchors from current report observations.
// No snapshot is taken by this helper, and suggestions are not evidence of completed verification.
fn build_snapshot_suggestions(
    report: &DebugReport,
    diff_summary: &[String],
) -> Vec<SnapshotSuggestion> {
    let mut out = Vec::new();
    if report.summary.halted_on_unsupported_opcode {
        out.push(SnapshotSuggestion {
            kind: "event".to_string(),
            value: "unsupported".to_string(),
            reason: "Execution stopped on an unsupported opcode in this stage.".to_string(),
        });
        if let Some(first) = report.unsupported_opcodes.first() {
            out.push(SnapshotSuggestion {
                kind: "event".to_string(),
                value: format!("{:02X}", first.opcode),
                reason: "Capture the exact unsupported opcode hit to continue CPU coverage."
                    .to_string(),
            });
        }
    }
    if report.summary.bank_switch_count > 0 {
        out.push(SnapshotSuggestion {
            kind: "event".to_string(),
            value: "bank".to_string(),
            reason:
                "Bank switching occurred; snapshot on bank transitions to isolate call/return flow."
                    .to_string(),
        });
    }
    if report.summary.wait_vblank_count > 0 {
        out.push(SnapshotSuggestion {
            kind: "event".to_string(),
            value: "vblank".to_string(),
            reason: "WaitVBlank-like paths were hit; snapshoting around VBlank can expose present timing.".to_string(),
        });
    }
    if report.summary.oam_dma_start_count > 0 {
        out.push(SnapshotSuggestion {
            kind: "event".to_string(),
            value: "oam_dma".to_string(),
            reason: "Core OAM DMA started in this stage; snapshot around the DMA edge to inspect shadow/live OAM timing.".to_string(),
        });
    }
    if report.summary.hdma_block_count > 0 {
        out.push(SnapshotSuggestion {
            kind: "event".to_string(),
            value: "hdma_block".to_string(),
            reason: "HDMA/GDMA block transfers were observed; snapshot on block boundaries to study VRAM write windows.".to_string(),
        });
    }
    if report.summary.gdma_stall_cycles_estimate > 0 {
        out.push(SnapshotSuggestion {
            kind: "event".to_string(),
            value: "gdma_stall".to_string(),
            reason: "GDMA introduced a coarse CPU-stall estimate; snapshot here to compare transfer length against real hardware traces.".to_string(),
        });
    }
    if report.summary.hdma_deferred_count > 0 {
        out.push(SnapshotSuggestion {
            kind: "event".to_string(),
            value: "hdma_deferred".to_string(),
            reason: "HBlank HDMA windows were deferred while the CPU was halted; snapshot around the defer edge and interrupt wake-up path.".to_string(),
        });
    }
    if report.summary.joypad_read_count > 0 && !report.summary.screen_changed {
        out.push(SnapshotSuggestion {
            kind: "event".to_string(),
            value: "joypad_read".to_string(),
            reason: "FF00/P1 was polled without a visible framebuffer change in this stage; snapshot on joypad_read to correlate row selection with the current symbol or wait loop.".to_string(),
        });
    }
    if report.summary.lcd_toggle_count > 0 {
        out.push(SnapshotSuggestion {
            kind: "event".to_string(),
            value: "lcd_toggle".to_string(),
            reason: "LCDC toggled in this stage; snapshot around the LCD enable edge to isolate startup timing.".to_string(),
        });
    }
    if report.summary.stat_write_count > 0 || report.summary.lyc_write_count > 0 {
        out.push(SnapshotSuggestion {
            kind: "event".to_string(),
            value: "stat".to_string(),
            reason: "STAT/LYC control changed in this stage; snapshot around STAT signal edges to study IRQ timing.".to_string(),
        });
    }
    if let Some(symbol) = report.symbols.as_ref().and_then(|s| s.pc_symbol.clone()) {
        out.push(SnapshotSuggestion {
            kind: "symbol".to_string(),
            value: symbol,
            reason: "Current PC symbol is a stable snapshot anchor for this stage.".to_string(),
        });
    }
    if let Some(source) = report
        .symbols
        .as_ref()
        .and_then(|s| s.current_source.as_ref())
    {
        out.push(SnapshotSuggestion {
            kind: "source".to_string(),
            value: format!("{}:{}", source.path, source.line),
            reason: "Current source span is available; use it as a debugger anchor when stepping through KITAQGB-generated code.".to_string(),
        });
    }
    if diff_summary.iter().any(|line| line.contains("Watch '")) {
        out.push(SnapshotSuggestion {
            kind: "frame".to_string(),
            value: "1".to_string(),
            reason: "Memory watch deltas were seen; a per-frame snapshot often narrows the change window.".to_string(),
        });
    }
    dedup_suggestions(out)
}

// Keep the first proposal for each kind/value label, preserving its original reason and order.
fn dedup_suggestions(suggestions: Vec<SnapshotSuggestion>) -> Vec<SnapshotSuggestion> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for suggestion in suggestions {
        let key = suggestion_label(&suggestion.kind, &suggestion.value);
        if seen.insert(key) {
            out.push(suggestion);
        }
    }
    out
}

// Enable instruction-level observation for triggers, baseline captures or timeline output.
// Diagnostic filters and media destinations alone do not enable this mode.
fn observation_mode_enabled(job: &LoadedJob) -> bool {
    !job.snapshot_at.is_empty()
        || !job.run_until.is_empty()
        || !job.trace_points.is_empty()
        || !job.capture_watch_baseline.is_empty()
        || job.timeline_out.is_some()
}

// Map the CLI baseline enum directly to the debugger baseline mode.
fn parse_watch_baseline_mode_arg(arg: WatchBaselineModeArg) -> MemoryWatchBaselineMode {
    match arg {
        WatchBaselineModeArg::Initial => MemoryWatchBaselineMode::Initial,
        WatchBaselineModeArg::PreviousFrame => MemoryWatchBaselineMode::PreviousFrame,
        WatchBaselineModeArg::Named => MemoryWatchBaselineMode::Named,
    }
}

// Split a comma list into unique trimmed lowercase tokens; reject an explicitly empty
// list but do not validate token names against available report fields.
fn parse_csv_filter_set(spec: Option<&str>) -> Result<Option<BTreeSet<String>>> {
    let Some(spec) = spec else {
        return Ok(None);
    };
    let mut out = BTreeSet::new();
    for part in spec.split(',') {
        let trimmed = part.trim().to_ascii_lowercase();
        if !trimmed.is_empty() {
            out.insert(trimmed);
        }
    }
    if out.is_empty() {
        bail!("filter list must not be empty");
    }
    Ok(Some(out))
}

// Use report-minimal as the section-list override and parse optional watch-field groups.
fn build_output_filter_options(job: &LoadedJob) -> Result<OutputFilterOptions> {
    let report_sections = if let Some(minimal) = job.report_minimal.as_deref() {
        parse_csv_filter_set(Some(minimal))?
    } else {
        parse_csv_filter_set(job.report_sections.as_deref())?
    };
    let watch_fields = parse_csv_filter_set(job.watch_fields.as_deref())?;
    Ok(OutputFilterOptions {
        report_sections,
        watch_fields,
    })
}

// Serialize the full ordinary/staged result, then filter the JSON representation before pretty-printing.
fn render_output_json(output: &OutputEnvelope, args: &Args) -> Result<String> {
    let filters = OutputFilterOptions {
        report_sections: if let Some(minimal) = args.report_minimal.as_deref() {
            parse_csv_filter_set(Some(minimal))?
        } else {
            parse_csv_filter_set(args.report_sections.as_deref())?
        },
        watch_fields: parse_csv_filter_set(args.watch_fields.as_deref())?,
    };
    let mut value = serde_json::to_value(output)?;
    filter_output_value(&mut value, &filters);
    Ok(serde_json::to_string_pretty(&value)?)
}

// Serialize the full link envelope, then filter nested session reports before pretty-printing.
fn render_link_output_json(output: &LinkOutputEnvelope, args: &Args) -> Result<String> {
    let filters = OutputFilterOptions {
        report_sections: if let Some(minimal) = args.report_minimal.as_deref() {
            parse_csv_filter_set(Some(minimal))?
        } else {
            parse_csv_filter_set(args.report_sections.as_deref())?
        },
        watch_fields: parse_csv_filter_set(args.watch_fields.as_deref())?,
    };
    let mut value = serde_json::to_value(output)?;
    filter_output_value(&mut value, &filters);
    Ok(serde_json::to_string_pretty(&value)?)
}

// Recognize a report by meta/cpu/video keys or visit known staged/link wrapper locations.
// This traversal does not recursively filter arbitrary unrelated JSON objects.
fn filter_output_value(value: &mut JsonValue, filters: &OutputFilterOptions) {
    match value {
        JsonValue::Object(map) => {
            if map.contains_key("meta") && map.contains_key("cpu") && map.contains_key("video") {
                filter_debug_report_value(map, filters);
                return;
            }
            if let Some(final_report) = map.get_mut("final_report") {
                filter_output_value(final_report, filters);
            }
            if let Some(stage_reports) = map
                .get_mut("stage_reports")
                .and_then(JsonValue::as_array_mut)
            {
                for stage in stage_reports {
                    if let Some(report) = stage.get_mut("report") {
                        filter_output_value(report, filters);
                    }
                }
            }
            if let Some(sessions) = map.get_mut("sessions").and_then(JsonValue::as_array_mut) {
                for session in sessions {
                    if let Some(report) = session.get_mut("report") {
                        filter_output_value(report, filters);
                    }
                }
            }
        }
        _ => {}
    }
}

// Retain schema/meta plus selected top-level sections, then optionally prune fields
// inside any remaining watched-memory array.
fn filter_debug_report_value(map: &mut JsonMap<String, JsonValue>, filters: &OutputFilterOptions) {
    if let Some(sections) = filters.report_sections.as_ref() {
        // Unlike watch-field groups, section filters have no special all token; schema and meta always survive.
        let keep = |key: &str| {
            key == "schema_version" || key == "meta" || sections.contains(&key.to_ascii_lowercase())
        };
        let to_remove = map
            .keys()
            .filter(|key| !keep(key))
            .cloned()
            .collect::<Vec<_>>();
        for key in to_remove {
            map.remove(&key);
        }
    }

    if let Some(watch_fields) = filters.watch_fields.as_ref() {
        if let Some(JsonValue::Array(watches)) = map.get_mut("watched_memory") {
            for watch in watches {
                if let JsonValue::Object(watch_map) = watch {
                    filter_watch_value(watch_map, watch_fields);
                }
            }
        }
    }
}

// Retain watch identity and requested field groups. Unrecognized fields are kept for compatibility.
fn filter_watch_value(map: &mut JsonMap<String, JsonValue>, watch_fields: &BTreeSet<String>) {
    let keep_all = watch_fields.contains("all");
    let keep_key = |key: &str| match key {
        "name" | "addr" | "size" => true,
        "hash" => keep_all || watch_fields.contains("hash"),
        "nonzero_bytes" | "changed" | "changed_bytes" | "first_change_addr" => {
            keep_all || watch_fields.contains("activity")
        }
        "preview_bytes" | "preview_truncated" => keep_all || watch_fields.contains("preview"),
        "baseline_preview_bytes" => keep_all || watch_fields.contains("baseline"),
        "diff_preview" | "diff_preview_truncated" => keep_all || watch_fields.contains("diff"),
        "watch_insights" => keep_all || watch_fields.contains("insights"),
        _ => true,
    };
    let to_remove = map
        .keys()
        .filter(|key| !keep_key(key))
        .cloned()
        .collect::<Vec<_>>();
    for key in to_remove {
        map.remove(&key);
    }
}

// Parse condition=>path, or derive a numbered state filename from the ROM stem.
// The generated filename is relative to the working directory, not the ROM directory.
fn parse_snapshot_request(spec: &str, index: usize, rom_path: &str) -> Result<SnapshotRequest> {
    let (condition_text, save_path) = if let Some((condition, path)) = spec.split_once("=>") {
        (condition.trim(), Some(path.trim().to_string()))
    } else {
        (spec.trim(), None)
    };
    let condition = parse_observation_condition(condition_text)?;
    let label = format!("snapshot_{:02}", index + 1);
    let save_path = save_path.filter(|path| !path.is_empty()).or_else(|| {
        Path::new(rom_path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| format!("{stem}_{label}.kqs"))
    });
    Ok(SnapshotRequest {
        label,
        condition,
        save_path,
    })
}

// Require name=>condition and parse the condition; name validation is deferred to baseline capture.
fn parse_baseline_capture_request(spec: &str) -> Result<BaselineCaptureRequest> {
    let (name, condition) = spec
        .split_once("=>")
        .ok_or_else(|| anyhow!("baseline capture must use name=>condition"))?;
    Ok(BaselineCaptureRequest {
        name: name.trim().to_string(),
        condition: parse_observation_condition(condition.trim())?,
    })
}

// Assign a numbered trace label and parse its observation condition.
fn parse_trace_point_request(spec: &str, index: usize) -> Result<TracePointRequest> {
    Ok(TracePointRequest {
        label: format!("trace_{:02}", index + 1),
        condition: parse_observation_condition(spec)?,
    })
}

// Parse an AND-only sequence of supported coordinate, text and basis terms.
// Ignore empty separators, reject unknown keys, and expand VBlank shorthand into event-basis terms.
fn parse_observation_condition(spec: &str) -> Result<ObservationCondition> {
    let raw = spec.trim();
    if raw.is_empty() {
        bail!("observation condition must not be empty");
    }
    let mut terms = Vec::new();
    for part in raw.split("&&") {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower == "frame_start" {
            terms.push(ObservationTerm::Basis(ObservationBasis::FrameStart));
            continue;
        }
        if lower == "frame_end" {
            terms.push(ObservationTerm::Basis(ObservationBasis::FrameEnd));
            continue;
        }
        if lower == "vblank" || lower == "vblank_enter" {
            terms.push(ObservationTerm::Basis(ObservationBasis::Event));
            terms.push(ObservationTerm::EventContains("vblank".to_string()));
            continue;
        }
        if lower == "stop" {
            terms.push(ObservationTerm::Basis(ObservationBasis::Stop));
            continue;
        }
        let (key, value) = trimmed
            .split_once('=')
            .ok_or_else(|| anyhow!("unsupported observation condition term: {trimmed}"))?;
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        match key.as_str() {
            "frame" => {
                terms.push(ObservationTerm::Frame(value.parse::<u64>().with_context(
                    || format!("invalid frame in condition: {trimmed}"),
                )?))
            }
            "ly" => terms.push(ObservationTerm::Ly(
                value
                    .parse::<u8>()
                    .with_context(|| format!("invalid LY in condition: {trimmed}"))?,
            )),
            "pc" => terms.push(ObservationTerm::Pc(parse_u16_value(value)?)),
            "bank" => terms.push(ObservationTerm::Bank(parse_u16_value(value)?)),
            // Numeric observation terms use parse_u16_value rules, which differ from the always-hex disassembly range parser.
            "bank_pc" => {
                let (bank, pc) = value
                    .split_once(':')
                    .ok_or_else(|| anyhow!("bank_pc must use bank:pc syntax"))?;
                terms.push(ObservationTerm::BankPc(
                    parse_u16_value(bank)?,
                    parse_u16_value(pc)?,
                ));
            }
            "symbol" => terms.push(ObservationTerm::SymbolContains(value.to_string())),
            "source" => terms.push(ObservationTerm::SourceContains(value.to_string())),
            "event" => terms.push(ObservationTerm::EventContains(value.to_string())),
            "ppu_mode" | "mode" => terms.push(ObservationTerm::PpuMode(value.to_ascii_lowercase())),
            "basis" => {
                let basis = match value.to_ascii_lowercase().as_str() {
                    "frame_start" => ObservationBasis::FrameStart,
                    "frame_end" => ObservationBasis::FrameEnd,
                    "step" => ObservationBasis::Step,
                    "event" => ObservationBasis::Event,
                    "trace" | "trace_point" => ObservationBasis::TracePoint,
                    "snapshot" => ObservationBasis::Snapshot,
                    "stop" => ObservationBasis::Stop,
                    other => bail!("unsupported observation basis '{other}'"),
                };
                terms.push(ObservationTerm::Basis(basis));
            }
            _ => bail!("unsupported observation condition key: {key}"),
        }
    }
    if terms.is_empty() {
        bail!("observation condition must contain at least one term");
    }
    Ok(ObservationCondition { terms })
}

// Format source path and line, appending the column only when available.
fn observation_source_label(snapshot: &kokura_debug::ObservationSnapshot) -> Option<String> {
    snapshot.source.as_ref().map(|source| match source.column {
        Some(column) => format!("{}:{}:{}", source.path, source.line, column),
        None => format!("{}:{}", source.path, source.line),
    })
}

// Count new events by normalized type and render the first four types in sorted order,
// not necessarily the four most frequent types.
fn observation_event_summary(new_events: &[DebugEvent]) -> String {
    if new_events.is_empty() {
        return String::new();
    }
    let mut counts = BTreeMap::new();
    for event in new_events {
        *counts
            .entry(normalize_event_type_from_debug_event(event))
            .or_insert(0usize) += 1;
    }
    counts
        .into_iter()
        .take(4)
        .map(|(name, count)| format!("{name}:{count}"))
        .collect::<Vec<_>>()
        .join(" | ")
}

// Clear unselected watch data in copied typed results while preserving their schema
// and identity. Zero/false/empty placeholders here mean omitted output, not measured values.
fn filter_watch_results(
    watched_memory: Vec<MemoryWatchResult>,
    watch_fields: Option<&BTreeSet<String>>,
) -> Vec<MemoryWatchResult> {
    let Some(watch_fields) = watch_fields else {
        return watched_memory;
    };

    watched_memory
        .into_iter()
        .map(|mut watch| {
            let keep_all = watch_fields.contains("all");
            let keep_hash = keep_all || watch_fields.contains("hash");
            let keep_activity = keep_all || watch_fields.contains("activity");
            let keep_preview = keep_all || watch_fields.contains("preview");
            let keep_baseline = keep_all || watch_fields.contains("baseline");
            let keep_diff = keep_all || watch_fields.contains("diff");
            let keep_insights = keep_all || watch_fields.contains("insights");

            if !keep_hash {
                watch.hash = 0;
            }
            if !keep_activity {
                watch.nonzero_bytes = 0;
                watch.changed = false;
                watch.changed_bytes = 0;
                watch.first_change_addr = None;
            }
            if !keep_preview {
                watch.preview_bytes.clear();
                watch.preview_truncated = false;
            }
            if !keep_baseline {
                watch.baseline_preview_bytes.clear();
            }
            if !keep_diff {
                watch.diff_preview.clear();
                watch.diff_preview_truncated = false;
            }
            if !keep_insights {
                watch.watch_insights.clear();
            }
            watch
        })
        .collect()
}

// Sample current registers/flags, six words from raw memory at SP through SP+10,
// and filtered watch results. Stack samples bypass CPU bus/device access rules and are
// observations of memory words, not proven function arguments.
fn build_timeline_row(
    session: &DebugSession,
    basis: ObservationBasis,
    label: String,
    new_events: &[DebugEvent],
    watch_fields: Option<&BTreeSet<String>>,
) -> TimelineRow {
    let snapshot = session.observation_snapshot();
    let source = observation_source_label(&snapshot);
    let watched_memory =
        filter_watch_results(session.watched_memory_results_snapshot(), watch_fields);
    // The summary is built after filtering: removing activity fields also suppresses changed-watch labels.
    let watch_summary = watched_memory
        .iter()
        .filter(|watch| watch.changed)
        .take(4)
        .map(|watch| format!("{}:{}", watch.name, watch.changed_bytes))
        .collect::<Vec<_>>()
        .join(" | ");
    let stack_word = |addr: u16| -> u16 {
        let lo = u16::from(session.machine().memory.read8(addr));
        let hi = u16::from(session.machine().memory.read8(addr.wrapping_add(1)));
        lo | (hi << 8)
    };
    let registers = BTreeMap::from([
        ("A".to_string(), u64::from(snapshot.a)),
        (
            "AF".to_string(),
            u64::from(((u16::from(snapshot.a)) << 8) | u16::from(snapshot.f)),
        ),
        (
            "BC".to_string(),
            u64::from(((u16::from(snapshot.b)) << 8) | u16::from(snapshot.c)),
        ),
        (
            "DE".to_string(),
            u64::from(((u16::from(snapshot.d)) << 8) | u16::from(snapshot.e)),
        ),
        (
            "HL".to_string(),
            u64::from(((u16::from(snapshot.h)) << 8) | u16::from(snapshot.l)),
        ),
    ]);
    let flags = BTreeMap::from([
        ("Z".to_string(), snapshot.f & 0x80 != 0),
        ("N".to_string(), snapshot.f & 0x40 != 0),
        ("H".to_string(), snapshot.f & 0x20 != 0),
        ("C".to_string(), snapshot.f & 0x10 != 0),
    ]);
    let stack_slot_values = BTreeMap::from([
        ("0".to_string(), u64::from(stack_word(snapshot.sp))),
        (
            "2".to_string(),
            u64::from(stack_word(snapshot.sp.wrapping_add(2))),
        ),
        (
            "4".to_string(),
            u64::from(stack_word(snapshot.sp.wrapping_add(4))),
        ),
        (
            "6".to_string(),
            u64::from(stack_word(snapshot.sp.wrapping_add(6))),
        ),
        (
            "8".to_string(),
            u64::from(stack_word(snapshot.sp.wrapping_add(8))),
        ),
        (
            "10".to_string(),
            u64::from(stack_word(snapshot.sp.wrapping_add(10))),
        ),
    ]);
    TimelineRow {
        basis,
        label,
        completed_frames: snapshot.completed_frames,
        active_frame: snapshot.active_frame,
        cycle: snapshot.cycle,
        ly: snapshot.ly,
        pc: snapshot.pc,
        rom_bank: snapshot.rom_bank,
        ppu_mode: snapshot.ppu_mode,
        symbol: snapshot.symbol,
        source,
        registers,
        flags,
        stack_slot_values,
        watch_summary,
        event_summary: observation_event_summary(new_events),
        stop_reason: session
            .stop_reason()
            .as_ref()
            .map(|reason| format!("{}:{}", reason.kind, reason.label)),
        watched_memory,
    }
}

// Require every term to match the current snapshot and newly supplied events. FrameEnd
// uses completed frames; other bases use active frame. Symbol/source matching is case-sensitive.
fn condition_matches(
    session: &DebugSession,
    condition: &ObservationCondition,
    basis: ObservationBasis,
    new_events: &[DebugEvent],
) -> bool {
    let snapshot = session.observation_snapshot();
    let observed_frame = match basis {
        ObservationBasis::FrameStart => snapshot.active_frame,
        ObservationBasis::FrameEnd => snapshot.completed_frames,
        _ => snapshot.active_frame,
    };
    let source_label = observation_source_label(&snapshot).unwrap_or_default();
    condition.terms.iter().all(|term| match term {
        ObservationTerm::Frame(frame) => observed_frame == *frame,
        ObservationTerm::Ly(ly) => snapshot.ly == *ly,
        ObservationTerm::Pc(pc) => snapshot.pc == *pc,
        ObservationTerm::Bank(bank) => snapshot.rom_bank == *bank,
        ObservationTerm::BankPc(bank, pc) => snapshot.rom_bank == *bank && snapshot.pc == *pc,
        ObservationTerm::SymbolContains(needle) => snapshot
            .symbol
            .as_ref()
            .is_some_and(|symbol| symbol.contains(needle)),
        ObservationTerm::SourceContains(needle) => source_label.contains(needle),
        ObservationTerm::EventContains(needle) => {
            new_events.iter().any(|event| event_matches(event, needle))
        }
        ObservationTerm::PpuMode(mode) => snapshot.ppu_mode.to_ascii_lowercase() == *mode,
        ObservationTerm::Basis(expected) => *expected == basis,
    })
}

// Advance capture-relative frame numbering, drain all queued PCM and retain only
// selected media ranges. Video retains the scalar framebuffer; screenshots also keep RGB555.
fn apply_capture_state_for_frame(capture: &mut OutputCaptureState, session: &mut DebugSession) {
    capture.executed_frames = capture.executed_frames.saturating_add(1);
    let absolute_frame = capture.executed_frames;
    let available_audio = session.machine.audio_frames_available();
    if available_audio > 0 {
        let drained = session
            .machine
            .drain_audio_frames_interleaved_i16(available_audio);
        if capture.record_wav_path.is_some() && capture.record_wav_range.contains(absolute_frame) {
            capture.recorded_audio.extend(drained);
        }
    }
    let wants_screenshot = capture
        .screenshot_range
        .is_some_and(|range| range.contains(absolute_frame));
    let wants_video =
        capture.record_video_path.is_some() && capture.record_video_range.contains(absolute_frame);
    if wants_screenshot || wants_video {
        let pixels = session.machine.framebuffer().to_vec();
        let rgb555 = session.machine.framebuffer_rgb555().to_vec();
        if wants_screenshot {
            capture
                .captured_screenshots
                .push((absolute_frame, pixels.clone()));
            capture
                .captured_screenshot_rgb555
                .push((absolute_frame, rgb555));
        }
        if wants_video {
            capture.captured_video_frames.push((absolute_frame, pixels));
        }
    }
}

// Step with pre-step, post-step/event and completed-frame trigger checks, saving
// matching states and timeline rows until a stop, run-until match or frame budget.
// Requests are not one-shot; repeated matches can overwrite a snapshot path.
fn run_observation_session(
    session: &mut DebugSession,
    job: &LoadedJob,
    mut capture_state: Option<&mut OutputCaptureState>,
) -> Result<()> {
    let snapshot_requests = job
        .snapshot_at
        .iter()
        .enumerate()
        .map(|(idx, spec)| parse_snapshot_request(spec, idx, &job.rom_path))
        .collect::<Result<Vec<_>>>()?;
    let trace_points = job
        .trace_points
        .iter()
        .enumerate()
        .map(|(idx, spec)| parse_trace_point_request(spec, idx))
        .collect::<Result<Vec<_>>>()?;
    let run_until = job
        .run_until
        .iter()
        .map(|spec| parse_observation_condition(spec))
        .collect::<Result<Vec<_>>>()?;
    let baseline_requests = job
        .capture_watch_baseline
        .iter()
        .map(|spec| parse_baseline_capture_request(spec))
        .collect::<Result<Vec<_>>>()?;
    let filter_options = build_output_filter_options(job)?;
    let mut timeline = job
        .timeline_out
        .as_deref()
        .map(|path| TimelineWriter::create(path, job.timeline_format))
        .transpose()?;

    let requested_baseline_mode = parse_watch_baseline_mode_arg(job.watch_baseline_mode);
    if requested_baseline_mode != MemoryWatchBaselineMode::Named {
        session
            .set_watch_baseline_mode(requested_baseline_mode, None)
            .map_err(anyhow::Error::msg)?;
    }

    let mut event_cursor = session.event_log.len();
    let start_completed_frames = session.machine.clocks.frames;
    let target_completed_frames = start_completed_frames.saturating_add(job.run_frames);

    loop {
        // This FrameStart pass runs before every debug step, not only when a new physical frame starts.
        let frame_start_events: [DebugEvent; 0] = [];
        for request in &baseline_requests {
            if condition_matches(
                session,
                &request.condition,
                ObservationBasis::FrameStart,
                &frame_start_events,
            ) {
                session
                    .capture_watch_baseline(&request.name)
                    .map_err(anyhow::Error::msg)?;
                if requested_baseline_mode == MemoryWatchBaselineMode::Named
                    && job.watch_baseline_tag.as_deref() == Some(request.name.as_str())
                {
                    session
                        .set_watch_baseline_mode(
                            MemoryWatchBaselineMode::Named,
                            Some(request.name.clone()),
                        )
                        .map_err(anyhow::Error::msg)?;
                }
            }
        }
        for request in &snapshot_requests {
            if condition_matches(
                session,
                &request.condition,
                ObservationBasis::FrameStart,
                &frame_start_events,
            ) {
                if let Some(path) = &request.save_path {
                    session
                        .save_state()
                        .save_to_path(path)
                        .with_context(|| format!("failed to save conditional snapshot: {path}"))?;
                }
                if let Some(writer) = timeline.as_mut() {
                    writer.write_row(&build_timeline_row(
                        session,
                        ObservationBasis::Snapshot,
                        format!("{}@frame_start", request.label),
                        &frame_start_events,
                        filter_options.watch_fields.as_ref(),
                    ))?;
                }
            }
        }
        for request in &trace_points {
            if condition_matches(
                session,
                &request.condition,
                ObservationBasis::FrameStart,
                &frame_start_events,
            ) {
                if let Some(writer) = timeline.as_mut() {
                    writer.write_row(&build_timeline_row(
                        session,
                        ObservationBasis::TracePoint,
                        format!("{}@frame_start", request.label),
                        &frame_start_events,
                        filter_options.watch_fields.as_ref(),
                    ))?;
                }
            }
        }
        if run_until.iter().any(|condition| {
            condition_matches(
                session,
                condition,
                ObservationBasis::FrameStart,
                &frame_start_events,
            )
        }) {
            break;
        }

        let outcome = session.run_debug_step().map_err(anyhow::Error::from)?;
        // The cursor assumes an append-only event vector with no truncation below its previous length.
        let new_events = session.event_log[event_cursor..].to_vec();
        event_cursor = session.event_log.len();

        for request in &baseline_requests {
            if condition_matches(
                session,
                &request.condition,
                ObservationBasis::Step,
                &new_events,
            ) || condition_matches(
                session,
                &request.condition,
                ObservationBasis::Event,
                &new_events,
            ) {
                session
                    .capture_watch_baseline(&request.name)
                    .map_err(anyhow::Error::msg)?;
                if requested_baseline_mode == MemoryWatchBaselineMode::Named
                    && job.watch_baseline_tag.as_deref() == Some(request.name.as_str())
                {
                    session
                        .set_watch_baseline_mode(
                            MemoryWatchBaselineMode::Named,
                            Some(request.name.clone()),
                        )
                        .map_err(anyhow::Error::msg)?;
                }
            }
        }

        for request in &trace_points {
            let matched_step = condition_matches(
                session,
                &request.condition,
                ObservationBasis::Step,
                &new_events,
            );
            let matched_event = condition_matches(
                session,
                &request.condition,
                ObservationBasis::Event,
                &new_events,
            );
            if matched_step || matched_event {
                if let Some(writer) = timeline.as_mut() {
                    writer.write_row(&build_timeline_row(
                        session,
                        ObservationBasis::TracePoint,
                        request.label.clone(),
                        &new_events,
                        filter_options.watch_fields.as_ref(),
                    ))?;
                }
            }
        }

        for request in &snapshot_requests {
            let matched_step = condition_matches(
                session,
                &request.condition,
                ObservationBasis::Step,
                &new_events,
            );
            let matched_event = condition_matches(
                session,
                &request.condition,
                ObservationBasis::Event,
                &new_events,
            );
            if matched_step || matched_event {
                if let Some(path) = &request.save_path {
                    session
                        .save_state()
                        .save_to_path(path)
                        .with_context(|| format!("failed to save conditional snapshot: {path}"))?;
                }
                if let Some(writer) = timeline.as_mut() {
                    writer.write_row(&build_timeline_row(
                        session,
                        ObservationBasis::Snapshot,
                        request.label.clone(),
                        &new_events,
                        filter_options.watch_fields.as_ref(),
                    ))?;
                }
            }
        }

        if outcome.frame_completed {
            if let Some(capture) = capture_state.as_mut() {
                apply_capture_state_for_frame(capture, session);
            }
            for request in &baseline_requests {
                if condition_matches(
                    session,
                    &request.condition,
                    ObservationBasis::FrameEnd,
                    &new_events,
                ) {
                    session
                        .capture_watch_baseline(&request.name)
                        .map_err(anyhow::Error::msg)?;
                    if requested_baseline_mode == MemoryWatchBaselineMode::Named
                        && job.watch_baseline_tag.as_deref() == Some(request.name.as_str())
                    {
                        session
                            .set_watch_baseline_mode(
                                MemoryWatchBaselineMode::Named,
                                Some(request.name.clone()),
                            )
                            .map_err(anyhow::Error::msg)?;
                    }
                }
            }
            for request in &trace_points {
                if condition_matches(
                    session,
                    &request.condition,
                    ObservationBasis::FrameEnd,
                    &new_events,
                ) {
                    if let Some(writer) = timeline.as_mut() {
                        writer.write_row(&build_timeline_row(
                            session,
                            ObservationBasis::TracePoint,
                            request.label.clone(),
                            &new_events,
                            filter_options.watch_fields.as_ref(),
                        ))?;
                    }
                }
            }
            for request in &snapshot_requests {
                if condition_matches(
                    session,
                    &request.condition,
                    ObservationBasis::FrameEnd,
                    &new_events,
                ) {
                    if let Some(path) = &request.save_path {
                        session.save_state().save_to_path(path).with_context(|| {
                            format!("failed to save conditional snapshot: {path}")
                        })?;
                    }
                    if let Some(writer) = timeline.as_mut() {
                        writer.write_row(&build_timeline_row(
                            session,
                            ObservationBasis::Snapshot,
                            request.label.clone(),
                            &new_events,
                            filter_options.watch_fields.as_ref(),
                        ))?;
                    }
                }
            }
            if let Some(writer) = timeline.as_mut() {
                writer.write_row(&build_timeline_row(
                    session,
                    ObservationBasis::FrameEnd,
                    "frame_end".to_string(),
                    &new_events,
                    filter_options.watch_fields.as_ref(),
                ))?;
            }
        }

        if outcome.stop_triggered || outcome.halted_on_unsupported_opcode {
            if let Some(writer) = timeline.as_mut() {
                writer.write_row(&build_timeline_row(
                    session,
                    ObservationBasis::Stop,
                    "stop".to_string(),
                    &new_events,
                    filter_options.watch_fields.as_ref(),
                ))?;
            }
            break;
        }

        if run_until.iter().any(|condition| {
            condition_matches(session, condition, ObservationBasis::Step, &new_events)
                || condition_matches(session, condition, ObservationBasis::Event, &new_events)
                || (outcome.frame_completed
                    && condition_matches(
                        session,
                        condition,
                        ObservationBasis::FrameEnd,
                        &new_events,
                    ))
        }) {
            break;
        }

        // Budget is checked after stepping, so zero requested frames can still execute one step unless a pre-step condition stops it.
        if session.machine.clocks.frames >= target_completed_frames {
            break;
        }
    }

    if let Some(writer) = timeline.as_mut() {
        writer.finish()?;
    }
    Ok(())
}

// Run through the debugger frame callback, accumulating requested media by local frame
// number. Drain queued audio at every callback even when audio capture is disabled.
fn run_session_frames(
    session: &mut DebugSession,
    frames: u64,
    capture_state: Option<&mut OutputCaptureState>,
) -> Result<()> {
    if let Some(capture) = capture_state {
        let base_frame = capture.executed_frames;
        session
            .run_frames_with_callback(frames, |session, local_frame| {
                let absolute_frame = base_frame + local_frame;
                capture.executed_frames = absolute_frame;
                let available_audio = session.machine.audio_frames_available();
                if available_audio > 0 {
                    let drained = session
                        .machine
                        .drain_audio_frames_interleaved_i16(available_audio);
                    if capture.record_wav_path.is_some()
                        && capture.record_wav_range.contains(absolute_frame)
                    {
                        capture.recorded_audio.extend(drained);
                    }
                }
                let wants_screenshot = capture
                    .screenshot_range
                    .is_some_and(|range| range.contains(absolute_frame));
                let wants_video = capture.record_video_path.is_some()
                    && capture.record_video_range.contains(absolute_frame);
                if wants_screenshot || wants_video {
                    let pixels = session.machine.framebuffer().to_vec();
                    let rgb555 = session.machine.framebuffer_rgb555().to_vec();
                    if wants_screenshot {
                        capture
                            .captured_screenshots
                            .push((absolute_frame, pixels.clone()));
                        capture
                            .captured_screenshot_rgb555
                            .push((absolute_frame, rgb555));
                    }
                    if wants_video {
                        capture.captured_video_frames.push((absolute_frame, pixels));
                    }
                }
            })
            .map_err(anyhow::Error::from)
    } else {
        session
            .run_frames_with_callback(frames, |session, _local_frame| {
                let available_audio = session.machine.audio_frames_available();
                if available_audio > 0 {
                    let _ = session
                        .machine
                        .drain_audio_frames_interleaved_i16(available_audio);
                }
            })
            .map_err(anyhow::Error::from)
    }
}

// Run one-frame chunks and evaluate stage triggers afterward, saving each distinct
// path at most once. Conditions use the FrameEnd basis even if execution stopped mid-frame.
fn run_stage_with_snapshot_list(
    session: &mut DebugSession,
    idx: usize,
    stage: &JobStage,
    autosave_prefix: Option<&str>,
    mut capture_state: Option<&mut OutputCaptureState>,
) -> Result<Vec<String>> {
    let mut saved = Vec::new();
    let mut cursor = session.event_log.len();
    for frame_idx in 0..stage.frames {
        let capture = capture_state.as_mut().map(|capture| &mut **capture);
        run_session_frames(session, 1, capture)?;
        for trigger in &stage.snapshots {
            let hit = if let Some(condition_text) = &trigger.condition {
                let condition = parse_observation_condition(condition_text)?;
                condition_matches(
                    session,
                    &condition,
                    ObservationBasis::FrameEnd,
                    &session.event_log[cursor..],
                )
            } else {
                match trigger.kind.as_str() {
                    "frame" => trigger
                        .value
                        .parse::<u64>()
                        .ok()
                        .map(|v| v == frame_idx + 1)
                        .unwrap_or(false),
                    "symbol" => session
                        .report()
                        .symbols
                        .as_ref()
                        .and_then(|s| s.pc_symbol.clone())
                        .map(|s| s.contains(&trigger.value))
                        .unwrap_or(false),
                    "source" => session
                        .report()
                        .symbols
                        .as_ref()
                        .and_then(|s| s.current_source.as_ref())
                        .map(|s| format!("{}:{}", s.path, s.line).contains(&trigger.value))
                        .unwrap_or(false),
                    "event" => session.event_log[cursor..]
                        .iter()
                        .any(|e| event_matches(e, &trigger.value)),
                    _ => false,
                }
            };
            if hit {
                let path = trigger.save_state.clone().or_else(|| {
                    autosave_prefix.map(|p| {
                        format!(
                            "{}_stage{:02}_snap_{}.kqs",
                            p,
                            idx + 1,
                            sanitize_name(&trigger.value)
                        )
                    })
                });
                if let Some(path) = path {
                    if !saved.iter().any(|existing| existing == &path) {
                        session.save_state().save_to_path(&path).with_context(|| {
                            format!("failed to save conditional snapshot: {}", path)
                        })?;
                        saved.push(path);
                    }
                }
            }
        }
        cursor = session.event_log.len();
        // This stage loop checks unsupported-opcode halt here, but not a debugger stop reason.
        if session.halted_on_unsupported_opcode() {
            break;
        }
    }
    Ok(saved)
}

// Execute all stage capture triggers and return only the last saved path to the outer stage executor.
fn run_stage_with_snapshots(
    session: &mut DebugSession,
    idx: usize,
    stage: &JobStage,
    autosave_prefix: Option<&str>,
    capture_state: Option<&mut OutputCaptureState>,
) -> Result<Option<String>> {
    Ok(
        run_stage_with_snapshot_list(session, idx, stage, autosave_prefix, capture_state)?
            .into_iter()
            .last(),
    )
}

// Parse recognized recommendation prefixes directly into validated stop specifications.
// Unknown prefixes are ignored; no recommendation is executed as a shell command.
fn parse_recommended_stop_specs(specs: &[String]) -> Result<StopConditionSet> {
    let mut out = StopConditionSet::default();
    for spec in specs {
        let trimmed = spec.trim();
        if let Some(value) = trimmed.strip_prefix("--breakpoint ") {
            out.breakpoints.push(parse_breakpoint_spec(value)?);
        } else if let Some(value) = trimmed.strip_prefix("--watchpoint ") {
            out.watchpoints.push(parse_watchpoint_spec(value)?);
        } else if let Some(value) = trimmed.strip_prefix("--stop-on-mmio ") {
            out.mmio_writes.push(parse_mmio_stop_spec(value)?);
        } else if let Some(value) = trimmed.strip_prefix("--stop-on-irq ") {
            out.interrupts.push(parse_interrupt_stop_spec(value)?);
        } else if let Some(value) = trimmed.strip_prefix("--stop-on-dma ") {
            out.dma_events.push(parse_dma_stop_spec(value)?);
        }
    }
    out.validate().map_err(|err| anyhow!(err))
}

// Parse recognized watch-window recommendations and deduplicate normalized name/address/size triples.
fn parse_recommended_watch_windows(specs: &[String]) -> Result<Vec<MemoryWatchSpec>> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for spec in specs {
        let trimmed = spec.trim();
        if let Some(value) = trimmed.strip_prefix("--watch-window ") {
            let parsed = parse_watch_window_spec(value)?;
            let key = (parsed.name.to_ascii_lowercase(), parsed.addr, parsed.size);
            if seen.insert(key) {
                out.push(parsed);
            }
        }
    }
    Ok(out)
}

// Plan relative first/mismatch/last frame captures plus relevant event triggers
// inside a proposed replay window. Optional filenames are proposals until follow-up actually runs.
fn build_replay_mismatch_snapshot_plan(
    mismatch: &ReplayReferenceMismatchReport,
    helper: &ReplayDivergenceHelperReport,
    prefix: Option<&str>,
) -> ReplayConditionalSnapshotPlanReport {
    let frame_window_start = helper.frame_window_start.or(mismatch.frame).unwrap_or(0);
    let frame_window_end = helper
        .frame_window_end
        .unwrap_or(frame_window_start.max(mismatch.frame.unwrap_or(frame_window_start)));
    let run_frames = frame_window_end
        .saturating_sub(frame_window_start)
        .saturating_add(1)
        .max(1);

    let mut frame_points = BTreeSet::from([1u64, run_frames]);
    if let Some(mismatch_frame) = mismatch.frame {
        if mismatch_frame >= frame_window_start && mismatch_frame <= frame_window_end {
            frame_points.insert(mismatch_frame - frame_window_start + 1);
        }
    }

    let mut snapshot_triggers = frame_points
        .into_iter()
        .map(|relative_frame| ReplayConditionalSnapshotTriggerReport {
            kind: "frame".to_string(),
            value: relative_frame.to_string(),
            save_state: prefix.map(|prefix| {
                format!(
                    "{}_replay_mismatch_f{:06}.kqs",
                    prefix,
                    frame_window_start + relative_frame - 1
                )
            }),
        })
        .collect::<Vec<_>>();

    let slice = helper
        .actual_slice
        .as_ref()
        .or(helper.reference_slice.as_ref());
    if slice.is_some_and(|slice| slice.dma_event_count > 0) {
        snapshot_triggers.push(ReplayConditionalSnapshotTriggerReport {
            kind: "event".to_string(),
            value: "oam_dma".to_string(),
            save_state: prefix.map(|prefix| format!("{prefix}_replay_mismatch_dma.kqs")),
        });
    }
    if slice.is_some_and(|slice| slice.interrupt_event_count > 0) {
        snapshot_triggers.push(ReplayConditionalSnapshotTriggerReport {
            kind: "event".to_string(),
            value: "irq_service".to_string(),
            save_state: prefix.map(|prefix| format!("{prefix}_replay_mismatch_irq.kqs")),
        });
    }
    if slice.is_some_and(|slice| slice.render_event_count > 0) {
        snapshot_triggers.push(ReplayConditionalSnapshotTriggerReport {
            kind: "event".to_string(),
            value: "vblank".to_string(),
            save_state: prefix.map(|prefix| format!("{prefix}_replay_mismatch_vblank.kqs")),
        });
    }

    let mut seen_triggers = BTreeSet::new();
    snapshot_triggers.retain(|trigger| {
        seen_triggers.insert((
            trigger.kind.to_ascii_lowercase(),
            trigger.value.to_ascii_lowercase(),
            trigger.save_state.clone().unwrap_or_default(),
        ))
    });

    ReplayConditionalSnapshotPlanReport {
        frame_window_start,
        frame_window_end,
        run_frames,
        stop_specs: helper.recommended_cli_stop_specs.clone(),
        watch_windows: helper.recommended_watch_windows.clone(),
        snapshot_triggers,
        notes: vec![
            "Generated from the first replay mismatch so the next run can rewind into a narrower frame window.".to_string(),
            "Frame triggers are relative to the rewound window start, not absolute ROM-start frame numbers.".to_string(),
        ],
    }
}

// Attempt rewind into the retained window, merge recommended watches/stops/replay
// settings and execute snapshot triggers. This mutates the supplied live session and does
// not restore its original position/configuration afterward; returned frame counts are requested bounds.
fn run_replay_mismatch_snapshot_followup(
    session: &mut DebugSession,
    plan: &ReplayConditionalSnapshotPlanReport,
    prefix: &str,
) -> Result<ReplayConditionalSnapshotCaptureReport> {
    let current_frame = session.machine.clocks.frames;
    let mut effective_start = plan.frame_window_start;
    let mut notes = Vec::new();

    if let Some(oldest_checkpoint) = session.oldest_replay_checkpoint_frame() {
        if effective_start < oldest_checkpoint {
            notes.push(format!(
                "Requested replay follow-up start frame {} predates the oldest retained checkpoint {}; clamping the follow-up window.",
                effective_start, oldest_checkpoint
            ));
            effective_start = oldest_checkpoint;
        }
    }

    let effective_run_frames = plan
        .frame_window_end
        .saturating_sub(effective_start)
        .saturating_add(1)
        .max(1);
    // If the requested start lies after current time, this becomes zero and no forward seek is performed.
    let frames_back = current_frame.saturating_sub(effective_start);

    let rewound = if frames_back == 0 {
        true
    } else {
        session.rewind_frames(frames_back)
    };
    if !rewound {
        notes.push(format!(
            "Could not rewind from frame {} back to {} using the currently retained replay checkpoints.",
            current_frame, effective_start
        ));
        return Ok(ReplayConditionalSnapshotCaptureReport {
            attempted: true,
            rewound,
            start_frame: effective_start,
            end_frame: plan.frame_window_end,
            run_frames: effective_run_frames,
            prefix: Some(prefix.to_string()),
            generated_states: Vec::new(),
            notes,
        });
    }

    let mut merged_watch_windows = session.watch_windows().to_vec();
    merged_watch_windows.extend(parse_recommended_watch_windows(&plan.watch_windows)?);
    merged_watch_windows = normalize_watch_windows(merged_watch_windows)?;
    session.set_watch_windows(merged_watch_windows);

    let merged_stop_conditions = session
        .stop_conditions()
        .merged(&parse_recommended_stop_specs(&plan.stop_specs)?)
        .validate()
        .map_err(|err| anyhow!(err))?;
    session
        .set_stop_conditions(merged_stop_conditions)
        .map_err(|err| anyhow!("failed to apply replay mismatch stop overlay: {err}"))?;

    // Keep the follow-up checkpoint window at least as large as the planned interval; original settings are not restored.
    let replay_overlay = ReplayControlSet {
        enabled: true,
        checkpoint_interval_frames: 1,
        max_checkpoints: session
            .replay_control()
            .max_checkpoints
            .max((plan.run_frames as usize).saturating_add(2)),
        auto_rewind_on_stop_frames: None,
        stop_on_divergence: false,
    };
    let merged_replay = session
        .replay_control()
        .merged(&replay_overlay)
        .validate()
        .map_err(|err| anyhow!(err))?;
    session
        .set_replay_control(merged_replay)
        .map_err(|err| anyhow!("failed to apply replay mismatch replay overlay: {err}"))?;

    let skipped_window_frames = effective_start.saturating_sub(plan.frame_window_start);
    let stage = JobStage {
        frames: effective_run_frames,
        input: None,
        save_state: None,
        dump_report: None,
        snapshots: plan
            .snapshot_triggers
            .iter()
            .filter_map(|trigger| {
                if trigger.kind == "frame" {
                    let parsed = trigger.value.parse::<u64>().ok()?;
                    if parsed <= skipped_window_frames {
                        return None;
                    }
                    Some(SnapshotTrigger {
                        condition: None,
                        kind: trigger.kind.clone(),
                        value: (parsed - skipped_window_frames).to_string(),
                        save_state: trigger.save_state.clone(),
                    })
                } else {
                    Some(SnapshotTrigger {
                        condition: None,
                        kind: trigger.kind.clone(),
                        value: trigger.value.clone(),
                        save_state: trigger.save_state.clone(),
                    })
                }
            })
            .collect(),
        debugger: StopConditionSet::default(),
        replay: ReplayControlSet::default(),
    };

    let generated_states = run_stage_with_snapshot_list(session, 0, &stage, None, None)?;
    if generated_states.is_empty() {
        notes.push(
            "Follow-up replay window ran, but none of the generated conditional snapshot triggers fired."
                .to_string(),
        );
    } else {
        notes.push(format!(
            "Generated {} follow-up snapshot(s) inside the narrowed replay window.",
            generated_states.len()
        ));
    }

    Ok(ReplayConditionalSnapshotCaptureReport {
        attempted: true,
        rewound,
        start_frame: effective_start,
        end_frame: plan.frame_window_end,
        run_frames: effective_run_frames,
        prefix: Some(prefix.to_string()),
        generated_states,
        notes,
    })
}

// Match event-specific aliases and selected payload text ignoring ASCII case. Most
// branches test whether the filter contains a keyword; this is broader than exact event-type matching.
fn event_matches(event: &DebugEvent, needle: &str) -> bool {
    let n = needle.to_ascii_lowercase();
    match event {
        DebugEvent::ScanlineAdvance { .. } => n.contains("scanline"),
        DebugEvent::PpuModeChange { to_mode, .. } => {
            n == to_mode.to_ascii_lowercase() || n.contains("mode")
        }
        DebugEvent::ScanlineRender { .. } => n.contains("render") || n.contains("scanline_render"),
        DebugEvent::OamDmaStart { .. } | DebugEvent::OamDmaComplete { .. } => {
            n.contains("oam_dma") || n == "dma"
        }
        DebugEvent::GdmaStallEstimate { .. } => {
            n.contains("gdma_stall") || n.contains("dma_stall") || n == "gdma"
        }
        DebugEvent::HdmaStart { .. } => n.contains("hdma_start") || n == "hdma" || n == "gdma",
        DebugEvent::HdmaBlock { hblank_mode, .. } => {
            n.contains("hdma_block")
                || n.contains("dma_block")
                || (n == "hdma" && *hblank_mode)
                || (n == "gdma" && !*hblank_mode)
        }
        DebugEvent::HdmaComplete { hblank_mode, .. } => {
            n.contains("dma_complete")
                || (n == "hdma" && *hblank_mode)
                || (n == "gdma" && !*hblank_mode)
        }
        DebugEvent::HdmaCancel { .. } => n.contains("hdma_cancel") || n.contains("dma_cancel"),
        DebugEvent::HdmaDeferred { .. } => {
            n.contains("hdma_deferred") || n.contains("dma_deferred")
        }
        DebugEvent::HdmaWriteIgnored { .. } => {
            n.contains("hdma_write_ignored") || n.contains("dma_ignored")
        }
        DebugEvent::MapperControlWrite { .. } => {
            n.contains("mapper") || n.contains("mapper_ctrl") || n.contains("mapper_control")
        }
        DebugEvent::MapperRomBankChange { .. } => {
            n.contains("mapper_bank") || n.contains("mapper_rom_bank") || n.contains("mapper")
        }
        DebugEvent::MapperRamBankChange { .. } => {
            n.contains("mapper_ram_bank") || n.contains("mapper_bank") || n.contains("mapper")
        }
        DebugEvent::ApuMasterToggle { .. } => {
            n.contains("apu_master") || n.contains("nr52") || n == "apu"
        }
        DebugEvent::ApuChannelTrigger { .. } => {
            n.contains("apu_trigger") || n.contains("channel_trigger") || n == "apu"
        }
        DebugEvent::ApuChannelLengthExpired { .. } => {
            n.contains("apu_length") || n.contains("length") || n == "apu"
        }
        DebugEvent::ApuEnvelopeStep { .. } => {
            n.contains("apu_env") || n.contains("envelope") || n == "apu"
        }
        DebugEvent::ApuSweepStep { .. } => {
            n.contains("apu_sweep") || n.contains("sweep") || n == "apu"
        }
        DebugEvent::ApuChannelDisabled { .. } => {
            n.contains("apu_disable") || n.contains("apu_off") || n == "apu"
        }
        DebugEvent::ApuDacStateChange { .. } => {
            n.contains("apu_dac") || n.contains("dac") || n == "apu"
        }
        DebugEvent::ApuPopRisk { .. } => n.contains("apu_pop") || n.contains("pop") || n == "apu",
        DebugEvent::ApuFrameSequencerStep { .. } => {
            n.contains("apu_frame") || n.contains("frame_seq") || n == "apu"
        }
        DebugEvent::ApuMixerControl { .. } => {
            n.contains("apu_mix_ctrl") || n.contains("mixer") || n == "apu"
        }
        DebugEvent::ApuMixedOutput { .. } => {
            n.contains("apu_mix") || n.contains("mix_output") || n == "apu"
        }
        DebugEvent::ApuPcmFramesBuffered { .. } => {
            n.contains("apu_pcm") || n.contains("pcm") || n.contains("audio_buffer") || n == "apu"
        }
        DebugEvent::ApuPcmBufferWrapped { .. } => {
            n.contains("apu_drop")
                || n.contains("audio_drop")
                || n.contains("pcm_drop")
                || n == "apu"
        }
        DebugEvent::ApuWaveRamWrite { .. } => {
            n.contains("wave_ram") || n.contains("apu_wave") || n == "apu"
        }
        DebugEvent::ApuWaveRamAccessAliased { .. } => {
            n.contains("apu_wave_alias") || n.contains("wave_alias") || n == "apu"
        }
        DebugEvent::ApuCh3TriggerRetainsSample { .. } => {
            n.contains("ch3_hold") || n.contains("sample_hold") || n == "apu"
        }
        DebugEvent::ApuNoiseClockFrozen { .. } => {
            n.contains("noise_lock") || n.contains("noise_freeze") || n == "apu"
        }
        DebugEvent::CgbModeSelected { .. } => n.contains("cgb_mode") || n.contains("cgb"),
        DebugEvent::CgbVramBankSwitch { .. } => {
            n.contains("cgb_vram_bank") || n.contains("vbk") || n.contains("cgb_bank")
        }
        DebugEvent::CgbWramBankSwitch { .. } => {
            n.contains("cgb_wram_bank") || n.contains("svbk") || n.contains("cgb_bank")
        }
        DebugEvent::CgbBgPaletteIndexWrite { .. } => {
            n.contains("cgb_bgpi") || n.contains("bgpi") || n.contains("cgb_palette")
        }
        DebugEvent::CgbBgPaletteDataWrite { .. } => {
            n.contains("cgb_bgpd") || n.contains("bgpd") || n.contains("cgb_palette")
        }
        DebugEvent::CgbObjPaletteIndexWrite { .. } => {
            n.contains("cgb_obpi") || n.contains("obpi") || n.contains("cgb_palette")
        }
        DebugEvent::CgbObjPaletteDataWrite { .. } => {
            n.contains("cgb_obpd") || n.contains("obpd") || n.contains("cgb_palette")
        }
        DebugEvent::CgbKey1Write { .. } => {
            n.contains("key1") || n.contains("speed_arm") || n.contains("cgb_speed")
        }
        DebugEvent::CgbSpeedSwitch { .. } => {
            n.contains("speed_switch") || n.contains("double_speed") || n.contains("cgb_speed")
        }
        DebugEvent::CgbSpeedSwitchFreeze { .. } => {
            n.contains("speed_freeze") || n.contains("stop_freeze") || n.contains("cgb_speed")
        }
        DebugEvent::LcdToggle { enabled, .. } => {
            n.contains("lcd") || (n == "on" && *enabled) || (n == "off" && !*enabled)
        }
        DebugEvent::StatWrite { .. } => {
            n.contains("stat_write") || n.contains("statcfg") || n == "statw"
        }
        DebugEvent::LycWrite { .. } => n.contains("lyc") || n.contains("lyc_write"),
        DebugEvent::StatSignal { .. } => n.contains("stat") || n.contains("coincidence"),
        DebugEvent::VblankEnter { .. } => n.contains("vblank"),
        DebugEvent::FrameComplete { .. } => n.contains("frame"),
        DebugEvent::BankSwitch { .. } => n.contains("bank"),
        DebugEvent::FarCallSuspected { .. } => n.contains("farcall"),
        DebugEvent::KitaqgbIntrinsic {
            symbol,
            intrinsic_kind,
            ..
        } => {
            symbol.to_ascii_lowercase().contains(&n)
                || intrinsic_kind.to_ascii_lowercase().contains(&n)
        }
        DebugEvent::BankReturnMissing { .. } => n.contains("returnmissing"),
        DebugEvent::TimerInterrupt { .. } => n.contains("timer"),
        DebugEvent::TimerOverflow { .. } => n.contains("timer_overflow") || n.contains("overflow"),
        DebugEvent::TimerReload { .. } => n.contains("timer_reload") || n.contains("reload"),
        DebugEvent::TimerControlWrite { .. } => n.contains("timer_ctrl") || n.contains("tac"),
        DebugEvent::DivResetEdge { .. } => n.contains("div_reset") || n == "div",
        DebugEvent::SerialTransferStart { .. } | DebugEvent::SerialTransferComplete { .. } => {
            n.contains("serial") || n.contains("serial_transfer")
        }
        DebugEvent::JoypadEdge { .. } => {
            n.contains("joypad_edge") || n.contains("joypad") || n.contains("input_edge")
        }
        DebugEvent::JoypadRead { .. } => {
            n.contains("joypad_read") || n.contains("p1_read") || n.contains("joypad")
        }
        DebugEvent::JoypadSelectionWrite { .. } => {
            n.contains("joypad_select") || n.contains("p1_write")
        }
        DebugEvent::JoypadInterrupt { .. } => {
            n.contains("joypad_irq") || n.contains("joypad_interrupt")
        }
        // Any filter containing irq matches this request branch, including more specific-looking strings such as irq_service.
        DebugEvent::InterruptRequested { source, .. } => {
            n.contains("irq") || n == source.to_ascii_lowercase()
        }
        DebugEvent::InterruptServiced { source, .. } => {
            n.contains("irq_service")
                || n.contains("interrupt_service")
                || n == format!("{}_service", source.to_ascii_lowercase())
        }
        DebugEvent::InterruptPendingBlocked { .. } => {
            n.contains("irq_blocked") || n.contains("interrupt_blocked")
        }
        DebugEvent::ExecutionStop {
            kind,
            label,
            source,
            symbol,
            ..
        } => {
            n.contains("stop")
                || kind.to_ascii_lowercase().contains(&n)
                || label.to_ascii_lowercase().contains(&n)
                || source
                    .as_ref()
                    .is_some_and(|v| v.to_ascii_lowercase().contains(&n))
                || symbol
                    .as_ref()
                    .is_some_and(|v| v.to_ascii_lowercase().contains(&n))
        }
        DebugEvent::SymbolContextChange { symbol, source, .. } => {
            n.contains("symbol")
                || n.contains("source")
                || symbol
                    .as_ref()
                    .is_some_and(|v| v.to_ascii_lowercase().contains(&n))
                || source
                    .as_ref()
                    .is_some_and(|v| v.to_ascii_lowercase().contains(&n))
        }
        DebugEvent::UnsupportedOpcode { opcode, .. } => {
            n.contains("unsupported") || n.contains("opcode") || n == format!("{opcode:02x}")
        }
        DebugEvent::ReplayCheckpointSaved { .. } => {
            n.contains("replay") || n.contains("checkpoint")
        }
        DebugEvent::ReplayRewindApplied { .. } => n.contains("replay") || n.contains("rewind"),
        DebugEvent::ReplayDivergenceDetected { .. } => {
            n.contains("replay") || n.contains("divergence")
        }
        DebugEvent::BankThrashSuspected { .. } => n.contains("thrash"),
    }
}

// Keep ASCII letters and digits; replace every other character with an underscore for generated labels.
fn sanitize_name(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

// Accept an entire hexadecimal byte or comma-separated button aliases and OR their bits.
// Empty input releases every button; decimal numeric masks other than zero are not accepted.
fn parse_input_mask(spec: &str) -> Result<u8> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Ok(0);
    }

    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        let value = u8::from_str_radix(hex, 16)
            .with_context(|| format!("invalid hexadecimal input mask: {trimmed}"))?;
        return Ok(value);
    }

    let mut mask = 0u8;
    for token in trimmed
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        let bit = match token.to_ascii_uppercase().as_str() {
            "RIGHT" | "R" => 0x01,
            "LEFT" | "L" => 0x02,
            "UP" | "U" => 0x04,
            "DOWN" | "D" => 0x08,
            "A" => 0x10,
            "B" => 0x20,
            "SELECT" | "SEL" => 0x40,
            "START" | "ST" => 0x80,
            "NONE" | "0" => 0x00,
            other => bail!("unknown input token: {other}"),
        };
        mask |= bit;
    }

    Ok(mask)
}

// Parse semicolon-separated INPUT:FRAMES entries in order. Empty entries are ignored
// and zero durations are accepted; the frame substring is not separately trimmed.
fn parse_input_sequence(spec: &str) -> Result<Vec<(u8, u64)>> {
    let mut out = Vec::new();
    for part in spec.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let (input_part, frame_part) = part.split_once(':').ok_or_else(|| {
            anyhow!("invalid input sequence entry: {part}; expected INPUT:FRAMES")
        })?;
        let mask = parse_input_mask(input_part)?;
        let frames: u64 = frame_part
            .parse()
            .with_context(|| format!("invalid frame count in input sequence entry: {part}"))?;
        out.push((mask, frames));
    }
    Ok(out)
}

// Require exactly NAME:ADDR:SIZE, parse decimal or prefixed hexadecimal numbers,
// then validate the name and memory range through the shared watch specification.
fn parse_watch_window_spec(spec: &str) -> Result<MemoryWatchSpec> {
    let mut parts = spec.split(':');
    let name = parts
        .next()
        .ok_or_else(|| anyhow!("invalid watch-window '{spec}'; expected NAME:ADDR:SIZE"))?;
    let addr = parts
        .next()
        .ok_or_else(|| anyhow!("invalid watch-window '{spec}'; expected NAME:ADDR:SIZE"))?;
    let size = parts
        .next()
        .ok_or_else(|| anyhow!("invalid watch-window '{spec}'; expected NAME:ADDR:SIZE"))?;
    if parts.next().is_some() {
        bail!("invalid watch-window '{spec}'; expected NAME:ADDR:SIZE");
    }

    MemoryWatchSpec {
        name: name.to_string(),
        addr: parse_u16_value(addr)
            .with_context(|| format!("invalid watch-window address in '{spec}'"))?,
        size: parse_u16_value(size)
            .with_context(|| format!("invalid watch-window size in '{spec}'"))?,
    }
    .validate()
    .map_err(|message| anyhow!(message))
}

// Validate each supplied watch in order; this helper does not deduplicate names or ranges.
fn normalize_watch_windows(specs: Vec<MemoryWatchSpec>) -> Result<Vec<MemoryWatchSpec>> {
    specs
        .into_iter()
        .map(|spec| spec.validate().map_err(|message| anyhow!(message)))
        .collect()
}

// Parse each repeated CLI stop option, then validate the combined condition set.
// Any invalid entry rejects the configuration before execution.
fn parse_cli_debugger_args(args: &Args) -> Result<StopConditionSet> {
    let breakpoints = args
        .breakpoints
        .iter()
        .map(|spec| parse_breakpoint_spec(spec))
        .collect::<Result<Vec<_>>>()?;
    let watchpoints = args
        .watchpoints
        .iter()
        .map(|spec| parse_watchpoint_spec(spec))
        .collect::<Result<Vec<_>>>()?;
    let mmio_writes = args
        .stop_on_mmio
        .iter()
        .map(|spec| parse_mmio_stop_spec(spec))
        .collect::<Result<Vec<_>>>()?;
    let interrupts = args
        .stop_on_irq
        .iter()
        .map(|spec| parse_interrupt_stop_spec(spec))
        .collect::<Result<Vec<_>>>()?;
    let dma_events = args
        .stop_on_dma
        .iter()
        .map(|spec| parse_dma_stop_spec(spec))
        .collect::<Result<Vec<_>>>()?;
    StopConditionSet {
        breakpoints,
        watchpoints,
        mmio_writes,
        interrupts,
        dma_events,
    }
    .validate()
    .map_err(|err| anyhow!(err))
}

// Enable checkpoint recording when replay timing, retention, rewind or divergence
// controls are supplied. Export/compare paths alone do not enable recording here.
fn parse_cli_replay_args(args: &Args) -> Result<ReplayControlSet> {
    let enabled = args.replay_interval.is_some()
        || args.replay_max_checkpoints.is_some()
        || args.rewind_on_stop_frames.is_some()
        || args.stop_on_divergence;
    ReplayControlSet {
        enabled,
        checkpoint_interval_frames: args.replay_interval.unwrap_or(1),
        max_checkpoints: args.replay_max_checkpoints.unwrap_or(16),
        auto_rewind_on_stop_frames: args.rewind_on_stop_frames,
        stop_on_divergence: args.stop_on_divergence,
    }
    .validate()
    .map_err(|err| anyhow!(err))
}

// Parse a symbol or numeric PC with an optional @bank qualifier, then validate it.
// The symbol:, pc: and @bank: syntax markers are case-sensitive.
fn parse_breakpoint_spec(spec: &str) -> Result<ExecuteBreakpointSpec> {
    let trimmed = spec.trim();
    let (head, bank) = if let Some((head, bank_part)) = trimmed.split_once("@bank:") {
        (
            head,
            Some(
                parse_u16_value(bank_part)
                    .with_context(|| format!("invalid breakpoint bank in '{spec}'"))?,
            ),
        )
    } else {
        (trimmed, None)
    };
    let parsed = if let Some(symbol) = head.strip_prefix("symbol:") {
        ExecuteBreakpointSpec {
            pc: None,
            bank,
            symbol: Some(symbol.trim().to_string()),
        }
    } else {
        let pc_token = head.strip_prefix("pc:").unwrap_or(head);
        ExecuteBreakpointSpec {
            pc: Some(
                parse_u16_value(pc_token)
                    .with_context(|| format!("invalid breakpoint pc in '{spec}'"))?,
            ),
            bank,
            symbol: None,
        }
    };
    parsed.validate().map_err(|err| anyhow!(err))
}

// Parse an optional name@ prefix and address+size range; an omitted size watches one byte.
fn parse_watchpoint_spec(spec: &str) -> Result<MemoryWatchpointSpec> {
    let trimmed = spec.trim();
    let (name, body) = if let Some((name, body)) = trimmed.split_once('@') {
        (Some(name.trim().to_string()), body.trim())
    } else {
        (None, trimmed)
    };
    let (addr, size) = if let Some((addr, size)) = body.split_once('+') {
        (
            parse_u16_value(addr)
                .with_context(|| format!("invalid watchpoint address in '{spec}'"))?,
            parse_u16_value(size)
                .with_context(|| format!("invalid watchpoint size in '{spec}'"))?,
        )
    } else {
        (
            parse_u16_value(body)
                .with_context(|| format!("invalid watchpoint address in '{spec}'"))?,
            1,
        )
    };
    MemoryWatchpointSpec { name, addr, size }
        .validate()
        .map_err(|err| anyhow!(err))
}

// Parse an optional name@ prefix and validate the address as an MMIO write stop.
fn parse_mmio_stop_spec(spec: &str) -> Result<MmioWriteStopSpec> {
    let trimmed = spec.trim();
    let (name, addr_token) = if let Some((name, addr)) = trimmed.split_once('@') {
        (Some(name.trim().to_string()), addr.trim())
    } else {
        (None, trimmed)
    };
    MmioWriteStopSpec {
        addr: parse_u16_value(addr_token)
            .with_context(|| format!("invalid MMIO stop address in '{spec}'"))?,
        name,
    }
    .validate()
    .map_err(|err| anyhow!(err))
}

// Parse a source, source:phase or one of the three lowercase phase-only forms.
// Empty colon-separated components are skipped before shape validation.
fn parse_interrupt_stop_spec(spec: &str) -> Result<InterruptStopSpec> {
    let trimmed = spec.trim();
    let mut parts = trimmed
        .split(':')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty());
    let first = parts
        .next()
        .ok_or_else(|| anyhow!("empty interrupt stop specification"))?;
    let second = parts.next();
    let third = parts.next();
    if parts.next().is_some() {
        bail!("invalid interrupt stop specification '{spec}'");
    }
    let (source, phase) = match (first, second, third) {
        ("requested", None, None) => (None, InterruptStopPhase::Requested),
        ("serviced", None, None) => (None, InterruptStopPhase::Serviced),
        ("blocked", None, None) => (None, InterruptStopPhase::Blocked),
        (source, None, None) => (Some(source.to_string()), InterruptStopPhase::Any),
        (source, Some(phase), None) => {
            (Some(source.to_string()), parse_interrupt_stop_phase(phase)?)
        }
        _ => bail!("invalid interrupt stop specification '{spec}'"),
    };
    InterruptStopSpec { source, phase }
        .validate()
        .map_err(|err| anyhow!(err))
}

// Resolve case-insensitive phase aliases; reject unknown phase names instead of assuming Any.
fn parse_interrupt_stop_phase(token: &str) -> Result<InterruptStopPhase> {
    match token.trim().to_ascii_lowercase().as_str() {
        "requested" | "request" | "req" => Ok(InterruptStopPhase::Requested),
        "serviced" | "service" | "svc" => Ok(InterruptStopPhase::Serviced),
        "blocked" | "pending_blocked" | "block" => Ok(InterruptStopPhase::Blocked),
        "any" => Ok(InterruptStopPhase::Any),
        other => bail!("unsupported interrupt stop phase '{other}'"),
    }
}

// Normalize the DMA event name to lowercase and validate it against supported stop events.
fn parse_dma_stop_spec(spec: &str) -> Result<DmaStopSpec> {
    DmaStopSpec {
        event: spec.trim().to_ascii_lowercase(),
    }
    .validate()
    .map_err(|err| anyhow!(err))
}

// Trim surrounding whitespace and parse an unsigned 16-bit decimal value, or hexadecimal
// when explicitly prefixed by 0x/0X. Overflow and malformed values return contextual errors.
fn parse_u16_value(token: &str) -> Result<u16> {
    let trimmed = token.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return u16::from_str_radix(hex, 16)
            .with_context(|| format!("invalid hexadecimal value: {trimmed}"));
    }
    trimmed
        .parse::<u16>()
        .with_context(|| format!("invalid decimal value: {trimmed}"))
}

// Rebase job and stage input/output paths against the job directory. Absolute paths
// are preserved; joining does not canonicalize paths or check whether files exist.
fn resolve_job_paths(job: &mut JobSpec, base: &Path) {
    job.rom = normalize_path(base, &job.rom).display().to_string();
    job.symbols = job
        .symbols
        .as_ref()
        .map(|path| normalize_path(base, path).display().to_string());
    job.source_map = job
        .source_map
        .as_ref()
        .map(|path| normalize_path(base, path).display().to_string());
    job.toolchain_metadata = job
        .toolchain_metadata
        .as_ref()
        .map(|path| normalize_path(base, path).display().to_string());
    job.load_state = job
        .load_state
        .as_ref()
        .map(|path| normalize_path(base, path).display().to_string());
    job.save_state = job
        .save_state
        .as_ref()
        .map(|path| normalize_path(base, path).display().to_string());
    job.dump_report = job
        .dump_report
        .as_ref()
        .map(|path| normalize_path(base, path).display().to_string());
    job.dump_replay_tape = job
        .dump_replay_tape
        .as_ref()
        .map(|path| normalize_path(base, path).display().to_string());
    job.compare_replay_tape = job
        .compare_replay_tape
        .as_ref()
        .map(|path| normalize_path(base, path).display().to_string());
    job.snapshot_on_replay_mismatch = job
        .snapshot_on_replay_mismatch
        .as_ref()
        .map(|path| normalize_path(base, path).display().to_string());
    job.screenshot = job
        .screenshot
        .as_ref()
        .map(|path| normalize_path(base, path).display().to_string());
    job.record_video = job
        .record_video
        .as_ref()
        .map(|path| normalize_path(base, path).display().to_string());
    job.record_wav = job
        .record_wav
        .as_ref()
        .map(|path| normalize_path(base, path).display().to_string());
    job.autosave_prefix = job
        .autosave_prefix
        .as_ref()
        .map(|path| normalize_path(base, path).display().to_string());
    for stage in &mut job.stages {
        stage.save_state = stage
            .save_state
            .as_ref()
            .map(|path| normalize_path(base, path).display().to_string());
        stage.dump_report = stage
            .dump_report
            .as_ref()
            .map(|path| normalize_path(base, path).display().to_string());
        for trigger in &mut stage.snapshots {
            trigger.save_state = trigger
                .save_state
                .as_ref()
                .map(|path| normalize_path(base, path).display().to_string());
        }
    }
}

// Rebase the linked-job report and each session asset/state path against its job directory.
fn resolve_link_job_paths(job: &mut LinkJobSpec, base: &Path) {
    job.dump_report = job
        .dump_report
        .as_ref()
        .map(|path| normalize_path(base, path).display().to_string());
    for session in &mut job.sessions {
        session.rom = normalize_path(base, &session.rom).display().to_string();
        session.symbols = session
            .symbols
            .as_ref()
            .map(|path| normalize_path(base, path).display().to_string());
        session.source_map = session
            .source_map
            .as_ref()
            .map(|path| normalize_path(base, path).display().to_string());
        session.toolchain_metadata = session
            .toolchain_metadata
            .as_ref()
            .map(|path| normalize_path(base, path).display().to_string());
        session.load_state = session
            .load_state
            .as_ref()
            .map(|path| normalize_path(base, path).display().to_string());
        session.save_state = session
            .save_state
            .as_ref()
            .map(|path| normalize_path(base, path).display().to_string());
    }
}

// Replace the ROM extension and return the sidecar only when that path is an existing file.
fn detect_sidecar_path(rom_path: &str, extension: &str) -> Option<String> {
    let mut candidate = PathBuf::from(rom_path);
    candidate.set_extension(extension);
    candidate.is_file().then(|| candidate.display().to_string())
}

// Read UTF-8 symbol metadata and deserialize its structure with a path-specific error.
fn load_symbol_table_json_file(path: &str) -> Result<SymbolTable> {
    let text = fs::read_to_string(path).with_context(|| format!("failed to read file: {path}"))?;
    serde_json::from_str::<SymbolTable>(&text)
        .with_context(|| format!("failed to parse SymbolTable JSON: {path}"))
}

// Read and deserialize build metadata; this does not independently verify its ROM hash.
fn load_toolchain_build_report_file(path: &str) -> Result<ToolchainBuildReport> {
    let text = fs::read_to_string(path).with_context(|| format!("failed to read file: {path}"))?;
    serde_json::from_str::<ToolchainBuildReport>(&text)
        .with_context(|| format!("failed to parse build report JSON: {path}"))
}

// Read annotation JSON into the shared model; interpretation occurs in the decompiler.
fn load_decompile_annotation_file(path: &str) -> Result<DecompileAnnotationFile> {
    let text = fs::read_to_string(path).with_context(|| format!("failed to read file: {path}"))?;
    serde_json::from_str::<DecompileAnnotationFile>(&text)
        .with_context(|| format!("failed to parse decompile annotation JSON: {path}"))
}

// Read nonempty JSONL rows in file order, requiring numeric PC and bank fields.
// Optional maps retain only correctly typed values; this loader does not establish ROM identity
// or chronological ordering, and numeric PC/bank/SP casts retain only their low 16 bits.
fn load_decompile_trace_rows(path: &str) -> Result<Vec<DecompileTraceObservation>> {
    let text = fs::read_to_string(path).with_context(|| format!("failed to read file: {path}"))?;
    let mut rows = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: JsonValue = serde_json::from_str(trimmed).with_context(|| {
            format!(
                "failed to parse decompile trace row {} as JSON: {}",
                line_index + 1,
                path
            )
        })?;
        rows.push(DecompileTraceObservation {
            completed_frames: value.get("completed_frames").and_then(JsonValue::as_u64),
            active_frame: value.get("active_frame").and_then(JsonValue::as_u64),
            cycle: value.get("cycle").and_then(JsonValue::as_u64),
            pc: value
                .get("pc")
                .and_then(JsonValue::as_u64)
                .ok_or_else(|| anyhow!("decompile trace row {} missing pc", line_index + 1))?
                as u16,
            rom_bank: value
                .get("rom_bank")
                .and_then(JsonValue::as_u64)
                .ok_or_else(|| anyhow!("decompile trace row {} missing rom_bank", line_index + 1))?
                as u16,
            hit_count: value
                .get("hit_count")
                .and_then(JsonValue::as_u64)
                .unwrap_or(1)
                .max(1),
            symbol: value
                .get("symbol")
                .and_then(JsonValue::as_str)
                .map(str::to_string),
            source: value
                .get("source")
                .and_then(JsonValue::as_str)
                .map(str::to_string),
            event_summary: value
                .get("event_summary")
                .and_then(JsonValue::as_str)
                .map(str::to_string),
            watch_summary: value
                .get("watch_summary")
                .and_then(JsonValue::as_str)
                .map(str::to_string),
            registers: value
                .get("registers")
                .and_then(JsonValue::as_object)
                .map(|map| {
                    map.iter()
                        .filter_map(|(key, value)| {
                            value.as_u64().map(|number| (key.clone(), number))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            flags: value
                .get("flags")
                .and_then(JsonValue::as_object)
                .map(|map| {
                    map.iter()
                        .filter_map(|(key, value)| value.as_bool().map(|flag| (key.clone(), flag)))
                        .collect()
                })
                .unwrap_or_default(),
            stack_slot_values: value
                .get("stack_slot_values")
                .and_then(JsonValue::as_object)
                .map(|map| {
                    map.iter()
                        .filter_map(|(key, value)| {
                            value.as_u64().map(|number| (key.clone(), number))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            sp: value
                .get("sp")
                .and_then(JsonValue::as_u64)
                .map(|raw| raw as u16),
        });
    }
    Ok(rows)
}

// Wrap an existing replay report with its path and optional build-reported ROM hash.
// No tape is produced without replay data, and the ROM is not freshly hashed here.
fn build_replay_tape(report: &DebugReport, rom_path: &str) -> Option<ReplayTapeEnvelope> {
    let replay = report.replay.clone()?;
    Some(ReplayTapeEnvelope {
        schema_version: "1".to_string(),
        rom_path: rom_path.to_string(),
        rom_sha256: report
            .toolchain_build
            .as_ref()
            .and_then(|build| build.output_sha256.clone()),
        replay,
    })
}

// Deserialize the replay envelope; schema compatibility and ROM identity are not checked here.
fn load_replay_tape_file(path: &str) -> Result<ReplayTapeEnvelope> {
    let text = fs::read_to_string(path).with_context(|| format!("failed to read file: {path}"))?;
    serde_json::from_str::<ReplayTapeEnvelope>(&text)
        .with_context(|| format!("failed to parse replay tape JSON: {path}"))
}

// Locate the mismatch slice and adjacent slices by list position, then suggest a
// frame window and diagnostic controls from observed activity. These suggestions are
// heuristics, not evidence that a particular subsystem caused the divergence.
fn build_divergence_focus(
    reference: &ReplayReport,
    actual: &ReplayReport,
    mismatch: &ReplayReferenceMismatchReport,
) -> Option<ReplayDivergenceHelperReport> {
    let mismatch_slice_index = mismatch
        .checkpoint_index
        .and_then(|checkpoint_index| {
            actual
                .slices
                .iter()
                .position(|slice| slice.to_checkpoint_index == checkpoint_index)
                .or_else(|| {
                    reference
                        .slices
                        .iter()
                        .position(|slice| slice.to_checkpoint_index == checkpoint_index)
                })
        })
        .or_else(|| match mismatch.kind.as_str() {
            "slice_count" => Some(reference.slices.len().min(actual.slices.len())),
            _ => None,
        });

    let slice_ref = mismatch_slice_index.and_then(|idx| reference.slices.get(idx).cloned());
    let slice_actual = mismatch_slice_index.and_then(|idx| actual.slices.get(idx).cloned());
    let previous_reference_slice = mismatch_slice_index.and_then(|idx| {
        idx.checked_sub(1)
            .and_then(|i| reference.slices.get(i).cloned())
    });
    let previous_actual_slice = mismatch_slice_index.and_then(|idx| {
        idx.checked_sub(1)
            .and_then(|i| actual.slices.get(i).cloned())
    });
    let next_reference_slice =
        mismatch_slice_index.and_then(|idx| reference.slices.get(idx + 1).cloned());
    let next_actual_slice =
        mismatch_slice_index.and_then(|idx| actual.slices.get(idx + 1).cloned());

    let frame_window_start = previous_actual_slice
        .as_ref()
        .map(|slice| slice.start_frame)
        .or_else(|| {
            previous_reference_slice
                .as_ref()
                .map(|slice| slice.start_frame)
        })
        .or_else(|| slice_actual.as_ref().map(|slice| slice.start_frame))
        .or_else(|| slice_ref.as_ref().map(|slice| slice.start_frame))
        .or(mismatch.frame);
    let frame_window_end = next_actual_slice
        .as_ref()
        .map(|slice| slice.end_frame)
        .or_else(|| next_reference_slice.as_ref().map(|slice| slice.end_frame))
        .or_else(|| slice_actual.as_ref().map(|slice| slice.end_frame))
        .or_else(|| slice_ref.as_ref().map(|slice| slice.end_frame))
        .or(mismatch.frame);

    let mut recommended_focus = Vec::new();
    let mut recommended_cli_stop_specs = Vec::new();
    let mut recommended_watch_windows = Vec::new();
    if let (Some(start), Some(end)) = (frame_window_start, frame_window_end) {
        recommended_focus.push(format!(
            "Re-run with replay enabled and place a tighter checkpoint window around frames {}..{}.",
            start, end
        ));
    }
    if let Some(slice) = slice_actual.as_ref().or(slice_ref.as_ref()) {
        if slice.bank_switch_count > 0 || slice.far_call_count > 0 {
            recommended_focus.push(
                "The mismatching slice crosses bank/call boundaries; watch current ROM bank and stop on bank/far-call edges."
                    .to_string(),
            );
        }
        if slice.dma_event_count > 0 {
            recommended_cli_stop_specs.push("--stop-on-dma oam_start".to_string());
            recommended_cli_stop_specs.push("--stop-on-dma oam_complete".to_string());
            recommended_watch_windows.push("--watch-window oam:0xFE00:0xA0".to_string());
            recommended_focus.push(
                "DMA activity appears inside the mismatching slice; add snapshots on oam_dma/hdma/gdma edges."
                    .to_string(),
            );
        }
        // The MMIO recommendation below stops on FF00 writes, not on the observed reads themselves.
        if slice.joypad_read_count > 0 {
            recommended_cli_stop_specs.push("--stop-on-mmio 0xFF00".to_string());
            recommended_watch_windows.push("--watch-window p1:0xFF00:0x01".to_string());
            recommended_focus.push(
                "Joypad polling appears inside the mismatching slice; pair replay compare with FF00 watch windows and joypad_read stops."
                    .to_string(),
            );
        }
        if slice.interrupt_event_count > 0 {
            recommended_cli_stop_specs.push("--stop-on-irq vblank:request".to_string());
            recommended_cli_stop_specs.push("--stop-on-irq lcd_stat:request".to_string());
            recommended_focus.push(
                "Interrupt activity differs in the mismatching slice; stop on irq request/service and compare IME/SP around that boundary."
                    .to_string(),
            );
        }
        if slice.render_event_count > 0 {
            recommended_watch_windows.push("--watch-window lcd:0xFF40:0x06".to_string());
            recommended_watch_windows.push("--watch-window vram:0x9800:0x40".to_string());
            recommended_focus.push(
                "PPU/render events appear in the mismatching slice; compare LCDC/STAT/LYC plus relevant VRAM watches around the same frames."
                    .to_string(),
            );
        }
    }
    if recommended_focus.is_empty() {
        recommended_focus.push(
            "Use the previous and mismatching slices together to narrow the first branching point, then add targeted watch windows before replaying."
                .to_string(),
        );
    }
    recommended_cli_stop_specs.sort();
    recommended_cli_stop_specs.dedup();
    recommended_watch_windows.sort();
    recommended_watch_windows.dedup();

    Some(ReplayDivergenceHelperReport {
        mismatch_slice_index,
        mismatch_checkpoint_index: mismatch.checkpoint_index,
        frame_window_start,
        frame_window_end,
        previous_reference_slice,
        previous_actual_slice,
        reference_slice: slice_ref,
        actual_slice: slice_actual,
        next_reference_slice,
        next_actual_slice,
        recommended_cli_stop_specs,
        recommended_watch_windows,
        recommended_focus,
    })
}

// Compare checkpoints in list order and report the first checked difference, then
// compare available slice digests unless watch-only mode is selected. A match means
// no checked difference was found; missing recordings and absent watch digests limit coverage.
fn compare_replay_reports(
    reference: &ReplayReport,
    actual: &ReplayReport,
    reference_path: Option<String>,
    watch_only: bool,
) -> ReplayReferenceComparisonReport {
    let mut notes = Vec::new();
    let mut first_mismatch: Option<ReplayReferenceMismatchReport> = None;

    // Pair by array position, not by checkpoint identifier; ROM identity is not an input to this comparison.
    let min_checkpoint_count = reference.checkpoints.len().min(actual.checkpoints.len());
    for idx in 0..min_checkpoint_count {
        let expected = &reference.checkpoints[idx];
        let observed = &actual.checkpoints[idx];
        let mismatch = if expected.frame != observed.frame {
            Some(ReplayReferenceMismatchReport {
                kind: "checkpoint_frame".to_string(),
                checkpoint_index: Some(observed.checkpoint_index),
                frame: Some(observed.frame),
                detail: "checkpoint frame changed".to_string(),
                expected: expected.frame.to_string(),
                actual: observed.frame.to_string(),
            })
        } else if !watch_only && expected.digest != observed.digest {
            Some(ReplayReferenceMismatchReport {
                kind: "checkpoint_digest".to_string(),
                checkpoint_index: Some(observed.checkpoint_index),
                frame: Some(observed.frame),
                detail: "checkpoint digest diverged".to_string(),
                expected: format!("{:016X}", expected.digest),
                actual: format!("{:016X}", observed.digest),
            })
        } else if !watch_only && expected.frame_hash != observed.frame_hash {
            Some(ReplayReferenceMismatchReport {
                kind: "checkpoint_frame_hash".to_string(),
                checkpoint_index: Some(observed.checkpoint_index),
                frame: Some(observed.frame),
                detail: "framebuffer hash diverged at checkpoint".to_string(),
                expected: format!("{:08X}", expected.frame_hash),
                actual: format!("{:08X}", observed.frame_hash),
            })
        // A zero digest on either side skips this watch comparison, even in watch-only mode.
        } else if expected.watch_digest != 0
            && observed.watch_digest != 0
            && expected.watch_digest != observed.watch_digest
        {
            Some(ReplayReferenceMismatchReport {
                kind: "checkpoint_watch_digest".to_string(),
                checkpoint_index: Some(observed.checkpoint_index),
                frame: Some(observed.frame),
                detail: "watch-window digest diverged at checkpoint".to_string(),
                expected: format!("{:016X}", expected.watch_digest),
                actual: format!("{:016X}", observed.watch_digest),
            })
        } else {
            None
        };
        if mismatch.is_some() {
            first_mismatch = mismatch;
            break;
        }
    }

    if first_mismatch.is_none() && reference.checkpoints.len() != actual.checkpoints.len() {
        first_mismatch = Some(ReplayReferenceMismatchReport {
            kind: "checkpoint_count".to_string(),
            checkpoint_index: None,
            frame: None,
            detail: "checkpoint count changed".to_string(),
            expected: reference.checkpoints.len().to_string(),
            actual: actual.checkpoints.len().to_string(),
        });
    }

    let min_slice_count = reference.slices.len().min(actual.slices.len());
    if watch_only {
        notes.push(
            "Replay comparison ran in watch-only mode; checkpoint digests and slice digests were ignored."
                .to_string(),
        );
    // Missing slices add a coverage note but do not themselves make the reports mismatched.
    } else if reference.slices.is_empty() || actual.slices.is_empty() {
        notes.push(
            "One side does not contain replay slices; slice-level comparison was limited."
                .to_string(),
        );
    } else if first_mismatch.is_none() {
        for idx in 0..min_slice_count {
            let expected = &reference.slices[idx];
            let observed = &actual.slices[idx];
            if expected.slice_digest != observed.slice_digest {
                first_mismatch = Some(ReplayReferenceMismatchReport {
                    kind: "slice_digest".to_string(),
                    checkpoint_index: Some(observed.to_checkpoint_index),
                    frame: Some(observed.end_frame),
                    detail: "checkpoint-to-checkpoint execution slice diverged".to_string(),
                    expected: format!("{:016X}", expected.slice_digest),
                    actual: format!("{:016X}", observed.slice_digest),
                });
                break;
            }
        }
        if first_mismatch.is_none() && reference.slices.len() != actual.slices.len() {
            first_mismatch = Some(ReplayReferenceMismatchReport {
                kind: "slice_count".to_string(),
                checkpoint_index: None,
                frame: None,
                detail: "slice count changed".to_string(),
                expected: reference.slices.len().to_string(),
                actual: actual.slices.len().to_string(),
            });
        }
    }

    if reference.checkpoints.is_empty() && actual.checkpoints.is_empty() {
        notes.push(
            "Neither replay report recorded checkpoints; comparison stayed structural only."
                .to_string(),
        );
    }
    if reference
        .checkpoints
        .iter()
        .all(|checkpoint| checkpoint.watch_hashes.is_empty())
        || actual
            .checkpoints
            .iter()
            .all(|checkpoint| checkpoint.watch_hashes.is_empty())
    {
        notes.push(
            "Watch-window hashes were absent on at least one side, so checkpoint watch comparisons were coarse."
                .to_string(),
        );
    }

    let divergence_helper = first_mismatch
        .as_ref()
        .and_then(|mismatch| build_divergence_focus(reference, actual, mismatch));

    ReplayReferenceComparisonReport {
        reference_path,
        matched: first_mismatch.is_none(),
        reference_checkpoint_count: reference.checkpoints.len(),
        actual_checkpoint_count: actual.checkpoints.len(),
        reference_slice_count: reference.slices.len(),
        actual_slice_count: actual.slices.len(),
        first_mismatch,
        divergence_helper,
        conditional_snapshot_plan: None,
        conditional_snapshot_capture: None,
        carry_forward_notes: notes,
    }
}

// Preserve absolute paths and join relative paths to the supplied base without canonicalization.
fn normalize_path(base: &Path, candidate: &str) -> PathBuf {
    let path = Path::new(candidate);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

// Parse one decimal frame or an inclusive start:end range, rejecting zero and reversed bounds.
fn parse_frame_range_spec(spec: &str) -> Result<FrameRange> {
    let trimmed = spec.trim();
    if let Some((start, end)) = trimmed.split_once(':') {
        let start = start
            .trim()
            .parse::<u64>()
            .with_context(|| format!("invalid frame range start: {trimmed}"))?;
        let end = end
            .trim()
            .parse::<u64>()
            .with_context(|| format!("invalid frame range end: {trimmed}"))?;
        if start == 0 || end == 0 {
            bail!("frame ranges are 1-based; got '{trimmed}'");
        }
        if end < start {
            bail!("frame range end must be >= start; got '{trimmed}'");
        }
        Ok(FrameRange { start, end })
    } else {
        let frame = trimmed
            .parse::<u64>()
            .with_context(|| format!("invalid frame number: {trimmed}"))?;
        if frame == 0 {
            bail!("frame numbers are 1-based; got '{trimmed}'");
        }
        Ok(FrameRange {
            start: frame,
            end: frame,
        })
    }
}

// Write selected screenshots, video and audio in that order. Parent directories must
// already exist; a later write error can leave earlier output files in place.
fn flush_output_captures(capture: &OutputCaptureState, session: &DebugSession) -> Result<()> {
    if let Some(path) = &capture.screenshot_path {
        if capture.screenshot_range.is_some() {
            for ((frame, pixels), (_, rgb555)) in capture
                .captured_screenshots
                .iter()
                .zip(capture.captured_screenshot_rgb555.iter())
            {
                let output_path = screenshot_output_path(path, *frame, true)?;
                let framebuffer = pixels
                    .as_slice()
                    .try_into()
                    .map_err(|_| anyhow!("captured screenshot framebuffer had invalid length"))?;
                write_screenshot(
                    &output_path,
                    framebuffer,
                    Some(rgb555.as_slice()),
                    capture.screenshot_cgb_compat_mode,
                )
                .with_context(|| {
                    format!(
                        "failed to write screenshot for frame {}: {}",
                        frame, output_path
                    )
                })?;
            }
        } else {
            let output_path = screenshot_output_path(path, capture.executed_frames.max(1), false)?;
            write_screenshot(
                &output_path,
                session.machine.framebuffer(),
                Some(session.machine.framebuffer_rgb555()),
                session.machine.is_cgb_compat_mode(),
            )
            .with_context(|| format!("failed to write screenshot: {}", output_path))?;
        }
    }
    if let Some(path) = &capture.record_video_path {
        if capture.captured_video_frames.is_empty() {
            bail!("requested video recording captured no frames");
        }
        let output_path = video_output_path(path);
        write_video(&output_path, &capture.captured_video_frames)
            .with_context(|| format!("failed to write video recording: {}", output_path))?;
    }
    if let Some(path) = &capture.record_wav_path {
        let output_path = wav_output_path(path);
        write_wav(
            &output_path,
            session.machine.audio_sample_rate(),
            &capture.recorded_audio,
        )
        .with_context(|| format!("failed to write WAV recording: {}", output_path))?;
    }
    Ok(())
}

// Append .gif only when no extension exists; the writer validates explicit extensions later.
fn video_output_path(base: &str) -> String {
    let path = Path::new(base);
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());
    match ext.as_deref() {
        Some("gif") | Some("y4m") => base.to_string(),
        Some(_) => base.to_string(),
        None => format!("{base}.gif"),
    }
}

// Validate the image extension and optionally add a zero-padded frame suffix.
// Extensionless screenshot requests default to PNG.
fn screenshot_output_path(base: &str, frame: u64, force_sequence: bool) -> Result<String> {
    let base_path = Path::new(base);
    let ext = base_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());
    match ext.as_deref() {
        Some("bmp") | Some("ppm") | Some("png") => {
            if force_sequence {
                let stem = base_path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .ok_or_else(|| anyhow!("invalid screenshot path: {}", base))?;
                let ext = ext.expect("checked above");
                let mut filename = format!("{}_frame{:06}.{}", stem, frame, ext);
                if filename.is_empty() {
                    filename = format!("frame{:06}.{}", frame, ext);
                }
                Ok(base_path
                    .with_file_name(filename)
                    .to_string_lossy()
                    .into_owned())
            } else {
                Ok(base.to_string())
            }
        }
        None => {
            if force_sequence {
                Ok(format!("{}_frame{:06}.png", base, frame))
            } else {
                Ok(format!("{base}.png"))
            }
        }
        _ => bail!("unsupported screenshot format; use .png, .bmp, or .ppm"),
    }
}

// Append .wav when needed; an existing extension is preserved even though output is always WAV.
fn wav_output_path(base: &str) -> String {
    let path = Path::new(base);
    if path.extension().is_some() {
        base.to_string()
    } else {
        format!("{base}.wav")
    }
}

// Dispatch screenshot encoding by extension. Direct extensionless calls default to BMP;
// the CLI path helper normally supplies an explicit PNG extension first.
fn write_screenshot(
    path: &str,
    framebuffer: &[u8; 160 * 144],
    rgb555: Option<&[u16]>,
    cgb_compat_mode: bool,
) -> Result<()> {
    let ext = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_else(|| "bmp".to_string());
    match ext.as_str() {
        "bmp" => fs::write(path, encode_bmp(framebuffer, rgb555, cgb_compat_mode)?)
            .map_err(anyhow::Error::from),
        "ppm" => fs::write(path, encode_ppm(framebuffer, rgb555, cgb_compat_mode))
            .map_err(anyhow::Error::from),
        "png" => write_png(path, framebuffer, rgb555, cgb_compat_mode),
        _ => bail!("unsupported screenshot format; use .png, .bmp, or .ppm"),
    }
}

// Convert the frame to RGB and save a 160 by 144 image through the image library.
fn write_png(
    path: &str,
    framebuffer: &[u8; 160 * 144],
    rgb555: Option<&[u16]>,
    cgb_compat_mode: bool,
) -> Result<()> {
    let rgb = rgb_from_framebuffer(framebuffer, rgb555, cgb_compat_mode);
    let image: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_vec(160, 144, rgb)
        .ok_or_else(|| anyhow!("failed to construct 160x144 RGB image"))?;
    image
        .save(path)
        .with_context(|| format!("failed to write PNG screenshot: {path}"))?;
    Ok(())
}

// Encode the supplied interleaved samples and write the resulting WAV bytes to the given path.
fn write_wav(path: &str, sample_rate: u32, samples: &[i16]) -> Result<()> {
    fs::write(path, encode_wav(sample_rate, samples)).map_err(anyhow::Error::from)
}

// Encode the retained scalar frames as GIF or monochrome Y4M; other extensions are rejected.
fn write_video(path: &str, frames: &[(u64, Vec<u8>)]) -> Result<()> {
    let ext = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_else(|| "gif".to_string());
    match ext.as_str() {
        "gif" => fs::write(path, encode_gif(frames)?).map_err(anyhow::Error::from),
        "y4m" => fs::write(path, encode_y4m(frames)).map_err(anyhow::Error::from),
        _ => bail!("unsupported video format; use .gif or .y4m"),
    }
}

// Map four scalar shades to the fixed compatibility palette, clamping indices above three.
fn compat_palette_rgb(framebuffer: &[u8; 160 * 144]) -> Vec<u8> {
    const CGB_COMPAT: [(u8, u8, u8); 4] = [
        (255, 255, 214),
        (181, 230, 115),
        (82, 148, 65),
        (16, 49, 24),
    ];
    let mut rgb = Vec::with_capacity(framebuffer.len() * 3);
    for &shade in framebuffer {
        let (r, g, b) = CGB_COMPAT[usize::from(shade.min(3))];
        rgb.extend_from_slice(&[r, g, b]);
    }
    rgb
}

// Prefer the compatibility palette when selected, otherwise expand a complete RGB555
// frame to RGB888. Missing or incorrectly sized RGB555 data falls back to scalar grayscale.
fn rgb_from_framebuffer(
    framebuffer: &[u8; 160 * 144],
    rgb555: Option<&[u16]>,
    cgb_compat_mode: bool,
) -> Vec<u8> {
    if cgb_compat_mode {
        return compat_palette_rgb(framebuffer);
    }
    if let Some(rgb555) = rgb555.filter(|rgb555| rgb555.len() == 160 * 144) {
        let mut rgb = Vec::with_capacity(rgb555.len() * 3);
        for &value in rgb555 {
            let r = ((value & 0x1F) as u32 * 255 / 31) as u8;
            let g = (((value >> 5) & 0x1F) as u32 * 255 / 31) as u8;
            let b = (((value >> 10) & 0x1F) as u32 * 255 / 31) as u8;
            rgb.extend_from_slice(&[r, g, b]);
        }
        return rgb;
    }
    let max_value = framebuffer_intensity_max(framebuffer);
    let mut rgb = Vec::with_capacity(framebuffer.len() * 3);
    for &value in framebuffer {
        let gray = framebuffer_gray_to_u8(value, max_value);
        rgb.extend_from_slice(&[gray, gray, gray]);
    }
    rgb
}

// Emit a binary P6 image with a fixed 160 by 144 RGB payload and an 8-bit channel range.
fn encode_ppm(
    framebuffer: &[u8; 160 * 144],
    rgb555: Option<&[u16]>,
    cgb_compat_mode: bool,
) -> Vec<u8> {
    let rgb = rgb_from_framebuffer(framebuffer, rgb555, cgb_compat_mode);
    let mut out = Vec::with_capacity(32 + rgb.len());
    out.extend_from_slice(b"P6\n160 144\n255\n");
    out.extend_from_slice(&rgb);
    out
}

// Build an uncompressed 24-bit BMP with checked header sizes. Store rows bottom-up
// and channels in BGR order, padding each row to a four-byte boundary.
fn encode_bmp(
    framebuffer: &[u8; 160 * 144],
    rgb555: Option<&[u16]>,
    cgb_compat_mode: bool,
) -> Result<Vec<u8>> {
    const WIDTH: usize = 160;
    const HEIGHT: usize = 144;
    const HEADER_LEN: usize = 14 + 40;
    let row_stride = (WIDTH * 3 + 3) & !3;
    let pixel_bytes = row_stride * HEIGHT;
    let file_size = HEADER_LEN
        .checked_add(pixel_bytes)
        .ok_or_else(|| anyhow!("BMP size overflow"))?;
    let file_size_u32 = u32::try_from(file_size).map_err(|_| anyhow!("BMP too large"))?;
    let pixel_bytes_u32 = u32::try_from(pixel_bytes).map_err(|_| anyhow!("BMP too large"))?;
    let width_i32 = i32::try_from(WIDTH).map_err(|_| anyhow!("invalid BMP width"))?;
    let height_i32 = i32::try_from(HEIGHT).map_err(|_| anyhow!("invalid BMP height"))?;

    let mut out = Vec::with_capacity(file_size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&file_size_u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(HEADER_LEN as u32).to_le_bytes());

    out.extend_from_slice(&(40u32).to_le_bytes());
    out.extend_from_slice(&width_i32.to_le_bytes());
    out.extend_from_slice(&height_i32.to_le_bytes());
    out.extend_from_slice(&(1u16).to_le_bytes());
    out.extend_from_slice(&(24u16).to_le_bytes());
    out.extend_from_slice(&(0u32).to_le_bytes());
    out.extend_from_slice(&pixel_bytes_u32.to_le_bytes());
    out.extend_from_slice(&(2835u32).to_le_bytes());
    out.extend_from_slice(&(2835u32).to_le_bytes());
    out.extend_from_slice(&(0u32).to_le_bytes());
    out.extend_from_slice(&(0u32).to_le_bytes());

    let rgb = rgb_from_framebuffer(framebuffer, rgb555, cgb_compat_mode);
    let padding = row_stride - WIDTH * 3;
    for y in (0..HEIGHT).rev() {
        let base = y * WIDTH;
        for x in 0..WIDTH {
            let idx = (base + x) * 3;
            out.extend_from_slice(&[rgb[idx + 2], rgb[idx + 1], rgb[idx]]);
        }
        out.extend(std::iter::repeat_n(0u8, padding));
    }
    Ok(out)
}

// Emit a PCM stereo, 16-bit little-endian RIFF/WAVE stream. The caller supplies the
// sample rate and complete interleaved stereo frames; this helper does not validate them
// or support RF64, and oversized 32-bit length fields saturate rather than returning an error.
fn encode_wav(sample_rate: u32, samples: &[i16]) -> Vec<u8> {
    let data_len = samples.len() * std::mem::size_of::<i16>();
    let riff_len = 36usize + data_len;
    let data_len_u32 = u32::try_from(data_len).unwrap_or(u32::MAX);
    let riff_len_u32 = u32::try_from(riff_len).unwrap_or(u32::MAX);
    let byte_rate = sample_rate.saturating_mul(4);
    let mut out = Vec::with_capacity(44 + data_len);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_len_u32.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&(16u32).to_le_bytes());
    out.extend_from_slice(&(1u16).to_le_bytes());
    out.extend_from_slice(&(2u16).to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&(4u16).to_le_bytes());
    out.extend_from_slice(&(16u16).to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len_u32.to_le_bytes());
    for &sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

// Encode grayscale frames with an infinite loop and centisecond delays approximating
// 59.727 frames per second. Stored frame numbers are ignored; input order determines playback.
fn encode_gif(frames: &[(u64, Vec<u8>)]) -> Result<Vec<u8>> {
    const WIDTH: u16 = 160;
    const HEIGHT: u16 = 144;
    const FPS_NUM: u32 = 59_727;
    const FPS_DEN: u32 = 1_000;

    let mut palette = Vec::with_capacity(256 * 3);
    for value in 0u16..=255 {
        let gray = value as u8;
        palette.extend_from_slice(&[gray, gray, gray]);
    }

    let mut out = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut out, WIDTH, HEIGHT, &palette)
            .map_err(|err| anyhow!("GIF encoder setup failed: {err}"))?;
        encoder
            .set_repeat(Repeat::Infinite)
            .map_err(|err| anyhow!("GIF repeat setup failed: {err}"))?;
        let mut delay_remainder = 0u32;
        for (_, pixels) in frames {
            let framebuffer = pixels
                .as_slice()
                .try_into()
                .map_err(|_| anyhow!("captured video framebuffer had invalid length"))?;
            let mut frame = GifFrame::default();
            frame.width = WIDTH;
            frame.height = HEIGHT;
            frame.buffer = Cow::Owned(framebuffer_to_grayscale_indices(framebuffer));
            // Carry the fractional centisecond remainder between frames to avoid a constant-delay timing bias.
            delay_remainder = delay_remainder.saturating_add(100 * FPS_DEN);
            let mut delay = delay_remainder / FPS_NUM;
            delay_remainder %= FPS_NUM;
            if delay == 0 {
                delay = 1;
            }
            frame.delay = delay as u16;
            encoder
                .write_frame(&frame)
                .map_err(|err| anyhow!("GIF frame encode failed: {err}"))?;
        }
    }
    Ok(out)
}

// Emit a monochrome 160 by 144 Y4M sequence at 59727/1000 frames per second.
// Stored frame numbers are ignored; malformed buffer lengths panic under the internal capture invariant.
fn encode_y4m(frames: &[(u64, Vec<u8>)]) -> Vec<u8> {
    const WIDTH: usize = 160;
    const HEIGHT: usize = 144;
    let mut out = Vec::with_capacity(64 + frames.len() * (6 + WIDTH * HEIGHT));
    out.extend_from_slice(b"YUV4MPEG2 W160 H144 F59727:1000 Ip A1:1 Cmono\n");
    for (_, pixels) in frames {
        let framebuffer: &[u8; WIDTH * HEIGHT] = pixels
            .as_slice()
            .try_into()
            .expect("framebuffer size validated before encode");
        out.extend_from_slice(b"FRAME\n");
        out.extend_from_slice(&framebuffer_to_grayscale_indices(framebuffer));
    }
    out
}

// Choose the scalar range from frame contents and map every shade to an 8-bit grayscale index.
fn framebuffer_to_grayscale_indices(framebuffer: &[u8; 160 * 144]) -> Vec<u8> {
    let max_value = framebuffer_intensity_max(framebuffer);
    framebuffer
        .iter()
        .copied()
        .map(|value| framebuffer_gray_to_u8(value, max_value))
        .collect()
}

// Treat a frame containing any value above three as a 0..31 scalar frame; otherwise use 0..3.
// This is a content-based choice rather than an explicit hardware-mode tag.
fn framebuffer_intensity_max(framebuffer: &[u8; 160 * 144]) -> u32 {
    if framebuffer.iter().copied().any(|pixel| pixel > 3) {
        31
    } else {
        3
    }
}

// Clamp to the chosen shade range, scale with rounding and invert so shade zero becomes white.
fn framebuffer_gray_to_u8(value: u8, max_value: u32) -> u8 {
    let shade = u32::from(value).min(max_value);
    let scaled = (shade * 255 + (max_value / 2)) / max_value.max(1);
    255u8.saturating_sub(scaled as u8)
}

// Build a case-folded kind=value comparison label without trimming either input.
fn suggestion_label(kind: &str, value: &str) -> String {
    format!(
        "{}={}",
        kind.to_ascii_lowercase(),
        value.to_ascii_lowercase()
    )
}

// Convert all supplied expectations to labels; stage selection is handled by the caller.
fn expected_suggestions_to_labels(expected: &[ExpectedSuggestion]) -> Vec<String> {
    expected
        .iter()
        .map(|entry| suggestion_label(&entry.kind, &entry.value))
        .collect()
}

// Trim and deduplicate nonempty diagnostic codes while preserving their case.
fn normalize_diagnostic_expectations(values: &[String]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

// Remove hexadecimal prefixes, lowercase, sort and deduplicate opcode labels.
// This is text normalization; it does not parse byte values or add leading zeroes.
fn normalize_opcode_expectations(values: &[String]) -> Vec<String> {
    let mut out = values
        .iter()
        .map(|value| {
            value
                .trim()
                .trim_start_matches("0x")
                .trim_start_matches("0X")
                .to_ascii_lowercase()
        })
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}

// Resolve event aliases and return sorted unique nonempty labels.
fn normalize_event_expectations(values: &[String]) -> Vec<String> {
    let mut out = values
        .iter()
        .map(|value| normalize_event_type_label(value))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}

// Merge aliases under canonical labels, keeping the greatest minimum count for each event.
fn normalize_event_count_floor_expectations(values: &BTreeMap<String, u64>) -> Vec<(String, u64)> {
    let mut merged = BTreeMap::new();
    for (event_type, floor) in values {
        let key = normalize_event_type_label(event_type);
        if key.is_empty() {
            continue;
        }
        let entry = merged.entry(key).or_insert(0);
        *entry = (*entry).max(*floor);
    }
    merged.into_iter().collect()
}

// Render a canonical event label and its required minimum count for regression output.
fn format_event_count_floor_label(event_type: &str, floor: u64) -> String {
    format!("{}>={}", event_type, floor)
}

// Normalize and merge count requirements before formatting the expectation list.
fn expected_event_count_floor_to_labels(values: &BTreeMap<String, u64>) -> Vec<String> {
    normalize_event_count_floor_expectations(values)
        .into_iter()
        .map(|(event_type, floor)| format_event_count_floor_label(&event_type, floor))
        .collect()
}

// Trim and lowercase watch names, then sort and deduplicate nonempty entries.
fn normalize_watch_name_expectations(values: &[String]) -> Vec<String> {
    let mut out = values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}

// Map each event variant to a regression label. OAM and serial start/completion
// events share labels, while HDMA versus GDMA block/completion labels depend on mode.
fn normalize_event_type_from_debug_event(event: &DebugEvent) -> String {
    match event {
        DebugEvent::ScanlineAdvance { .. } => "scanline".to_string(),
        DebugEvent::PpuModeChange { .. } => "ppu_mode".to_string(),
        DebugEvent::ScanlineRender { .. } => "scanline_render".to_string(),
        DebugEvent::OamDmaStart { .. } | DebugEvent::OamDmaComplete { .. } => "oam_dma".to_string(),
        DebugEvent::GdmaStallEstimate { .. } => "gdma_stall".to_string(),
        DebugEvent::HdmaStart { .. } => "hdma_start".to_string(),
        DebugEvent::HdmaBlock { hblank_mode, .. } => {
            if *hblank_mode {
                "hdma_block".to_string()
            } else {
                "gdma_block".to_string()
            }
        }
        DebugEvent::HdmaComplete { hblank_mode, .. } => {
            if *hblank_mode {
                "hdma_complete".to_string()
            } else {
                "gdma_complete".to_string()
            }
        }
        DebugEvent::HdmaCancel { .. } => "hdma_cancel".to_string(),
        DebugEvent::HdmaDeferred { .. } => "hdma_deferred".to_string(),
        DebugEvent::HdmaWriteIgnored { .. } => "hdma_write_ignored".to_string(),
        DebugEvent::MapperControlWrite { .. } => "mapper_ctrl".to_string(),
        DebugEvent::MapperRomBankChange { .. } => "mapper_rom_bank".to_string(),
        DebugEvent::MapperRamBankChange { .. } => "mapper_ram_bank".to_string(),
        DebugEvent::ApuMasterToggle { .. } => "apu_master".to_string(),
        DebugEvent::ApuChannelTrigger { .. } => "apu_trigger".to_string(),
        DebugEvent::ApuChannelLengthExpired { .. } => "apu_length".to_string(),
        DebugEvent::ApuEnvelopeStep { .. } => "apu_env".to_string(),
        DebugEvent::ApuSweepStep { .. } => "apu_sweep".to_string(),
        DebugEvent::ApuChannelDisabled { .. } => "apu_disable".to_string(),
        DebugEvent::ApuDacStateChange { .. } => "apu_dac".to_string(),
        DebugEvent::ApuPopRisk { .. } => "apu_pop".to_string(),
        DebugEvent::ApuFrameSequencerStep { .. } => "apu_frame".to_string(),
        DebugEvent::ApuMixerControl { .. } => "apu_mix_ctrl".to_string(),
        DebugEvent::ApuMixedOutput { .. } => "apu_mix".to_string(),
        DebugEvent::ApuPcmFramesBuffered { .. } => "apu_pcm".to_string(),
        DebugEvent::ApuPcmBufferWrapped { .. } => "apu_drop".to_string(),
        DebugEvent::ApuWaveRamWrite { .. } => "wave_ram".to_string(),
        DebugEvent::ApuWaveRamAccessAliased { .. } => "apu_wave_alias".to_string(),
        DebugEvent::ApuCh3TriggerRetainsSample { .. } => "ch3_hold".to_string(),
        DebugEvent::ApuNoiseClockFrozen { .. } => "noise_lock".to_string(),
        DebugEvent::CgbModeSelected { .. } => "cgb_mode".to_string(),
        DebugEvent::CgbVramBankSwitch { .. } => "cgb_vram_bank".to_string(),
        DebugEvent::CgbWramBankSwitch { .. } => "cgb_wram_bank".to_string(),
        DebugEvent::CgbBgPaletteIndexWrite { .. } => "cgb_bgpi".to_string(),
        DebugEvent::CgbBgPaletteDataWrite { .. } => "cgb_bgpd".to_string(),
        DebugEvent::CgbObjPaletteIndexWrite { .. } => "cgb_obpi".to_string(),
        DebugEvent::CgbObjPaletteDataWrite { .. } => "cgb_obpd".to_string(),
        DebugEvent::CgbKey1Write { .. } => "key1".to_string(),
        DebugEvent::CgbSpeedSwitch { .. } => "speed_switch".to_string(),
        DebugEvent::CgbSpeedSwitchFreeze { .. } => "speed_freeze".to_string(),
        DebugEvent::LcdToggle { .. } => "lcd_toggle".to_string(),
        DebugEvent::StatWrite { .. } => "stat_write".to_string(),
        DebugEvent::LycWrite { .. } => "lyc_write".to_string(),
        DebugEvent::StatSignal { .. } => "stat".to_string(),
        DebugEvent::VblankEnter { .. } => "vblank".to_string(),
        DebugEvent::FrameComplete { .. } => "frame".to_string(),
        DebugEvent::BankSwitch { .. } => "bank".to_string(),
        DebugEvent::FarCallSuspected { .. } => "farcall".to_string(),
        DebugEvent::KitaqgbIntrinsic { .. } => "intrinsic".to_string(),
        DebugEvent::BankReturnMissing { .. } => "bank_return_missing".to_string(),
        DebugEvent::TimerInterrupt { .. } => "timer".to_string(),
        DebugEvent::TimerOverflow { .. } => "timer_overflow".to_string(),
        DebugEvent::TimerReload { .. } => "timer_reload".to_string(),
        DebugEvent::TimerControlWrite { .. } => "timer_ctrl".to_string(),
        DebugEvent::DivResetEdge { .. } => "div_reset".to_string(),
        DebugEvent::SerialTransferStart { .. } | DebugEvent::SerialTransferComplete { .. } => {
            "serial".to_string()
        }
        DebugEvent::JoypadEdge { .. } => "joypad_edge".to_string(),
        DebugEvent::JoypadRead { .. } => "joypad_read".to_string(),
        DebugEvent::JoypadSelectionWrite { .. } => "joypad_select".to_string(),
        DebugEvent::JoypadInterrupt { .. } => "joypad_irq".to_string(),
        DebugEvent::InterruptRequested { .. } => "irq_request".to_string(),
        DebugEvent::InterruptServiced { .. } => "irq_service".to_string(),
        DebugEvent::InterruptPendingBlocked { .. } => "irq_blocked".to_string(),
        DebugEvent::ExecutionStop { .. } => "stop".to_string(),
        DebugEvent::SymbolContextChange { .. } => "symbol_context".to_string(),
        DebugEvent::UnsupportedOpcode { .. } => "unsupported".to_string(),
        DebugEvent::ReplayCheckpointSaved { .. } => "replay_checkpoint".to_string(),
        DebugEvent::ReplayRewindApplied { .. } => "replay_rewind".to_string(),
        DebugEvent::ReplayDivergenceDetected { .. } => "replay_divergence".to_string(),
        DebugEvent::BankThrashSuspected { .. } => "thrash".to_string(),
    }
}

// Resolve supported case-insensitive aliases to regression labels; retain unknown
// trimmed lowercase labels. This exact label mapping differs from event_matches substring filters.
fn normalize_event_type_label(value: &str) -> String {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "scanlineadvance" | "scanline_advance" | "scanline" => "scanline".to_string(),
        "ppumodechange" | "ppu_mode_change" | "ppumode" | "mode" | "ppu_mode" => {
            "ppu_mode".to_string()
        }
        "scanlinerender" | "scanline_render" | "render" => "scanline_render".to_string(),
        "oamdma" | "oam_dma" | "dma" => "oam_dma".to_string(),
        "gdmastall" | "gdma_stall" | "dma_stall" => "gdma_stall".to_string(),
        "hdmastart" | "hdma_start" => "hdma_start".to_string(),
        "hdmablock" | "hdma_block" => "hdma_block".to_string(),
        "gdmablock" | "gdma_block" => "gdma_block".to_string(),
        "hdmacomplete" | "hdma_complete" => "hdma_complete".to_string(),
        "gdmacomplete" | "gdma_complete" => "gdma_complete".to_string(),
        "hdmacancel" | "hdma_cancel" | "dma_cancel" => "hdma_cancel".to_string(),
        "hdmadeferred" | "hdma_deferred" | "dma_deferred" => "hdma_deferred".to_string(),
        "hdmawriteignored" | "hdma_write_ignored" | "dma_ignored" => {
            "hdma_write_ignored".to_string()
        }
        "mappercontrolwrite" | "mapper_control_write" | "mapperctrl" | "mapper_ctrl" | "mapper" => {
            "mapper_ctrl".to_string()
        }
        "mapperrombankchange" | "mapper_rom_bank_change" | "mapperrombank" | "mapper_rom_bank" => {
            "mapper_rom_bank".to_string()
        }
        "mapperrambankchange" | "mapper_ram_bank_change" | "mapperrambank" | "mapper_ram_bank" => {
            "mapper_ram_bank".to_string()
        }
        "apumaster" | "apu_master" | "nr52" => "apu_master".to_string(),
        "aputrigger" | "apu_trigger" | "channel_trigger" => "apu_trigger".to_string(),
        "apuframe" | "apu_frame" | "frame_seq" | "framesequencer" => "apu_frame".to_string(),
        "apupcm" | "apu_pcm" | "pcm" | "audio_buffer" => "apu_pcm".to_string(),
        "apudrop" | "apu_drop" | "pcm_drop" | "audio_drop" => "apu_drop".to_string(),
        "apudac" | "apu_dac" | "dac" => "apu_dac".to_string(),
        "apupop" | "apu_pop" | "pop" => "apu_pop".to_string(),
        "cgbmode" | "cgb_mode" | "cgb" => "cgb_mode".to_string(),
        "cgbvrambank" | "cgb_vram_bank" | "vbk" => "cgb_vram_bank".to_string(),
        "cgbwrambank" | "cgb_wram_bank" | "svbk" => "cgb_wram_bank".to_string(),
        "cgbbgpi" | "cgb_bgpi" | "bgpi" => "cgb_bgpi".to_string(),
        "cgbbgpd" | "cgb_bgpd" | "bgpd" | "cgbpalette" | "cgb_palette" => "cgb_bgpd".to_string(),
        "cgbobpi" | "cgb_obpi" | "obpi" => "cgb_obpi".to_string(),
        "cgbobpd" | "cgb_obpd" | "obpd" => "cgb_obpd".to_string(),
        "key1" | "speedarm" | "speed_arm" | "cgbspeed" | "cgb_speed" => "key1".to_string(),
        "speedswitch" | "speed_switch" | "double_speed" => "speed_switch".to_string(),
        "speedfreeze" | "speed_freeze" | "stopfreeze" | "stop_freeze" => "speed_freeze".to_string(),
        "waveram" | "wave_ram" | "apu_wave" => "wave_ram".to_string(),
        "apuwavealias" | "apu_wave_alias" | "wavealias" | "wave_alias" => {
            "apu_wave_alias".to_string()
        }
        "ch3hold" | "ch3_hold" | "samplehold" | "sample_hold" => "ch3_hold".to_string(),
        "noiselock" | "noise_lock" | "noisefreeze" | "noise_freeze" => "noise_lock".to_string(),
        "lcdtoggle" | "lcd_toggle" | "lcdc_toggle" | "lcd" => "lcd_toggle".to_string(),
        "statwrite" | "stat_write" | "statcfg" => "stat_write".to_string(),
        "lycwrite" | "lyc_write" | "lyc" => "lyc_write".to_string(),
        "statsignal" | "stat_signal" | "stat" | "stat_irq" => "stat".to_string(),
        "vblankenter" | "vblank_enter" | "vblank" => "vblank".to_string(),
        "framecomplete" | "frame_complete" | "frame" => "frame".to_string(),
        "bankswitch" | "bank_switch" | "bank" => "bank".to_string(),
        "farcallsuspected" | "far_call_suspected" | "farcall" | "far_call" => "farcall".to_string(),
        "kitaqgbintrinsic" | "intrinsic" => "intrinsic".to_string(),
        "bankreturnmissing" | "bank_return_missing" | "returnmissing" => {
            "bank_return_missing".to_string()
        }
        "timerinterrupt" | "timer_interrupt" | "timer" => "timer".to_string(),
        "timeroverflow" | "timer_overflow" | "overflow" => "timer_overflow".to_string(),
        "timerreload" | "timer_reload" | "reload" => "timer_reload".to_string(),
        "timerctrl" | "timer_ctrl" | "tac" => "timer_ctrl".to_string(),
        "divreset" | "div_reset" | "div" => "div_reset".to_string(),
        "serialtransfer" | "serial_transfer" | "serial" => "serial".to_string(),
        "joypadedge" | "joypad_edge" | "input_edge" => "joypad_edge".to_string(),
        "joypadread" | "joypad_read" | "p1_read" => "joypad_read".to_string(),
        "joypadselect" | "joypad_select" | "p1_write" => "joypad_select".to_string(),
        "joypadirq" | "joypad_interrupt" | "joypad_irq" => "joypad_irq".to_string(),
        "irqrequest" | "irq_request" => "irq_request".to_string(),
        "irqservice" | "irq_service" | "interrupt_service" => "irq_service".to_string(),
        "irqblocked" | "irq_blocked" | "interrupt_blocked" => "irq_blocked".to_string(),
        "executionstop" | "execution_stop" | "stop" | "breakpoint" | "watchpoint" => {
            "stop".to_string()
        }
        "symbolcontextchange"
        | "symbol_context_change"
        | "symbolcontext"
        | "symbol_context"
        | "sourcecontext"
        | "source_context"
        | "source"
        | "symbol" => "symbol_context".to_string(),
        "unsupportedopcode" | "unsupported_opcode" | "unsupported" | "opcode" => {
            "unsupported".to_string()
        }
        "bankthrashsuspected" | "bank_thrash_suspected" | "thrash" => "thrash".to_string(),
        _ => normalized,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use kokura_bridge::{DecompileArtifact, DecompileFunctionInfo, DecompileReport};
    use kokura_core::Machine;
    use kokura_debug::report::{ReplayCheckpointReport, ReplayReport, ReplaySliceReport};
    use kokura_debug::{DebugSession, ReplayControlSet, StopConditionSet};
    use serde_json::json;

    use super::{
        build_link_runner_layout, build_replay_mismatch_snapshot_plan, build_replay_tape,
        classify_diff_severity, compare_replay_reports, encode_gif, encode_wav, encode_y4m,
        load_toolchain_build_report_file, normalize_event_count_floor_expectations,
        normalize_event_expectations, normalize_event_type_label,
        normalize_inline_link_session_key, normalize_link_topology_label,
        normalize_opcode_expectations, normalize_watch_name_expectations, parse_breakpoint_spec,
        parse_dma_stop_spec, parse_frame_range_spec, parse_inline_link_session_spec,
        parse_interrupt_stop_spec, parse_mmio_stop_spec, parse_observation_condition,
        parse_recommended_stop_specs, parse_recommended_watch_windows, parse_u16_value,
        parse_watch_window_spec, parse_watchpoint_spec, render_decompile_markdown,
        screenshot_output_path, suggestion_label, video_output_path, LoadedLinkJob,
        LoadedLinkSession, OutputFilterOptions, SessionInputProgram,
    };

    // Build a synthetic session description with a physical slot and default controls; no ROM is opened.
    fn test_link_session(name: &str, slot: u8) -> LoadedLinkSession {
        LoadedLinkSession {
            name: Some(name.to_string()),
            slot: Some(slot),
            rom_path: format!("{name}.gb"),
            symbols_path: None,
            source_map_path: None,
            toolchain_metadata_path: None,
            load_state: None,
            save_state: None,
            input: None,
            input_sequence: None,
            audio_buffer_frames: None,
            watch_windows: Vec::new(),
            debugger: StopConditionSet::default(),
            replay: ReplayControlSet::default(),
        }
    }

    #[test]
    // Check equivalent decimal and explicitly prefixed hexadecimal inputs.
    fn parse_u16_value_accepts_hex_and_decimal() {
        assert_eq!(parse_u16_value("255").unwrap(), 255);
        assert_eq!(parse_u16_value("0x00FF").unwrap(), 255);
    }

    #[test]
    // Check the parsed name, address and size of one WRAM watch.
    fn parse_watch_window_spec_accepts_hex_window() {
        let spec = parse_watch_window_spec("wram:0xC000:0x40").unwrap();
        assert_eq!(spec.name, "wram");
        assert_eq!(spec.addr, 0xC000);
        assert_eq!(spec.size, 0x40);
    }

    #[test]
    // Check that a missing size reports the expected watch-window syntax.
    fn parse_watch_window_spec_rejects_missing_parts() {
        let err = parse_watch_window_spec("wram:0xC000")
            .unwrap_err()
            .to_string();
        assert!(err.contains("NAME:ADDR:SIZE"));
    }

    #[test]
    // Filter a synthetic linked report, retaining runner totals and session metadata
    // while removing unrequested CPU, video and event sections.
    fn link_session_reports_honor_minimal_section_filtering() {
        let mut value = json!({
            "runner_summary": {"exchange_count": 3},
            "sessions": [{
                "report": {
                    "schema_version": "1",
                    "meta": {},
                    "cpu": {},
                    "video": {},
                    "summary": {"serial_interrupt_count": 3},
                    "events": [1, 2, 3]
                }
            }]
        });
        let filters = OutputFilterOptions {
            report_sections: Some(BTreeSet::from(["summary".to_string()])),
            watch_fields: None,
        };

        super::filter_output_value(&mut value, &filters);

        let report = &value["sessions"][0]["report"];
        assert_eq!(value["runner_summary"]["exchange_count"], 3);
        assert_eq!(report["summary"]["serial_interrupt_count"], 3);
        assert!(report.get("events").is_none());
        assert!(report.get("cpu").is_none());
        assert!(report.get("video").is_none());
        assert!(report.get("meta").is_some());
    }

    #[test]
    // Check the input-mask and DMA diagnostic mappings remain distinct.
    fn diagnostic_mapping_keeps_joypad_mask_out_of_dma_bucket() {
        assert_eq!(
            super::map_kokura_diagnostic_code_to_sarakura("InputReadWithoutJoypadMask"),
            "INPUT_POLL_JITTER"
        );
        assert_eq!(
            super::map_kokura_diagnostic_code_to_sarakura("DmaOrHdmaConflict"),
            "DMA_OR_HDMA_CONFLICT"
        );
    }

    #[test]
    // Check representative scores in each severity class, without calibrating real fault likelihood.
    fn diff_severity_classification_is_stable() {
        assert_eq!(classify_diff_severity(0), "none");
        assert_eq!(classify_diff_severity(8), "low");
        assert_eq!(classify_diff_severity(16), "medium");
        assert_eq!(classify_diff_severity(32), "high");
        assert_eq!(classify_diff_severity(64), "critical");
    }

    #[test]
    // Check that case and hexadecimal-prefix variants collapse to one opcode label.
    fn opcode_expectations_normalize_hex_prefix() {
        assert_eq!(
            normalize_opcode_expectations(&["0xF4".into(), "f4".into()]),
            vec!["f4"]
        );
    }

    #[test]
    // Check case folding of both parts of a suggestion comparison label.
    fn suggestion_labels_are_case_insensitive() {
        assert_eq!(suggestion_label("Event", "VBLANK"), "event=vblank");
    }

    #[test]
    // Check selected PPU, bank and VBlank aliases and duplicate elimination.
    fn event_expectations_normalize_aliases() {
        assert_eq!(normalize_event_type_label("PpuModeChange"), "ppu_mode");
        assert_eq!(normalize_event_type_label("bank_switch"), "bank");
        assert_eq!(
            normalize_event_expectations(&["VBLANK".into(), "vblank_enter".into()]),
            vec!["vblank"]
        );
    }

    #[test]
    // Check that alias collisions preserve the largest requested floor and sorted output.
    fn event_count_floor_expectations_merge_aliases() {
        let mut values = BTreeMap::new();
        values.insert("VBLANK".to_string(), 1);
        values.insert("vblank_enter".to_string(), 3);
        values.insert("bank_switch".to_string(), 2);
        assert_eq!(
            normalize_event_count_floor_expectations(&values),
            vec![("bank".to_string(), 2), ("vblank".to_string(), 3)]
        );
    }

    #[test]
    // Check case variants of one watch name collapse to a single expectation.
    fn watch_name_expectations_normalize_case() {
        assert_eq!(
            normalize_watch_name_expectations(&["Board_OAM".into(), "board_oam".into()]),
            vec!["board_oam"]
        );
    }

    #[test]
    // Check the render, LCD toggle and LYC labels used by regression expectations.
    fn event_expectations_cover_ppu_timing_aliases() {
        assert_eq!(
            normalize_event_type_label("scanline_render"),
            "scanline_render"
        );
        assert_eq!(normalize_event_type_label("lcd"), "lcd_toggle");
        assert_eq!(normalize_event_type_label("lyc"), "lyc_write");
    }

    #[test]
    // Check symbol and bank extraction from one execution-breakpoint specification.
    fn breakpoint_spec_accepts_symbol_and_bank() {
        let spec = parse_breakpoint_spec("symbol:MainLoop@bank:1").unwrap();
        assert_eq!(spec.symbol.as_deref(), Some("MainLoop"));
        assert_eq!(spec.bank, Some(1));
    }

    #[test]
    // Check parsing of a three-term observation condition; no ROM or symbol resolution runs here.
    fn observation_condition_accepts_frame_ly_and_symbol_terms() {
        let condition =
            parse_observation_condition("frame=120&&ly=42&&symbol=DrawFrame")
                .unwrap();
        assert_eq!(condition.terms.len(), 3);
    }

    #[test]
    // Make both machine and watch digests differ, then check watch-only comparison reports the watch mismatch.
    fn compare_replay_reports_watch_only_prefers_watch_digest() {
        let mut reference = sample_replay_report(0xAAAA_BBBB_CCCC_DDDD);
        let mut actual = sample_replay_report(0x1111_2222_3333_4444);
        reference.checkpoints[0].digest = 0x1111;
        actual.checkpoints[0].digest = 0x2222;
        reference.checkpoints[0].watch_digest = 0x3333;
        actual.checkpoints[0].watch_digest = 0x4444;
        let comparison = compare_replay_reports(&reference, &actual, None, true);
        assert_eq!(
            comparison
                .first_mismatch
                .as_ref()
                .map(|mismatch| mismatch.kind.as_str()),
            Some("checkpoint_watch_digest")
        );
    }

    #[test]
    // Check the address and length of a named OAM watchpoint.
    fn watchpoint_spec_accepts_range() {
        let spec = parse_watchpoint_spec("oam@0xFE00+0x00A0").unwrap();
        assert_eq!(spec.addr, 0xFE00);
        assert_eq!(spec.size, 0x00A0);
    }

    #[test]
    // Check acceptance of an MMIO address and rejection of a WRAM address.
    fn mmio_stop_spec_requires_ff_range() {
        assert!(parse_mmio_stop_spec("0xFF46").is_ok());
        assert!(parse_mmio_stop_spec("0xC000").is_err());
    }

    #[test]
    // Check successful parsing of a source:phase pair and its source field; the phase is not asserted separately.
    fn interrupt_stop_spec_accepts_source_and_phase() {
        let spec = parse_interrupt_stop_spec("timer:requested").unwrap();
        assert_eq!(spec.source.as_deref(), Some("timer"));
    }

    #[test]
    // Check acceptance and retention of the OAM-start stop label.
    fn dma_stop_spec_accepts_known_event() {
        let spec = parse_dma_stop_spec("oam_start").unwrap();
        assert_eq!(spec.event, "oam_start");
    }

    #[test]
    // Check canonical DMA, mapper and APU labels plus cancellation and ignored-write aliases.
    fn event_expectations_cover_dma_aliases() {
        assert_eq!(normalize_event_type_label("oam_dma"), "oam_dma");
        assert_eq!(normalize_event_type_label("gdma_stall"), "gdma_stall");
        assert_eq!(normalize_event_type_label("mapper_ctrl"), "mapper_ctrl");
        assert_eq!(normalize_event_type_label("apu_master"), "apu_master");
        assert_eq!(normalize_event_type_label("hdma_start"), "hdma_start");
        assert_eq!(normalize_event_type_label("hdma_block"), "hdma_block");
        assert_eq!(normalize_event_type_label("gdma_block"), "gdma_block");
        assert_eq!(normalize_event_type_label("hdma_complete"), "hdma_complete");
        assert_eq!(normalize_event_type_label("gdma_complete"), "gdma_complete");
        assert_eq!(normalize_event_type_label("dma_cancel"), "hdma_cancel");
        assert_eq!(normalize_event_type_label("hdma_deferred"), "hdma_deferred");
        assert_eq!(
            normalize_event_type_label("dma_ignored"),
            "hdma_write_ignored"
        );
    }

    #[test]
    // Check inclusive frame parsing and rejection of zero or reversed bounds.
    fn frame_range_spec_accepts_single_and_range_forms() {
        assert_eq!(
            parse_frame_range_spec("12").unwrap(),
            super::FrameRange { start: 12, end: 12 }
        );
        assert_eq!(
            parse_frame_range_spec("7:19").unwrap(),
            super::FrameRange { start: 7, end: 19 }
        );
        assert!(parse_frame_range_spec("0").is_err());
        assert!(parse_frame_range_spec("5:4").is_err());
    }

    #[test]
    // Check a numbered BMP path and the extensionless PNG default using Windows-style paths.
    fn screenshot_output_path_uses_sequence_names_for_ranges() {
        assert_eq!(
            screenshot_output_path("captures\\title.bmp", 42, true).unwrap(),
            "captures\\title_frame000042.bmp"
        );
        assert_eq!(
            screenshot_output_path("captures\\title", 42, false).unwrap(),
            "captures\\title.png"
        );
    }

    #[test]
    // Check the GIF default and preservation of an explicit Y4M extension.
    fn video_output_path_defaults_to_gif() {
        assert_eq!(video_output_path("captures\\title"), "captures\\title.gif");
        assert_eq!(
            video_output_path("captures\\title.y4m"),
            "captures\\title.y4m"
        );
    }

    #[test]
    // Advance a finite input sequence and check it returns to the configured fixed START mask.
    fn session_input_program_replays_sequence_then_fallback_mask() {
        let session = LoadedLinkSession {
            name: None,
            slot: None,
            rom_path: "demo.gb".to_string(),
            symbols_path: None,
            source_map_path: None,
            toolchain_metadata_path: None,
            load_state: None,
            save_state: None,
            input: Some("START".to_string()),
            input_sequence: Some("RIGHT:2;A:1".to_string()),
            audio_buffer_frames: None,
            watch_windows: Vec::new(),
            debugger: StopConditionSet::default(),
            replay: ReplayControlSet::default(),
        };
        let mut program = SessionInputProgram::from_session(&session).expect("input program");
        assert_eq!(program.next_mask(), 0x01);
        assert_eq!(program.next_mask(), 0x01);
        assert_eq!(program.next_mask(), 0x10);
        assert_eq!(program.next_mask(), 0x80);
    }

    #[test]
    // Check hyphenated option keys normalize to underscore-based field names.
    fn inline_link_session_key_normalization_supports_hyphen_aliases() {
        assert_eq!(
            normalize_inline_link_session_key("input-seq"),
            "input_seq".to_string()
        );
        assert_eq!(
            normalize_inline_link_session_key("toolchain-metadata"),
            "toolchain_metadata".to_string()
        );
    }

    #[test]
    // Parse an inline Windows-style session description and check paths, sequence,
    // audio capacity and both watches. This test does not access the named files.
    fn inline_link_session_spec_parses_paths_and_watch_windows() {
        let base = std::path::Path::new("C:\\kitaqgb_project\\kokura");
        let session = parse_inline_link_session_spec(
            "name=host|slot=0|rom=..\\tmp\\host.gb|symbols=..\\tmp\\host.map|toolchain-metadata=..\\tmp\\host.dbg2.json|input-seq=RIGHT:2;A:1|audio-buffer-frames=2048|watch-window=state:0xC000:0x10,io:0xFF00:0x02",
            base,
        )
        .expect("inline session");
        assert_eq!(session.name.as_deref(), Some("host"));
        assert_eq!(session.slot, Some(0));
        assert!(session.rom_path.ends_with("tmp\\host.gb"));
        assert!(session
            .symbols_path
            .as_deref()
            .unwrap()
            .ends_with("tmp\\host.map"));
        assert!(session
            .toolchain_metadata_path
            .as_deref()
            .unwrap()
            .ends_with("tmp\\host.dbg2.json"));
        assert_eq!(session.input_sequence.as_deref(), Some("RIGHT:2;A:1"));
        assert_eq!(session.audio_buffer_frames, Some(2048));
        assert_eq!(session.watch_windows.len(), 2);
        assert_eq!(session.watch_windows[0].name, "state");
        assert_eq!(session.watch_windows[1].addr, 0xFF00);
    }

    #[test]
    // Check a shuffled three-session description becomes host-first with the selected peer index.
    // This verifies layout configuration, not serial exchange or physical adapter behavior.
    fn four_player_link_layout_sorts_sessions_by_slot() {
        let job = LoadedLinkJob {
            topology: normalize_link_topology_label("link4").unwrap(),
            initial_peer_slot: Some(2),
            dump_report: None,
            sessions: vec![
                LoadedLinkSession {
                    name: Some("peer2".to_string()),
                    slot: Some(2),
                    rom_path: "peer2.gb".to_string(),
                    symbols_path: None,
                    source_map_path: None,
                    toolchain_metadata_path: None,
                    load_state: None,
                    save_state: None,
                    input: None,
                    input_sequence: None,
                    audio_buffer_frames: None,
                    watch_windows: Vec::new(),
                    debugger: StopConditionSet::default(),
                    replay: ReplayControlSet::default(),
                },
                LoadedLinkSession {
                    name: Some("host".to_string()),
                    slot: Some(0),
                    rom_path: "host.gb".to_string(),
                    symbols_path: None,
                    source_map_path: None,
                    toolchain_metadata_path: None,
                    load_state: None,
                    save_state: None,
                    input: None,
                    input_sequence: None,
                    audio_buffer_frames: None,
                    watch_windows: Vec::new(),
                    debugger: StopConditionSet::default(),
                    replay: ReplayControlSet::default(),
                },
                LoadedLinkSession {
                    name: Some("peer1".to_string()),
                    slot: Some(1),
                    rom_path: "peer1.gb".to_string(),
                    symbols_path: None,
                    source_map_path: None,
                    toolchain_metadata_path: None,
                    load_state: None,
                    save_state: None,
                    input: None,
                    input_sequence: None,
                    audio_buffer_frames: None,
                    watch_windows: Vec::new(),
                    debugger: StopConditionSet::default(),
                    replay: ReplayControlSet::default(),
                },
            ],
            run_frames: 4,
        };

        let (ordered, topology, note) = build_link_runner_layout(&job).expect("layout");
        assert_eq!(ordered[0].name.as_deref(), Some("host"));
        assert_eq!(ordered[1].name.as_deref(), Some("peer1"));
        assert_eq!(ordered[2].name.as_deref(), Some("peer2"));
        assert!(matches!(
            topology,
            kokura_debug::LinkTopology::FourPlayerAdapter {
                host_session: 0,
                active_peer: 2
            }
        ));
        assert!(note.contains("slot 0 is the host"));
    }

    #[test]
    // Check accepted spellings resolve to the DMG-07 topology label.
    fn dmg07_topology_aliases_normalize_to_physical_adapter_label() {
        for alias in ["dmg07", "DMG-07", "dmg_07", "four_player_adapter_dmg07"] {
            assert_eq!(normalize_link_topology_label(alias).unwrap(), "dmg07");
        }
    }

    #[test]
    // Check DMG-07 slot ordering and the selected topology for a contiguous three-player layout.
    fn dmg07_link_layout_sorts_contiguous_physical_slots() {
        let job = LoadedLinkJob {
            topology: normalize_link_topology_label("DMG-07").unwrap(),
            initial_peer_slot: None,
            dump_report: None,
            sessions: vec![
                test_link_session("player3", 2),
                test_link_session("player1", 0),
                test_link_session("player2", 1),
            ],
            run_frames: 4,
        };

        let (ordered, topology, note) = build_link_runner_layout(&job).expect("layout");
        assert_eq!(ordered[0].name.as_deref(), Some("player1"));
        assert_eq!(ordered[1].name.as_deref(), Some("player2"));
        assert_eq!(ordered[2].name.as_deref(), Some("player3"));
        assert_eq!(topology, kokura_debug::LinkTopology::Dmg07);
        assert!(note.contains("external-clock"));
    }

    #[test]
    // Check that a missing middle slot rejects the DMG-07 layout with a specific error.
    fn dmg07_link_layout_rejects_slot_gaps() {
        let job = LoadedLinkJob {
            topology: "dmg07".to_string(),
            initial_peer_slot: None,
            dump_report: None,
            sessions: vec![
                test_link_session("player1", 0),
                test_link_session("player3", 2),
            ],
            run_frames: 1,
        };

        let error = build_link_runner_layout(&job).unwrap_err().to_string();
        assert!(error.contains("dmg07 sessions must cover contiguous slots"));
    }

    #[test]
    // Check RIFF/WAVE/data markers and payload length for two stereo frames; playback is not tested.
    fn wav_encoder_emits_expected_header_sizes() {
        let wav = encode_wav(32_768, &[1, -2, 3, -4]);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 8);
        assert_eq!(wav.len(), 52);
    }

    #[test]
    // Check the Y4M stream prefix and one frame marker, without decoding its pixels.
    fn y4m_encoder_emits_header_and_frame_marker() {
        let frames = vec![(1u64, vec![0u8; 160 * 144])];
        let y4m = encode_y4m(&frames);
        assert!(y4m.starts_with(b"YUV4MPEG2 W160 H144 F59727:1000"));
        assert!(y4m.windows(6).any(|window| window == b"FRAME\n"));
    }

    #[test]
    // Check a one-frame encoding starts with GIF89a; this does not validate playback timing.
    fn gif_encoder_emits_gif_header() {
        let frames = vec![(1u64, vec![0u8; 160 * 144])];
        let gif = encode_gif(&frames).unwrap();
        assert!(gif.starts_with(b"GIF89a"));
    }

    #[test]
    // Write a small synthetic metadata file, load selected fields and attempt to remove it.
    // The fixed temporary filename assumes this test is not running concurrently in multiple processes.
    fn toolchain_build_report_loader_accepts_json() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("kokura_toolchain_build_report_test.json");
        std::fs::write(
            &path,
            r#"{"function_count":2,"call_edge_count":1,"optimizer_pass_count":3,"abi_issue_count":0,"source_files":["quick_hello.c"],"hotspots":[{"name":"WaitVBlank","incoming_calls":1,"size_bytes":51,"score":51}],"notes":["ok"]}"#,
        )
        .unwrap();
        let report = load_toolchain_build_report_file(path.to_str().unwrap()).unwrap();
        assert_eq!(report.function_count, 2);
        assert_eq!(report.hotspots.len(), 1);
        let _ = std::fs::remove_file(path);
    }

    // Build a deterministic single-checkpoint/slice report whose digest can be varied without executing a ROM.
    fn sample_replay_report(digest: u64) -> ReplayReport {
        ReplayReport {
            enabled: true,
            checkpoint_interval_frames: 1,
            max_checkpoints: 8,
            checkpoints_recorded: 1,
            current_generation: 0,
            checkpoints: vec![ReplayCheckpointReport {
                checkpoint_index: 0,
                generation: 0,
                frame: 4,
                cycle: 64,
                rom_bank: 1,
                pc: 0x4000,
                frame_hash: 0x1122_3344,
                digest,
                symbol: Some("MainLoop".to_string()),
                source: None,
                watch_hashes: Vec::new(),
                watch_digest: 0,
            }],
            last_rewind: None,
            divergence: None,
            slices: vec![ReplaySliceReport {
                from_checkpoint_index: None,
                to_checkpoint_index: 0,
                generation: 0,
                start_frame: 0,
                end_frame: 4,
                start_cycle: 0,
                end_cycle: 64,
                start_rom_bank: 0,
                start_pc: 0x0100,
                start_symbol: None,
                start_source: None,
                end_rom_bank: 1,
                end_pc: 0x4000,
                end_symbol: Some("MainLoop".to_string()),
                end_source: None,
                instruction_samples: 12,
                cpu_cycles: 64,
                event_count: 3,
                bank_switch_count: 1,
                far_call_count: 0,
                joypad_read_count: 0,
                dma_event_count: 0,
                interrupt_event_count: 0,
                render_event_count: 1,
                mmio_event_count: 1,
                slice_digest: digest ^ 0x55AA,
            }],
            reference_compare: None,
        }
    }

    #[test]
    // Check the first mismatch kind and the derived frame window and LCD watch recommendation.
    fn replay_report_comparison_flags_checkpoint_digest_mismatch() {
        let reference = sample_replay_report(0xAAAA_BBBB_CCCC_DDDD);
        let actual = sample_replay_report(0x1111_2222_3333_4444);
        let comparison = compare_replay_reports(
            &reference,
            &actual,
            Some("baseline.tape.json".to_string()),
            false,
        );
        assert!(!comparison.matched);
        assert_eq!(
            comparison.reference_path.as_deref(),
            Some("baseline.tape.json")
        );
        let mismatch = comparison.first_mismatch.expect("mismatch");
        assert_eq!(mismatch.kind, "checkpoint_digest");
        let helper = comparison.divergence_helper.expect("divergence helper");
        assert_eq!(helper.mismatch_slice_index, Some(0));
        assert_eq!(helper.frame_window_start, Some(0));
        assert!(!helper.recommended_focus.is_empty());
        assert!(helper
            .recommended_watch_windows
            .iter()
            .any(|value| value == "--watch-window lcd:0xFF40:0x06"));
    }

    #[test]
    // Check the proposed relative first/last capture frames and watch overlay.
    // This constructs a plan only; no snapshot files are saved.
    fn replay_mismatch_snapshot_plan_builds_relative_triggers_and_overlay_specs() {
        let reference = sample_replay_report(0xAAAA_BBBB_CCCC_DDDD);
        let actual = sample_replay_report(0x1111_2222_3333_4444);
        let comparison = compare_replay_reports(&reference, &actual, None, false);
        let mismatch = comparison.first_mismatch.as_ref().expect("mismatch");
        let helper = comparison.divergence_helper.as_ref().expect("helper");
        let plan = build_replay_mismatch_snapshot_plan(mismatch, helper, Some("tmp/followup/demo"));
        assert_eq!(plan.frame_window_start, 0);
        assert_eq!(plan.frame_window_end, 4);
        assert_eq!(plan.run_frames, 5);
        assert!(plan
            .snapshot_triggers
            .iter()
            .any(|trigger| trigger.kind == "frame" && trigger.value == "1"));
        assert!(plan
            .snapshot_triggers
            .iter()
            .any(|trigger| trigger.kind == "frame" && trigger.value == "5"));
        assert!(plan
            .watch_windows
            .iter()
            .any(|spec| spec == "--watch-window lcd:0xFF40:0x06"));
        assert!(plan.stop_specs.is_empty());
    }

    #[test]
    // Parse recommended DMA/IRQ stops and watch windows back into debugger configuration.
    fn recommended_replay_followup_specs_round_trip_into_debugger_overlays() {
        let stops = parse_recommended_stop_specs(&[
            "--stop-on-dma oam_start".to_string(),
            "--stop-on-dma oam_complete".to_string(),
            "--stop-on-irq lcd_stat:request".to_string(),
            "--stop-on-irq vblank:request".to_string(),
        ])
        .expect("stop overlay");
        assert_eq!(stops.dma_events.len(), 2);
        assert_eq!(stops.interrupts.len(), 2);

        let watches = parse_recommended_watch_windows(&[
            "--watch-window lcd:0xFF40:0x06".to_string(),
            "--watch-window vram:0x9800:0x40".to_string(),
        ])
        .expect("watch overlay");
        assert_eq!(watches.len(), 2);
        assert_eq!(watches[0].addr, 0xFF40);
        assert_eq!(watches[1].addr, 0x9800);
    }

    #[test]
    // Check path and reported-hash propagation into a tape, plus the no-replay case.
    // The synthetic hash is metadata, not a digest computed from an actual ROM.
    fn replay_tape_builder_carries_rom_path_and_sha() {
        let session = DebugSession::new(Machine::new());
        let mut report = session.report();
        report.toolchain_build = Some(kokura_debug::ToolchainBuildReport {
            output_rom: Some("demo.gb".to_string()),
            output_sha256: Some("abc123".to_string()),
            rom_size_bytes: Some(0),
            used_bytes: Some(0),
            abi_mode: None,
            function_count: 0,
            call_edge_count: 0,
            optimizer_pass_count: 0,
            abi_issue_count: 0,
            cross_bank_call_count: 0,
            source_files: Vec::new(),
            abi_issues: Vec::new(),
            hotspots: Vec::new(),
            cross_bank_edges: Vec::new(),
            notes: Vec::new(),
        });
        report.replay = Some(sample_replay_report(0xCAFE_BABE_DEAD_BEEF));
        let tape = build_replay_tape(&report, "C:\\demo\\game.gb").expect("replay tape");
        assert_eq!(tape.rom_path, "C:\\demo\\game.gb");
        assert_eq!(tape.rom_sha256.as_deref(), Some("abc123"));
        report.replay = None;
        assert!(build_replay_tape(&report, "C:\\demo\\game.gb").is_none());
    }

    #[test]
    // Render one synthetic function and check canonical name, user note and calling-convention text.
    fn markdown_renderer_emits_canonical_name_and_user_notes() {
        let report = DecompileReport {
            schema_version: "1",
            rom_size_bytes: 0x8000,
            rom_bank_count: 2,
            title: Some("demo".to_string()),
            mapper: Some("0x00".to_string()),
            functions: vec![DecompileFunctionInfo {
                id: "bank00:0100".to_string(),
                bank: 0,
                start_address: 0x0100,
                end_address: 0x0101,
                name: "EntryPoint".to_string(),
                canonical_name: "sub_0100_bank00".to_string(),
                source_kind: "auto".to_string(),
                calling_convention_guess: Some("fastcall".to_string()),
                user_notes: vec!["seed note".to_string()],
                is_banked: false,
                blocks: Vec::new(),
                xrefs_in: Vec::new(),
                xrefs_out: Vec::new(),
                intrinsic_match: Vec::new(),
                warnings: Vec::new(),
                confidence_score: 0.9,
                switch_candidates: Vec::new(),
                artifact: DecompileArtifact {
                    function_id: "bank00:0100".to_string(),
                    disassembly_text: "RET".to_string(),
                    cfg_json: serde_json::json!({}),
                    pseudocode_text: "return;".to_string(),
                    warnings: Vec::new(),
                    confidence_score: 0.9,
                    stack_slots: Vec::new(),
                    temp_slots: Vec::new(),
                    call_sites: Vec::new(),
                    fingerprints: Vec::new(),
                    suggestions: Vec::new(),
                    trace_annotations: Vec::new(),
                    trace_summary: None,
                },
            }],
            xrefs: Vec::new(),
            labels: Vec::new(),
            data_ranges: Vec::new(),
            notes: Vec::new(),
        };
        let markdown = render_decompile_markdown(&report);
        assert!(markdown.contains("canonical name: sub_0100_bank00"));
        assert!(markdown.contains("note: seed note"));
        assert!(markdown.contains("calling convention: fastcall"));
    }
}
