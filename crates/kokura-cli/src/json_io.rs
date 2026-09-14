//! Read and write JSON jobs, link jobs and regression matrices.
//!
//! Keep schema versions and file-format details separate from CLI execution.
//! Published job examples refer to user-supplied ROMs by relative path rather
//! than embedding ROM data.

pub const JOB_SPEC_SCHEMA_VERSION: &str = "1";
pub const LINK_JOB_SPEC_SCHEMA_VERSION: &str = "1";
pub const REGRESSION_MATRIX_SCHEMA_VERSION: &str = "1";

use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result};
use kokura_debug::{MemoryWatchSpec, ReplayControlSet, StopConditionSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JobRun {
    #[serde(default = "default_frames")]
    pub frames: u64,
}

// Use one frame when a present run or stage object omits its frames field.
// Derived Rust Default still yields zero, including an entirely omitted JobSpec.run object.
fn default_frames() -> u64 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
// Store the trigger selector and optional snapshot destination for later execution-layer interpretation.
pub struct SnapshotTrigger {
    #[serde(default)]
    pub condition: Option<String>,
    pub kind: String,
    pub value: String,
    #[serde(default)]
    pub save_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
// Keep per-stage input, outputs, stop conditions and replay controls separate from job-wide settings.
pub struct JobStage {
    #[serde(default = "default_frames")]
    pub frames: u64,
    #[serde(default)]
    pub input: Option<String>,
    #[serde(default)]
    pub save_state: Option<String>,
    #[serde(default)]
    pub dump_report: Option<String>,
    #[serde(default)]
    pub snapshots: Vec<SnapshotTrigger>,
    #[serde(default)]
    pub debugger: StopConditionSet,
    #[serde(default)]
    pub replay: ReplayControlSet,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
// Describe one ROM job. Serde defaults make optional collections/settings empty;
// this data model performs no path rebasing, file access or cross-field validation.
pub struct JobSpec {
    #[serde(default)]
    pub schema_version: Option<String>,
    pub rom: String,
    #[serde(default)]
    pub symbols: Option<String>,
    #[serde(default)]
    pub source_map: Option<String>,
    #[serde(default)]
    pub toolchain_metadata: Option<String>,
    #[serde(default)]
    pub input: Option<String>,
    #[serde(default)]
    pub input_sequence: Option<String>,
    #[serde(default)]
    pub stages: Vec<JobStage>,
    #[serde(default)]
    pub load_state: Option<String>,
    #[serde(default)]
    pub save_state: Option<String>,
    #[serde(default)]
    pub dump_report: Option<String>,
    #[serde(default)]
    pub dump_replay_tape: Option<String>,
    #[serde(default)]
    pub compare_replay_tape: Option<String>,
    #[serde(default)]
    pub snapshot_on_replay_mismatch: Option<String>,
    #[serde(default)]
    pub screenshot: Option<String>,
    #[serde(default)]
    pub screenshot_frames: Option<String>,
    #[serde(default)]
    pub record_wav: Option<String>,
    #[serde(default)]
    pub record_wav_frames: Option<String>,
    #[serde(default)]
    pub record_video: Option<String>,
    #[serde(default)]
    pub record_video_frames: Option<String>,
    #[serde(default)]
    pub audio_buffer_frames: Option<usize>,
    #[serde(default)]
    pub autosave_prefix: Option<String>,
    #[serde(default)]
    pub watch_windows: Vec<MemoryWatchSpec>,
    #[serde(default)]
    pub debugger: StopConditionSet,
    #[serde(default)]
    pub replay: ReplayControlSet,
    #[serde(default)]
    pub run: JobRun,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
// Describe each participant independently, including its slot, ROM, state and debugger controls.
pub struct LinkSessionSpec {
    #[serde(default)]
    pub name: Option<String>,
    pub rom: String,
    #[serde(default)]
    pub slot: Option<u8>,
    #[serde(default)]
    pub symbols: Option<String>,
    #[serde(default)]
    pub source_map: Option<String>,
    #[serde(default)]
    pub toolchain_metadata: Option<String>,
    #[serde(default)]
    pub load_state: Option<String>,
    #[serde(default)]
    pub save_state: Option<String>,
    #[serde(default)]
    pub input: Option<String>,
    #[serde(default)]
    pub input_sequence: Option<String>,
    #[serde(default)]
    pub audio_buffer_frames: Option<usize>,
    #[serde(default)]
    pub watch_windows: Vec<MemoryWatchSpec>,
    #[serde(default)]
    pub debugger: StopConditionSet,
    #[serde(default)]
    pub replay: ReplayControlSet,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
// Combine topology text, optional initial peer and participant specs with the shared run budget.
pub struct LinkJobSpec {
    #[serde(default)]
    pub schema_version: Option<String>,
    pub topology: String,
    #[serde(default)]
    pub initial_peer_slot: Option<u8>,
    #[serde(default)]
    pub dump_report: Option<String>,
    #[serde(default)]
    pub sessions: Vec<LinkSessionSpec>,
    #[serde(default)]
    pub run: JobRun,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
// Pair an expected suggestion kind/value with an optional stage selector.
pub struct ExpectedSuggestion {
    pub kind: String,
    pub value: String,
    #[serde(default)]
    pub stage_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
// Represent expected observations and lower-bound counters as data. The matrix runner
// decides whether observed events, watches and diagnostic output meet these expectations.
pub struct RegressionCase {
    pub name: String,
    pub job: String,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub stage_index: Option<usize>,
    #[serde(default)]
    pub expected_suggestions: Vec<ExpectedSuggestion>,
    #[serde(default)]
    pub expected_diff_severity: Option<String>,
    #[serde(default)]
    pub expected_diagnostics: Vec<String>,
    #[serde(default)]
    pub expected_unsupported_opcodes: Vec<String>,
    #[serde(default)]
    pub expected_event_types: Vec<String>,
    #[serde(default)]
    pub expected_event_count_floor: BTreeMap<String, u64>,
    #[serde(default)]
    pub expected_bank_switch_floor: Option<u64>,
    #[serde(default)]
    pub expected_far_call_floor: Option<u64>,
    #[serde(default)]
    pub expected_intrinsic_floor: Option<u64>,
    #[serde(default)]
    pub expected_oam_dma_floor: Option<u64>,
    #[serde(default)]
    pub expected_hdma_block_floor: Option<u64>,
    #[serde(default)]
    pub expected_dma_complete_floor: Option<u64>,
    #[serde(default)]
    pub expected_dma_stall_cycle_floor: Option<u64>,
    #[serde(default)]
    pub expected_bank_thrash_floor: Option<u32>,
    #[serde(default)]
    pub expected_timer_interrupt_floor: Option<u64>,
    #[serde(default)]
    pub expected_vblank_floor: Option<u64>,
    #[serde(default)]
    pub expected_watch_changes: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
// Allow an omitted cases list to decode as empty; version compatibility is checked after decoding.
pub struct RegressionMatrix {
    #[serde(default)]
    pub schema_version: Option<String>,
    #[serde(default)]
    pub cases: Vec<RegressionCase>,
}

// Read UTF-8 JSON, decode the job shape and reject an explicitly unsupported schema version.
// Referenced paths and execution constraints are resolved by the caller, not by this loader.
pub fn load_job_file<P: AsRef<Path>>(path: P) -> Result<JobSpec> {
    let text = fs::read_to_string(path.as_ref())
        .with_context(|| format!("failed to read job file: {}", path.as_ref().display()))?;
    let spec: JobSpec = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse job file JSON: {}", path.as_ref().display()))?;
    validate_schema_version(
        path.as_ref(),
        spec.schema_version.as_deref(),
        JOB_SPEC_SCHEMA_VERSION,
        "job file",
    )?;
    Ok(spec)
}

// Decode a regression matrix with path-qualified read/parse errors, then check its optional version.
pub fn load_regression_matrix<P: AsRef<Path>>(path: P) -> Result<RegressionMatrix> {
    let text = fs::read_to_string(path.as_ref()).with_context(|| {
        format!(
            "failed to read regression matrix file: {}",
            path.as_ref().display()
        )
    })?;
    let matrix: RegressionMatrix = serde_json::from_str(&text).with_context(|| {
        format!(
            "failed to parse regression matrix JSON: {}",
            path.as_ref().display()
        )
    })?;
    validate_schema_version(
        path.as_ref(),
        matrix.schema_version.as_deref(),
        REGRESSION_MATRIX_SCHEMA_VERSION,
        "regression matrix",
    )?;
    Ok(matrix)
}

// Decode link topology/session specifications and check their optional schema version.
// This step does not construct or validate a running link topology.
pub fn load_link_job_file<P: AsRef<Path>>(path: P) -> Result<LinkJobSpec> {
    let text = fs::read_to_string(path.as_ref())
        .with_context(|| format!("failed to read link job file: {}", path.as_ref().display()))?;
    let spec: LinkJobSpec = serde_json::from_str(&text).with_context(|| {
        format!(
            "failed to parse link job file JSON: {}",
            path.as_ref().display()
        )
    })?;
    validate_schema_version(
        path.as_ref(),
        spec.schema_version.as_deref(),
        LINK_JOB_SPEC_SCHEMA_VERSION,
        "link job file",
    )?;
    Ok(spec)
}

// Accept an omitted version for legacy files; otherwise require an exact string match.
fn validate_schema_version(
    path: &Path,
    actual: Option<&str>,
    expected: &str,
    kind: &str,
) -> Result<()> {
    if let Some(actual) = actual {
        if actual != expected {
            anyhow::bail!(
                "unsupported {} schema_version '{}' in {} (expected '{}')",
                kind,
                actual,
                path.display(),
                expected
            );
        }
    }
    Ok(())
}
