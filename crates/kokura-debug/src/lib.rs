//! Debugging facilities for observing and reproducing emulator runs.
//!
//! Combine snapshots, watches, stop conditions, events, diagnostics, reports,
//! replays and serial-link execution. kokura-core runs the ROM; this crate
//! organizes observations for JSON and other report formats.

pub mod diagnostics;
pub mod events;
pub mod frame_diff;
pub mod hashes;
pub mod input;
pub mod link;
pub mod report;
pub mod session;
pub mod snapshot;
pub mod stop;
pub mod watch;

pub use diagnostics::{
    AutoDiagnosisCategory, AutoDiagnosisReport, AutoDiagnosisSuspect, Diagnostic, DiagnosticCode,
    Severity, TimingPackEntry, TimingPackLevel, TimingPackReport, TimingPackVerification,
};
pub use events::DebugEvent;
pub use link::{LinkRunSummary, LinkRunnerError, LinkTopology, TimingAwareLinkRunner};
pub use report::{DebugReport, ToolchainBuildReport, ToolchainHotspotReport};
pub use session::{DebugSession, DebugStepOutcome, ObservationSnapshot};
pub use watch::{MemoryWatchBaselineMode, MemoryWatchDiffByte, MemoryWatchResult, MemoryWatchSpec};

pub use stop::{
    DmaStopSpec, ExecuteBreakpointSpec, ExecutionContextFrame, InterruptStopPhase,
    InterruptStopSpec, MemoryWatchpointSpec, MmioWriteStopSpec, ReplayControlSet,
    SourceLocationStop, StopConditionSet, StopReason,
};
