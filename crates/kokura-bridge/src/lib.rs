//! KITAQGB が出力するデバッグ副ファイルとKOKURAを接続する層。
//!
//! map/source-mapの解析、シンボルとソース位置の保持、KITAQGB intrinsicの
//! 分類、バンク切替の説明、ROMの逆アセンブル補助を提供します。

pub mod banktrace;
pub mod decompile;
pub mod intrinsics;
pub mod map_parser;
pub mod source_map;
pub mod symbols;

pub use banktrace::{describe_bank_switch, BankTraceInfo};
pub use intrinsics::{classify_symbol_name, KitaqgbIntrinsicKind, KitaqgbIntrinsicMatch};
pub use source_map::parse_source_map_file;
pub use symbols::{
    CallEdgeInfo, FunctionInfo, SourceLocationInfo, StaticEstimateInfo, SymbolInfo, SymbolTable,
    VariableInfo,
};

pub use decompile::{
    analyze_rom, apply_annotations, apply_trace_observations, disassemble_range, AutoLabel,
    DataRangeInfo, DecodedInstruction, DecompileAnnotationFile, DecompileArtifact,
    DecompileBasicBlockInfo, DecompileCallSiteHint, DecompileFunctionInfo,
    DecompileFunctionOverride, DecompileLabelOverride, DecompileOptions, DecompileReport,
    DecompileStackSlotHint, DecompileTempSlotHint, DecompileTraceObservation, DecompileTracePcHit,
    DecompileTraceSummary, SwitchCandidateInfo, SwitchCaseInfo, XrefInfo,
};
