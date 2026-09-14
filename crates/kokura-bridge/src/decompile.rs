use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{
    intrinsics::{classify_symbol_name, KitaqgbIntrinsicMatch},
    symbols::{FunctionInfo, SymbolInfo, SymbolTable},
};

const BANK_WINDOW_START: u16 = 0x4000;
const BANK_WINDOW_END: u16 = 0x7FFF;
const FIXED_BANK_END: u16 = 0x3FFF;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoLabel {
    pub bank: u16,
    pub addr: u16,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// Keep unresolved destination banks explicit; confidence strings describe static evidence strength.
pub struct XrefInfo {
    pub from_bank: u16,
    pub from_addr: u16,
    pub to_bank: Option<u16>,
    pub to_addr: u16,
    pub kind: String,
    #[serde(default)]
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// Describe a candidate data interval with an exclusive end address.
pub struct DataRangeInfo {
    pub bank: u16,
    pub start: u16,
    pub end: u16,
    pub classification: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SwitchCaseInfo {
    pub index: u16,
    pub target_bank: Option<u16>,
    pub target_addr: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// Retain the inferred table location and decoded target candidates separately from the jump site.
pub struct SwitchCandidateInfo {
    pub table_bank: u16,
    pub table_addr: u16,
    pub jump_bank: u16,
    pub jump_addr: u16,
    pub cases: Vec<SwitchCaseInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// Separate available bytes from text, target and control-flow metadata for one decoded start.
pub struct DecodedInstruction {
    pub bank: u16,
    pub addr: u16,
    pub linear_address: u32,
    pub bytes: Vec<u8>,
    pub mnemonic: String,
    pub operand_text: String,
    pub text: String,
    #[serde(default)]
    pub target_bank: Option<u16>,
    #[serde(default)]
    pub target_addr: Option<u16>,
    pub flow_kind: String,
    pub is_conditional: bool,
    pub fallthrough: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// Store half-open address bounds and local graph edges; role strings are inferred structure hints.
pub struct DecompileBasicBlockInfo {
    pub id: String,
    pub start_address: u16,
    pub end_address: u16,
    pub instructions: Vec<DecodedInstruction>,
    pub predecessors: Vec<String>,
    pub successors: Vec<String>,
    #[serde(default)]
    pub loop_role: Option<String>,
    #[serde(default)]
    pub structure_role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// Bundle static presentation with optional trace overlays and explicit warnings/candidate evidence.
pub struct DecompileArtifact {
    pub function_id: String,
    pub disassembly_text: String,
    pub cfg_json: serde_json::Value,
    pub pseudocode_text: String,
    pub warnings: Vec<String>,
    pub confidence_score: f32,
    #[serde(default)]
    pub stack_slots: Vec<DecompileStackSlotHint>,
    #[serde(default)]
    pub temp_slots: Vec<DecompileTempSlotHint>,
    #[serde(default)]
    pub call_sites: Vec<DecompileCallSiteHint>,
    #[serde(default)]
    pub fingerprints: Vec<String>,
    #[serde(default)]
    pub suggestions: Vec<String>,
    pub trace_annotations: Vec<String>,
    #[serde(default)]
    pub trace_summary: Option<DecompileTraceSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// Keep a stable ID/canonical name alongside editable presentation and inferred per-function analysis.
pub struct DecompileFunctionInfo {
    pub id: String,
    pub bank: u16,
    pub start_address: u16,
    pub end_address: u16,
    pub name: String,
    pub canonical_name: String,
    pub source_kind: String,
    #[serde(default)]
    pub calling_convention_guess: Option<String>,
    #[serde(default)]
    pub user_notes: Vec<String>,
    pub is_banked: bool,
    pub blocks: Vec<DecompileBasicBlockInfo>,
    pub xrefs_in: Vec<XrefInfo>,
    pub xrefs_out: Vec<XrefInfo>,
    pub intrinsic_match: Vec<KitaqgbIntrinsicMatch>,
    pub warnings: Vec<String>,
    pub confidence_score: f32,
    pub switch_candidates: Vec<SwitchCandidateInfo>,
    pub artifact: DecompileArtifact,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// Report analysis of the supplied bytes; title and mapper fields come from that input ROM header.
pub struct DecompileReport {
    pub schema_version: &'static str,
    pub rom_size_bytes: u32,
    pub rom_bank_count: u16,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub mapper: Option<String>,
    pub functions: Vec<DecompileFunctionInfo>,
    pub xrefs: Vec<XrefInfo>,
    pub labels: Vec<AutoLabel>,
    pub data_ranges: Vec<DataRangeInfo>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// Record a candidate SP-relative variable, static access counts and optional supplied trace evidence.
pub struct DecompileStackSlotHint {
    pub offset: i16,
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub inferred_type: Option<String>,
    #[serde(default)]
    pub runtime_role: Option<String>,
    #[serde(default)]
    pub read_count: u32,
    #[serde(default)]
    pub write_count: u32,
    #[serde(default)]
    pub access_patterns: Vec<String>,
    #[serde(default)]
    pub trace_value_hints: Vec<String>,
    #[serde(default)]
    pub lifetime_hint: Option<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// Describe a register spill candidate, not a reconstructed source-language local variable.
pub struct DecompileTempSlotHint {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub inferred_type: Option<String>,
    #[serde(default)]
    pub runtime_role: Option<String>,
    #[serde(default)]
    pub read_count: u32,
    #[serde(default)]
    pub write_count: u32,
    #[serde(default)]
    pub access_patterns: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// Keep static setup guesses separate from values observed at a call PC; neither establishes a signature.
pub struct DecompileCallSiteHint {
    pub call_bank: u16,
    pub call_addr: u16,
    #[serde(default)]
    pub target_bank: Option<u16>,
    #[serde(default)]
    pub target_addr: Option<u16>,
    #[serde(default)]
    pub target_name: Option<String>,
    #[serde(default)]
    pub static_argument_hints: Vec<String>,
    #[serde(default)]
    pub runtime_argument_hints: Vec<String>,
    #[serde(default)]
    pub calling_convention_hint: Option<String>,
    #[serde(default)]
    pub hit_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// Accept caller-supplied runtime evidence. Frame/PC/slot values are not collected or verified by this type.
pub struct DecompileTraceObservation {
    #[serde(default)]
    pub completed_frames: Option<u64>,
    #[serde(default)]
    pub active_frame: Option<u64>,
    #[serde(default)]
    pub cycle: Option<u64>,
    pub pc: u16,
    pub rom_bank: u16,
    #[serde(default = "default_trace_observation_hit_count")]
    pub hit_count: u64,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub event_summary: Option<String>,
    #[serde(default)]
    pub watch_summary: Option<String>,
    #[serde(default)]
    pub registers: BTreeMap<String, u64>,
    #[serde(default)]
    pub flags: BTreeMap<String, bool>,
    #[serde(default)]
    pub stack_slot_values: BTreeMap<String, u64>,
    #[serde(default)]
    pub sp: Option<u16>,
}

// Treat a missing serialized hit count as one observation.
fn default_trace_observation_hit_count() -> u64 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecompileTracePcHit {
    pub rom_bank: u16,
    pub pc: u16,
    pub hit_count: u64,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
// Summarize matched observations with bounded previews rather than storing a complete execution trace.
pub struct DecompileTraceSummary {
    pub hit_count: u64,
    #[serde(default)]
    pub first_frame: Option<u64>,
    #[serde(default)]
    pub last_frame: Option<u64>,
    #[serde(default)]
    pub first_cycle: Option<u64>,
    #[serde(default)]
    pub last_cycle: Option<u64>,
    #[serde(default)]
    pub observed_symbols: Vec<String>,
    #[serde(default)]
    pub observed_sources: Vec<String>,
    #[serde(default)]
    pub event_samples: Vec<String>,
    #[serde(default)]
    pub watch_samples: Vec<String>,
    #[serde(default)]
    pub hot_pcs: Vec<DecompileTracePcHit>,
}

#[derive(Debug, Clone, Default)]
// Select reported roots after discovery; recovery may also append plausible callees outside that selection.
pub struct DecompileOptions {
    pub selected_functions: Vec<String>,
    pub include_all_named_functions: bool,
}

// Decode sequentially within a validated inclusive address range, stopping on empty bytes
// or address overflow. The final instruction may extend beyond the requested end address.
pub fn disassemble_range(
    rom: &[u8],
    bank: u16,
    start_addr: u16,
    end_addr_inclusive: u16,
) -> Vec<DecodedInstruction> {
    let rom_bank_count = ((rom.len() + 0x3FFF) / 0x4000) as u16;
    if !is_code_addr(bank, start_addr, rom_bank_count)
        || !is_code_addr(bank, end_addr_inclusive, rom_bank_count)
        || start_addr > end_addr_inclusive
    {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut addr = start_addr;
    while addr <= end_addr_inclusive {
        let raw = decode_instruction(rom, rom_bank_count, bank, addr);
        let len = raw.bytes.len().max(1) as u16;
        if raw.bytes.is_empty() {
            break;
        }
        out.push(to_decoded_instruction(bank, raw));
        let Some(next_addr) = addr.checked_add(len) else {
            break;
        };
        if next_addr <= addr {
            break;
        }
        addr = next_addr;
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecompileFunctionOverride {
    pub selector: String,
    #[serde(default)]
    pub rename: Option<String>,
    #[serde(default)]
    pub calling_convention: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecompileLabelOverride {
    pub bank: u16,
    pub addr: u16,
    pub name: String,
    #[serde(default = "default_user_label_kind")]
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
// Deserialize editable names/notes independently of instruction decoding; schema validation is external.
pub struct DecompileAnnotationFile {
    #[serde(default = "default_decompile_annotation_schema_version")]
    pub schema_version: String,
    #[serde(default)]
    pub function_overrides: Vec<DecompileFunctionOverride>,
    #[serde(default)]
    pub label_overrides: Vec<DecompileLabelOverride>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct PseudocodeContext {
    stack_slot_names: BTreeMap<i16, String>,
}

#[derive(Debug, Clone, Default)]
// Track only a tentative HL-to-SP offset alias for text rendering and pattern scans.
struct BlockRenderState {
    hl_stack_alias: Option<i16>,
}

#[derive(Debug, Clone, Default)]
struct StackAccessProfile {
    read_count: u32,
    write_count: u32,
    patterns: BTreeSet<String>,
    literal_writes: BTreeSet<String>,
    bit_indices: BTreeSet<u8>,
}

#[derive(Debug, Clone, Default)]
struct TraceValueProfile {
    values: BTreeSet<u16>,
    repeated_increments: u32,
    repeated_toggles: u32,
    likely_pointer_hits: u32,
}

#[derive(Debug, Clone, Default)]
struct ControlFlowHints {
    loop_headers: BTreeSet<usize>,
    if_headers: BTreeSet<usize>,
    if_else_headers: BTreeSet<usize>,
    break_blocks: BTreeSet<usize>,
    continue_blocks: BTreeSet<usize>,
}

// Supply the default annotation format identifier when deserializing an omitted field.
fn default_decompile_annotation_schema_version() -> String {
    "decompile_annotation_v1".to_string()
}

// Classify a label override as user-authored unless its serialized kind says otherwise.
fn default_user_label_kind() -> String {
    "user".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlowKind {
    None,
    Jump,
    JumpCond,
    Call,
    CallCond,
    Return,
    ReturnCond,
    Reti,
    Rst,
    IndirectJump,
    Stop,
    Halt,
    Invalid,
}

#[derive(Debug, Clone)]
struct RawInstruction {
    bank: u16,
    addr: u16,
    linear_address: u32,
    len: u8,
    bytes: Vec<u8>,
    mnemonic: String,
    operand_text: String,
    flow_kind: FlowKind,
    target_addr: Option<u16>,
    is_conditional: bool,
    fallthrough: bool,
    is_invalid: bool,
}

#[derive(Debug, Clone)]
struct FunctionAnalysis {
    bank: u16,
    start: u16,
    name: String,
    source_kind: String,
    instructions: BTreeMap<u16, RawInstruction>,
    xrefs_out: Vec<XrefInfo>,
    warnings: Vec<String>,
    confidence_score: f32,
    metadata: Option<FunctionInfo>,
}

#[derive(Debug, Clone, Copy)]
struct FunctionRange {
    bank: u16,
    start: u16,
    end: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowKind {
    Fixed,
    Switchable,
}

impl WindowKind {
    // Test only fixed versus switchable CPU-address bounds, independently of ROM length.
    fn contains(self, addr: u16) -> bool {
        match self {
            Self::Fixed => addr <= FIXED_BANK_END,
            Self::Switchable => (BANK_WINDOW_START..=BANK_WINDOW_END).contains(&addr),
        }
    }
}

// Discover bank/address roots from metadata, vectors and selectors, follow resolvable calls,
// and build static code/data, CFG and pseudocode reports. Bank assumptions and semantic
// patterns are heuristics; this function does not execute the ROM or recover original source.
pub fn analyze_rom(
    rom: &[u8],
    symbol_table: Option<&SymbolTable>,
    options: &DecompileOptions,
) -> DecompileReport {
    let rom_bank_count = ((rom.len() + 0x3FFF) / 0x4000) as u16;
    let mut roots = BTreeSet::new();
    let mut function_meta_by_addr: BTreeMap<(u16, u16), FunctionInfo> = BTreeMap::new();
    let mut symbol_by_addr: BTreeMap<(u16, u16), SymbolInfo> = BTreeMap::new();
    let mut labels: BTreeMap<(u16, u16), AutoLabel> = BTreeMap::new();

    if let Some(table) = symbol_table {
        for func in &table.functions {
            if is_code_addr(func.bank, func.start, rom_bank_count) {
                roots.insert((func.bank, func.start));
                function_meta_by_addr.insert((func.bank, func.start), func.clone());
            }
        }
        for sym in &table.symbols {
            if is_code_addr(sym.bank, sym.start, rom_bank_count) {
                roots.insert((sym.bank, sym.start));
                symbol_by_addr.insert((sym.bank, sym.start), sym.clone());
            }
        }
    }

    for &addr in &[
        0x0000u16, 0x0008, 0x0010, 0x0018, 0x0020, 0x0028, 0x0030, 0x0038, 0x0040, 0x0048, 0x0050,
        0x0058, 0x0060, 0x0100,
    ] {
        if addr <= FIXED_BANK_END {
            roots.insert((0, addr));
        }
    }

    for selected in &options.selected_functions {
        if let Some((bank, addr)) = parse_bank_addr_selector(selected) {
            if is_code_addr(bank, addr, rom_bank_count) {
                roots.insert((bank, addr));
            }
        }
    }

    // Begin with a deterministic sorted root queue; discoveries append new targets and do not reanalyze older roots.
    let named_roots: Vec<(u16, u16)> = roots.iter().copied().collect();
    let mut pending: VecDeque<(u16, u16)> = named_roots.into_iter().collect();
    let mut analyzed: BTreeMap<(u16, u16), FunctionAnalysis> = BTreeMap::new();

    while let Some((bank, start)) = pending.pop_front() {
        if analyzed.contains_key(&(bank, start)) {
            continue;
        }
        if !is_code_addr(bank, start, rom_bank_count) {
            continue;
        }
        let meta = function_meta_by_addr.get(&(bank, start)).cloned();
        let symbol = symbol_by_addr.get(&(bank, start)).cloned();
        let name = meta
            .as_ref()
            .map(|f| f.name.clone())
            .or_else(|| symbol.as_ref().map(|s| s.name.clone()))
            .unwrap_or_else(|| auto_function_name(bank, start));
        let source_kind = if meta.is_some() {
            "metadata".to_string()
        } else if symbol.is_some() {
            "symbol".to_string()
        } else {
            "inferred".to_string()
        };
        labels.entry((bank, start)).or_insert_with(|| AutoLabel {
            bank,
            addr: start,
            name: name.clone(),
            kind: "function".to_string(),
        });

        let analysis = analyze_function(
            rom,
            rom_bank_count,
            symbol_table,
            &roots,
            bank,
            start,
            name,
            source_kind,
            meta,
        );

        for xref in &analysis.xrefs_out {
            if matches!(xref.kind.as_str(), "call" | "call_cond" | "rst") {
                if let Some(target_bank) = xref.to_bank {
                    if is_code_addr(target_bank, xref.to_addr, rom_bank_count)
                        && !analyzed.contains_key(&(target_bank, xref.to_addr))
                    {
                        roots.insert((target_bank, xref.to_addr));
                        pending.push_back((target_bank, xref.to_addr));
                    }
                }
            }
            if let Some(target_bank) = xref.to_bank {
                labels
                    .entry((target_bank, xref.to_addr))
                    .or_insert_with(|| AutoLabel {
                        bank: target_bank,
                        addr: xref.to_addr,
                        name: auto_label_name(target_bank, xref.to_addr, &xref.kind),
                        kind: if xref.kind.contains("call") || xref.kind == "rst" {
                            "function_candidate".to_string()
                        } else {
                            "code_target".to_string()
                        },
                    });
            }
        }

        analyzed.insert((bank, start), analysis);
    }

    promote_recovered_functions(
        rom,
        rom_bank_count,
        symbol_table,
        &mut roots,
        &mut labels,
        &mut analyzed,
    );

    let mut all_functions: Vec<_> = analyzed.into_values().collect();
    all_functions.sort_by_key(|f| (f.bank, f.start));

    let selected_filter = parse_selected_filters(&options.selected_functions);
    let selected_functions = if selected_filter.is_empty() && !options.include_all_named_functions {
        all_functions.clone()
    } else {
        all_functions
            .clone()
            .into_iter()
            .filter(|function| match_selected_filter(function, &selected_filter, options))
            .collect()
    };

    // Retain global evidence before output selection so unselected callers can still contribute incoming references.
    let global_xrefs = collect_global_xrefs(&all_functions);
    let data_ranges = classify_data_ranges(rom, rom_bank_count, &all_functions);
    for data in &data_ranges {
        labels
            .entry((data.bank, data.start))
            .or_insert_with(|| AutoLabel {
                bank: data.bank,
                addr: data.start,
                name: format!("data_bank{:02X}_{:04X}", data.bank, data.start),
                kind: data.classification.clone(),
            });
    }

    let mut notes = vec![
        "Static decompilation support is bank-aware for the fixed bank and switchable banks.".to_string(),
        "Unknown cross-bank targets in bank 0 switchable calls remain marked as ambiguous unless metadata resolves them.".to_string(),
        "Control-flow structuring prefers readable pseudocode with goto fallback when certainty is low.".to_string(),
    ];
    if symbol_table.is_none() {
        notes.push("No symbol/source metadata was supplied, so names and cross-bank resolution are heuristic only.".to_string());
    }

    let function_lookup = build_function_name_lookup(&all_functions);
    let xrefs_in_map = build_xrefs_in_map(&global_xrefs, &function_lookup);
    let mut report_functions = Vec::new();

    for function in selected_functions {
        let blocks = build_basic_blocks(&function, &global_xrefs);
        let switch_candidates = detect_switch_candidates(rom, rom_bank_count, &function, &blocks);
        let artifact = build_artifact(&function, &blocks, &switch_candidates, &function_lookup);
        let intrinsic_match = classify_intrinsics(&function);
        let xrefs_in = xrefs_in_map
            .get(&(function.bank, function.start))
            .cloned()
            .unwrap_or_default();
        let xrefs_out = function
            .xrefs_out
            .iter()
            .filter(|xref| xref.to_bank.is_some())
            .cloned()
            .collect::<Vec<_>>();
        let calling_convention_guess = function.metadata.as_ref().map(|meta| {
            if meta.is_fast_call {
                "fastcall".to_string()
            } else if meta.is_stack_call {
                "stack_call".to_string()
            } else {
                "unknown".to_string()
            }
        });
        report_functions.push(DecompileFunctionInfo {
            id: format!("bank{:02X}:{:04X}", function.bank, function.start),
            bank: function.bank,
            start_address: function.start,
            end_address: function
                .instructions
                .values()
                .map(|ins| ins.addr.saturating_add(ins.len as u16))
                .max()
                .unwrap_or(function.start),
            name: function.name.clone(),
            canonical_name: function.name.clone(),
            source_kind: function.source_kind.clone(),
            calling_convention_guess,
            user_notes: Vec::new(),
            is_banked: function.bank != 0,
            blocks,
            xrefs_in,
            xrefs_out,
            intrinsic_match,
            warnings: function.warnings.clone(),
            confidence_score: function.confidence_score,
            switch_candidates,
            artifact,
        });
    }

    // This second recovery pass operates after selection and can add plausible referenced callees to the output.
    let existing_function_keys = report_functions
        .iter()
        .map(|function| (function.bank, function.start_address))
        .collect::<BTreeSet<_>>();
    for candidate in labels.values() {
        if !matches!(
            candidate.kind.as_str(),
            "function_candidate" | "recovered_function"
        ) {
            continue;
        }
        if existing_function_keys.contains(&(candidate.bank, candidate.addr)) {
            continue;
        }
        let analysis = analyze_function(
            rom,
            rom_bank_count,
            symbol_table,
            &roots,
            candidate.bank,
            candidate.addr,
            candidate.name.clone(),
            "recovered".to_string(),
            function_meta_by_addr
                .get(&(candidate.bank, candidate.addr))
                .cloned(),
        );
        if analysis.instructions.is_empty()
            || !should_promote_recovered_function(
                &analysis,
                &report_functions
                    .iter()
                    .map(|function| FunctionRange {
                        bank: function.bank,
                        start: function.start_address,
                        end: function.end_address,
                    })
                    .collect::<Vec<_>>(),
                &collect_incoming_xref_counts(&global_xrefs),
            )
        {
            continue;
        }
        let blocks = build_basic_blocks(&analysis, &global_xrefs);
        let switch_candidates = detect_switch_candidates(rom, rom_bank_count, &analysis, &blocks);
        let artifact = build_artifact(&analysis, &blocks, &switch_candidates, &function_lookup);
        let intrinsic_match = classify_intrinsics(&analysis);
        let xrefs_in = xrefs_in_map
            .get(&(analysis.bank, analysis.start))
            .cloned()
            .unwrap_or_default();
        let xrefs_out = analysis
            .xrefs_out
            .iter()
            .filter(|xref| xref.to_bank.is_some())
            .cloned()
            .collect::<Vec<_>>();
        report_functions.push(DecompileFunctionInfo {
            id: format!("bank{:02X}:{:04X}", analysis.bank, analysis.start),
            bank: analysis.bank,
            start_address: analysis.start,
            end_address: analysis
                .instructions
                .values()
                .map(|ins| ins.addr.saturating_add(ins.len as u16))
                .max()
                .unwrap_or(analysis.start),
            name: analysis.name.clone(),
            canonical_name: analysis.name.clone(),
            source_kind: "recovered".to_string(),
            calling_convention_guess: None,
            user_notes: Vec::new(),
            is_banked: analysis.bank != 0,
            blocks,
            xrefs_in,
            xrefs_out,
            intrinsic_match,
            warnings: analysis.warnings.clone(),
            confidence_score: analysis.confidence_score,
            switch_candidates,
            artifact,
        });
    }
    report_functions.sort_by_key(|function| (function.bank, function.start_address));

    let title = if rom.len() >= 0x144 {
        // Read the full legacy title span from the supplied image; it includes byte 143 used by CGB headers.
        let raw = &rom[0x134..0x144];
        let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        let text = String::from_utf8_lossy(&raw[..end]).trim().to_string();
        (!text.is_empty()).then_some(text)
    } else {
        None
    };
    let mapper = rom.get(0x147).map(|v| format!("0x{v:02X}"));

    DecompileReport {
        schema_version: "1",
        rom_size_bytes: rom.len() as u32,
        rom_bank_count,
        title,
        mapper,
        functions: report_functions,
        xrefs: global_xrefs,
        labels: labels.into_values().collect(),
        data_ranges,
        notes,
    }
}

// Apply ordered function/label overrides and unique notes, returning the number of changed
// fields or added entries. Preserve canonical names and previously rendered artifact text;
// this helper does not validate the schema identifier or regenerate disassembly/pseudocode.
pub fn apply_annotations(
    report: &mut DecompileReport,
    annotations: &DecompileAnnotationFile,
) -> usize {
    let mut applied = 0usize;

    for function in &mut report.functions {
        for override_spec in &annotations.function_overrides {
            if !function_selector_matches(function, &override_spec.selector) {
                continue;
            }
            if let Some(rename) = &override_spec.rename {
                if function.name != *rename {
                    function.name = rename.clone();
                    applied += 1;
                }
            }
            if let Some(cc) = &override_spec.calling_convention {
                if function.calling_convention_guess.as_deref() != Some(cc.as_str()) {
                    function.calling_convention_guess = Some(cc.clone());
                    applied += 1;
                }
            }
            for note in &override_spec.notes {
                if !function.user_notes.iter().any(|existing| existing == note) {
                    function.user_notes.push(note.clone());
                    applied += 1;
                }
            }
        }
    }

    for label in &annotations.label_overrides {
        let replacement = AutoLabel {
            bank: label.bank,
            addr: label.addr,
            name: label.name.clone(),
            kind: label.kind.clone(),
        };
        if let Some(existing) = report
            .labels
            .iter_mut()
            .find(|entry| entry.bank == label.bank && entry.addr == label.addr)
        {
            if *existing != replacement {
                *existing = replacement;
                applied += 1;
            }
        } else {
            report.labels.push(replacement);
            applied += 1;
        }
    }

    for note in &annotations.notes {
        if !report.notes.iter().any(|existing| existing == note) {
            report.notes.push(note.clone());
            applied += 1;
        }
    }

    report
        .labels
        .sort_by_key(|label| (label.bank, label.addr, label.name.clone()));
    applied
}

// Try at most four expansion rounds. Calls seed candidates immediately; jumps require
// two incoming references and an entry-like byte pattern before overlap/validity screening.
fn promote_recovered_functions(
    rom: &[u8],
    rom_bank_count: u16,
    symbol_table: Option<&SymbolTable>,
    roots: &mut BTreeSet<(u16, u16)>,
    labels: &mut BTreeMap<(u16, u16), AutoLabel>,
    analyzed: &mut BTreeMap<(u16, u16), FunctionAnalysis>,
) {
    for _round in 0..4 {
        let incoming_counts = collect_incoming_xref_counts(
            &analyzed
                .values()
                .flat_map(|analysis| analysis.xrefs_out.iter().cloned())
                .collect::<Vec<_>>(),
        );
        // Each expansion round compares candidates with this snapshot; new candidates from the same round are not added here.
        let existing_ranges = analyzed
            .values()
            .map(|analysis| FunctionRange {
                bank: analysis.bank,
                start: analysis.start,
                end: function_analysis_end(analysis),
            })
            .collect::<Vec<_>>();

        let mut inferred_call_targets = BTreeMap::<(u16, u16), u32>::new();
        let mut inferred_jump_targets = BTreeMap::<(u16, u16), u32>::new();
        for analysis in analyzed.values() {
            for xref in &analysis.xrefs_out {
                let Some(target_bank) = xref.to_bank else {
                    continue;
                };
                if !is_code_addr(target_bank, xref.to_addr, rom_bank_count) {
                    continue;
                }
                match xref.kind.as_str() {
                    "call" | "call_cond" | "rst" => {
                        *inferred_call_targets
                            .entry((target_bank, xref.to_addr))
                            .or_default() += 1;
                    }
                    "jump" | "jump_cond" => {
                        *inferred_jump_targets
                            .entry((target_bank, xref.to_addr))
                            .or_default() += 1;
                    }
                    _ => {}
                }
            }
        }

        let mut secondary_roots = BTreeSet::new();
        for (target, hit_count) in inferred_call_targets {
            if analyzed.contains_key(&target) {
                continue;
            }
            if hit_count >= 1 || looks_like_function_entry(rom, rom_bank_count, target.0, target.1)
            {
                secondary_roots.insert(target);
            }
        }
        for (target, hit_count) in inferred_jump_targets {
            if analyzed.contains_key(&target) {
                continue;
            }
            if hit_count >= 2 && looks_like_function_entry(rom, rom_bank_count, target.0, target.1)
            {
                secondary_roots.insert(target);
            }
        }

        let mut added_any = false;
        for (bank, start) in secondary_roots {
            if analyzed.contains_key(&(bank, start)) {
                continue;
            }
            let name = auto_function_name(bank, start);
            let analysis = analyze_function(
                rom,
                rom_bank_count,
                symbol_table,
                roots,
                bank,
                start,
                name.clone(),
                "recovered".to_string(),
                None,
            );
            if !should_promote_recovered_function(&analysis, &existing_ranges, &incoming_counts) {
                continue;
            }
            labels.entry((bank, start)).or_insert_with(|| AutoLabel {
                bank,
                addr: start,
                name,
                kind: "recovered_function".to_string(),
            });
            analyzed.insert((bank, start), analysis);
            roots.insert((bank, start));
            added_any = true;
        }
        if !added_any {
            break;
        }
    }
}

// Count supplied references per resolved destination, regardless of reference kind.
fn collect_incoming_xref_counts(xrefs: &[XrefInfo]) -> BTreeMap<(u16, u16), u32> {
    let mut counts = BTreeMap::new();
    for xref in xrefs {
        let Some(bank) = xref.to_bank else {
            continue;
        };
        *counts.entry((bank, xref.to_addr)).or_default() += 1;
    }
    counts
}

// Return the maximum decoded instruction end, exclusive, saturating at the address limit.
fn function_analysis_end(function: &FunctionAnalysis) -> u16 {
    function
        .instructions
        .values()
        .map(|ins| ins.addr.saturating_add(ins.len as u16))
        .max()
        .unwrap_or(function.start)
}

// Reject empty, overlapping or invalid-heavy candidates; require an incoming reference
// and either a terminal instruction or an entry-like first byte. This is a boundary heuristic.
fn should_promote_recovered_function(
    analysis: &FunctionAnalysis,
    existing_ranges: &[FunctionRange],
    incoming_counts: &BTreeMap<(u16, u16), u32>,
) -> bool {
    if analysis.instructions.is_empty() {
        return false;
    }
    let end = function_analysis_end(analysis);
    let instruction_count = analysis.instructions.len() as u32;
    let invalid_count = analysis
        .warnings
        .iter()
        .filter(|warning| warning.contains("invalid opcode"))
        .count() as u32;
    let incoming = incoming_counts
        .get(&(analysis.bank, analysis.start))
        .copied()
        .unwrap_or(0);
    let has_terminal = analysis.instructions.values().any(|ins| {
        matches!(
            ins.flow_kind,
            FlowKind::Return
                | FlowKind::ReturnCond
                | FlowKind::Reti
                | FlowKind::Jump
                | FlowKind::IndirectJump
        )
    });
    let overlaps_existing = existing_ranges.iter().any(|range| {
        range.bank == analysis.bank
            && analysis.start < range.end
            && end > range.start
            && analysis.start != range.start
    });
    if overlaps_existing {
        return false;
    }
    let invalid_ratio = if instruction_count == 0 {
        1.0
    } else {
        invalid_count as f32 / instruction_count as f32
    };
    if invalid_ratio > 0.25 {
        return false;
    }
    incoming >= 1 && (has_terminal || looks_like_function_entry_placeholder(analysis))
}

// Recognize selected PUSH, CALL and HL-setup first opcodes as weak entry evidence.
fn looks_like_function_entry_placeholder(analysis: &FunctionAnalysis) -> bool {
    analysis
        .instructions
        .values()
        .next()
        .map(|ins| {
            matches!(
                ins.bytes.first().copied(),
                Some(0xC5 | 0xD5 | 0xE5 | 0xF5 | 0xCD | 0xF8 | 0x21)
            )
        })
        .unwrap_or(false)
}

// Overlay supplied observations by exact bank and block address range. Replace matched
// function summaries while appending hints/annotations; unmatched old summaries remain.
// Input order drives value-transition guesses; no ROM identity or chronological validation occurs here.
pub fn apply_trace_observations(
    report: &mut DecompileReport,
    observations: &[DecompileTraceObservation],
) -> usize {
    let mut applied = 0usize;
    for function in &mut report.functions {
        let mut summary = DecompileTraceSummary::default();
        let mut pc_hits: BTreeMap<(u16, u16), DecompileTracePcHit> = BTreeMap::new();
        let mut slot_value_profiles = BTreeMap::<i16, TraceValueProfile>::new();
        let mut last_slot_values = BTreeMap::<i16, u16>::new();
        let mut matched_any = false;
        for observation in observations {
            if observation.rom_bank != function.bank
                || !function_contains_pc(function, observation.pc)
            {
                continue;
            }
            matched_any = true;
            // A supplied zero count still contributes one hit; timestamps bound the summary but do not reorder observations.
            let observation_hit_count = observation.hit_count.max(1);
            summary.hit_count = summary.hit_count.saturating_add(observation_hit_count);
            let frame = observation.completed_frames.or(observation.active_frame);
            summary.first_frame = min_optional(summary.first_frame, frame);
            summary.last_frame = max_optional(summary.last_frame, frame);
            summary.first_cycle = min_optional(summary.first_cycle, observation.cycle);
            summary.last_cycle = max_optional(summary.last_cycle, observation.cycle);
            push_unique_limited(
                &mut summary.observed_symbols,
                observation.symbol.clone().filter(|s| !s.is_empty()),
                6,
            );
            push_unique_limited(
                &mut summary.observed_sources,
                observation.source.clone().filter(|s| !s.is_empty()),
                6,
            );
            push_unique_limited(
                &mut summary.event_samples,
                observation
                    .event_summary
                    .clone()
                    .filter(|s| !s.is_empty() && s != "<none>"),
                6,
            );
            push_unique_limited(
                &mut summary.watch_samples,
                observation.watch_summary.clone().filter(|s| !s.is_empty()),
                6,
            );
            let hit = pc_hits
                .entry((observation.rom_bank, observation.pc))
                .or_insert_with(|| DecompileTracePcHit {
                    rom_bank: observation.rom_bank,
                    pc: observation.pc,
                    hit_count: 0,
                    symbol: observation.symbol.clone(),
                    source: observation.source.clone(),
                });
            hit.hit_count = hit.hit_count.saturating_add(observation_hit_count);

            for (offset, value) in normalized_trace_stack_slot_values(observation) {
                let profile = slot_value_profiles.entry(offset).or_default();
                profile.values.insert(value);
                if looks_like_pointer_value(value) {
                    profile.likely_pointer_hits = profile.likely_pointer_hits.saturating_add(1);
                }
                if let Some(previous) = last_slot_values.insert(offset, value) {
                    if previous.wrapping_add(1) == value || previous.wrapping_sub(1) == value {
                        profile.repeated_increments = profile.repeated_increments.saturating_add(1);
                    }
                    if (previous == 0 && value == 1) || (previous == 1 && value == 0) {
                        profile.repeated_toggles = profile.repeated_toggles.saturating_add(1);
                    }
                }
            }
        }
        if matched_any {
            let mut hot_pcs = pc_hits.into_values().collect::<Vec<_>>();
            hot_pcs.sort_by(|a, b| {
                b.hit_count
                    .cmp(&a.hit_count)
                    .then_with(|| a.rom_bank.cmp(&b.rom_bank))
                    .then_with(|| a.pc.cmp(&b.pc))
            });
            hot_pcs.truncate(6);
            summary.hot_pcs = hot_pcs;
            refine_variable_hints_from_trace(function, &summary, &slot_value_profiles);
            refine_call_sites_from_trace(function, observations);
            let hot_block_starts = summary
                .hot_pcs
                .iter()
                .map(|hit| hit.pc)
                .collect::<BTreeSet<_>>();
            // This label uses only the six retained hottest PCs, so an observed block outside that preview can appear cold.
            let cold_blocks = function
                .blocks
                .iter()
                .filter(|block| {
                    !hot_block_starts
                        .iter()
                        .any(|pc| *pc >= block.start_address && *pc < block.end_address)
                })
                .map(|block| block.id.clone())
                .take(4)
                .collect::<Vec<_>>();
            if !cold_blocks.is_empty() {
                function.artifact.suggestions.push(format!(
                    "cold blocks not seen in current trace: {}",
                    cold_blocks.join(", ")
                ));
            }
            function.artifact.trace_summary = Some(summary.clone());
            push_trace_summary_annotations(&mut function.artifact.trace_annotations, &summary);
            applied = applied.saturating_add(1);
        }
    }
    if applied > 0 {
        report.notes.push(format!(
            "trace-assisted decompile overlay applied to {} function(s) from {} observation(s)",
            applied,
            observations.len()
        ));
    }
    applied
}

// Match a parsed bank/address exactly, or match current name, canonical name or ID ignoring ASCII case.
fn function_selector_matches(function: &DecompileFunctionInfo, selector: &str) -> bool {
    if let Some((bank, addr)) = parse_bank_addr_selector(selector) {
        return function.bank == bank && function.start_address == addr;
    }
    let selector = selector.to_ascii_lowercase();
    function.name.to_ascii_lowercase() == selector
        || function.canonical_name.to_ascii_lowercase() == selector
        || function.id.to_ascii_lowercase() == selector
}

// Trim selection strings and drop empty values before report filtering.
fn parse_selected_filters(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

// Match selected names or bank/address pairs. With no filters, include-all accepts every
// source kind; its name does not imply a separate named-only restriction in this helper.
fn match_selected_filter(
    function: &FunctionAnalysis,
    filters: &[String],
    options: &DecompileOptions,
) -> bool {
    if filters.is_empty() {
        return options.include_all_named_functions
            || function.source_kind == "inferred"
            || function.source_kind == "metadata"
            || function.source_kind == "symbol";
    }
    filters.iter().any(|filter| {
        if filter.eq_ignore_ascii_case(&function.name) {
            return true;
        }
        if let Some((bank, addr)) = parse_bank_addr_selector(filter) {
            return bank == function.bank && addr == function.start;
        }
        false
    })
}

// Parse addresses as hexadecimal. Banks with a hex prefix or at most two hex digits
// are hexadecimal; longer unprefixed bank strings use decimal parsing.
fn parse_bank_addr_selector(text: &str) -> Option<(u16, u16)> {
    let (bank_text, addr_text) = text.split_once(':')?;
    let bank = if bank_text.starts_with("0x") || bank_text.starts_with("0X") {
        u16::from_str_radix(
            bank_text.trim_start_matches("0x").trim_start_matches("0X"),
            16,
        )
        .ok()?
    } else if bank_text.chars().all(|c| c.is_ascii_hexdigit()) && bank_text.len() <= 2 {
        u16::from_str_radix(bank_text, 16).ok()?
    } else {
        bank_text.parse::<u16>().ok()?
    };
    let addr_text = addr_text.trim_start_matches("0x").trim_start_matches("0X");
    let addr = u16::from_str_radix(addr_text, 16).ok()?;
    Some((bank, addr))
}

// Classify the function name and generated names for call destinations, deduplicating
// canonical intrinsic names. Generated destination names do not incorporate symbol lookup.
fn classify_intrinsics(function: &FunctionAnalysis) -> Vec<KitaqgbIntrinsicMatch> {
    let mut out = Vec::new();
    if let Some(m) = classify_symbol_name(&function.name) {
        out.push(m);
    }
    for xref in &function.xrefs_out {
        if let Some(name) = xref_name_guess(xref) {
            if let Some(m) = classify_symbol_name(&name) {
                if !out
                    .iter()
                    .any(|existing| existing.canonical_name == m.canonical_name)
                {
                    out.push(m);
                }
            }
        }
    }
    out
}

// Collect sorted, unique helper-shape labels from names, text patterns and call hints.
// These labels suggest code roles; they do not prove compiler provenance or behavior.
fn infer_kitaqgb_fingerprints(
    function: &FunctionAnalysis,
    call_sites: &[DecompileCallSiteHint],
) -> Vec<String> {
    let mut out = Vec::new();
    let lower_name = function.name.to_ascii_lowercase();
    let upper_name = function.name.to_ascii_uppercase();
    let instruction_texts = function
        .instructions
        .values()
        .map(ins_display_text_raw)
        .collect::<Vec<_>>();
    let upper_texts = instruction_texts
        .iter()
        .map(|text| text.to_ascii_uppercase())
        .collect::<Vec<_>>();
    if instruction_texts
        .iter()
        .any(|text| text.eq_ignore_ascii_case("JP HL"))
        && !call_sites.is_empty()
    {
        out.push("kitaqgb_indirect_dispatch_or_switch".to_string());
    }
    if upper_texts
        .iter()
        .filter(|text| *text == "LD [HL+],A" || *text == "LD A,[HL+]" || *text == "LD [HL-],A")
        .count()
        >= 2
        || lower_name.contains("memcpy")
    {
        out.push("kitaqgb_memcpy_like_helper".to_string());
    }
    if upper_texts.iter().any(|text| text.starts_with("LD [HL],"))
        && upper_texts.iter().any(|text| text == "INC HL")
        && upper_texts
            .iter()
            .any(|text| text == "DEC BC" || text == "DEC C")
        && upper_texts.iter().any(|text| text.starts_with("JR NZ"))
        || lower_name.contains("memset")
    {
        out.push("kitaqgb_memset_like_loop".to_string());
    }
    if upper_texts.iter().any(|text| text.contains("__KQ_THUNK_B"))
        || upper_texts
            .iter()
            .any(|text| text.contains("SWITCH_ROM_BANK"))
        || upper_name.starts_with("__KQ_THUNK_B")
    {
        out.push("kitaqgb_bank_thunk_shape".to_string());
    }
    if upper_texts.iter().any(|text| text.contains("PAD_READ"))
        || upper_texts.iter().any(|text| text.contains("UPDATEINPUT"))
        || upper_texts.iter().any(|text| text.contains("CLEARINPUT"))
        || lower_name.contains("updateinput")
        || lower_name.contains("clearinput")
        || lower_name.contains("pad_read")
    {
        out.push("kitaqgb_input_helper_shape".to_string());
    }
    if upper_texts.iter().any(|text| text.contains("__SETTILE"))
        || upper_texts.iter().any(|text| text.contains("FLUSHTILE"))
        || lower_name.contains("settile")
        || lower_name.contains("tile")
    {
        out.push("kitaqgb_tile_helper_shape".to_string());
    }
    if lower_name.starts_with("cgb_bg_")
        || lower_name.starts_with("cgb_obj_")
        || lower_name.contains("cgb_palette")
    {
        out.push("kitaqgb_cgb_palette_helper".to_string());
    }
    if upper_texts
        .iter()
        .filter(|text| text.starts_with("LD HL,SP+") || text.starts_with("LD HL,SP-"))
        .count()
        >= 2
    {
        out.push("kitaqgb_lowerer_stack_frame_shape".to_string());
    }
    if call_sites
        .iter()
        .any(|site| site.calling_convention_hint.as_deref() == Some("fastcall"))
    {
        out.push("kitaqgb_fastcall_helper_shape".to_string());
    }
    if function.name.to_ascii_lowercase().contains("farcall")
        || call_sites.iter().any(|site| {
            site.target_name
                .as_deref()
                .map(|name| name.to_ascii_lowercase().contains("farcall"))
                .unwrap_or(false)
        })
    {
        out.push("kitaqgb_farcall_named_helper".to_string());
    }
    if let Some(m) = classify_symbol_name(&function.name) {
        out.push(format!("kitaqgb_intrinsic::{}", m.canonical_name));
    }
    out.sort();
    out.dedup();
    out
}

// Recognize the thunk prefix ignoring ASCII case, skip decimal bank digits and an optional
// underscore, then return the nonempty target suffix without validating the bank number.
fn parse_kitaqgb_bank_thunk_target_name(name: &str) -> Option<String> {
    let prefix = "__kq_thunk_b";
    let lower = name.to_ascii_lowercase();
    if !lower.starts_with(prefix) {
        return None;
    }
    let suffix = &name[prefix.len()..];
    let mut chars = suffix.chars();
    while matches!(chars.clone().next(), Some(c) if c.is_ascii_digit()) {
        chars.next();
    }
    let remainder = chars.as_str();
    let target = remainder.strip_prefix('_').unwrap_or(remainder).trim();
    (!target.is_empty()).then(|| target.to_string())
}

// Turn boundary, slot, call and helper-pattern hints into manual-review suggestions.
// The suggestions describe candidate interpretations, not verified signatures or recovered types.
fn build_initial_suggestions(
    function: &FunctionAnalysis,
    stack_slots: &[DecompileStackSlotHint],
    temp_slots: &[DecompileTempSlotHint],
    call_sites: &[DecompileCallSiteHint],
    switch_candidates: &[SwitchCandidateInfo],
) -> Vec<String> {
    let mut out = Vec::new();
    if function.source_kind == "recovered" {
        out.push(
            "recovered function boundary: validate entry point and rename if needed".to_string(),
        );
    }
    if let Some(target_name) = parse_kitaqgb_bank_thunk_target_name(&function.name) {
        out.push(format!(
            "bank thunk wrapper forwards into {}; argument order is usually inherited from the target helper",
            target_name
        ));
    }
    if function
        .warnings
        .iter()
        .any(|warning| warning.contains("invalid opcode"))
    {
        out.push(
            "function body contains invalid opcodes; code/data split may need manual review"
                .to_string(),
        );
    }
    for slot in stack_slots {
        if slot.name.starts_with("arg_")
            && slot.runtime_role.as_deref() == Some("runtime_flag_like")
        {
            out.push(format!(
                "{} looks bool-like; consider a flag-style argument name",
                slot.name
            ));
        } else if slot.inferred_type.as_deref() == Some("pointer_candidate") {
            let prefix = if slot
                .trace_value_hints
                .iter()
                .any(|hint| hint.contains("$C"))
            {
                "wram_"
            } else {
                ""
            };
            out.push(format!(
                "{} looks pointer-like; {}ptr/buffer naming may fit",
                slot.name, prefix
            ));
        } else if slot.inferred_type.as_deref() == Some("enum_like_candidate") {
            out.push(format!(
                "{} stays within a small set; enum/state naming may fit",
                slot.name
            ));
        }
    }
    if temp_slots
        .iter()
        .any(|slot| slot.runtime_role.as_deref() == Some("runtime_spill_like"))
    {
        out.push("temporary spill slots observed at runtime; nearby call boundaries may expose helper ABI".to_string());
    }
    if call_sites
        .iter()
        .any(|site| site.calling_convention_hint.as_deref() == Some("mixed_call"))
    {
        out.push(
            "mixed register+stack argument setup detected; consider annotating helper signature"
                .to_string(),
        );
    }
    if call_sites
        .iter()
        .any(|site| site.calling_convention_hint.as_deref() == Some("fastcall"))
    {
        out.push(
            "fastcall-like helper call detected; register argument naming may improve readability"
                .to_string(),
        );
    }
    if !switch_candidates.is_empty() {
        out.push(
            "switch-like dispatch detected; verify jump table width and label candidate cases"
                .to_string(),
        );
    }
    if function.name.starts_with("sub_")
        && !call_sites.is_empty()
        && call_sites.iter().all(|site| site.hit_count > 0)
    {
        out.push(
            "runtime-observed unnamed helper: consider promoting to a semantic function name"
                .to_string(),
        );
    }
    if function.name.to_ascii_lowercase().contains("memcpy")
        || function.name.to_ascii_lowercase().contains("memset")
    {
        out.push("helper fingerprint matches common runtime primitive; verify argument order and lengths".to_string());
    }
    if function.name.to_ascii_lowercase().starts_with("cgb_bg_")
        || function.name.to_ascii_lowercase().starts_with("cgb_obj_")
    {
        out.push("CGB palette helper detected; color-slot arguments and palette index naming may improve readability".to_string());
    }
    out
}

// Generate an automatic function name only for a call/RST with a resolved target bank.
fn xref_name_guess(xref: &XrefInfo) -> Option<String> {
    if xref.kind.starts_with("call") || xref.kind == "rst" {
        if let Some(bank) = xref.to_bank {
            return Some(auto_function_name(bank, xref.to_addr));
        }
    }
    None
}

// Index analyzed names by exact bank and function entry address.
fn build_function_name_lookup(functions: &[FunctionAnalysis]) -> BTreeMap<(u16, u16), String> {
    functions
        .iter()
        .map(|f| ((f.bank, f.start), f.name.clone()))
        .collect()
}

// Attach incoming references only when their resolved destination equals a known function entry.
fn build_xrefs_in_map(
    xrefs: &[XrefInfo],
    functions: &BTreeMap<(u16, u16), String>,
) -> BTreeMap<(u16, u16), Vec<XrefInfo>> {
    let mut map: BTreeMap<(u16, u16), Vec<XrefInfo>> = BTreeMap::new();
    for xref in xrefs {
        if let Some(bank) = xref.to_bank {
            if functions.contains_key(&(bank, xref.to_addr)) {
                map.entry((bank, xref.to_addr))
                    .or_default()
                    .push(xref.clone());
            }
        }
    }
    map
}

// Sort references by source/destination/kind and remove adjacent fully equal entries.
fn collect_global_xrefs(functions: &[FunctionAnalysis]) -> Vec<XrefInfo> {
    let mut all = functions
        .iter()
        .flat_map(|function| function.xrefs_out.iter().cloned())
        .collect::<Vec<_>>();
    all.sort_by_key(|xref| {
        (
            xref.from_bank,
            xref.from_addr,
            xref.to_bank.unwrap_or(0xFFFF),
            xref.to_addr,
            xref.kind.clone(),
        )
    });
    all.dedup();
    all
}

// Walk reachable instruction starts inside one ROM window, stopping at other known roots.
// Follow in-window jumps and call fallthrough; resolved callees are discovered by the outer pass.
// The fixed confidence score summarizes available hints and is not a measured probability.
fn analyze_function(
    rom: &[u8],
    rom_bank_count: u16,
    symbol_table: Option<&SymbolTable>,
    known_roots: &BTreeSet<(u16, u16)>,
    bank: u16,
    start: u16,
    name: String,
    source_kind: String,
    metadata: Option<FunctionInfo>,
) -> FunctionAnalysis {
    let window = if bank == 0 {
        WindowKind::Fixed
    } else {
        WindowKind::Switchable
    };
    let mut queue = VecDeque::from([start]);
    let mut instructions = BTreeMap::new();
    let mut xrefs_out = Vec::new();
    let mut warnings = Vec::new();
    let mut invalid_count = 0u32;

    while let Some(addr) = queue.pop_front() {
        if !window.contains(addr) || instructions.contains_key(&addr) {
            continue;
        }
        if addr != start && known_roots.contains(&(bank, addr)) {
            continue;
        }
        let ins = decode_instruction(rom, rom_bank_count, bank, addr);
        if ins.is_invalid {
            invalid_count += 1;
            warnings.push(format!("invalid opcode at {:02X}:{:04X}", bank, addr));
        }
        let next = addr.saturating_add(ins.len as u16);
        if let Some(target_addr) = ins.target_addr {
            let target_bank =
                resolve_target_bank(bank, target_addr, rom_bank_count, symbol_table, &ins);
            let kind = match ins.flow_kind {
                FlowKind::Call => "call",
                FlowKind::CallCond => "call_cond",
                FlowKind::Jump => "jump",
                FlowKind::JumpCond => "jump_cond",
                FlowKind::Rst => "rst",
                _ => "ref",
            }
            .to_string();
            xrefs_out.push(XrefInfo {
                from_bank: bank,
                from_addr: addr,
                to_bank: target_bank,
                to_addr: target_addr,
                kind,
                confidence: if target_bank.is_some() {
                    "high".to_string()
                } else {
                    "medium".to_string()
                },
            });
            match ins.flow_kind {
                FlowKind::Jump | FlowKind::JumpCond => {
                    if target_bank == Some(bank)
                        && window.contains(target_addr)
                        && !known_roots.contains(&(bank, target_addr))
                    {
                        queue.push_back(target_addr);
                    }
                    if ins.fallthrough && window.contains(next) {
                        queue.push_back(next);
                    }
                }
                FlowKind::Call | FlowKind::CallCond | FlowKind::Rst => {
                    if ins.fallthrough && window.contains(next) {
                        queue.push_back(next);
                    }
                }
                // The target-bearing non-control case currently adds no next address here;
                // fallthrough is queued below only when target_addr is absent.
                _ => {}
            }
        } else if ins.fallthrough && window.contains(next) {
            queue.push_back(next);
        }
        instructions.insert(addr, ins);
    }

    let mut confidence = 0.45f32;
    if source_kind != "inferred" {
        confidence += 0.15;
    }
    if invalid_count == 0 {
        confidence += 0.15;
    }
    if !xrefs_out.is_empty() {
        confidence += 0.10;
    }
    if metadata.is_some() {
        confidence += 0.10;
    }

    FunctionAnalysis {
        bank,
        start,
        name,
        source_kind,
        instructions,
        xrefs_out,
        warnings,
        confidence_score: confidence.clamp(0.0, 1.0),
        metadata,
    }
}

// Split decoded starts at selected branch/call leaders, connect local successors and
// annotate structural candidates using dominators. This approximate graph is for inspection,
// not a proof that emitted structured pseudocode preserves every execution path.
fn build_basic_blocks(
    function: &FunctionAnalysis,
    xrefs: &[XrefInfo],
) -> Vec<DecompileBasicBlockInfo> {
    if function.instructions.is_empty() {
        return Vec::new();
    }
    let mut leaders = BTreeSet::new();
    leaders.insert(function.start);
    for ins in function.instructions.values() {
        let next = ins.addr.saturating_add(ins.len as u16);
        match ins.flow_kind {
            FlowKind::Jump
            | FlowKind::JumpCond
            | FlowKind::Call
            | FlowKind::CallCond
            | FlowKind::Rst => {
                if ins.fallthrough && function.instructions.contains_key(&next) {
                    leaders.insert(next);
                }
                if let Some(target) = ins.target_addr {
                    if function.instructions.contains_key(&target) {
                        leaders.insert(target);
                    }
                }
            }
            _ => {}
        }
    }

    let leader_list = leaders.iter().copied().collect::<Vec<_>>();
    let mut blocks = Vec::new();
    for (index, leader) in leader_list.iter().enumerate() {
        let end_limit = leader_list.get(index + 1).copied();
        let mut block_instructions = Vec::new();
        for (&addr, ins) in function.instructions.range(*leader..) {
            if Some(addr) == end_limit {
                break;
            }
            block_instructions.push(ins.clone());
            if !ins.fallthrough
                || matches!(
                    ins.flow_kind,
                    FlowKind::Jump
                        | FlowKind::Return
                        | FlowKind::ReturnCond
                        | FlowKind::Reti
                        | FlowKind::IndirectJump
                )
            {
                break;
            }
        }
        if block_instructions.is_empty() {
            continue;
        }
        let start_address = block_instructions.first().unwrap().addr;
        let end_address = block_instructions
            .last()
            .map(|ins| ins.addr.saturating_add(ins.len as u16))
            .unwrap_or(start_address);
        blocks.push(DecompileBasicBlockInfo {
            id: format!("bb_{:02X}_{:04X}", function.bank, start_address),
            start_address,
            end_address,
            instructions: block_instructions
                .into_iter()
                .map(|ins| to_decoded_instruction(function.bank, ins))
                .collect(),
            predecessors: Vec::new(),
            successors: Vec::new(),
            loop_role: None,
            structure_role: None,
        });
    }

    let mut block_index_by_addr = BTreeMap::new();
    for (index, block) in blocks.iter().enumerate() {
        block_index_by_addr.insert(block.start_address, index);
    }

    let mut edges: Vec<(usize, usize)> = Vec::new();
    for i in 0..blocks.len() {
        let Some(last) = blocks[i].instructions.last().cloned() else {
            continue;
        };
        // Successors are chosen from sorted block starts rather than exact next instruction addresses.
        let next_block = blocks.get(i + 1).map(|block| block.start_address);
        match last.flow_kind.as_str() {
            "jump" => {
                if let Some(target) = last
                    .target_addr
                    .and_then(|addr| block_index_by_addr.get(&addr).copied())
                {
                    edges.push((i, target));
                }
            }
            "jump_cond" => {
                if let Some(target) = last
                    .target_addr
                    .and_then(|addr| block_index_by_addr.get(&addr).copied())
                {
                    edges.push((i, target));
                }
                if let Some(next_addr) =
                    next_block.and_then(|addr| block_index_by_addr.get(&addr).copied())
                {
                    edges.push((i, next_addr));
                }
            }
            "call" | "call_cond" | "rst" | "halt" | "stop" | "none" | "invalid" => {
                if let Some(next_addr) =
                    next_block.and_then(|addr| block_index_by_addr.get(&addr).copied())
                {
                    edges.push((i, next_addr));
                }
            }
            _ => {}
        }
    }

    for (from, to) in &edges {
        let from_id = blocks[*from].id.clone();
        let to_id = blocks[*to].id.clone();
        if !blocks[*from].successors.contains(&to_id) {
            blocks[*from].successors.push(to_id.clone());
        }
        if !blocks[*to].predecessors.contains(&from_id) {
            blocks[*to].predecessors.push(from_id);
        }
    }

    let dominators = compute_dominators(&blocks);
    let post_dominators = compute_post_dominators(&blocks);
    for i in 0..blocks.len() {
        if i == 0 {
            blocks[i].structure_role = Some("entry".to_string());
        }
        if blocks[i].successors.is_empty() {
            blocks[i].structure_role = Some("exit".to_string());
        }
    }
    for (from, to) in edges {
        if dominators[from].contains(&to) {
            blocks[to].loop_role = Some("loop_header".to_string());
            if blocks[from].loop_role.is_none() {
                blocks[from].loop_role = Some("loop_latch".to_string());
            }
        }
    }
    for index in 0..blocks.len() {
        if blocks[index].successors.len() == 2 {
            let left = blocks[index]
                .successors
                .first()
                .and_then(|id| block_index_by_addr.get(&parse_block_id_addr(id)?).copied());
            let right = blocks[index]
                .successors
                .get(1)
                .and_then(|id| block_index_by_addr.get(&parse_block_id_addr(id)?).copied());
            if let (Some(left_idx), Some(right_idx)) = (left, right) {
                let shared_postdom = post_dominators[left_idx]
                    .intersection(&post_dominators[right_idx])
                    .copied()
                    .find(|candidate| *candidate != index);
                blocks[index].structure_role = Some(if shared_postdom.is_some() {
                    "if_else_header".to_string()
                } else {
                    "if_header".to_string()
                });
            }
        } else if blocks[index]
            .instructions
            .last()
            .map(|ins| ins.flow_kind.as_str() == "indirect_jump")
            .unwrap_or(false)
        {
            blocks[index].structure_role = Some("switch_header".to_string());
        }
    }

    // The target role below currently tests reference source addresses, not destination addresses.
    let function_targets = xrefs
        .iter()
        .filter(|xref| xref.from_bank == function.bank)
        .map(|xref| xref.from_addr)
        .collect::<BTreeSet<_>>();
    for block in &mut blocks {
        if function_targets.contains(&block.start_address) && block.structure_role.is_none() {
            block.structure_role = Some("target".to_string());
        }
    }

    blocks
}

// Read the final underscore-delimited block-ID component as a hexadecimal address.
fn parse_block_id_addr(id: &str) -> Option<u16> {
    id.rsplit('_')
        .next()
        .and_then(|hex| u16::from_str_radix(hex, 16).ok())
}

// Combine static listings, graph sets and candidate variable/call/helper hints.
// Trace annotations initially contain static notices; a runtime summary is absent until overlaid.
fn build_artifact(
    function: &FunctionAnalysis,
    blocks: &[DecompileBasicBlockInfo],
    switch_candidates: &[SwitchCandidateInfo],
    function_lookup: &BTreeMap<(u16, u16), String>,
) -> DecompileArtifact {
    let (stack_slots, temp_slots, context) = infer_stack_and_temp_hints(function);
    let call_sites = infer_call_site_hints(function, function_lookup);
    let disassembly_text = render_disassembly(function, function_lookup);
    let pseudocode_text = render_pseudocode(
        function,
        blocks,
        function_lookup,
        &context,
        &stack_slots,
        &temp_slots,
    );
    let dominators = compute_dominators(blocks);
    let post_dominators = compute_post_dominators(blocks);
    let cfg_json = serde_json::json!({
        "blocks": blocks,
        "switch_candidates": switch_candidates,
        "dominators": dominators.iter().map(|set| set.iter().copied().collect::<Vec<_>>()).collect::<Vec<_>>(),
        "post_dominators": post_dominators.iter().map(|set| set.iter().copied().collect::<Vec<_>>()).collect::<Vec<_>>(),
    });
    let mut trace_annotations = Vec::new();
    if !switch_candidates.is_empty() {
        trace_annotations
            .push("static annotation: switch/jump-table candidate detected".to_string());
    }
    if function
        .xrefs_out
        .iter()
        .any(|xref| xref.to_bank.is_none() && xref.kind.contains("call"))
    {
        trace_annotations
            .push("static annotation: ambiguous cross-bank call remains unresolved".to_string());
    }
    let fingerprints = infer_kitaqgb_fingerprints(function, &call_sites);
    let suggestions = build_initial_suggestions(
        function,
        &stack_slots,
        &temp_slots,
        &call_sites,
        switch_candidates,
    );
    DecompileArtifact {
        function_id: format!("bank{:02X}:{:04X}", function.bank, function.start),
        disassembly_text,
        cfg_json,
        pseudocode_text,
        warnings: function.warnings.clone(),
        confidence_score: function.confidence_score,
        stack_slots,
        temp_slots,
        call_sites,
        fingerprints,
        suggestions,
        trace_annotations,
        trace_summary: None,
    }
}

// Render bank/address, available bytes and decoded text, adding target names when the
// render-time bank assumption resolves them. This is a listing, not round-trip assembly source.
fn render_disassembly(
    function: &FunctionAnalysis,
    function_lookup: &BTreeMap<(u16, u16), String>,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "; function {} ({:02X}:{:04X})",
        function.name, function.bank, function.start
    ));
    for (addr, ins) in &function.instructions {
        if *addr == function.start {
            lines.push(format!("{}:", function.name));
        }
        if let Some(target) = ins.target_addr {
            if let Some(target_bank) = resolve_render_bank(function.bank, target) {
                if let Some(name) = function_lookup.get(&(target_bank, target)) {
                    lines.push(format!("; -> {}", name));
                }
            }
        }
        let bytes = ins
            .bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        lines.push(format!(
            "{:02X}:{:04X}  {:<10} {}",
            ins.bank,
            ins.addr,
            bytes,
            ins_display_text_raw(ins)
        ));
    }
    lines.join("\n")
}

// Render readable instruction summaries and tentative loop/branch structure with labels.
// This output is explanatory pseudocode: it is not executable C or a semantics-preserving translation.
fn render_pseudocode(
    function: &FunctionAnalysis,
    blocks: &[DecompileBasicBlockInfo],
    function_lookup: &BTreeMap<(u16, u16), String>,
    context: &PseudocodeContext,
    stack_slots: &[DecompileStackSlotHint],
    temp_slots: &[DecompileTempSlotHint],
) -> String {
    let mut lines = Vec::new();
    lines.push(format!("{}() {{", function.name));
    if blocks.is_empty() {
        lines.push("    /* no decoded blocks */".to_string());
        lines.push("}".to_string());
        return lines.join("\n");
    }
    if !stack_slots.is_empty() {
        lines.push("    /* inferred stack slots:".to_string());
        for slot in stack_slots {
            lines.push(format!(
                "       {} ({}, {}, SP{:+}{}, r{}, w{}, patterns=[{}]{}{}) */",
                slot.name,
                slot.kind,
                slot.inferred_type.as_deref().unwrap_or("unknown"),
                slot.offset,
                slot.runtime_role
                    .as_ref()
                    .map(|role| format!(", {}", role))
                    .unwrap_or_default(),
                slot.read_count,
                slot.write_count,
                slot.access_patterns.join(", "),
                slot.lifetime_hint
                    .as_ref()
                    .map(|hint| format!(", lifetime={hint}"))
                    .unwrap_or_default(),
                if slot.trace_value_hints.is_empty() {
                    String::new()
                } else {
                    format!(", trace=[{}]", slot.trace_value_hints.join("; "))
                }
            ));
        }
    }
    if !temp_slots.is_empty() {
        lines.push("    /* inferred temp slots:".to_string());
        for temp in temp_slots {
            lines.push(format!(
                "       {} ({}, {}{}, r{}, w{}, patterns=[{}]) */",
                temp.name,
                temp.kind,
                temp.inferred_type.as_deref().unwrap_or("unknown"),
                temp.runtime_role
                    .as_ref()
                    .map(|role| format!(", {}", role))
                    .unwrap_or_default(),
                temp.read_count,
                temp.write_count,
                temp.access_patterns.join(", ")
            ));
        }
    }
    if !function.xrefs_out.is_empty() {
        let call_sites = function
            .xrefs_out
            .iter()
            .filter(|xref| xref.kind.contains("call") || xref.kind == "rst")
            .count();
        if call_sites > 0 {
            lines.push(format!("    /* inferred call sites: {} */", call_sites));
        }
    }

    let block_index_by_addr = blocks
        .iter()
        .enumerate()
        .map(|(i, block)| (block.start_address, i))
        .collect::<BTreeMap<_, _>>();
    let dominators = compute_dominators(blocks);
    let post_dominators = compute_post_dominators(blocks);
    let control_flow_hints = analyze_control_flow_hints(blocks, &dominators, &post_dominators);
    let mut loop_headers = BTreeMap::<usize, BTreeSet<usize>>::new();
    for (i, block) in blocks.iter().enumerate() {
        for succ in &block.successors {
            if let Some(target_addr) = succ
                .strip_prefix(&format!("bb_{:02X}_", function.bank))
                .and_then(|hex| u16::from_str_radix(hex, 16).ok())
                .and_then(|addr| block_index_by_addr.get(&addr).copied())
            {
                if dominators[i].contains(&target_addr) {
                    loop_headers.entry(target_addr).or_default().insert(i);
                }
            }
        }
    }

    // Loop rendering emits headers and recorded latches, not a complete natural-loop region.
    // Latches are not marked emitted here and may also appear in the outer block listing.
    let mut emitted_loops = BTreeSet::new();
    let mut emitted_blocks = BTreeSet::new();
    let mut i = 0usize;
    while i < blocks.len() {
        if loop_headers.contains_key(&i) && !emitted_loops.contains(&i) {
            emitted_loops.insert(i);
            lines.push(format!("    while (true) {{ /* {} */", blocks[i].id));
            let header = &blocks[i];
            emit_block_body(&mut lines, header, function_lookup, context, 2);
            if header.successors.len() >= 2 {
                let target_a = &header.successors[0];
                let target_b = &header.successors[1];
                let back_to_header = |succ: &String| succ == &header.id;
                if back_to_header(target_a) || back_to_header(target_b) {
                    lines.push("        continue;".to_string());
                } else {
                    lines.push(format!(
                        "        if ({}) break;",
                        branch_condition_from_block(header).unwrap_or("cond")
                    ));
                }
            }
            if let Some(latches) = loop_headers.get(&i) {
                for latch_index in latches {
                    if *latch_index != i && *latch_index < blocks.len() {
                        emit_block_body(
                            &mut lines,
                            &blocks[*latch_index],
                            function_lookup,
                            context,
                            2,
                        );
                    }
                }
            }
            lines.push("    }".to_string());
            emitted_blocks.insert(i);
            i += 1;
            continue;
        }
        if emitted_blocks.insert(i) {
            emit_block_with_structure(
                &mut lines,
                &blocks[i],
                function_lookup,
                context,
                &control_flow_hints,
                i,
            );
        }
        i += 1;
    }

    lines.push("}".to_string());
    lines.join("\n")
}

// Emit a block body followed by return/jump notation and structure hints.
// Conditional jump notation requires two recorded successors; unresolved targets stay explicit.
fn emit_block_with_structure(
    lines: &mut Vec<String>,
    block: &DecompileBasicBlockInfo,
    function_lookup: &BTreeMap<(u16, u16), String>,
    context: &PseudocodeContext,
    control_flow_hints: &ControlFlowHints,
    block_index: usize,
) {
    if block.structure_role.as_deref() != Some("entry") {
        lines.push(format!("    {}:", block.id));
    }
    if control_flow_hints.if_else_headers.contains(&block_index) {
        lines.push("    /* if/else header */".to_string());
    } else if control_flow_hints.if_headers.contains(&block_index) {
        lines.push("    /* if header */".to_string());
    } else if block.structure_role.as_deref() == Some("switch_header") {
        lines.push("    /* switch-like dispatch */".to_string());
    }
    emit_block_body(lines, block, function_lookup, context, 1);
    if let Some(last) = block.instructions.last() {
        match last.flow_kind.as_str() {
            "return" | "reti" => lines.push("    return;".to_string()),
            "return_cond" => lines.push(format!(
                "    if ({}) return;",
                branch_condition_for_instruction(last).unwrap_or("cond")
            )),
            "jump_cond" => {
                if block.successors.len() == 2 {
                    let true_target = &block.successors[0];
                    let false_target = &block.successors[1];
                    lines.push(format!(
                        "    if ({}) goto {}; else goto {};",
                        branch_condition_for_instruction(last).unwrap_or("cond"),
                        true_target,
                        false_target
                    ));
                }
            }
            "jump" => {
                if let Some(target_addr) = last.target_addr {
                    if let Some(target_bank) = last.target_bank {
                        let target_name = function_lookup
                            .get(&(target_bank, target_addr))
                            .cloned()
                            .unwrap_or_else(|| auto_label_name(target_bank, target_addr, "jump"));
                        lines.push(format!("    goto {};", target_name));
                    } else {
                        lines.push(format!("    goto /* ambiguous */ 0x{:04X};", target_addr));
                    }
                }
            }
            "indirect_jump" => lines.push("    goto *HL;".to_string()),
            _ => {}
        }
    }
    if control_flow_hints.break_blocks.contains(&block_index) {
        lines.push("    /* break-like edge observed */".to_string());
    }
    if control_flow_hints.continue_blocks.contains(&block_index) {
        lines.push("    /* continue-like edge observed */".to_string());
    }
}

// Start fresh HL alias tracking for each block, emit call summaries separately and
// leave other control instructions to the outer renderer. Update aliases after each instruction.
fn emit_block_body(
    lines: &mut Vec<String>,
    block: &DecompileBasicBlockInfo,
    function_lookup: &BTreeMap<(u16, u16), String>,
    context: &PseudocodeContext,
    indent_level: usize,
) {
    let indent = "    ".repeat(indent_level);
    let mut state = BlockRenderState::default();
    for ins in &block.instructions {
        if is_control_instruction(ins) {
            if ins.flow_kind == "call" || ins.flow_kind == "call_cond" || ins.flow_kind == "rst" {
                lines.push(format!(
                    "{}{};",
                    indent,
                    call_statement(ins, function_lookup)
                ));
            }
            update_render_state(&mut state, ins);
            continue;
        }
        lines.push(format!(
            "{}{}",
            indent,
            instruction_to_statement(ins, context, &mut state)
        ));
        update_render_state(&mut state, ins);
    }
}

// Translate recognized instruction text into compact statements and retain unhandled
// instructions as comments. Flag effects, delayed EI and bus timing are not fully represented.
fn instruction_to_statement(
    ins: &DecodedInstruction,
    context: &PseudocodeContext,
    state: &mut BlockRenderState,
) -> String {
    let text = ins.text.to_ascii_lowercase();
    if text == "nop" {
        return "/* nop */".to_string();
    }
    if let Some(offset) = parse_sp_relative_offset(&text) {
        return format!("HL = &{};", stack_slot_name(offset, context));
    }
    if let Some(rest) = text.strip_prefix("ld ") {
        if let Some((lhs, rhs)) = rest.split_once(',') {
            return format!(
                "{} = {};",
                normalize_operand(lhs, context, state),
                normalize_operand(rhs, context, state)
            );
        }
    }
    if let Some(rest) = text.strip_prefix("inc ") {
        return format!("{}++;", normalize_operand(rest, context, state));
    }
    if let Some(rest) = text.strip_prefix("dec ") {
        return format!("{}--;", normalize_operand(rest, context, state));
    }
    if let Some(rest) = text.strip_prefix("add sp,") {
        return format!("SP += {};", normalize_operand(rest, context, state));
    }
    if let Some(rest) = text.strip_prefix("add a,") {
        return format!("A += {};", normalize_operand(rest, context, state));
    }
    if let Some(rest) = text.strip_prefix("adc a,") {
        return format!(
            "A = A + {} + carry;",
            normalize_operand(rest, context, state)
        );
    }
    if let Some(rest) = text.strip_prefix("sub ") {
        return format!("A -= {};", normalize_operand(rest, context, state));
    }
    if let Some(rest) = text.strip_prefix("sbc a,") {
        return format!(
            "A = A - {} - carry;",
            normalize_operand(rest, context, state)
        );
    }
    if let Some(rest) = text.strip_prefix("and ") {
        return format!("A &= {};", normalize_operand(rest, context, state));
    }
    if let Some(rest) = text.strip_prefix("or ") {
        return format!("A |= {};", normalize_operand(rest, context, state));
    }
    if let Some(rest) = text.strip_prefix("xor ") {
        return format!("A ^= {};", normalize_operand(rest, context, state));
    }
    if let Some(rest) = text.strip_prefix("cp ") {
        return format!("compare(A, {});", normalize_operand(rest, context, state));
    }
    if text == "daa" {
        return "A = decimal_adjust(A);".to_string();
    }
    if text == "cpl" {
        return "A = ~A;".to_string();
    }
    if text == "scf" {
        return "carry = 1;".to_string();
    }
    if text == "ccf" {
        return "carry = !carry;".to_string();
    }
    if text == "di" {
        return "disable_interrupts();".to_string();
    }
    if text == "ei" {
        return "enable_interrupts();".to_string();
    }
    if text == "halt" {
        return "halt();".to_string();
    }
    if text == "stop" {
        return "stop();".to_string();
    }
    if let Some(register) = text.strip_prefix("push ") {
        let reg = register.trim().to_ascii_uppercase();
        return format!("push16(tmp_{} = {});", reg.to_ascii_lowercase(), reg);
    }
    if let Some(register) = text.strip_prefix("pop ") {
        let reg = register.trim().to_ascii_uppercase();
        return format!("{} = pop16(/* tmp_{} */);", reg, reg.to_ascii_lowercase());
    }
    format!("/* {} */", ins.text)
}

// Render RST, known names and selected intrinsic aliases; retain ambiguous targets by address.
// Arguments and conditional-call guards are not reconstructed by this formatter.
fn call_statement(
    ins: &DecodedInstruction,
    function_lookup: &BTreeMap<(u16, u16), String>,
) -> String {
    if ins.flow_kind == "rst" {
        if let Some(addr) = ins.target_addr {
            return format!("rst_{:02X}()", addr);
        }
    }
    if let (Some(bank), Some(addr)) = (ins.target_bank, ins.target_addr) {
        if let Some(name) = function_lookup.get(&(bank, addr)) {
            if let Some(intrinsic) = classify_symbol_name(name) {
                return match intrinsic.kind {
                    crate::intrinsics::KitaqgbIntrinsicKind::WaitVBlank => {
                        "wait_vblank_like()".to_string()
                    }
                    crate::intrinsics::KitaqgbIntrinsicKind::Memcpy => "memcpy_like()".to_string(),
                    crate::intrinsics::KitaqgbIntrinsicKind::Memset => "memset_like()".to_string(),
                    crate::intrinsics::KitaqgbIntrinsicKind::Bank => {
                        format!("__farcall(bank_{:02X}, {})", bank, name)
                    }
                    _ => format!("{}()", name),
                };
            }
            return format!("{}()", name);
        }
        return format!("{}()", auto_function_name(bank, addr));
    }
    if let Some(addr) = ins.target_addr {
        return format!("sub_ambiguous_{:04X}()", addr);
    }
    "call_unknown()".to_string()
}

// Derive a flag expression only from the last instruction of a block.
fn branch_condition_from_block(block: &DecompileBasicBlockInfo) -> Option<&str> {
    block
        .instructions
        .last()
        .and_then(branch_condition_for_instruction)
}

// Translate the four recognized Z/C conditions on JR, JP, CALL and RET into flag expressions.
fn branch_condition_for_instruction(ins: &DecodedInstruction) -> Option<&str> {
    let upper = ins.text.to_ascii_uppercase();
    if upper.starts_with("JR NZ,")
        || upper.starts_with("JP NZ,")
        || upper.starts_with("CALL NZ,")
        || upper == "RET NZ"
    {
        Some("!flag_Z")
    } else if upper.starts_with("JR Z,")
        || upper.starts_with("JP Z,")
        || upper.starts_with("CALL Z,")
        || upper == "RET Z"
    {
        Some("flag_Z")
    } else if upper.starts_with("JR NC,")
        || upper.starts_with("JP NC,")
        || upper.starts_with("CALL NC,")
        || upper == "RET NC"
    {
        Some("!flag_C")
    } else if upper.starts_with("JR C,")
        || upper.starts_with("JP C,")
        || upper.starts_with("CALL C,")
        || upper == "RET C"
    {
        Some("flag_C")
    } else {
        None
    }
}

// Identify branch/call/return families handled outside ordinary statement rendering; HALT/STOP remain statements.
fn is_control_instruction(ins: &DecodedInstruction) -> bool {
    matches!(
        ins.flow_kind.as_str(),
        "jump"
            | "jump_cond"
            | "call"
            | "call_cond"
            | "return"
            | "return_cond"
            | "reti"
            | "rst"
            | "indirect_jump"
    )
}

// Intersect predecessor sets to a fixed point with block zero as entry.
// Non-entry blocks without known predecessors retain the initial universe set.
fn compute_dominators(blocks: &[DecompileBasicBlockInfo]) -> Vec<BTreeSet<usize>> {
    let all = (0..blocks.len()).collect::<BTreeSet<_>>();
    let mut dom = vec![all.clone(); blocks.len()];
    if !blocks.is_empty() {
        dom[0] = BTreeSet::from([0]);
    }
    let block_index_by_id = blocks
        .iter()
        .enumerate()
        .map(|(i, block)| (block.id.clone(), i))
        .collect::<BTreeMap<_, _>>();
    let mut changed = true;
    while changed {
        changed = false;
        for i in 1..blocks.len() {
            let predecessors = blocks[i]
                .predecessors
                .iter()
                .filter_map(|id| block_index_by_id.get(id).copied())
                .collect::<Vec<_>>();
            if predecessors.is_empty() {
                continue;
            }
            let mut new_set = all.clone();
            for pred in predecessors {
                new_set = new_set.intersection(&dom[pred]).copied().collect();
            }
            new_set.insert(i);
            if new_set != dom[i] {
                dom[i] = new_set;
                changed = true;
            }
        }
    }
    dom
}

// Intersect successor sets backward from known exits. There is no synthetic common exit;
// closed cycles or missing successors can retain the initial universe set.
fn compute_post_dominators(blocks: &[DecompileBasicBlockInfo]) -> Vec<BTreeSet<usize>> {
    if blocks.is_empty() {
        return Vec::new();
    }
    let mut block_index_by_id = BTreeMap::new();
    for (index, block) in blocks.iter().enumerate() {
        block_index_by_id.insert(block.id.clone(), index);
    }
    let universe = (0..blocks.len()).collect::<BTreeSet<_>>();
    let exit_blocks = blocks
        .iter()
        .enumerate()
        .filter_map(|(index, block)| block.successors.is_empty().then_some(index))
        .collect::<Vec<_>>();
    let mut post_dominators = vec![universe.clone(); blocks.len()];
    for exit in &exit_blocks {
        post_dominators[*exit] = BTreeSet::from([*exit]);
    }
    let mut changed = true;
    while changed {
        changed = false;
        for index in (0..blocks.len()).rev() {
            if exit_blocks.contains(&index) {
                continue;
            }
            let successor_sets = blocks[index]
                .successors
                .iter()
                .filter_map(|id| block_index_by_id.get(id).copied())
                .map(|succ| post_dominators[succ].clone())
                .collect::<Vec<_>>();
            if successor_sets.is_empty() {
                continue;
            }
            let mut intersection = successor_sets[0].clone();
            for set in successor_sets.iter().skip(1) {
                intersection = intersection.intersection(set).copied().collect();
            }
            intersection.insert(index);
            if intersection != post_dominators[index] {
                post_dominators[index] = intersection;
                changed = true;
            }
        }
    }
    post_dominators
}

// Infer loop/if/break/continue labels from graph sets in block order.
// These are presentation hints, and loop headers are accumulated during this same pass.
fn analyze_control_flow_hints(
    blocks: &[DecompileBasicBlockInfo],
    dominators: &[BTreeSet<usize>],
    post_dominators: &[BTreeSet<usize>],
) -> ControlFlowHints {
    let mut hints = ControlFlowHints::default();
    let mut block_index_by_id = BTreeMap::new();
    for (index, block) in blocks.iter().enumerate() {
        block_index_by_id.insert(block.id.clone(), index);
    }
    for (index, block) in blocks.iter().enumerate() {
        if block.successors.len() == 2 {
            let successors = block
                .successors
                .iter()
                .filter_map(|id| block_index_by_id.get(id).copied())
                .collect::<Vec<_>>();
            if successors.len() == 2 {
                let shared_postdom = post_dominators[successors[0]]
                    .intersection(&post_dominators[successors[1]])
                    .copied()
                    .find(|candidate| *candidate != index);
                if shared_postdom.is_some() {
                    hints.if_else_headers.insert(index);
                } else {
                    hints.if_headers.insert(index);
                }
            }
        }
        for successor in &block.successors {
            if let Some(target) = block_index_by_id.get(successor).copied() {
                if dominators[index].contains(&target) {
                    hints.loop_headers.insert(target);
                }
                if hints.loop_headers.contains(&target) && !post_dominators[index].contains(&target)
                {
                    hints.break_blocks.insert(index);
                }
                if hints.loop_headers.contains(&target) && target <= index {
                    hints.continue_blocks.insert(index);
                }
            }
        }
    }
    hints
}

// Look backward at most five instructions before JP HL for a table base and indexing
// shape, then read at most sixteen little-endian targets. Candidate cases are not executed.
fn detect_switch_candidates(
    rom: &[u8],
    rom_bank_count: u16,
    function: &FunctionAnalysis,
    blocks: &[DecompileBasicBlockInfo],
) -> Vec<SwitchCandidateInfo> {
    let mut out = Vec::new();
    for block in blocks {
        let Some(last) = block.instructions.last() else {
            continue;
        };
        if last.flow_kind != "indirect_jump" {
            continue;
        }
        let mut table_addr = None;
        let mut saw_loader_pattern = false;
        for ins in block.instructions.iter().rev().skip(1).take(5) {
            let upper = ins.text.to_ascii_uppercase();
            if let Some(value) = upper.strip_prefix("LD HL,") {
                let value = value.trim().trim_start_matches('$');
                if let Ok(addr) = u16::from_str_radix(value, 16) {
                    table_addr = Some(addr);
                    break;
                }
            } else if upper == "ADD HL,DE"
                || upper == "ADD HL,BC"
                || upper == "ADD HL,HL"
                || upper.starts_with("LD E,A")
                || upper.starts_with("LD D,$00")
                || upper.starts_with("LD C,A")
                || upper.starts_with("LD B,$00")
            {
                saw_loader_pattern = true;
            }
        }
        let Some(table_addr) = table_addr else {
            continue;
        };
        if !is_code_addr(function.bank, table_addr, rom_bank_count) {
            continue;
        }
        let mut cases = Vec::new();
        for index in 0..16u16 {
            let ptr_addr = table_addr.saturating_add(index.saturating_mul(2));
            let Some(ptr_linear) = rom_linear_index(function.bank, ptr_addr, rom_bank_count) else {
                break;
            };
            if ptr_linear + 1 >= rom.len() {
                break;
            }
            let target_addr = u16::from(rom[ptr_linear]) | (u16::from(rom[ptr_linear + 1]) << 8);
            // These table entries use fixed/current-bank assumptions and do not infer mapper changes.
            let target_bank =
                resolve_static_pointer_bank(function.bank, target_addr, rom_bank_count);
            if target_bank.is_none() && target_addr > 0x0100 {
                break;
            }
            cases.push(SwitchCaseInfo {
                index,
                target_bank,
                target_addr,
            });
        }
        if cases.len() >= 2 && (saw_loader_pattern || cases.len() >= 3) {
            out.push(SwitchCandidateInfo {
                table_bank: function.bank,
                table_addr,
                jump_bank: function.bank,
                jump_addr: last.addr,
                cases,
            });
        }
    }
    out
}

// Mark decoded instruction bytes as code and classify remaining bank-window runs of
// at least four bytes. Unreached executable bytes may therefore be classified as data.
fn classify_data_ranges(
    rom: &[u8],
    rom_bank_count: u16,
    functions: &[FunctionAnalysis],
) -> Vec<DataRangeInfo> {
    let mut code_bytes = BTreeSet::new();
    for function in functions {
        for ins in function.instructions.values() {
            for offset in 0..ins.len {
                code_bytes.insert((function.bank, ins.addr.saturating_add(offset as u16)));
            }
        }
    }

    let mut ranges = Vec::new();
    for bank in 0..rom_bank_count {
        let (start_addr, end_addr) = if bank == 0 {
            (0x0000u16, 0x3FFFu16)
        } else {
            (0x4000u16, 0x7FFFu16)
        };
        let mut cursor = start_addr;
        while cursor <= end_addr {
            if code_bytes.contains(&(bank, cursor)) {
                cursor = cursor.saturating_add(1);
                continue;
            }
            let range_start = cursor;
            while cursor <= end_addr && !code_bytes.contains(&(bank, cursor)) {
                cursor = cursor.saturating_add(1);
            }
            let range_end = cursor;
            if range_end.saturating_sub(range_start) < 4 {
                continue;
            }
            let classification =
                classify_data_range_bytes(rom, bank, range_start, range_end, rom_bank_count);
            ranges.push(DataRangeInfo {
                bank,
                start: range_start,
                end: range_end,
                classification,
            });
        }
    }
    ranges
}

// Prefer an even ROM-address-like word array, then a sixteen-byte-aligned varied-byte
// tile shape, otherwise readonly data. Mapping uses bank count; callers must provide backed ranges.
fn classify_data_range_bytes(
    rom: &[u8],
    bank: u16,
    start: u16,
    end: u16,
    rom_bank_count: u16,
) -> String {
    let Some(start_index) = rom_linear_index(bank, start, rom_bank_count) else {
        return "readonly_data".to_string();
    };
    let Some(end_index) = rom_linear_index(bank, end.saturating_sub(1), rom_bank_count) else {
        return "readonly_data".to_string();
    };
    let bytes = &rom[start_index..=end_index.min(rom.len().saturating_sub(1))];
    if bytes.len() >= 8 && bytes.len() % 2 == 0 {
        let pointer_like = bytes.chunks_exact(2).all(|chunk| {
            let value = u16::from(chunk[0]) | (u16::from(chunk[1]) << 8);
            value <= FIXED_BANK_END || (BANK_WINDOW_START..=BANK_WINDOW_END).contains(&value)
        });
        if pointer_like {
            return "pointer_array".to_string();
        }
    }
    if bytes.len() >= 16 && bytes.len() % 16 == 0 {
        let unique = bytes.iter().copied().collect::<BTreeSet<_>>().len();
        if unique > 4 {
            return "tile_like_data".to_string();
        }
    }
    "readonly_data".to_string()
}

// Prefer fixed/same-switchable-bank mapping; from bank zero, use a unique metadata
// entry bank when available. Dynamic mapper state is not tracked and ambiguous targets remain absent.
fn resolve_target_bank(
    current_bank: u16,
    target_addr: u16,
    rom_bank_count: u16,
    symbol_table: Option<&SymbolTable>,
    ins: &RawInstruction,
) -> Option<u16> {
    if let Some(bank) = resolve_static_pointer_bank(current_bank, target_addr, rom_bank_count) {
        return Some(bank);
    }
    if current_bank == 0 && target_addr >= BANK_WINDOW_START {
        if let Some(table) = symbol_table {
            let mut candidates = table
                .functions
                .iter()
                .filter(|function| function.start == target_addr)
                .map(|function| function.bank)
                .collect::<BTreeSet<_>>();
            if candidates.is_empty() {
                candidates = table
                    .symbols
                    .iter()
                    .filter(|symbol| symbol.start == target_addr)
                    .map(|symbol| symbol.bank)
                    .collect();
            }
            if candidates.len() == 1 {
                return candidates.into_iter().next();
            }
        }
    }
    if matches!(ins.flow_kind, FlowKind::Rst) {
        return Some(0);
    }
    None
}

// Map the fixed window to bank zero and the switchable window to a valid current
// nonzero bank. A bank-zero call into the switchable window cannot be resolved statically here.
fn resolve_static_pointer_bank(
    current_bank: u16,
    target_addr: u16,
    rom_bank_count: u16,
) -> Option<u16> {
    if target_addr <= FIXED_BANK_END {
        Some(0)
    } else if (BANK_WINDOW_START..=BANK_WINDOW_END).contains(&target_addr) {
        if current_bank > 0 && current_bank < rom_bank_count {
            Some(current_bank)
        } else {
            None
        }
    } else {
        None
    }
}

// Use fixed/same-bank rendering assumptions without ROM-size or metadata validation.
fn resolve_render_bank(current_bank: u16, target_addr: u16) -> Option<u16> {
    if target_addr <= FIXED_BANK_END {
        Some(0)
    } else if (BANK_WINDOW_START..=BANK_WINDOW_END).contains(&target_addr) {
        (current_bank != 0).then_some(current_bank)
    } else {
        None
    }
}

// Move decoded fields into the report representation and derive target bank from the
// rendering rule; this does not carry forward metadata-resolved xref bank information.
fn to_decoded_instruction(bank: u16, ins: RawInstruction) -> DecodedInstruction {
    let target_bank = ins
        .target_addr
        .and_then(|target| resolve_render_bank(bank, target));
    let text = ins_display_text_raw(&ins);
    DecodedInstruction {
        bank: ins.bank,
        addr: ins.addr,
        linear_address: ins.linear_address,
        bytes: ins.bytes,
        mnemonic: ins.mnemonic,
        operand_text: ins.operand_text,
        text,
        target_bank,
        target_addr: ins.target_addr,
        flow_kind: flow_kind_name(ins.flow_kind).to_string(),
        is_conditional: ins.is_conditional,
        fallthrough: ins.fallthrough,
    }
}

// Join mnemonic and operands with one space, omitting that space when operands are empty.
fn ins_display_text_raw(ins: &RawInstruction) -> String {
    if ins.operand_text.is_empty() {
        ins.mnemonic.clone()
    } else {
        format!("{} {}", ins.mnemonic, ins.operand_text)
    }
}

// Expose stable lowercase flow-category names used in serialized blocks and render dispatch.
fn flow_kind_name(kind: FlowKind) -> &'static str {
    match kind {
        FlowKind::None => "none",
        FlowKind::Jump => "jump",
        FlowKind::JumpCond => "jump_cond",
        FlowKind::Call => "call",
        FlowKind::CallCond => "call_cond",
        FlowKind::Return => "return",
        FlowKind::ReturnCond => "return_cond",
        FlowKind::Reti => "reti",
        FlowKind::Rst => "rst",
        FlowKind::IndirectJump => "indirect_jump",
        FlowKind::Stop => "stop",
        FlowKind::Halt => "halt",
        FlowKind::Invalid => "invalid",
    }
}

// Generate a deterministic function name containing its bank and entry address.
fn auto_function_name(bank: u16, addr: u16) -> String {
    format!("sub_bank{:02X}_{:04X}", bank, addr)
}

// Use function-style names for call/RST targets and local-label names for other references.
fn auto_label_name(bank: u16, addr: u16, kind: &str) -> String {
    if kind.contains("call") || kind == "rst" {
        auto_function_name(bank, addr)
    } else {
        format!("loc_bank{:02X}_{:04X}", bank, addr)
    }
}

// Replace recognized HL references with aliases or dereferences, then apply literal
// case-sensitive replacements for other memory tokens. This does not parse general expressions.
fn normalize_operand(text: &str, context: &PseudocodeContext, state: &BlockRenderState) -> String {
    let token = text.trim();
    if token.eq_ignore_ascii_case("[HL+]") {
        if let Some(offset) = state.hl_stack_alias {
            return format!("*({}++)", stack_slot_name(offset, context));
        }
        return "*(HL++)".to_string();
    }
    if token.eq_ignore_ascii_case("[HL-]") {
        if let Some(offset) = state.hl_stack_alias {
            return format!("*({}--)", stack_slot_name(offset, context));
        }
        return "*(HL--)".to_string();
    }
    if token.eq_ignore_ascii_case("[HL]") {
        if let Some(offset) = state.hl_stack_alias {
            return stack_slot_name(offset, context);
        }
        return "*HL".to_string();
    }
    token
        .replace("[BC]", "*BC")
        .replace("[DE]", "*DE")
        .replace("[C]", "io[C]")
        .replace("[$", "mem[$")
}

// Track explicit SP-relative HL setup and INC/DEC HL, clearing on selected HL clobbers.
// This local heuristic does not model every H/L write, call clobber or implicit HL increment.
fn update_render_state(state: &mut BlockRenderState, ins: &DecodedInstruction) {
    let upper = ins.text.to_ascii_uppercase();
    if let Some(offset) = parse_sp_relative_offset(&upper) {
        state.hl_stack_alias = Some(offset);
        return;
    }
    match upper.as_str() {
        "INC HL" => {
            if let Some(alias) = state.hl_stack_alias.as_mut() {
                *alias = alias.saturating_add(1);
            }
        }
        "DEC HL" => {
            if let Some(alias) = state.hl_stack_alias.as_mut() {
                *alias = alias.saturating_sub(1);
            }
        }
        _ => {
            let clears_alias = upper.starts_with("LD HL,")
                || upper.starts_with("ADD HL,")
                || upper == "POP HL"
                || upper == "LD SP,HL"
                || upper == "JP HL"
                || upper.starts_with("INC H")
                || upper.starts_with("DEC H")
                || upper.starts_with("INC L")
                || upper.starts_with("DEC L");
            if clears_alias {
                state.hl_stack_alias = None;
            }
        }
    }
}

// Name explicit SP-relative references and PUSH/POP register spill candidates using
// static access/lifetime profiles. Keep a bounded evidence preview and a rendering name map.
fn infer_stack_and_temp_hints(
    function: &FunctionAnalysis,
) -> (
    Vec<DecompileStackSlotHint>,
    Vec<DecompileTempSlotHint>,
    PseudocodeContext,
) {
    let mut stack_slot_evidence = BTreeMap::<i16, BTreeSet<String>>::new();
    let mut temp_slot_evidence = BTreeMap::<String, BTreeSet<String>>::new();
    let stack_access_profiles = analyze_stack_slot_access_patterns(function);
    let stack_lifetime_hints = analyze_stack_slot_lifetimes(function);
    for ins in function.instructions.values() {
        let ins_text = ins_display_text_raw(ins);
        if let Some(offset) = parse_sp_relative_offset(&ins_text) {
            stack_slot_evidence
                .entry(offset)
                .or_default()
                .insert(format!("{:02X}:{:04X} {}", ins.bank, ins.addr, ins_text));
        }
        let upper = ins_text.to_ascii_uppercase();
        if let Some(reg) = upper.strip_prefix("PUSH ") {
            temp_slot_evidence
                .entry(format!("tmp_{}", reg.trim().to_ascii_lowercase()))
                .or_default()
                .insert(format!("{:02X}:{:04X} {}", ins.bank, ins.addr, ins_text));
        }
        if let Some(reg) = upper.strip_prefix("POP ") {
            temp_slot_evidence
                .entry(format!("tmp_{}", reg.trim().to_ascii_lowercase()))
                .or_default()
                .insert(format!("{:02X}:{:04X} {}", ins.bank, ins.addr, ins_text));
        }
    }

    let mut context = PseudocodeContext::default();
    let mut stack_slots = stack_slot_evidence
        .into_iter()
        .map(|(offset, evidence)| {
            let (name, kind) = classify_stack_slot(offset);
            let profile = stack_access_profiles
                .get(&offset)
                .cloned()
                .unwrap_or_default();
            let confidence = infer_stack_slot_confidence(&profile);
            let access_patterns = profile.patterns.iter().cloned().collect();
            context.stack_slot_names.insert(offset, name.clone());
            DecompileStackSlotHint {
                offset,
                name,
                kind,
                inferred_type: Some(infer_stack_slot_type(function, offset, &profile)),
                runtime_role: None,
                read_count: profile.read_count,
                write_count: profile.write_count,
                access_patterns,
                trace_value_hints: Vec::new(),
                lifetime_hint: stack_lifetime_hints.get(&offset).cloned(),
                evidence: evidence.into_iter().take(4).collect(),
                confidence,
            }
        })
        .collect::<Vec<_>>();
    stack_slots.sort_by_key(|slot| slot.offset);

    let mut temp_slots = temp_slot_evidence
        .into_iter()
        .map(|(name, evidence)| DecompileTempSlotHint {
            name,
            kind: "register_spill_candidate".to_string(),
            inferred_type: Some("u16".to_string()),
            runtime_role: None,
            read_count: evidence.iter().filter(|line| line.contains("POP")).count() as u32,
            write_count: evidence.iter().filter(|line| line.contains("PUSH")).count() as u32,
            access_patterns: evidence
                .iter()
                .map(|line| {
                    if line.contains("PUSH") {
                        "push_spill".to_string()
                    } else if line.contains("POP") {
                        "pop_restore".to_string()
                    } else {
                        "spill".to_string()
                    }
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            evidence: evidence.into_iter().take(4).collect(),
            confidence: "medium".to_string(),
        })
        .collect::<Vec<_>>();
    temp_slots.sort_by(|a, b| a.name.cmp(&b.name));
    (stack_slots, temp_slots, context)
}

// Rank pointer, enum, flag, counter and byte/word candidates from access patterns.
// Some byte/word signals are function-wide, so a classification is not proof about one slot.
fn infer_stack_slot_type(
    function: &FunctionAnalysis,
    offset: i16,
    profile: &StackAccessProfile,
) -> String {
    let pattern = format!("LD HL,SP{:+}", offset);
    let mut saw_word_pair = false;
    let mut saw_byte_access = false;
    for ins in function.instructions.values() {
        let text = ins_display_text_raw(ins).to_ascii_uppercase();
        if text == pattern {
            saw_byte_access = true;
            continue;
        }
        if text.contains("[HL]") || text.contains("[HL+]") || text.contains("[HL-]") {
            saw_byte_access = true;
        }
        if text == "PUSH HL" || text == "POP HL" || text.starts_with("LD SP,HL") {
            saw_word_pair = true;
        }
    }
    let pattern_set = &profile.patterns;
    if pattern_set.contains("pointer_load")
        || pattern_set.contains("pointer_store")
        || (saw_word_pair && profile.read_count + profile.write_count <= 2)
    {
        "pointer_candidate".to_string()
    } else if profile.literal_writes.len() >= 3 {
        "enum_like_candidate".to_string()
    } else if pattern_set.contains("bit_test")
        || pattern_set.contains("bit_set")
        || pattern_set.contains("bit_clear")
        || (profile.literal_writes.len() > 0
            && profile
                .literal_writes
                .iter()
                .all(|value| value == "$00" || value == "$01"))
        || (profile.bit_indices.len() == 1
            && !pattern_set.contains("pointer_load")
            && !pattern_set.contains("pointer_store"))
    {
        "bool_candidate".to_string()
    } else if pattern_set.contains("inc")
        || pattern_set.contains("dec")
        || pattern_set.contains("counter_add")
        || pattern_set.contains("counter_sub")
    {
        "counter_candidate".to_string()
    } else if saw_word_pair && !saw_byte_access {
        "u16_candidate".to_string()
    } else {
        "u8_candidate".to_string()
    }
}

// Assign qualitative confidence from distinct pattern counts and static access totals.
fn infer_stack_slot_confidence(profile: &StackAccessProfile) -> String {
    let signal_count = profile.patterns.len() as u32 + profile.bit_indices.len() as u32;
    let access_weight = profile.read_count.saturating_add(profile.write_count);
    if signal_count >= 3 || access_weight >= 6 {
        "high".to_string()
    } else if signal_count >= 1 || access_weight >= 2 {
        "medium".to_string()
    } else {
        "low".to_string()
    }
}

// Scan instructions in address order, attributing recognized HL accesses to the
// current SP alias. This is not control-flow-sensitive alias or liveness analysis.
fn analyze_stack_slot_access_patterns(
    function: &FunctionAnalysis,
) -> BTreeMap<i16, StackAccessProfile> {
    let mut profiles = BTreeMap::<i16, StackAccessProfile>::new();
    let mut state = BlockRenderState::default();
    for ins in function.instructions.values() {
        let text = ins_display_text_raw(ins).to_ascii_uppercase();
        if let Some(offset) = state.hl_stack_alias {
            let profile = profiles.entry(offset).or_default();
            if text.starts_with("LD A,[HL]")
                || text.starts_with("LD B,[HL]")
                || text.starts_with("LD C,[HL]")
                || text.starts_with("LD D,[HL]")
                || text.starts_with("LD E,[HL]")
                || text.starts_with("LD H,[HL]")
                || text.starts_with("LD L,[HL]")
            {
                profile.read_count = profile.read_count.saturating_add(1);
                profile.patterns.insert("read_byte".to_string());
            } else if text.starts_with("LD [HL],") {
                profile.write_count = profile.write_count.saturating_add(1);
                profile.patterns.insert("write_byte".to_string());
                if let Some(literal) = parse_hl_write_literal(&text) {
                    profile.literal_writes.insert(literal);
                }
            } else if text == "INC [HL]" {
                profile.read_count = profile.read_count.saturating_add(1);
                profile.write_count = profile.write_count.saturating_add(1);
                profile.patterns.insert("inc".to_string());
            } else if text == "DEC [HL]" {
                profile.read_count = profile.read_count.saturating_add(1);
                profile.write_count = profile.write_count.saturating_add(1);
                profile.patterns.insert("dec".to_string());
            } else if text.starts_with("CP [HL]") {
                profile.read_count = profile.read_count.saturating_add(1);
                profile.patterns.insert("compare".to_string());
            } else if text.starts_with("BIT ") && text.contains(",[HL]") {
                profile.read_count = profile.read_count.saturating_add(1);
                profile.patterns.insert("bit_test".to_string());
                if let Some(bit) = parse_hl_bit_index(&text) {
                    profile.bit_indices.insert(bit);
                }
            } else if text.starts_with("RES ") && text.contains(",[HL]") {
                profile.read_count = profile.read_count.saturating_add(1);
                profile.write_count = profile.write_count.saturating_add(1);
                profile.patterns.insert("bit_clear".to_string());
                if let Some(bit) = parse_hl_bit_index(&text) {
                    profile.bit_indices.insert(bit);
                }
            } else if text.starts_with("SET ") && text.contains(",[HL]") {
                profile.read_count = profile.read_count.saturating_add(1);
                profile.write_count = profile.write_count.saturating_add(1);
                profile.patterns.insert("bit_set".to_string());
                if let Some(bit) = parse_hl_bit_index(&text) {
                    profile.bit_indices.insert(bit);
                }
            } else if text.starts_with("ADD A,[HL]") || text.starts_with("ADC A,[HL]") {
                profile.read_count = profile.read_count.saturating_add(1);
                profile.patterns.insert("counter_add".to_string());
            } else if text.starts_with("SUB [HL]") || text.starts_with("SBC A,[HL]") {
                profile.read_count = profile.read_count.saturating_add(1);
                profile.patterns.insert("counter_sub".to_string());
            } else if text == "LD A,[HL+]" || text == "LD A,[HL-]" {
                profile.read_count = profile.read_count.saturating_add(1);
                profile.patterns.insert("pointer_load".to_string());
            } else if text == "LD [HL+],A" || text == "LD [HL-],A" {
                profile.write_count = profile.write_count.saturating_add(1);
                profile.patterns.insert("pointer_store".to_string());
            }
        }
        let decoded = DecodedInstruction {
            bank: ins.bank,
            addr: ins.addr,
            linear_address: ins.linear_address,
            bytes: ins.bytes.clone(),
            mnemonic: ins.mnemonic.clone(),
            operand_text: ins.operand_text.clone(),
            text: ins_display_text_raw(ins),
            target_bank: None,
            target_addr: ins.target_addr,
            flow_kind: flow_kind_name(ins.flow_kind).to_string(),
            is_conditional: ins.is_conditional,
            fallthrough: ins.fallthrough,
        };
        update_render_state(&mut state, &decoded);
    }
    profiles
}

// Compare first/last alias-access indices with call indices in address order,
// producing candidate phase labels rather than path-sensitive live ranges.
fn analyze_stack_slot_lifetimes(function: &FunctionAnalysis) -> BTreeMap<i16, String> {
    let mut access_indices = BTreeMap::<i16, Vec<usize>>::new();
    let mut call_indices = Vec::<usize>::new();
    let mut state = BlockRenderState::default();
    for (index, ins) in function.instructions.values().enumerate() {
        let text = ins_display_text_raw(ins).to_ascii_uppercase();
        if matches!(
            ins.flow_kind,
            FlowKind::Call | FlowKind::CallCond | FlowKind::Rst
        ) {
            call_indices.push(index);
        }
        if let Some(offset) = state.hl_stack_alias {
            if text.contains("[HL]") || text.contains("[HL+]") || text.contains("[HL-]") {
                access_indices.entry(offset).or_default().push(index);
            }
        }
        let decoded = DecodedInstruction {
            bank: ins.bank,
            addr: ins.addr,
            linear_address: ins.linear_address,
            bytes: ins.bytes.clone(),
            mnemonic: ins.mnemonic.clone(),
            operand_text: ins.operand_text.clone(),
            text: ins_display_text_raw(ins),
            target_bank: None,
            target_addr: ins.target_addr,
            flow_kind: flow_kind_name(ins.flow_kind).to_string(),
            is_conditional: ins.is_conditional,
            fallthrough: ins.fallthrough,
        };
        update_render_state(&mut state, &decoded);
    }

    let mut lifetimes = BTreeMap::new();
    for (offset, indices) in access_indices {
        let Some(first) = indices.first().copied() else {
            continue;
        };
        let Some(last) = indices.last().copied() else {
            continue;
        };
        let access_count = indices.len();
        let lifetime = if call_indices
            .iter()
            .any(|call_index| *call_index > first && *call_index < last)
        {
            if access_count >= 4 {
                "persistent_across_call_boundary"
            } else {
                "across_call_boundary"
            }
        } else if let Some(first_call) = call_indices.first().copied() {
            if last < first_call {
                if access_count <= 2 {
                    "pre_call_temporary"
                } else {
                    "pre_call_local"
                }
            } else if first > call_indices.last().copied().unwrap_or(first_call) {
                if access_count <= 2 {
                    "post_call_temporary"
                } else {
                    "post_call_local"
                }
            } else {
                if access_count <= 1 {
                    "single_use_temp"
                } else {
                    "single_phase_local"
                }
            }
        } else {
            if access_count <= 1 {
                "single_use_temp"
            } else {
                "single_phase_local"
            }
        };
        lifetimes.insert(offset, lifetime.to_string());
    }
    lifetimes
}

// Collect register/stack setup from the preceding four decoded instructions by address.
// No def-use or intervening-control-flow proof establishes that these values are arguments.
fn infer_call_site_hints(
    function: &FunctionAnalysis,
    function_lookup: &BTreeMap<(u16, u16), String>,
) -> Vec<DecompileCallSiteHint> {
    let instructions = function.instructions.values().collect::<Vec<_>>();
    let mut hints = Vec::new();
    for (index, ins) in instructions.iter().enumerate() {
        if !matches!(
            ins.flow_kind,
            FlowKind::Call | FlowKind::CallCond | FlowKind::Rst
        ) {
            continue;
        }
        let target_bank = ins
            .target_addr
            .and_then(|target| resolve_render_bank(function.bank, target));
        let target_name = match (target_bank, ins.target_addr) {
            (Some(bank), Some(addr)) => function_lookup.get(&(bank, addr)).cloned(),
            _ => None,
        };
        let mut static_argument_hints = Vec::new();
        let start = index.saturating_sub(4);
        for previous in &instructions[start..index] {
            let text = ins_display_text_raw(previous).to_ascii_uppercase();
            if text.starts_with("LD A,")
                || text.starts_with("LD BC,")
                || text.starts_with("LD DE,")
                || text.starts_with("LD HL,")
                || text.starts_with("PUSH ")
                || text.starts_with("LD HL,SP")
            {
                static_argument_hints.push(format!(
                    "{:02X}:{:04X} {}",
                    previous.bank, previous.addr, text
                ));
            }
        }
        hints.push(DecompileCallSiteHint {
            call_bank: ins.bank,
            call_addr: ins.addr,
            target_bank,
            target_addr: ins.target_addr,
            target_name,
            calling_convention_hint: infer_static_calling_convention_hint(&static_argument_hints),
            static_argument_hints,
            runtime_argument_hints: Vec::new(),
            hit_count: 0,
        });
    }
    hints
}

// Guess register, stack or mixed setup from text hints. LD HL,SP can contribute
// to both categories, so this label alone does not establish the calling convention.
fn infer_static_calling_convention_hint(static_argument_hints: &[String]) -> Option<String> {
    let uses_stack = static_argument_hints
        .iter()
        .any(|hint| hint.contains("PUSH ") || hint.contains("LD HL,SP"));
    // A textual LD HL,SP setup matches this register check as well as the stack check above.
    let uses_registers = static_argument_hints.iter().any(|hint| {
        hint.contains(" LD A,")
            || hint.contains(" LD BC,")
            || hint.contains(" LD DE,")
            || hint.contains(" LD HL,")
    });
    match (uses_registers, uses_stack) {
        (true, false) => Some("fastcall".to_string()),
        (false, true) => Some("stack_call".to_string()),
        (true, true) => Some("mixed_call".to_string()),
        (false, false) => None,
    }
}

// Parse the decimal byte before the operand comma; opcode/range validation belongs to callers.
fn parse_hl_bit_index(text: &str) -> Option<u8> {
    let (_, tail) = text.split_once(' ')?;
    let (bit_text, _) = tail.split_once(',')?;
    bit_text.trim().parse::<u8>().ok()
}

// Extract a literal-looking HL store token; dollar-prefixed text is retained without numeric validation.
fn parse_hl_write_literal(text: &str) -> Option<String> {
    let literal = text.strip_prefix("LD [HL],")?.trim();
    if literal.starts_with('$') || literal.chars().all(|c| c.is_ascii_digit() || c == '-') {
        Some(literal.to_string())
    } else {
        None
    }
}

// Keep parseable SP-relative keys and narrow supplied values to sixteen bits.
fn normalized_trace_stack_slot_values(observation: &DecompileTraceObservation) -> Vec<(i16, u16)> {
    observation
        .stack_slot_values
        .iter()
        .filter_map(|(key, value)| {
            parse_trace_stack_slot_key(key).map(|offset| (offset, *value as u16))
        })
        .collect()
}

// Accept a signed decimal offset or an sp/SP-prefixed signed suffix after trimming.
fn parse_trace_stack_slot_key(key: &str) -> Option<i16> {
    let trimmed = key.trim();
    if let Ok(value) = trimmed.parse::<i16>() {
        return Some(value);
    }
    if let Some(rest) = trimmed
        .strip_prefix("sp")
        .or_else(|| trimmed.strip_prefix("SP"))
    {
        return parse_signed_suffix(rest.trim());
    }
    None
}

// Treat values in 0100-DFFF as pointer-like candidates without checking mapped memory or dereferencing.
fn looks_like_pointer_value(value: u16) -> bool {
    matches!(value, 0x8000..=0xDFFF) || matches!(value, 0x0100..=0x7FFF)
}

// Append a bounded value preview and rank flag, pointer, counter then enum role guesses.
// Promote selected static types/confidence while retaining stronger preexisting type labels.
fn apply_trace_value_profile_to_slot(
    slot: &mut DecompileStackSlotHint,
    profile: &TraceValueProfile,
) {
    let distinct_count = profile.values.len();
    if distinct_count == 0 {
        return;
    }

    let value_preview = profile
        .values
        .iter()
        .take(6)
        .map(|value| format!("${value:04X}"))
        .collect::<Vec<_>>()
        .join(", ");
    slot.trace_value_hints
        .push(format!("values=[{}]", value_preview));

    if profile.values.iter().all(|value| *value <= 1) {
        slot.runtime_role = Some("runtime_flag_like".to_string());
        slot.trace_value_hints
            .push("trace-values imply bool/flag".to_string());
        if slot.inferred_type.as_deref() == Some("u8_candidate") {
            slot.inferred_type = Some("bool_candidate".to_string());
        }
        slot.confidence = "high".to_string();
    } else if profile.likely_pointer_hits >= 2 {
        slot.runtime_role = Some("runtime_pointer_like".to_string());
        slot.trace_value_hints
            .push("trace-values look pointer-like".to_string());
        if slot.inferred_type.as_deref() == Some("u8_candidate") {
            slot.inferred_type = Some("pointer_candidate".to_string());
        }
        if slot.confidence == "low" {
            slot.confidence = "medium".to_string();
        }
    } else if profile.repeated_increments >= 2 {
        slot.runtime_role = Some("runtime_counter_like".to_string());
        slot.trace_value_hints
            .push("trace-values show incremental changes".to_string());
        if matches!(
            slot.inferred_type.as_deref(),
            Some("u8_candidate") | Some("u16_candidate")
        ) {
            slot.inferred_type = Some("counter_candidate".to_string());
        }
        if slot.confidence == "low" {
            slot.confidence = "medium".to_string();
        }
    } else if (2..=6).contains(&distinct_count) {
        slot.runtime_role = Some("runtime_enum_like".to_string());
        slot.trace_value_hints
            .push("trace-values stay within a small discrete set".to_string());
        if slot.inferred_type.as_deref() == Some("u8_candidate") {
            slot.inferred_type = Some("enum_like_candidate".to_string());
        }
        if slot.confidence == "low" {
            slot.confidence = "medium".to_string();
        }
    }

    if profile.repeated_toggles >= 1 {
        slot.trace_value_hints
            .push("trace-values toggle between states".to_string());
    }
}

// Match observations at exact call bank/PC and summarize selected registers and supplied
// stack values. These observed values suggest arguments but are not callee-use evidence.
fn refine_call_sites_from_trace(
    function: &mut DecompileFunctionInfo,
    observations: &[DecompileTraceObservation],
) {
    for call_site in &mut function.artifact.call_sites {
        // Repeated overlays replace the hit count; when no new values exist, old argument strings remain.
        let mut runtime_argument_hints = BTreeSet::new();
        let mut hit_count = 0u64;
        for observation in observations {
            if observation.rom_bank != call_site.call_bank || observation.pc != call_site.call_addr
            {
                continue;
            }
            hit_count = hit_count.saturating_add(observation.hit_count.max(1));
            for (register, value) in &observation.registers {
                let upper = register.to_ascii_uppercase();
                if matches!(upper.as_str(), "A" | "BC" | "DE" | "HL") {
                    runtime_argument_hints.insert(format!("{upper}=${value:04X}"));
                }
            }
            for (offset, value) in normalized_trace_stack_slot_values(observation) {
                runtime_argument_hints.insert(format!("SP{offset:+}=${value:04X}"));
            }
        }
        call_site.hit_count = hit_count;
        if !runtime_argument_hints.is_empty() {
            call_site.runtime_argument_hints = runtime_argument_hints.into_iter().collect();
        }
        let runtime_uses_stack = call_site
            .runtime_argument_hints
            .iter()
            .any(|hint| hint.starts_with("SP"));
        let runtime_uses_registers = call_site.runtime_argument_hints.iter().any(|hint| {
            matches!(
                hint.split('=').next(),
                Some("A" | "AF" | "BC" | "DE" | "HL")
            )
        });
        if hit_count > 0 {
            call_site.calling_convention_hint = match (runtime_uses_registers, runtime_uses_stack) {
                (true, false) => Some("fastcall".to_string()),
                (false, true) => Some("stack_call".to_string()),
                (true, true) => Some("mixed_call".to_string()),
                (false, false) => call_site.calling_convention_hint.clone(),
            };
        }
    }
}

// Use function hit thresholds plus optional per-slot value patterns to refine role labels
// and suggestions. Function activity alone does not demonstrate access to every listed slot.
fn refine_variable_hints_from_trace(
    function: &mut DecompileFunctionInfo,
    summary: &DecompileTraceSummary,
    slot_value_profiles: &BTreeMap<i16, TraceValueProfile>,
) {
    let runtime_role = if summary.hit_count >= 24 {
        Some("hot_runtime_path".to_string())
    } else if summary.hit_count >= 4 {
        Some("runtime_observed".to_string())
    } else {
        None
    };
    for slot in &mut function.artifact.stack_slots {
        if runtime_role.is_some() {
            slot.runtime_role = runtime_role.clone();
        }
        if slot.kind == "argument_candidate"
            && summary.hit_count >= 4
            && slot.inferred_type.as_deref() == Some("u8_candidate")
        {
            slot.runtime_role = Some("runtime_argument_like".to_string());
        }
        if summary.hit_count >= 8 && slot.inferred_type.as_deref() == Some("counter_candidate") {
            slot.runtime_role = Some("runtime_counter_like".to_string());
        }
        if summary.hit_count >= 4 && slot.inferred_type.as_deref() == Some("bool_candidate") {
            slot.runtime_role = Some("runtime_flag_like".to_string());
        }
        if summary.hit_count >= 4 && slot.inferred_type.as_deref() == Some("pointer_candidate") {
            slot.runtime_role = Some("runtime_pointer_like".to_string());
        }
        if let Some(profile) = slot_value_profiles.get(&slot.offset) {
            apply_trace_value_profile_to_slot(slot, profile);
            if profile.values.len() == 2 && slot.inferred_type.as_deref() == Some("bool_candidate")
            {
                function.artifact.suggestions.push(format!(
                    "{} toggles between two values at runtime; consider bool/flag naming",
                    slot.name
                ));
            } else if (2..=6).contains(&profile.values.len())
                && slot.inferred_type.as_deref() == Some("enum_like_candidate")
            {
                function.artifact.suggestions.push(format!(
                    "{} shows a small runtime value set; enum/state naming may fit",
                    slot.name
                ));
            } else if profile.repeated_increments >= 2
                && slot.inferred_type.as_deref() == Some("counter_candidate")
            {
                function.artifact.suggestions.push(format!(
                    "{} changes incrementally at runtime; counter/index naming may fit",
                    slot.name
                ));
            }
            if let Some(role) = &slot.runtime_role {
                function.artifact.suggestions.push(format!(
                    "runtime role for {} => {}; use role-specific rename/type annotation",
                    slot.name, role
                ));
            }
        }
    }
    for temp in &mut function.artifact.temp_slots {
        if runtime_role.is_some() {
            temp.runtime_role = Some("runtime_spill_like".to_string());
        }
    }
    function.artifact.suggestions.sort();
    function.artifact.suggestions.dedup();
}

// Recognize LD HL,SP text ignoring case and parse its signed decimal displacement.
fn parse_sp_relative_offset(text: &str) -> Option<i16> {
    let upper = text.trim().to_ascii_uppercase();
    let suffix = upper.strip_prefix("LD HL,SP")?;
    parse_signed_suffix(suffix.trim())
}

// Accept an empty suffix as zero or a signed decimal magnitude, optionally preceded by $.
// The dollar marker is stripped here; it does not select hexadecimal parsing.
fn parse_signed_suffix(text: &str) -> Option<i16> {
    if text.is_empty() {
        return Some(0);
    }
    let (sign, digits) = if let Some(rest) = text.strip_prefix('+') {
        (1i16, rest)
    } else if let Some(rest) = text.strip_prefix('-') {
        (-1i16, rest)
    } else {
        return None;
    };
    let magnitude = digits.trim().trim_start_matches('$').parse::<i16>().ok()?;
    Some(sign.saturating_mul(magnitude))
}

// Require a three-byte readable slice and recognize selected entry-like first opcodes.
fn looks_like_function_entry(rom: &[u8], rom_bank_count: u16, bank: u16, addr: u16) -> bool {
    let linear = match rom_linear_index(bank, addr, rom_bank_count) {
        Some(value) => value as usize,
        None => return false,
    };
    let bytes = rom.get(linear..linear.saturating_add(3)).unwrap_or(&[]);
    matches!(
        bytes,
        [0xC5, ..] | [0xD5, ..] | [0xE5, ..] | [0xF5, ..] | [0xCD, ..] | [0xF8, ..]
    ) || bytes.first().copied() == Some(0x21)
}

// Name negative offsets as locals and offsets from two onward as word-sized argument
// candidates. Adjacent bytes can share a name; this assumes a layout rather than proving one.
fn classify_stack_slot(offset: i16) -> (String, String) {
    if offset < 0 {
        (
            format!("local_{}", offset.unsigned_abs()),
            "local_candidate".to_string(),
        )
    } else if offset >= 2 {
        (
            format!("arg_{}", (offset - 2) / 2),
            "argument_candidate".to_string(),
        )
    } else {
        (format!("stack_{}", offset), "stack_slot".to_string())
    }
}

// Prefer the inferred context name, otherwise derive the conventional offset-based fallback.
fn stack_slot_name(offset: i16, context: &PseudocodeContext) -> String {
    context
        .stack_slot_names
        .get(&offset)
        .cloned()
        .unwrap_or_else(|| classify_stack_slot(offset).0)
}

// Test half-open block address intervals; a PC need not equal a decoded instruction start.
fn function_contains_pc(function: &DecompileFunctionInfo, pc: u16) -> bool {
    function
        .blocks
        .iter()
        .any(|block| pc >= block.start_address && pc < block.end_address)
}

// Keep the smaller available value while treating absence as no candidate.
fn min_optional(current: Option<u64>, candidate: Option<u64>) -> Option<u64> {
    match (current, candidate) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (None, Some(b)) => Some(b),
        (Some(a), None) => Some(a),
        (None, None) => None,
    }
}

// Keep the larger available value while treating absence as no candidate.
fn max_optional(current: Option<u64>, candidate: Option<u64>) -> Option<u64> {
    match (current, candidate) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (None, Some(b)) => Some(b),
        (Some(a), None) => Some(a),
        (None, None) => None,
    }
}

// Append a present, unseen string only while below the supplied capacity; preserve encounter order.
fn push_unique_limited(target: &mut Vec<String>, value: Option<String>, limit: usize) {
    let Some(value) = value else { return };
    if target.iter().any(|existing| existing == &value) {
        return;
    }
    if target.len() < limit {
        target.push(value);
    }
}

// Append hit, frame-span and hottest-PC summaries without clearing earlier annotations.
fn push_trace_summary_annotations(target: &mut Vec<String>, summary: &DecompileTraceSummary) {
    target.push(format!(
        "trace annotation: {} runtime hit(s)",
        summary.hit_count
    ));
    if let (Some(first), Some(last)) = (summary.first_frame, summary.last_frame) {
        target.push(format!(
            "trace annotation: observed between frames {} and {}",
            first, last
        ));
    }
    if let Some(pc_hit) = summary.hot_pcs.first() {
        target.push(format!(
            "trace annotation: hottest PC {:02X}:{:04X} hit {} time(s)",
            pc_hit.rom_bank, pc_hit.pc, pc_hit.hit_count
        ));
    }
}

// Validate the bank number and its fixed/switchable address window, not backing byte availability.
fn is_code_addr(bank: u16, addr: u16, rom_bank_count: u16) -> bool {
    if bank >= rom_bank_count {
        return false;
    }
    if bank == 0 {
        addr <= FIXED_BANK_END
    } else {
        (BANK_WINDOW_START..=BANK_WINDOW_END).contains(&addr)
    }
}

// Translate a valid bank/window pair to a linear file offset; the final partial bank
// can still map beyond the supplied ROM bytes.
fn rom_linear_index(bank: u16, addr: u16, rom_bank_count: u16) -> Option<usize> {
    if bank >= rom_bank_count {
        return None;
    }
    if bank == 0 {
        (addr <= FIXED_BANK_END).then_some(addr as usize)
    } else if (BANK_WINDOW_START..=BANK_WINDOW_END).contains(&addr) {
        Some((bank as usize) * 0x4000 + (addr as usize - 0x4000))
    } else {
        None
    }
}

// Decode static opcode text, length, targets and flow categories without emulation.
// Missing operand bytes default to zero; the stored byte slice retains only available bytes.
// Callers must ensure the starting file offset exists even within a partially filled bank.
fn decode_instruction(rom: &[u8], rom_bank_count: u16, bank: u16, addr: u16) -> RawInstruction {
    let linear_address = rom_linear_index(bank, addr, rom_bank_count).unwrap_or(0) as u32;
    let Some(index) = rom_linear_index(bank, addr, rom_bank_count) else {
        return invalid_instruction(bank, addr, linear_address, vec![]);
    };
    // Decode from the physical file slice; operand lookahead is not clipped at a 16 KiB bank boundary.
    let op = *rom.get(index).unwrap_or(&0x00);
    let b1 = *rom.get(index + 1).unwrap_or(&0x00);
    let b2 = *rom.get(index + 2).unwrap_or(&0x00);
    let imm8 = format!("${:02X}", b1);
    let imm16 = format!("${:04X}", u16::from(b1) | (u16::from(b2) << 8));
    let signed = b1 as i8;
    let jr_target = addr.wrapping_add(2).wrapping_add(signed as i16 as u16);
    let d8 = format!("${:02X}", b1);
    let regs8 = ["B", "C", "D", "E", "H", "L", "[HL]", "A"];
    let regs16 = ["BC", "DE", "HL", "SP"];
    let regs16_push = ["BC", "DE", "HL", "AF"];
    let conds = ["NZ", "Z", "NC", "C"];
    let alu = ["ADD A", "ADC A", "SUB", "SBC A", "AND", "XOR", "OR", "CP"];
    let op_text = |mnemonic: &str,
                   operand_text: String,
                   len: u8,
                   flow_kind: FlowKind,
                   target_addr: Option<u16>,
                   is_conditional: bool,
                   fallthrough: bool| RawInstruction {
        bank,
        addr,
        linear_address,
        len,
        bytes: rom[index..rom.len().min(index + len as usize)].to_vec(),
        mnemonic: mnemonic.to_string(),
        operand_text,
        flow_kind,
        target_addr,
        is_conditional,
        fallthrough,
        is_invalid: false,
    };

    match op {
        0x00 => op_text("NOP", String::new(), 1, FlowKind::None, None, false, true),
        0x01 | 0x11 | 0x21 | 0x31 => {
            let rr = regs16[((op >> 4) & 0x03) as usize];
            op_text(
                "LD",
                format!("{rr},{imm16}"),
                3,
                FlowKind::None,
                None,
                false,
                true,
            )
        }
        0x02 => op_text(
            "LD",
            "[BC],A".to_string(),
            1,
            FlowKind::None,
            None,
            false,
            true,
        ),
        0x12 => op_text(
            "LD",
            "[DE],A".to_string(),
            1,
            FlowKind::None,
            None,
            false,
            true,
        ),
        0x22 => op_text(
            "LD",
            "[HL+],A".to_string(),
            1,
            FlowKind::None,
            None,
            false,
            true,
        ),
        0x32 => op_text(
            "LD",
            "[HL-],A".to_string(),
            1,
            FlowKind::None,
            None,
            false,
            true,
        ),
        0x0A => op_text(
            "LD",
            "A,[BC]".to_string(),
            1,
            FlowKind::None,
            None,
            false,
            true,
        ),
        0x1A => op_text(
            "LD",
            "A,[DE]".to_string(),
            1,
            FlowKind::None,
            None,
            false,
            true,
        ),
        0x2A => op_text(
            "LD",
            "A,[HL+]".to_string(),
            1,
            FlowKind::None,
            None,
            false,
            true,
        ),
        0x3A => op_text(
            "LD",
            "A,[HL-]".to_string(),
            1,
            FlowKind::None,
            None,
            false,
            true,
        ),
        0x03 | 0x13 | 0x23 | 0x33 => {
            let rr = regs16[((op >> 4) & 0x03) as usize];
            op_text("INC", rr.to_string(), 1, FlowKind::None, None, false, true)
        }
        0x09 | 0x19 | 0x29 | 0x39 => {
            let rr = regs16[((op >> 4) & 0x03) as usize];
            op_text(
                "ADD",
                format!("HL,{rr}"),
                1,
                FlowKind::None,
                None,
                false,
                true,
            )
        }
        0x0B | 0x1B | 0x2B | 0x3B => {
            let rr = regs16[((op >> 4) & 0x03) as usize];
            op_text("DEC", rr.to_string(), 1, FlowKind::None, None, false, true)
        }
        0x04..=0x3D if op & 0x07 == 0x04 => {
            let r = regs8[((op >> 3) & 0x07) as usize];
            op_text("INC", r.to_string(), 1, FlowKind::None, None, false, true)
        }
        0x05..=0x3D if op & 0x07 == 0x05 => {
            let r = regs8[((op >> 3) & 0x07) as usize];
            op_text("DEC", r.to_string(), 1, FlowKind::None, None, false, true)
        }
        0x06..=0x3E if op & 0x07 == 0x06 => {
            let r = regs8[((op >> 3) & 0x07) as usize];
            op_text(
                "LD",
                format!("{r},{d8}"),
                2,
                FlowKind::None,
                None,
                false,
                true,
            )
        }
        0x07 => op_text("RLCA", String::new(), 1, FlowKind::None, None, false, true),
        0x0F => op_text("RRCA", String::new(), 1, FlowKind::None, None, false, true),
        0x17 => op_text("RLA", String::new(), 1, FlowKind::None, None, false, true),
        0x1F => op_text("RRA", String::new(), 1, FlowKind::None, None, false, true),
        0x27 => op_text("DAA", String::new(), 1, FlowKind::None, None, false, true),
        0x2F => op_text("CPL", String::new(), 1, FlowKind::None, None, false, true),
        0x37 => op_text("SCF", String::new(), 1, FlowKind::None, None, false, true),
        0x3F => op_text("CCF", String::new(), 1, FlowKind::None, None, false, true),
        0x08 => op_text(
            "LD",
            format!("[{imm16}],SP"),
            3,
            FlowKind::None,
            Some(u16::from(b1) | (u16::from(b2) << 8)),
            false,
            true,
        ),
        0x10 => op_text("STOP", String::new(), 2, FlowKind::Stop, None, false, true),
        0x18 => op_text(
            "JR",
            format!("${:04X}", jr_target),
            2,
            FlowKind::Jump,
            Some(jr_target),
            false,
            false,
        ),
        0x20 | 0x28 | 0x30 | 0x38 => {
            let cond = conds[((op >> 3) & 0x03) as usize];
            op_text(
                "JR",
                format!("{cond},${:04X}", jr_target),
                2,
                FlowKind::JumpCond,
                Some(jr_target),
                true,
                true,
            )
        }
        0x40..=0x7F => {
            if op == 0x76 {
                op_text("HALT", String::new(), 1, FlowKind::Halt, None, false, true)
            } else {
                let dst = regs8[((op >> 3) & 0x07) as usize];
                let src = regs8[(op & 0x07) as usize];
                op_text(
                    "LD",
                    format!("{dst},{src}"),
                    1,
                    FlowKind::None,
                    None,
                    false,
                    true,
                )
            }
        }
        0x80..=0xBF => {
            let op_name = alu[((op >> 3) & 0x07) as usize];
            let src = regs8[(op & 0x07) as usize];
            op_text(
                op_name,
                src.to_string(),
                1,
                FlowKind::None,
                None,
                false,
                true,
            )
        }
        0xC0 | 0xC8 | 0xD0 | 0xD8 => {
            let cond = conds[((op >> 3) & 0x03) as usize];
            op_text(
                "RET",
                cond.to_string(),
                1,
                FlowKind::ReturnCond,
                None,
                true,
                true,
            )
        }
        0xC9 => op_text(
            "RET",
            String::new(),
            1,
            FlowKind::Return,
            None,
            false,
            false,
        ),
        0xD9 => op_text("RETI", String::new(), 1, FlowKind::Reti, None, false, false),
        0xC2 | 0xCA | 0xD2 | 0xDA => {
            let cond = conds[((op >> 3) & 0x03) as usize];
            let target = u16::from(b1) | (u16::from(b2) << 8);
            op_text(
                "JP",
                format!("{cond},{imm16}"),
                3,
                FlowKind::JumpCond,
                Some(target),
                true,
                true,
            )
        }
        0xC3 => {
            let target = u16::from(b1) | (u16::from(b2) << 8);
            op_text(
                "JP",
                imm16.clone(),
                3,
                FlowKind::Jump,
                Some(target),
                false,
                false,
            )
        }
        0xE9 => op_text(
            "JP",
            "HL".to_string(),
            1,
            FlowKind::IndirectJump,
            None,
            false,
            false,
        ),
        0xC4 | 0xCC | 0xD4 | 0xDC => {
            let cond = conds[((op >> 3) & 0x03) as usize];
            let target = u16::from(b1) | (u16::from(b2) << 8);
            op_text(
                "CALL",
                format!("{cond},{imm16}"),
                3,
                FlowKind::CallCond,
                Some(target),
                true,
                true,
            )
        }
        0xCD => {
            let target = u16::from(b1) | (u16::from(b2) << 8);
            op_text(
                "CALL",
                imm16.clone(),
                3,
                FlowKind::Call,
                Some(target),
                false,
                true,
            )
        }
        0xC1 | 0xD1 | 0xE1 | 0xF1 => {
            let rr = regs16_push[((op >> 4) & 0x03) as usize];
            op_text("POP", rr.to_string(), 1, FlowKind::None, None, false, true)
        }
        0xC5 | 0xD5 | 0xE5 | 0xF5 => {
            let rr = regs16_push[((op >> 4) & 0x03) as usize];
            op_text("PUSH", rr.to_string(), 1, FlowKind::None, None, false, true)
        }
        0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => {
            let base = match op {
                0xC6 => "ADD A",
                0xCE => "ADC A",
                0xD6 => "SUB",
                0xDE => "SBC A",
                0xE6 => "AND",
                0xEE => "XOR",
                0xF6 => "OR",
                0xFE => "CP",
                _ => unreachable!(),
            };
            op_text(base, imm8.clone(), 2, FlowKind::None, None, false, true)
        }
        0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => {
            let target = u16::from(op & 0x38);
            op_text(
                "RST",
                format!("${target:02X}"),
                1,
                FlowKind::Rst,
                Some(target),
                false,
                true,
            )
        }
        0xCB => decode_cb_instruction(bank, addr, linear_address, op, b1, rom, index),
        // The static decoder marks F4 invalid even though the current execution core accepts an F4 call variant.
        0xD3 | 0xDB | 0xDD | 0xE3 | 0xE4 | 0xEB | 0xEC | 0xED | 0xF4 | 0xFC | 0xFD => {
            invalid_instruction(bank, addr, linear_address, vec![op])
        }
        0xE0 => op_text(
            "LDH",
            format!("[$FF00+{imm8}],A"),
            2,
            FlowKind::None,
            Some(0xFF00 + u16::from(b1)),
            false,
            true,
        ),
        0xE2 => op_text(
            "LD",
            "[C],A".to_string(),
            1,
            FlowKind::None,
            None,
            false,
            true,
        ),
        0xE8 => op_text(
            "ADD",
            format!("SP,{:+}", signed),
            2,
            FlowKind::None,
            None,
            false,
            true,
        ),
        0xEA => op_text(
            "LD",
            format!("[{imm16}],A"),
            3,
            FlowKind::None,
            Some(u16::from(b1) | (u16::from(b2) << 8)),
            false,
            true,
        ),
        0xF0 => op_text(
            "LDH",
            format!("A,[$FF00+{imm8}]"),
            2,
            FlowKind::None,
            Some(0xFF00 + u16::from(b1)),
            false,
            true,
        ),
        0xF2 => op_text(
            "LD",
            "A,[C]".to_string(),
            1,
            FlowKind::None,
            None,
            false,
            true,
        ),
        0xF3 => op_text("DI", String::new(), 1, FlowKind::None, None, false, true),
        0xF8 => op_text(
            "LD",
            format!("HL,SP{:+}", signed),
            2,
            FlowKind::None,
            None,
            false,
            true,
        ),
        0xF9 => op_text(
            "LD",
            "SP,HL".to_string(),
            1,
            FlowKind::None,
            None,
            false,
            true,
        ),
        0xFA => op_text(
            "LD",
            format!("A,[{imm16}]"),
            3,
            FlowKind::None,
            Some(u16::from(b1) | (u16::from(b2) << 8)),
            false,
            true,
        ),
        0xFB => op_text("EI", String::new(), 1, FlowKind::None, None, false, true),
        _ => invalid_instruction(bank, addr, linear_address, vec![op]),
    }
}

// Split the CB byte into rotation/bit group and register operand, retaining a two-byte
// logical length even if the available file slice is shorter.
fn decode_cb_instruction(
    bank: u16,
    addr: u16,
    linear_address: u32,
    op: u8,
    cb: u8,
    rom: &[u8],
    index: usize,
) -> RawInstruction {
    let regs8 = ["B", "C", "D", "E", "H", "L", "[HL]", "A"];
    let rot = ["RLC", "RRC", "RL", "RR", "SLA", "SRA", "SWAP", "SRL"];
    let x = cb >> 6;
    let y = (cb >> 3) & 0x07;
    let z = cb & 0x07;
    let operand = regs8[z as usize];
    let (mnemonic, operand_text) = match x {
        0 => (rot[y as usize].to_string(), operand.to_string()),
        1 => ("BIT".to_string(), format!("{y},{operand}")),
        2 => ("RES".to_string(), format!("{y},{operand}")),
        _ => ("SET".to_string(), format!("{y},{operand}")),
    };
    RawInstruction {
        bank,
        addr,
        linear_address,
        len: 2,
        bytes: rom[index..rom.len().min(index + 2)].to_vec(),
        mnemonic,
        operand_text,
        flow_kind: FlowKind::None,
        target_addr: None,
        is_conditional: false,
        fallthrough: true,
        is_invalid: op != 0xCB,
    }
}

// Represent an undecodable byte/address as a one-byte non-fallthrough DB placeholder.
fn invalid_instruction(
    bank: u16,
    addr: u16,
    linear_address: u32,
    bytes: Vec<u8>,
) -> RawInstruction {
    RawInstruction {
        bank,
        addr,
        linear_address,
        len: 1,
        bytes,
        mnemonic: "DB".to_string(),
        operand_text: "<invalid>".to_string(),
        flow_kind: FlowKind::Invalid,
        target_addr: None,
        is_conditional: false,
        fallthrough: false,
        is_invalid: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::{FunctionInfo, SymbolTable};
    use std::{fs, path::Path};

    #[test]
    // Decode synthetic CALL/RET bytes and check mnemonic and immediate call target.
    fn decodes_basic_call_and_ret() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0xCD;
        rom[0x101] = 0x10;
        rom[0x102] = 0x01;
        rom[0x103] = 0xC9;
        let call = decode_instruction(&rom, 2, 0, 0x0100);
        assert_eq!(call.mnemonic, "CALL");
        assert_eq!(call.target_addr, Some(0x0110));
        let ret = decode_instruction(&rom, 2, 0, 0x0103);
        assert_eq!(ret.mnemonic, "RET");
    }

    #[test]
    // Supply a one-byte named function and check metadata naming in the generated report.
    fn analyzes_named_function_from_metadata() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0xC9;
        let table = SymbolTable {
            functions: vec![FunctionInfo {
                name: "Main".to_string(),
                bank: 0,
                start: 0x0100,
                end: 0x0101,
                size_bytes: 1,
                section: None,
                is_stack_call: false,
                is_fast_call: false,
                has_fixed_bank: true,
                has_fixed_order: false,
                return_size: 0,
                param_sizes: vec![],
            }],
            ..Default::default()
        };
        let report = analyze_rom(&rom, Some(&table), &DecompileOptions::default());
        assert!(report.functions.iter().any(|f| f.name == "Main"));
    }

    #[test]
    // Check the entry appears with include-all and an empty selector list; despite the name,
    // this fixture does not pass an explicit bank/address selector.
    fn selected_function_filter_accepts_bank_addr() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0xC9;
        let report = analyze_rom(
            &rom,
            None,
            &DecompileOptions {
                selected_functions: Vec::new(),
                include_all_named_functions: true,
            },
        );
        assert!(report.functions.iter().any(|f| f.start_address == 0x0100));
    }

    #[test]
    // Apply function/label/report overrides and check the new display name with the canonical name retained.
    fn annotation_overlay_can_rename_and_note_function() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0xC9;
        let mut report = analyze_rom(
            &rom,
            None,
            &DecompileOptions {
                selected_functions: vec!["00:0100".to_string()],
                include_all_named_functions: false,
            },
        );
        let annotations = DecompileAnnotationFile {
            schema_version: "decompile_annotation_v1".to_string(),
            function_overrides: vec![DecompileFunctionOverride {
                selector: "00:0100".to_string(),
                rename: Some("EntryPoint".to_string()),
                calling_convention: Some("fastcall".to_string()),
                notes: vec!["seed note".to_string()],
            }],
            label_overrides: vec![DecompileLabelOverride {
                bank: 0,
                addr: 0x0100,
                name: "entry_label".to_string(),
                kind: "user".to_string(),
            }],
            notes: vec!["report note".to_string()],
        };
        let applied = apply_annotations(&mut report, &annotations);
        assert!(applied >= 4);
        let function = report
            .functions
            .iter()
            .find(|function| function.start_address == 0x0100)
            .expect("function");
        assert_eq!(function.name, "EntryPoint");
        assert_eq!(function.canonical_name, "sub_bank00_0100");
        assert_eq!(
            function.calling_convention_guess.as_deref(),
            Some("fastcall")
        );
        assert!(function.user_notes.iter().any(|note| note == "seed note"));
        assert!(report
            .labels
            .iter()
            .any(|label| label.bank == 0 && label.addr == 0x0100 && label.name == "entry_label"));
        assert!(report.notes.iter().any(|note| note == "report note"));
    }

    #[test]
    // Supply two synthetic observations and check one matched function, hit/frame summary and annotation.
    fn trace_overlay_records_runtime_hits() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0x00;
        rom[0x101] = 0x00;
        rom[0x102] = 0xC9;
        let mut report = analyze_rom(
            &rom,
            None,
            &DecompileOptions {
                selected_functions: vec!["00:0100".to_string()],
                include_all_named_functions: false,
            },
        );
        let observations = vec![
            DecompileTraceObservation {
                completed_frames: Some(10),
                active_frame: Some(10),
                cycle: Some(100),
                pc: 0x0100,
                rom_bank: 0,
                hit_count: 1,
                symbol: Some("Main".to_string()),
                source: Some("src/main.c:10".to_string()),
                event_summary: Some("event:a".to_string()),
                watch_summary: Some("watch:a".to_string()),
                registers: BTreeMap::new(),
                flags: BTreeMap::new(),
                stack_slot_values: BTreeMap::new(),
                sp: None,
            },
            DecompileTraceObservation {
                completed_frames: Some(12),
                active_frame: Some(12),
                cycle: Some(140),
                pc: 0x0101,
                rom_bank: 0,
                hit_count: 1,
                symbol: Some("Main".to_string()),
                source: Some("src/main.c:11".to_string()),
                event_summary: Some("event:b".to_string()),
                watch_summary: Some("watch:b".to_string()),
                registers: BTreeMap::new(),
                flags: BTreeMap::new(),
                stack_slot_values: BTreeMap::new(),
                sp: None,
            },
        ];
        let applied = apply_trace_observations(&mut report, &observations);
        assert_eq!(applied, 1);
        let summary = report.functions[0]
            .artifact
            .trace_summary
            .as_ref()
            .expect("trace summary");
        assert_eq!(summary.hit_count, 2);
        assert_eq!(summary.first_frame, Some(10));
        assert_eq!(summary.last_frame, Some(12));
        assert_eq!(summary.hot_pcs[0].pc, 0x0100);
        assert!(report.functions[0]
            .artifact
            .trace_annotations
            .iter()
            .any(|note| note.contains("runtime hit")));
    }

    #[test]
    // Decode an SP-relative read and PUSH/POP pair, checking candidate names/accesses and pseudocode mentions.
    fn pseudocode_infers_stack_and_temp_slots() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0xF8; // ld hl,sp+2
        rom[0x101] = 0x02;
        rom[0x102] = 0x7E; // ld a,[hl]
        rom[0x103] = 0xC5; // push bc
        rom[0x104] = 0xC1; // pop bc
        rom[0x105] = 0xC9; // ret
        let report = analyze_rom(
            &rom,
            None,
            &DecompileOptions {
                selected_functions: vec!["00:0100".to_string()],
                include_all_named_functions: false,
            },
        );
        let artifact = &report.functions[0].artifact;
        assert!(artifact
            .stack_slots
            .iter()
            .any(|slot| slot.name.starts_with("arg_")));
        assert!(artifact.stack_slots.iter().any(|slot| slot.read_count > 0));
        assert!(artifact.temp_slots.iter().any(|slot| slot.name == "tmp_bc"));
        assert!(artifact.pseudocode_text.contains("arg_0"));
        assert!(artifact.pseudocode_text.contains("tmp_bc"));
    }

    #[test]
    // Use INC, BIT and auto-increment loads through separate SP aliases to exercise three candidate type labels.
    fn stack_slot_type_inference_can_mark_bool_counter_and_pointer_patterns() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0xF8; // ld hl,sp+2
        rom[0x101] = 0x02;
        rom[0x102] = 0x34; // inc [hl]
        rom[0x103] = 0xF8; // ld hl,sp+4
        rom[0x104] = 0x04;
        rom[0x105] = 0xCB; // bit 0,[hl]
        rom[0x106] = 0x46;
        rom[0x107] = 0xF8; // ld hl,sp+6
        rom[0x108] = 0x06;
        rom[0x109] = 0x2A; // ld a,[hl+]
        rom[0x10A] = 0xC9; // ret
        let report = analyze_rom(
            &rom,
            None,
            &DecompileOptions {
                selected_functions: vec!["00:0100".to_string()],
                include_all_named_functions: false,
            },
        );
        let slots = &report.functions[0].artifact.stack_slots;
        assert!(slots
            .iter()
            .any(|slot| slot.inferred_type.as_deref() == Some("counter_candidate")));
        assert!(slots
            .iter()
            .any(|slot| slot.inferred_type.as_deref() == Some("bool_candidate")));
        assert!(slots
            .iter()
            .any(|slot| slot.inferred_type.as_deref() == Some("pointer_candidate")));
    }

    #[test]
    // Overlay three authored value snapshots and check counter, flag, pointer and enum candidate refinements.
    fn trace_values_can_refine_runtime_roles_and_types() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0xF8; // ld hl,sp+2
        rom[0x101] = 0x02;
        rom[0x102] = 0x34; // inc [hl]
        rom[0x103] = 0xF8; // ld hl,sp+4
        rom[0x104] = 0x04;
        rom[0x105] = 0xCB; // bit 0,[hl]
        rom[0x106] = 0x46;
        rom[0x107] = 0xF8; // ld hl,sp+6
        rom[0x108] = 0x06;
        rom[0x109] = 0x2A; // ld a,[hl+]
        rom[0x10A] = 0xF8; // ld hl,sp+8
        rom[0x10B] = 0x08;
        rom[0x10C] = 0x36; // ld [hl],$02
        rom[0x10D] = 0x02;
        rom[0x10E] = 0xC9; // ret
        let mut report = analyze_rom(
            &rom,
            None,
            &DecompileOptions {
                selected_functions: vec!["00:0100".to_string()],
                include_all_named_functions: false,
            },
        );
        let observations = vec![
            DecompileTraceObservation {
                completed_frames: Some(1),
                active_frame: Some(1),
                cycle: Some(10),
                pc: 0x0102,
                rom_bank: 0,
                hit_count: 1,
                symbol: None,
                source: None,
                event_summary: None,
                watch_summary: None,
                registers: BTreeMap::new(),
                flags: BTreeMap::new(),
                stack_slot_values: BTreeMap::from([
                    ("2".to_string(), 1),
                    ("4".to_string(), 0),
                    ("6".to_string(), 0xC120),
                    ("8".to_string(), 2),
                ]),
                sp: Some(0xDFF0),
            },
            DecompileTraceObservation {
                completed_frames: Some(2),
                active_frame: Some(2),
                cycle: Some(20),
                pc: 0x0105,
                rom_bank: 0,
                hit_count: 1,
                symbol: None,
                source: None,
                event_summary: None,
                watch_summary: None,
                registers: BTreeMap::new(),
                flags: BTreeMap::new(),
                stack_slot_values: BTreeMap::from([
                    ("2".to_string(), 2),
                    ("4".to_string(), 1),
                    ("6".to_string(), 0xC121),
                    ("8".to_string(), 4),
                ]),
                sp: Some(0xDFF0),
            },
            DecompileTraceObservation {
                completed_frames: Some(3),
                active_frame: Some(3),
                cycle: Some(30),
                pc: 0x0109,
                rom_bank: 0,
                hit_count: 1,
                symbol: None,
                source: None,
                event_summary: None,
                watch_summary: None,
                registers: BTreeMap::new(),
                flags: BTreeMap::new(),
                stack_slot_values: BTreeMap::from([
                    ("2".to_string(), 3),
                    ("4".to_string(), 0),
                    ("6".to_string(), 0xC122),
                    ("8".to_string(), 7),
                ]),
                sp: Some(0xDFF0),
            },
        ];
        apply_trace_observations(&mut report, &observations);
        let slots = &report.functions[0].artifact.stack_slots;
        assert!(slots
            .iter()
            .any(|slot| slot.offset == 2
                && slot.runtime_role.as_deref() == Some("runtime_counter_like")));
        assert!(slots
            .iter()
            .any(|slot| slot.offset == 4
                && slot.runtime_role.as_deref() == Some("runtime_flag_like")));
        assert!(slots
            .iter()
            .any(|slot| slot.offset == 6
                && slot.runtime_role.as_deref() == Some("runtime_pointer_like")));
        assert!(slots
            .iter()
            .any(|slot| slot.offset == 8
                && slot.inferred_type.as_deref() == Some("enum_like_candidate")));
    }

    #[test]
    // Place accesses to one SP offset before and after a call and check the address-order lifetime hint.
    fn stack_slot_lifetime_can_detect_across_call_boundary() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0xF8; // ld hl,sp+2
        rom[0x101] = 0x02;
        rom[0x102] = 0x7E; // ld a,[hl]
        rom[0x103] = 0xCD; // call 010A
        rom[0x104] = 0x0A;
        rom[0x105] = 0x01;
        rom[0x106] = 0xF8; // ld hl,sp+2
        rom[0x107] = 0x02;
        rom[0x108] = 0x77; // ld [hl],a
        rom[0x109] = 0xC9; // ret
        rom[0x10A] = 0x00; // nop
        rom[0x10B] = 0xC9; // ret
        let report = analyze_rom(
            &rom,
            None,
            &DecompileOptions {
                selected_functions: vec!["00:0100".to_string()],
                include_all_named_functions: false,
            },
        );
        let slots = &report.functions[0].artifact.stack_slots;
        assert!(slots.iter().any(|slot| slot.offset == 2
            && slot.lifetime_hint.as_deref() == Some("across_call_boundary")));
    }

    #[test]
    // Supply register and stack values at a synthetic call PC and check value strings plus a mixed-call guess.
    fn trace_values_can_refine_call_site_argument_hints() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0x3E; // ld a,$05
        rom[0x101] = 0x05;
        rom[0x102] = 0xF8; // ld hl,sp+2
        rom[0x103] = 0x02;
        rom[0x104] = 0xCD; // call 0110
        rom[0x105] = 0x10;
        rom[0x106] = 0x01;
        rom[0x107] = 0xC9; // ret
        rom[0x110] = 0xC9; // ret
        let mut report = analyze_rom(
            &rom,
            None,
            &DecompileOptions {
                selected_functions: vec!["00:0100".to_string()],
                include_all_named_functions: false,
            },
        );
        let observations = vec![DecompileTraceObservation {
            completed_frames: Some(1),
            active_frame: Some(1),
            cycle: Some(20),
            pc: 0x0104,
            rom_bank: 0,
            hit_count: 1,
            symbol: None,
            source: None,
            event_summary: None,
            watch_summary: None,
            registers: BTreeMap::from([("A".to_string(), 5), ("HL".to_string(), 0xDFF2)]),
            flags: BTreeMap::from([
                ("Z".to_string(), false),
                ("N".to_string(), false),
                ("H".to_string(), false),
                ("C".to_string(), false),
            ]),
            stack_slot_values: BTreeMap::from([("2".to_string(), 0x002A)]),
            sp: Some(0xDFF0),
        }];
        apply_trace_observations(&mut report, &observations);
        let call_sites = &report.functions[0].artifact.call_sites;
        assert_eq!(call_sites.len(), 1);
        assert!(call_sites[0]
            .runtime_argument_hints
            .iter()
            .any(|hint| hint == "A=$0005"));
        assert!(call_sites[0]
            .runtime_argument_hints
            .iter()
            .any(|hint| hint == "SP+2=$002A"));
        assert_eq!(
            call_sites[0].calling_convention_hint.as_deref(),
            Some("mixed_call")
        );
    }

    #[test]
    // Select a caller and verify its plausible callee is also emitted by the later recovery pass.
    fn recovered_function_heuristics_can_promote_call_target() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0xCD; // call 0110
        rom[0x101] = 0x10;
        rom[0x102] = 0x01;
        rom[0x103] = 0xC9; // ret
        rom[0x110] = 0xF5; // push af
        rom[0x111] = 0xF1; // pop af
        rom[0x112] = 0xC9; // ret
        let report = analyze_rom(
            &rom,
            None,
            &DecompileOptions {
                selected_functions: vec!["00:0100".to_string()],
                include_all_named_functions: false,
            },
        );
        assert!(
            report
                .functions
                .iter()
                .any(|function| function.start_address == 0x0110),
            "functions={:?} labels={:?}",
            report
                .functions
                .iter()
                .map(|function| (
                    function.bank,
                    function.start_address,
                    function.source_kind.clone()
                ))
                .collect::<Vec<_>>(),
            report
                .labels
                .iter()
                .map(|label| (label.bank, label.addr, label.kind.clone()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    // Select a caller whose target starts with invalid opcodes and check that recovery rejects that target.
    fn recovered_function_heuristics_reject_invalid_heavy_target() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0xCD; // call 0110
        rom[0x101] = 0x10;
        rom[0x102] = 0x01;
        rom[0x103] = 0xC9; // ret
        rom[0x110] = 0xD3; // invalid
        rom[0x111] = 0xDB; // invalid
        rom[0x112] = 0xDD; // invalid
        let report = analyze_rom(
            &rom,
            None,
            &DecompileOptions {
                selected_functions: vec!["00:0100".to_string()],
                include_all_named_functions: false,
            },
        );
        assert!(!report
            .functions
            .iter()
            .any(|function| function.start_address == 0x0110));
    }

    #[test]
    // Select only a callee and check incoming-reference evidence remains from the unselected caller.
    fn selected_function_reports_incoming_xrefs_from_unselected_callers() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0xCD; // call 0110
        rom[0x101] = 0x10;
        rom[0x102] = 0x01;
        rom[0x103] = 0xC9; // ret
        rom[0x110] = 0x00; // nop
        rom[0x111] = 0xC9; // ret
        let report = analyze_rom(
            &rom,
            None,
            &DecompileOptions {
                selected_functions: vec!["00:0110".to_string()],
                include_all_named_functions: false,
            },
        );
        assert_eq!(report.functions.len(), 1);
        assert!(report.functions[0]
            .xrefs_in
            .iter()
            .any(|xref| xref.from_addr == 0x0100));
    }

    #[test]
    // Optionally load local ROM/metadata fixtures and check a named thunk hint. Missing files
    // return immediately, so a successful test process does not establish that this fixture ran.
    fn sample_rom_bank_thunk_gets_precision_hints() {
        let rom_path = Path::new("C:\\kitaqgb_project\\sample_roms\\sample_game.gb");
        let dbg2_path = Path::new("C:\\kitaqgb_project\\sample_roms\\sample_game.dbg2.json");
        if !rom_path.exists() || !dbg2_path.exists() {
            return;
        }
        let rom = fs::read(rom_path).unwrap();
        let symbol_table: SymbolTable =
            serde_json::from_str(&fs::read_to_string(dbg2_path).unwrap()).unwrap();
        let report = analyze_rom(
            &rom,
            Some(&symbol_table),
            &DecompileOptions {
                selected_functions: vec!["00:0E4D".to_string()],
                include_all_named_functions: false,
            },
        );
        let function = report
            .functions
            .iter()
            .find(|function| function.name == "__kq_thunk_b6_InitCgbPalettes")
            .unwrap();
        assert!(function
            .artifact
            .fingerprints
            .iter()
            .any(|value| value == "kitaqgb_bank_thunk_shape"));
        assert!(function
            .artifact
            .suggestions
            .iter()
            .any(|value| value.contains("InitCgbPalettes")));
    }

    #[test]
    // Construct SP-setup and store-loop instruction shapes and assert the two heuristic fingerprint labels.
    fn fingerprint_dictionary_can_detect_stack_frame_and_memset_shapes() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0xF8; // ld hl,sp+2
        rom[0x101] = 0x02;
        rom[0x102] = 0xF8; // ld hl,sp+4
        rom[0x103] = 0x04;
        rom[0x104] = 0x36; // ld [hl],$00
        rom[0x105] = 0x00;
        rom[0x106] = 0x23; // inc hl
        rom[0x107] = 0x0B; // dec bc
        rom[0x108] = 0x20; // jr nz,+0
        rom[0x109] = 0x00;
        rom[0x10A] = 0xC9; // ret
        let report = analyze_rom(
            &rom,
            None,
            &DecompileOptions {
                selected_functions: vec!["00:0100".to_string()],
                include_all_named_functions: false,
            },
        );
        let fingerprints = &report.functions[0].artifact.fingerprints;
        assert!(fingerprints
            .iter()
            .any(|value| value == "kitaqgb_lowerer_stack_frame_shape"));
        assert!(fingerprints
            .iter()
            .any(|value| value == "kitaqgb_memset_like_loop"));
    }

    #[test]
    // Construct a loader-like sequence and pointer bytes before JP HL, then check candidate presence.
    // This test does not execute the dispatch or validate the inferred table length.
    fn switch_candidate_detection_can_use_loader_pattern() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0x21; // ld hl,$0120
        rom[0x101] = 0x20;
        rom[0x102] = 0x01;
        rom[0x103] = 0x5F; // ld e,a
        rom[0x104] = 0x16; // ld d,$00
        rom[0x105] = 0x00;
        rom[0x106] = 0x29; // add hl,hl
        rom[0x107] = 0x19; // add hl,de
        rom[0x108] = 0xE9; // jp hl
        rom[0x120] = 0x30;
        rom[0x121] = 0x01;
        rom[0x122] = 0x40;
        rom[0x123] = 0x01;
        let report = analyze_rom(
            &rom,
            None,
            &DecompileOptions {
                selected_functions: vec!["00:0100".to_string()],
                include_all_named_functions: false,
            },
        );
        assert!(!report.functions[0].switch_candidates.is_empty());
    }
}
