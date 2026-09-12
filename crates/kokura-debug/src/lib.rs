//! エミュレーション実行を観測・再現するためのデバッグ層。
//!
//! スナップショット、ウォッチ、停止条件、イベント、診断、レポート、
//! リプレイ、シリアルリンク実行をまとめます。ROMを動かす責務は
//! `kokura-core` に残し、このクレートは観測結果をJSON等へ整形します。

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
