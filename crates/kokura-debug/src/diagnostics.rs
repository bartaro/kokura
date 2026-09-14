use std::collections::BTreeSet;

use kokura_core::Machine;
use serde::Serialize;

use crate::stop::StopReason;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
// Stable diagnostic identities; a code may describe a routine observation rather than an error.
pub enum DiagnosticCode {
    WhiteScreenLcdcOff,
    WhiteScreenNoVblank,
    WhiteScreenFrameStatic,
    PcStuckLoop,
    TimerInactive,
    InterruptsPendingButImeOff,
    FarCallBankNotRestored,
    IntrinsicNoVisibleEffect,
    SetTileFlushNoVisibleEffect,
    FarCallStorm,
    RepeatedBankThrash,
    SetTileBufferedWithoutFlush,
    SetTile16CgbUsedInDmgMode,
    FlushRowsNoVisibleEffect,
    FlushLikeWithoutVblank,
    OamDmaNoVisibleEffect,
    CoreOamDmaNoOamChange,
    WaitVBlankSeenNoVblank,
    CgbPaletteUsedInDmgMode,
    TrapCheckHotLoop,
    BankGuardThrash,
    OamTransferNoVisibleEffect,
    PresentPathNoVisibleEffect,
    WaitVBlankPresentOrderingSuspicious,
    InputReadNoVisibleEffect,
    InputReadNoProgress,
    TransferOamWithoutPresent,
    VramChangedButFrameStatic,
    BgDisabledButRenderPathActive,
    TransferOamSpriteStatic,
    OamChangedButSpriteStatic,
    WindowEnabledButInvisible,
    InputReadWithoutJoypadMask,
    SpritesEnabledButOamEmpty,
    TransferOamNoSpriteChange,
    PresentNoLayerChange,
    WindowMapChangedButWindowStatic,
    BgMapChangedButBgStatic,
    SpriteHashStaticAfterOamTransfer,
    UnsupportedOpcodeEncountered,
    StatIrqPathSuspicious,
    ScanlineProgressSuspicious,
    LcdToggleDuringRun,
    StatRegisterWriteObserved,
    LycRetargetObserved,
    HdmaTransferNoVramEffect,
    HdmaDeferredWhileCpuHalted,
    HdmaControlWriteIgnored,
    SerialTransferNoInterrupt,
    JoypadEdgeNoInterrupt,
    InterruptPendingButNotServiced,
    ApuActiveNoPcm,
    ApuPcmBufferOverflow,
    ApuPopRiskObserved,
    ApuWaveRamAliasedWhileCh3Active,
    ApuNoiseClockFrozen,
    SgbHeaderObservedOutOfScope,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
// Express the rule's reporting importance separately from heuristic candidate priority.
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize)]
// Pair a structured identity and severity with an English explanation of the observed condition.
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoDiagnosisCategory {
    Rendering,
    Banking,
    Timing,
    Interrupts,
    Input,
    Audio,
    Unsupported,
    Replay,
}

#[derive(Debug, Clone, Serialize)]
// Keep score/confidence labels, concrete evidence and proposed next steps together.
// stop_related is a classification hint, not proof the condition caused the stop.
pub struct AutoDiagnosisSuspect {
    pub key: String,
    pub category: AutoDiagnosisCategory,
    pub score: u32,
    pub confidence: String,
    pub title: String,
    pub summary: String,
    pub evidence: Vec<String>,
    pub next_steps: Vec<String>,
    pub related_diagnostics: Vec<String>,
    pub stop_related: bool,
}

#[derive(Debug, Clone, Serialize)]
// Return a bounded ranked list and human-readable follow-up guidance without running those steps.
pub struct AutoDiagnosisReport {
    pub generated: bool,
    pub max_items: usize,
    pub suspects: Vec<AutoDiagnosisSuspect>,
    pub recommended_focus: Vec<String>,
    pub carry_forward_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
// Separate observation, implementation and validation labels in the report schema.
pub enum TimingPackLevel {
    Obs,
    Impl,
    Val,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
// Keep verification status independent of feature availability; the builder below never emits Validated.
pub enum TimingPackVerification {
    RegressionOnly,
    Candidate,
    Validated,
}

#[derive(Debug, Clone, Serialize)]
// Record evidence, known gaps and suggested job/alias references for one subsystem.
// Referenced jobs are labels here; the report builder does not check or execute those paths.
pub struct TimingPackEntry {
    pub key: String,
    pub title: String,
    pub level: TimingPackLevel,
    pub verification: TimingPackVerification,
    pub summary: String,
    pub evidence: Vec<String>,
    pub gaps: Vec<String>,
    pub related_jobs: Vec<String>,
    pub related_aliases: Vec<String>,
    pub next_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimingPackReport {
    pub generated: bool,
    pub packs: Vec<TimingPackEntry>,
    pub carry_forward_notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default)]
// Supply a consistent observation window and current-mode flags. Derived defaults
// are zero/false placeholders, not a substitute for collecting actual machine observations.
pub struct DiagnosticInput {
    pub vblank_events: u64,
    pub screen_changed: bool,
    pub repeated_pc_hits: u32,
    pub executed_frames: u64,
    pub timer_interrupts: u64,
    pub timer_overflows: u64,
    pub serial_interrupts: u64,
    pub joypad_interrupts: u64,
    pub joypad_read_count: u64,
    pub joypad_button_read_count: u64,
    pub joypad_dpad_read_count: u64,
    pub interrupt_services: u64,
    pub interrupt_pending_blocked_count: u64,
    pub apu_trigger_count: u64,
    pub apu_mix_output_count: u64,
    pub apu_pop_risk_count: u64,
    pub apu_wave_alias_count: u64,
    pub apu_noise_lock_count: u64,
    pub apu_pcm_frame_count: u64,
    pub apu_pcm_drop_count: u64,
    pub apu_pcm_peak_buffer_frames: u32,
    pub far_call_count: u64,
    pub intrinsic_count: u64,
    pub settile_flush_count: u64,
    pub bank_switches: u64,
    pub bank_restored: bool,
    pub bank_thrash_score: u32,
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
    pub cgb_palette_count: u64,
    pub bank_guard_count: u64,
    pub trap_check_count: u64,
    pub oam_transfer_count: u64,
    pub dmg_mode: bool,
    pub vram_changed: bool,
    pub bg_enabled: bool,
    pub present_path_count: u64,
    pub input_path_count: u64,
    pub input_active: bool,
    pub wait_before_present: bool,
    pub oam_changed: bool,
    pub visible_sprite_count: u32,
    pub nonzero_oam_entries: u32,
    pub sprite_enabled: bool,
    pub window_enabled: bool,
    pub bg_changed: bool,
    pub window_changed: bool,
    pub sprite_changed: bool,
    pub bg_hash_changed: bool,
    pub window_hash_changed: bool,
    pub sprite_hash_changed: bool,
    pub stat_signal_count: u64,
    pub scanline_event_count: u64,
    pub unsupported_opcode_count: u64,
    pub halted_on_unsupported_opcode: bool,
    pub lcd_toggle_count: u64,
    pub stat_write_count: u64,
    pub lyc_write_count: u64,
    pub scanline_render_count: u64,
}

// Recognize either a mapped input-routine hit or an actual FF00/P1 bus read as input sampling.
fn has_input_sampling(input: &DiagnosticInput) -> bool {
    input.input_path_count > 0 || input.joypad_read_count > 0
}

// Describe the observed sampling source and counts without treating missing routine symbols as no input.
fn input_sampling_clause(input: &DiagnosticInput) -> String {
    match (input.input_path_count > 0, input.joypad_read_count > 0) {
        (true, true) => format!(
            "Pad_Read-like path hit {} time(s) while FF00/P1 was read {} time(s) (buttons {}, dpad {})",
            input.input_path_count,
            input.joypad_read_count,
            input.joypad_button_read_count,
            input.joypad_dpad_read_count
        ),
        (true, false) => format!("Pad_Read-like path hit {} time(s)", input.input_path_count),
        (false, true) => format!(
            "FF00/P1 was read {} time(s) (buttons {}, dpad {})",
            input.joypad_read_count, input.joypad_button_read_count, input.joypad_dpad_read_count
        ),
        (false, false) => "No input sampling was observed".to_string(),
    }
}

// Map enum variants to stable snake-case cross-reference labels used in generated findings.
fn diagnostic_code_slug(code: DiagnosticCode) -> &'static str {
    match code {
        DiagnosticCode::WhiteScreenLcdcOff => "white_screen_lcdc_off",
        DiagnosticCode::WhiteScreenNoVblank => "white_screen_no_vblank",
        DiagnosticCode::WhiteScreenFrameStatic => "white_screen_frame_static",
        DiagnosticCode::PcStuckLoop => "pc_stuck_loop",
        DiagnosticCode::TimerInactive => "timer_inactive",
        DiagnosticCode::InterruptsPendingButImeOff => "interrupts_pending_but_ime_off",
        DiagnosticCode::FarCallBankNotRestored => "far_call_bank_not_restored",
        DiagnosticCode::IntrinsicNoVisibleEffect => "intrinsic_no_visible_effect",
        DiagnosticCode::SetTileFlushNoVisibleEffect => "settile_flush_no_visible_effect",
        DiagnosticCode::FarCallStorm => "far_call_storm",
        DiagnosticCode::RepeatedBankThrash => "repeated_bank_thrash",
        DiagnosticCode::SetTileBufferedWithoutFlush => "settile_buffered_without_flush",
        DiagnosticCode::SetTile16CgbUsedInDmgMode => "settile16_cgb_used_in_dmg_mode",
        DiagnosticCode::FlushRowsNoVisibleEffect => "flush_rows_no_visible_effect",
        DiagnosticCode::FlushLikeWithoutVblank => "flush_like_without_vblank",
        DiagnosticCode::OamDmaNoVisibleEffect => "oam_dma_no_visible_effect",
        DiagnosticCode::CoreOamDmaNoOamChange => "core_oam_dma_no_oam_change",
        DiagnosticCode::WaitVBlankSeenNoVblank => "wait_vblank_seen_no_vblank",
        DiagnosticCode::CgbPaletteUsedInDmgMode => "cgb_palette_used_in_dmg_mode",
        DiagnosticCode::TrapCheckHotLoop => "trap_check_hot_loop",
        DiagnosticCode::BankGuardThrash => "bank_guard_thrash",
        DiagnosticCode::OamTransferNoVisibleEffect => "oam_transfer_no_visible_effect",
        DiagnosticCode::PresentPathNoVisibleEffect => "present_path_no_visible_effect",
        DiagnosticCode::WaitVBlankPresentOrderingSuspicious => {
            "wait_vblank_present_ordering_suspicious"
        }
        DiagnosticCode::InputReadNoVisibleEffect => "input_read_no_visible_effect",
        DiagnosticCode::InputReadNoProgress => "input_read_no_progress",
        DiagnosticCode::TransferOamWithoutPresent => "transfer_oam_without_present",
        DiagnosticCode::VramChangedButFrameStatic => "vram_changed_but_frame_static",
        DiagnosticCode::BgDisabledButRenderPathActive => "bg_disabled_but_render_path_active",
        DiagnosticCode::TransferOamSpriteStatic => "transfer_oam_sprite_static",
        DiagnosticCode::OamChangedButSpriteStatic => "oam_changed_but_sprite_static",
        DiagnosticCode::WindowEnabledButInvisible => "window_enabled_but_invisible",
        DiagnosticCode::InputReadWithoutJoypadMask => "input_read_without_joypad_mask",
        DiagnosticCode::SpritesEnabledButOamEmpty => "sprites_enabled_but_oam_empty",
        DiagnosticCode::TransferOamNoSpriteChange => "transfer_oam_no_sprite_change",
        DiagnosticCode::PresentNoLayerChange => "present_no_layer_change",
        DiagnosticCode::WindowMapChangedButWindowStatic => "window_map_changed_but_window_static",
        DiagnosticCode::BgMapChangedButBgStatic => "bg_map_changed_but_bg_static",
        DiagnosticCode::SpriteHashStaticAfterOamTransfer => "sprite_hash_static_after_oam_transfer",
        DiagnosticCode::UnsupportedOpcodeEncountered => "unsupported_opcode_encountered",
        DiagnosticCode::StatIrqPathSuspicious => "stat_irq_path_suspicious",
        DiagnosticCode::ScanlineProgressSuspicious => "scanline_progress_suspicious",
        DiagnosticCode::LcdToggleDuringRun => "lcd_toggle_during_run",
        DiagnosticCode::StatRegisterWriteObserved => "stat_register_write_observed",
        DiagnosticCode::LycRetargetObserved => "lyc_retarget_observed",
        DiagnosticCode::HdmaTransferNoVramEffect => "hdma_transfer_no_vram_effect",
        DiagnosticCode::HdmaDeferredWhileCpuHalted => "hdma_deferred_while_cpu_halted",
        DiagnosticCode::HdmaControlWriteIgnored => "hdma_control_write_ignored",
        DiagnosticCode::SerialTransferNoInterrupt => "serial_transfer_no_interrupt",
        DiagnosticCode::JoypadEdgeNoInterrupt => "joypad_edge_no_interrupt",
        DiagnosticCode::InterruptPendingButNotServiced => "interrupt_pending_but_not_serviced",
        DiagnosticCode::ApuActiveNoPcm => "apu_active_no_pcm",
        DiagnosticCode::ApuPcmBufferOverflow => "apu_pcm_buffer_overflow",
        DiagnosticCode::ApuPopRiskObserved => "apu_pop_risk_observed",
        DiagnosticCode::ApuWaveRamAliasedWhileCh3Active => "apu_wave_ram_aliased_while_ch3_active",
        DiagnosticCode::ApuNoiseClockFrozen => "apu_noise_clock_frozen",
        DiagnosticCode::SgbHeaderObservedOutOfScope => "sgb_header_observed_out_of_scope",
    }
}

// Bucket the heuristic priority score into display labels; these are not calibrated probabilities.
fn confidence_from_score(score: u32) -> &'static str {
    match score {
        0..=39 => "low",
        40..=69 => "medium",
        70..=89 => "high",
        _ => "critical",
    }
}

// Test whether a supplied diagnostic list contains a code regardless of its severity or message.
fn has_code(diagnostics: &[Diagnostic], code: DiagnosticCode) -> bool {
    diagnostics.iter().any(|diagnostic| diagnostic.code == code)
}

// Select requested codes in diagnostic-list order and convert them to slugs. The wanted
// set is deduplicated, but duplicate occurrences in diagnostics remain in the result.
fn related_codes(diagnostics: &[Diagnostic], codes: &[DiagnosticCode]) -> Vec<String> {
    let wanted: BTreeSet<_> = codes.iter().copied().collect();
    diagnostics
        .iter()
        .filter(|diagnostic| wanted.contains(&diagnostic.code))
        .map(|diagnostic| diagnostic_code_slug(diagnostic.code).to_string())
        .collect()
}

// Rank investigation candidates from supplied run-window counters, diagnostics and
// current machine state. Scores prioritize follow-up; they do not establish a root cause or hardware accuracy.
pub fn build_auto_diagnosis(
    machine: &Machine,
    input: DiagnosticInput,
    diagnostics: &[Diagnostic],
    stop_reason: Option<&StopReason>,
) -> AutoDiagnosisReport {
    let mut suspects = Vec::new();

    // Create a rendering candidate only when the supplied list contains one of these selected
    // static/blank-effect codes; absence of VBlank alone is not this branch's trigger.
    let blank_like = has_code(diagnostics, DiagnosticCode::WhiteScreenLcdcOff)
        || has_code(diagnostics, DiagnosticCode::WhiteScreenFrameStatic)
        || has_code(diagnostics, DiagnosticCode::PresentPathNoVisibleEffect)
        || has_code(diagnostics, DiagnosticCode::VramChangedButFrameStatic)
        || has_code(diagnostics, DiagnosticCode::PresentNoLayerChange);
    if blank_like {
        let mut evidence = Vec::new();
        if machine.ppu.lcdc & 0x80 == 0 {
            evidence.push(
                "LCDC bit7 is currently clear, so the LCD pipeline is disabled at report time."
                    .to_string(),
            );
        }
        if input.vblank_events == 0 {
            evidence.push("No VBlank event was observed in the current run window.".to_string());
        }
        if !input.screen_changed {
            evidence.push("Framebuffer hash stayed static across the observed window.".to_string());
        }
        if input.present_path_count > 0 {
            evidence.push(format!(
                "Present/build-screen path was hit {} time(s).",
                input.present_path_count
            ));
        }
        suspects.push(AutoDiagnosisSuspect {
            key: "render_blank_or_static".to_string(),
            category: AutoDiagnosisCategory::Rendering,
            score: if machine.ppu.lcdc & 0x80 == 0 { 95 } else if input.vblank_events == 0 { 84 } else { 72 },
            confidence: confidence_from_score(if machine.ppu.lcdc & 0x80 == 0 { 95 } else if input.vblank_events == 0 { 84 } else { 72 }).to_string(),
            title: "Render path ran, but screen is still blank/static".to_string(),
            summary: "The strongest current signal points at LCD enable state, VBlank absence, or render/present work that never reaches a visible frame change.".to_string(),
            evidence,
            next_steps: vec![
                "Stop on lcd_toggle, vblank, and stat events to confirm whether the LCD pipeline starts at all.".to_string(),
                "Inspect report.symbols.current_source and execution_context_trail around present/build-screen routines.".to_string(),
                "Capture a replay checkpoint one frame before the first blank/static frame and compare VRAM/OAM deltas.".to_string(),
            ],
            related_diagnostics: related_codes(diagnostics, &[
                DiagnosticCode::WhiteScreenLcdcOff,
                DiagnosticCode::WhiteScreenNoVblank,
                DiagnosticCode::WhiteScreenFrameStatic,
                DiagnosticCode::PresentPathNoVisibleEffect,
                DiagnosticCode::PresentNoLayerChange,
                DiagnosticCode::VramChangedButFrameStatic,
                DiagnosticCode::BgDisabledButRenderPathActive,
            ]),
            stop_related: stop_reason.is_some(),
        });
    }

    // Treat high bank activity or restore/thrash warnings as an investigation lead; legitimate
    // bank-heavy programs can meet the same thresholds.
    let bank_like = input.bank_switches >= 6
        || input.bank_thrash_score >= 4
        || has_code(diagnostics, DiagnosticCode::FarCallBankNotRestored)
        || has_code(diagnostics, DiagnosticCode::FarCallStorm)
        || has_code(diagnostics, DiagnosticCode::RepeatedBankThrash);
    if bank_like {
        let score = if !input.bank_restored {
            88
        } else {
            74 + input.bank_thrash_score.min(16)
        };
        let mut evidence = vec![format!(
            "Observed {} bank switch(es); bank thrash score is {}.",
            input.bank_switches, input.bank_thrash_score
        )];
        if !input.bank_restored {
            evidence.push("A far-call return remained unresolved beyond the bounded grace period, which suggests missing return/bank restore choreography.".to_string());
        }
        if input.far_call_count > 0 {
            evidence.push(format!(
                "Far-call-like transitions observed {} time(s).",
                input.far_call_count
            ));
        }
        suspects.push(AutoDiagnosisSuspect {
            key: "bank_choreography_instability".to_string(),
            category: AutoDiagnosisCategory::Banking,
            score,
            confidence: confidence_from_score(score).to_string(),
            title: "Bank choreography looks unstable".to_string(),
            summary: "The run window shows either aggressive bank oscillation or failure to return to the expected ROM bank after far-call-like transitions.".to_string(),
            evidence,
            next_steps: vec![
                "Set debugger stop conditions on bank switches and far-call symbols to capture the first non-restored transition.".to_string(),
                "Use source-aware reports to inspect the bank guard / bank return path after the last successful far-call.".to_string(),
            ],
            related_diagnostics: related_codes(diagnostics, &[
                DiagnosticCode::FarCallBankNotRestored,
                DiagnosticCode::FarCallStorm,
                DiagnosticCode::RepeatedBankThrash,
                DiagnosticCode::BankGuardThrash,
            ]),
            stop_related: stop_reason.is_some_and(|reason| reason.kind.contains("break") || reason.kind.contains("watch")),
        });
    }

    // Any observed DMA start can produce this timing candidate, even without a no-effect warning.
    let timing_like = input.hdma_start_count > 0
        || input.oam_dma_start_count > 0
        || input.hdma_deferred_count > 0
        || input.hdma_ignored_write_count > 0
        || has_code(diagnostics, DiagnosticCode::HdmaTransferNoVramEffect)
        || has_code(diagnostics, DiagnosticCode::CoreOamDmaNoOamChange);
    if timing_like {
        let mut evidence = Vec::new();
        if input.hdma_start_count > 0 {
            evidence.push(format!("HDMA/GDMA started {} time(s) with coarse stall estimates GDMA={} / HDMA={} cycles.", input.hdma_start_count, input.gdma_stall_cycles_estimate, input.hdma_stall_cycles_estimate));
        }
        if input.oam_dma_start_count > 0 {
            evidence.push(format!(
                "OAM DMA started {} time(s).",
                input.oam_dma_start_count
            ));
        }
        if input.hdma_deferred_count > 0 {
            evidence.push(format!("HBlank HDMA deferred {} time(s), usually because CPU/HALT timing missed the transfer window.", input.hdma_deferred_count));
        }
        if input.hdma_ignored_write_count > 0 {
            evidence.push(format!(
                "FF55 control writes were ignored {} time(s) during active HDMA.",
                input.hdma_ignored_write_count
            ));
        }
        let score =
            68 + ((input.hdma_deferred_count + input.hdma_ignored_write_count) as u32).min(18);
        suspects.push(AutoDiagnosisSuspect {
            key: "dma_or_hblank_timing".to_string(),
            category: AutoDiagnosisCategory::Timing,
            score,
            confidence: confidence_from_score(score).to_string(),
            title: "DMA/HBlank timing is a likely culprit".to_string(),
            summary: "Observed DMA activity does not line up cleanly with visible VRAM/OAM effects, which often points at coarse timing mismatches or transfer-window quirks.".to_string(),
            evidence,
            next_steps: vec![
                "Stop on oam_dma/hdma/gdma events and compare against replay checkpoints around the first visible glitch.".to_string(),
                "Inspect whether HALT, STOP, or interrupt wake-up edges overlap HDMA windows.".to_string(),
            ],
            related_diagnostics: related_codes(diagnostics, &[
                DiagnosticCode::HdmaTransferNoVramEffect,
                DiagnosticCode::HdmaDeferredWhileCpuHalted,
                DiagnosticCode::HdmaControlWriteIgnored,
                DiagnosticCode::CoreOamDmaNoOamChange,
                DiagnosticCode::OamDmaNoVisibleEffect,
                DiagnosticCode::TransferOamNoSpriteChange,
            ]),
            stop_related: stop_reason.is_some_and(|reason| reason.kind.contains("dma")),
        });
    }

    // Combine interrupt-gating counters with selected timer/serial/joypad diagnostics.
    let irq_like = input.interrupt_pending_blocked_count > 0
        || has_code(diagnostics, DiagnosticCode::InterruptPendingButNotServiced)
        || has_code(diagnostics, DiagnosticCode::SerialTransferNoInterrupt)
        || has_code(diagnostics, DiagnosticCode::JoypadEdgeNoInterrupt)
        || has_code(diagnostics, DiagnosticCode::TimerInactive);
    if irq_like {
        let mut evidence = Vec::new();
        if input.interrupt_pending_blocked_count > 0 {
            evidence.push(format!(
                "Interrupt pending-but-blocked was observed {} time(s).",
                input.interrupt_pending_blocked_count
            ));
        }
        if machine.interrupt.has_pending() && !machine.cpu.ime {
            evidence.push(
                "IME is currently disabled while one or more interrupt sources remain pending."
                    .to_string(),
            );
        }
        if input.interrupt_services == 0 {
            evidence.push(
                "No interrupt service edge was observed in the current run window.".to_string(),
            );
        }
        let score = if machine.interrupt.has_pending() && !machine.cpu.ime {
            90
        } else {
            76
        };
        suspects.push(AutoDiagnosisSuspect {
            key: "interrupt_starvation_or_gating".to_string(),
            category: AutoDiagnosisCategory::Interrupts,
            score,
            confidence: confidence_from_score(score).to_string(),
            title: "Interrupt starvation or IME gating is likely blocking progress".to_string(),
            summary: "Pending interrupts are present, but service edges are missing or delayed enough to stall wait loops and device progress.".to_string(),
            evidence,
            next_steps: vec![
                "Stop on irq_request / irq_service / irq_blocked to capture the first blocked source.".to_string(),
                "Inspect EI-delay / HALT / STOP interactions around the blocked frame using replay checkpoints.".to_string(),
            ],
            related_diagnostics: related_codes(diagnostics, &[
                DiagnosticCode::InterruptsPendingButImeOff,
                DiagnosticCode::InterruptPendingButNotServiced,
                DiagnosticCode::TimerInactive,
                DiagnosticCode::SerialTransferNoInterrupt,
                DiagnosticCode::JoypadEdgeNoInterrupt,
                DiagnosticCode::StatIrqPathSuspicious,
            ]),
            stop_related: stop_reason.is_some_and(|reason| reason.kind.contains("irq")),
        });
    }

    // Use existing input diagnostics to distinguish a visible no-progress hint from mere absence of input metadata.
    let input_like = has_code(diagnostics, DiagnosticCode::InputReadNoProgress)
        || has_code(diagnostics, DiagnosticCode::InputReadNoVisibleEffect)
        || has_code(diagnostics, DiagnosticCode::InputReadWithoutJoypadMask);
    if input_like {
        let score = if input.input_active { 70 } else { 54 };
        let mut evidence = vec![format!("{}.", input_sampling_clause(&input))];
        if input.input_active {
            evidence.push("A joypad mask was active during the run window.".to_string());
        }
        if input.repeated_pc_hits >= 16 {
            evidence.push(format!(
                "Execution still appears loop-bound (hot-loop score {}).",
                input.repeated_pc_hits
            ));
        }
        suspects.push(AutoDiagnosisSuspect {
            key: "input_path_not_advancing_state".to_string(),
            category: AutoDiagnosisCategory::Input,
            score,
            confidence: confidence_from_score(score).to_string(),
            title: "Input path is active, but game state is not advancing".to_string(),
            summary: "Joypad reads are happening, but either no interrupt fires or visible state stays unchanged, suggesting a stuck wait loop or masking issue.".to_string(),
            evidence,
            next_steps: vec![
                "Stop on joypad_edge / joypad_irq and compare execution_context_trail before and after the read routine.".to_string(),
                "Verify selected P1 lines and whether the game expects interrupt-driven or polled input progression.".to_string(),
            ],
            related_diagnostics: related_codes(diagnostics, &[
                DiagnosticCode::InputReadNoProgress,
                DiagnosticCode::InputReadNoVisibleEffect,
                DiagnosticCode::InputReadWithoutJoypadMask,
                DiagnosticCode::JoypadEdgeNoInterrupt,
            ]),
            stop_related: stop_reason.is_some_and(|reason| reason.kind.contains("joypad") || reason.kind.contains("watch")),
        });
    }

    // A trigger alone can create an audio candidate; dropped PCM or missing PCM after a trigger raises its score.
    let audio_like = input.apu_trigger_count > 0
        || input.apu_pcm_drop_count > 0
        || has_code(diagnostics, DiagnosticCode::ApuActiveNoPcm)
        || has_code(diagnostics, DiagnosticCode::ApuPopRiskObserved);
    if audio_like {
        let score = if input.apu_pcm_drop_count > 0 {
            78
        } else if input.apu_trigger_count > 0 && input.apu_pcm_frame_count == 0 {
            82
        } else {
            58
        };
        let mut evidence = vec![format!(
            "APU trigger count={} mix-output count={} PCM frames={} dropped={}",
            input.apu_trigger_count,
            input.apu_mix_output_count,
            input.apu_pcm_frame_count,
            input.apu_pcm_drop_count
        )];
        if input.apu_pop_risk_count > 0 {
            evidence.push(format!(
                "Pop-risk edge observed {} time(s).",
                input.apu_pop_risk_count
            ));
        }
        if input.apu_wave_alias_count > 0 || input.apu_noise_lock_count > 0 {
            evidence.push(format!(
                "CH3 alias count={} CH4 frozen-noise count={}",
                input.apu_wave_alias_count, input.apu_noise_lock_count
            ));
        }
        suspects.push(AutoDiagnosisSuspect {
            key: "audio_pipeline_accuracy_or_backpressure".to_string(),
            category: AutoDiagnosisCategory::Audio,
            score,
            confidence: confidence_from_score(score).to_string(),
            title: "Audio pipeline is active, but output accuracy/backpressure is suspect".to_string(),
            summary: "The APU path is producing triggers or coarse mixer activity, but PCM output, queue pressure, or quirk-specific behavior still looks incomplete.".to_string(),
            evidence,
            next_steps: vec![
                "Inspect PCM queue drain rate and compare FF76/FF77 reads against the host-facing PCM buffer.".to_string(),
                "Capture replay checkpoints around DAC/mixer control writes to correlate pop-risk edges with audible artifacts.".to_string(),
            ],
            related_diagnostics: related_codes(diagnostics, &[
                DiagnosticCode::ApuActiveNoPcm,
                DiagnosticCode::ApuPcmBufferOverflow,
                DiagnosticCode::ApuPopRiskObserved,
                DiagnosticCode::ApuWaveRamAliasedWhileCh3Active,
                DiagnosticCode::ApuNoiseClockFrozen,
            ]),
            stop_related: stop_reason.is_some_and(|reason| reason.kind.contains("watch")),
        });
    }

    // Prioritize unsupported execution coverage, especially when it halted the run and can explain later symptoms.
    if input.unsupported_opcode_count > 0 {
        let score = if input.halted_on_unsupported_opcode {
            100
        } else {
            86
        };
        let mut evidence = vec![format!(
            "Unsupported opcode count observed: {}",
            input.unsupported_opcode_count
        )];
        if let Some(reason) = stop_reason {
            if reason.kind.contains("unsupported") {
                evidence.push(format!(
                    "Execution stopped directly on unsupported opcode at bank {} pc {:04X}.",
                    reason.rom_bank, reason.pc
                ));
            }
        }
        suspects.push(AutoDiagnosisSuspect {
            key: "unsupported_opcode_gap".to_string(),
            category: AutoDiagnosisCategory::Unsupported,
            score,
            confidence: confidence_from_score(score).to_string(),
            title: "Unsupported opcode coverage gap is currently the primary blocker".to_string(),
            summary: "Execution hit one or more unsupported opcodes, so later observations may be secondary fallout rather than root cause.".to_string(),
            evidence,
            next_steps: vec![
                "Inspect report.unsupported_opcodes and stop on the first unsupported opcode edge.".to_string(),
                "Prefer CPU opcode coverage work before deeper subsystem timing tuning for this ROM path.".to_string(),
            ],
            related_diagnostics: related_codes(diagnostics, &[DiagnosticCode::UnsupportedOpcodeEncountered]),
            stop_related: stop_reason.is_some_and(|reason| reason.kind.contains("unsupported")),
        });
    }

    if let Some(reason) = stop_reason {
        // Recognize replay divergence by the stop-kind label; this builder does not independently compare digests.
        if reason.kind.contains("divergence") {
            suspects.push(AutoDiagnosisSuspect {
                key: "replay_generation_diverged".to_string(),
                category: AutoDiagnosisCategory::Replay,
                score: 73,
                confidence: confidence_from_score(73).to_string(),
                title: "Replay generation diverged from previously recorded execution".to_string(),
                summary: "Rewind/replay reached a checkpoint whose digest no longer matches the historical generation, which is useful for narrowing the earliest branching point.".to_string(),
                evidence: vec![format!("Debugger stop reason was '{}' at frame {} cycle {}.", reason.kind, reason.frame, reason.cycle)],
                next_steps: vec![
                    "Capture one checkpoint earlier and compare symbol/source context around the first mismatching frame.".to_string(),
                    "Use watchpoints on the first mutated memory region that differs between generations.".to_string(),
                ],
                related_diagnostics: Vec::new(),
                stop_related: true,
            });
        }
    }

    // Order strongest scores first, break ties by stable key and retain at most five candidates.
    suspects.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.key.cmp(&b.key)));
    let max_items = 5;
    if suspects.len() > max_items {
        suspects.truncate(max_items);
    }
    // Build the focus list from the already truncated ranking so it matches the displayed candidates.
    let recommended_focus = suspects
        .iter()
        .map(|suspect| {
            format!(
                "{} [{} / {}]",
                suspect.title,
                format!("{:?}", suspect.category).to_ascii_lowercase(),
                suspect.confidence
            )
        })
        .collect();
    // These are literal guidance strings, not a live audit of referenced project files or tool availability.
    let carry_forward_notes = vec![
        "Suspects are prioritized from runtime observations and coarse heuristics; they are guides, not hardware proofs.".to_string(),
        "Past [x] items may still only mean regression/observability completion; consult docs/limited_completion_and_deferred_scope_v49.md and the latest open_remaining_issues file.".to_string(),
        "Always carry forward the latest handover_next_chat_v*.md, open_remaining_issues_v*.txt, roadmap.md, and NEXT_CHAT_START_HERE.md when moving to a fresh chat.".to_string(),
    ];

    AutoDiagnosisReport {
        generated: true,
        max_items,
        suspects,
        recommended_focus,
        carry_forward_notes,
    }
}

// Check code presence for timing-pack gating without requiring a particular diagnostic severity.
fn has_diag(diagnostics: &[Diagnostic], code: DiagnosticCode) -> bool {
    diagnostics.iter().any(|diag| diag.code == code)
}

// Create six subsystem summaries from the supplied evidence and diagnostic exclusions.
// This builder emits only observation/implementation and regression-only/candidate states; it never validates hardware.
pub fn build_timing_pack_report(
    input: DiagnosticInput,
    diagnostics: &[Diagnostic],
) -> TimingPackReport {
    let mut packs = Vec::new();

    // Use timer or unsupported-opcode observations as CPU-path evidence and exclude selected
    // warning codes before marking this run a candidate.
    let cpu_impl = input.timer_interrupts > 0
        || input.timer_overflows > 0
        || input.unsupported_opcode_count > 0;
    let cpu_candidate = cpu_impl
        && !has_diag(diagnostics, DiagnosticCode::TimerInactive)
        && !has_diag(diagnostics, DiagnosticCode::InterruptPendingButNotServiced)
        && !has_diag(diagnostics, DiagnosticCode::UnsupportedOpcodeEncountered);
    packs.push(TimingPackEntry {
        key: "cpu_edge_timing".into(),
        title: "CPU edge timing pack".into(),
        level: if cpu_impl { TimingPackLevel::Impl } else { TimingPackLevel::Obs },
        verification: if cpu_candidate { TimingPackVerification::Candidate } else { TimingPackVerification::RegressionOnly },
        summary: if cpu_candidate {
            "Timer/interrupt edge activity is visible and the run window did not raise the most common CPU-edge timing diagnostics.".into()
        } else if cpu_impl {
            "CPU-edge observability is present, but timing evidence is still regression-oriented or mixed with unresolved diagnostics.".into()
        } else {
            "CPU-edge pack is still mostly observation-only in this run window.".into()
        },
        evidence: vec![
            format!("timer_interrupts={}", input.timer_interrupts),
            format!("timer_overflows={}", input.timer_overflows),
            format!("interrupt_pending_blocked_count={}", input.interrupt_pending_blocked_count),
            format!("unsupported_opcode_count={}", input.unsupported_opcode_count),
        ],
        gaps: vec![
            "No hardware-side golden trace is attached in this environment.".into(),
            "IME/EI-delay, HALT, and STOP still need Rust-enabled build/test validation.".into(),
        ],
        related_jobs: vec![
            "samples/jobs/regression/cpu_edge_probe.json".into(),
            "samples/jobs/regression/cpu_ime_ei_delay_probe.json".into(),
            "samples/jobs/regression/cpu_halt_stop_probe.json".into(),
        ],
        related_aliases: vec!["timer".into(), "irq_request".into(), "irq_service".into(), "irq_blocked".into()],
        next_steps: vec![
            "Run the CPU edge probes in a Rust-enabled environment and tighten expected_timer_interrupt_floor / expected_event_count_floor.".into(),
            "Compare against a known-good ROM path before promoting this pack to [val].".into(),
        ],
    });

    // Require an observed render boundary, plus absence of selected timing warnings, for a PPU candidate.
    let ppu_impl = input.vblank_events > 0
        || input.scanline_event_count > 0
        || input.scanline_render_count > 0
        || input.stat_signal_count > 0;
    let ppu_candidate = ppu_impl
        && input.scanline_render_count > 0
        && !has_diag(diagnostics, DiagnosticCode::ScanlineProgressSuspicious)
        && !has_diag(diagnostics, DiagnosticCode::StatIrqPathSuspicious);
    packs.push(TimingPackEntry {
        key: "ppu_stat_lcd_timing".into(),
        title: "PPU / STAT / LCD timing pack".into(),
        level: if ppu_impl { TimingPackLevel::Impl } else { TimingPackLevel::Obs },
        verification: if ppu_candidate { TimingPackVerification::Candidate } else { TimingPackVerification::RegressionOnly },
        summary: if ppu_candidate {
            "Scanline, render-boundary, and STAT/LCD activity are visible without the most common timing warnings in this run window.".into()
        } else if ppu_impl {
            "PPU/STAT/LCD timing surfaces are implemented and observable, but still rely on regression-side evidence.".into()
        } else {
            "PPU timing pack has not gathered enough live evidence in this run window.".into()
        },
        evidence: vec![
            format!("vblank_events={}", input.vblank_events),
            format!("scanline_event_count={}", input.scanline_event_count),
            format!("scanline_render_count={}", input.scanline_render_count),
            format!("stat_signal_count={}", input.stat_signal_count),
            format!("lcd_toggle_count={}", input.lcd_toggle_count),
        ],
        gaps: vec![
            "Mode-accurate timing still needs hardware-side comparison.".into(),
            "LCD enable/disable edge cases are still validated via diagnostics rather than a proven external oracle.".into(),
        ],
        related_jobs: vec![
            "samples/jobs/regression/ppu_stat_timing_probe.json".into(),
            "samples/jobs/regression/ppu_lcdc_toggle_probe.json".into(),
            "samples/jobs/regression/ppu_stat_irq_window_probe.json".into(),
        ],
        related_aliases: vec!["scanline_render".into(), "lcd_toggle".into(), "stat_write".into(), "lyc_write".into()],
        next_steps: vec![
            "Use external captures or trusted emulator references before marking this pack [val].".into(),
            "Tighten expected_vblank_floor / expected_event_count_floor for the PPU jobs.".into(),
        ],
    });

    // Require start/block evidence and some completion evidence for a DMA candidate.
    // This gate checks diagnostic absence, not the actual bytes transferred.
    let dma_impl =
        input.oam_dma_start_count > 0 || input.hdma_start_count > 0 || input.hdma_block_count > 0;
    let dma_candidate = dma_impl
        && (input.oam_dma_complete_count > 0 || input.hdma_complete_count > 0)
        && !has_diag(diagnostics, DiagnosticCode::CoreOamDmaNoOamChange)
        && !has_diag(diagnostics, DiagnosticCode::HdmaTransferNoVramEffect);
    packs.push(TimingPackEntry {
        key: "dma_timing".into(),
        title: "DMA / OAM DMA / HDMA timing pack".into(),
        level: if dma_impl { TimingPackLevel::Impl } else { TimingPackLevel::Obs },
        verification: if dma_candidate { TimingPackVerification::Candidate } else { TimingPackVerification::RegressionOnly },
        summary: if dma_candidate {
            "DMA start/block/complete edges are visible and the run window shows data movement evidence.".into()
        } else if dma_impl {
            "DMA timing surfaces are implemented, but completion/data-movement proof is still incomplete or warning-backed.".into()
        } else {
            "DMA timing pack is still mostly observation-only in this run window.".into()
        },
        evidence: vec![
            format!("oam_dma_start_count={}", input.oam_dma_start_count),
            format!("oam_dma_complete_count={}", input.oam_dma_complete_count),
            format!("hdma_start_count={}", input.hdma_start_count),
            format!("hdma_block_count={}", input.hdma_block_count),
            format!("hdma_complete_count={}", input.hdma_complete_count),
            format!("gdma_stall_cycles_estimate={}", input.gdma_stall_cycles_estimate),
            format!("hdma_stall_cycles_estimate={}", input.hdma_stall_cycles_estimate),
        ],
        gaps: vec![
            "GDMA/HDMA stall lengths still need toolchain-backed and hardware-backed confirmation.".into(),
            "This report does not yet prove bus-contention parity with hardware.".into(),
        ],
        related_jobs: vec![
            "samples/jobs/regression/oam_dma_probe.json".into(),
            "samples/jobs/regression/gdma_cgb_dma_probe.json".into(),
            "samples/jobs/regression/hdma_hblank_dma_probe.json".into(),
        ],
        related_aliases: vec!["oam_dma".into(), "hdma_start".into(), "hdma_block".into(), "gdma_stall".into()],
        next_steps: vec![
            "Validate DMA stalls and completion edges in a Rust-enabled environment before promoting beyond regression-only.".into(),
            "Use dedicated VRAM/OAM watch windows alongside the DMA event families.".into(),
        ],
    });

    // Use observed timer/serial/joypad/service activity and exclude selected unserviced-path codes.
    let timer_irq_impl = input.timer_interrupts > 0
        || input.serial_interrupts > 0
        || input.joypad_interrupts > 0
        || input.interrupt_services > 0;
    let timer_irq_candidate = timer_irq_impl
        && !has_diag(diagnostics, DiagnosticCode::SerialTransferNoInterrupt)
        && !has_diag(diagnostics, DiagnosticCode::JoypadEdgeNoInterrupt)
        && !has_diag(diagnostics, DiagnosticCode::InterruptPendingButNotServiced);
    packs.push(TimingPackEntry {
        key: "timer_irq_timing".into(),
        title: "Timer / serial / joypad / interrupt timing pack".into(),
        level: if timer_irq_impl { TimingPackLevel::Impl } else { TimingPackLevel::Obs },
        verification: if timer_irq_candidate { TimingPackVerification::Candidate } else { TimingPackVerification::RegressionOnly },
        summary: if timer_irq_candidate {
            "Timer and interrupt-path edges are observable without the most common unserviced-path diagnostics.".into()
        } else if timer_irq_impl {
            "Timer/serial/joypad/IRQ timing is implemented and observable, but still needs stronger validation evidence.".into()
        } else {
            "Timer/IRQ pack has not gathered enough live evidence in this run window.".into()
        },
        evidence: vec![
            format!("timer_interrupts={}", input.timer_interrupts),
            format!("serial_interrupts={}", input.serial_interrupts),
            format!("joypad_interrupts={}", input.joypad_interrupts),
            format!("interrupt_services={}", input.interrupt_services),
            format!("interrupt_pending_blocked_count={}", input.interrupt_pending_blocked_count),
        ],
        gaps: vec![
            "External-link serial and finer timer reload quirks still need toolchain-backed execution.".into(),
            "IRQ service cycle timing remains a candidate rather than validated.".into(),
        ],
        related_jobs: vec![
            "samples/jobs/regression/timer_irq_edge_probe.json".into(),
            "samples/jobs/regression/serial_irq_probe.json".into(),
            "samples/jobs/regression/joypad_irq_probe.json".into(),
            "samples/jobs/regression/interrupt_priority_probe.json".into(),
        ],
        related_aliases: vec!["timer_overflow".into(), "serial".into(), "joypad_irq".into(), "irq_service".into()],
        next_steps: vec![
            "Run the edge probes under cargo test / real CLI execution and compare IRQ ordering against a trusted reference.".into(),
            "Promote to [val] only after interrupt priority and gating behavior are externally checked.".into(),
        ],
    });

    // CGB mode alone qualifies as available-path evidence here; no speed-switch event is required by this gate.
    let cgb_impl = input.cgb_palette_count > 0 || !input.dmg_mode;
    let cgb_candidate = cgb_impl && !has_diag(diagnostics, DiagnosticCode::CgbPaletteUsedInDmgMode);
    packs.push(TimingPackEntry {
        key: "cgb_timing".into(),
        title: "CGB double-speed / palette timing pack".into(),
        level: if cgb_impl { TimingPackLevel::Impl } else { TimingPackLevel::Obs },
        verification: if cgb_candidate { TimingPackVerification::Candidate } else { TimingPackVerification::RegressionOnly },
        summary: if cgb_candidate {
            "CGB register/palette/speed-switch surfaces are present without the obvious DMG-mode misuse diagnostic.".into()
        } else if cgb_impl {
            "CGB timing surfaces are present, but still rely on coarse evidence or mode-mismatch diagnostics.".into()
        } else {
            "CGB timing pack has not been exercised in this run window.".into()
        },
        evidence: vec![
            format!("dmg_mode={}", input.dmg_mode),
            format!("cgb_palette_count={}", input.cgb_palette_count),
        ],
        gaps: vec![
            "Double-speed timing remains coarse until externally validated.".into(),
            "Palette-access quirks still need hardware-side comparison.".into(),
        ],
        related_jobs: vec![
            "samples/jobs/regression/cgb_bank_palette_speed_probe.json".into(),
            "samples/jobs/regression/cgb_double_speed_palette_probe.json".into(),
        ],
        related_aliases: vec!["cgb_palette".into(), "speed_switch".into(), "speed_freeze".into()],
        next_steps: vec![
            "Use CGB-specific ROM probes and compare speed-switch windows against a trusted emulator or hardware capture.".into(),
            "Do not mark [val] until palette blocking and double-speed side effects are externally confirmed.".into(),
        ],
    });

    // Require PCM generation before candidate status and reject selected no-PCM/overflow diagnostics.
    // Queue activity alone does not measure waveform fidelity.
    let apu_impl = input.apu_trigger_count > 0
        || input.apu_mix_output_count > 0
        || input.apu_pcm_frame_count > 0;
    let apu_candidate = apu_impl
        && input.apu_pcm_frame_count > 0
        && !has_diag(diagnostics, DiagnosticCode::ApuActiveNoPcm)
        && !has_diag(diagnostics, DiagnosticCode::ApuPcmBufferOverflow);
    packs.push(TimingPackEntry {
        key: "apu_timing".into(),
        title: "APU / audio timing pack".into(),
        level: if apu_impl { TimingPackLevel::Impl } else { TimingPackLevel::Obs },
        verification: if apu_candidate { TimingPackVerification::Candidate } else { TimingPackVerification::RegressionOnly },
        summary: if apu_candidate {
            "APU trigger/mixer/PCM surfaces are observable and the current run window did not report the most common audio-path diagnostics.".into()
        } else if apu_impl {
            "APU timing surfaces exist, but remain regression-oriented and still need external accuracy validation.".into()
        } else {
            "APU timing pack is still mostly observation-only in this run window.".into()
        },
        evidence: vec![
            format!("apu_trigger_count={}", input.apu_trigger_count),
            format!("apu_mix_output_count={}", input.apu_mix_output_count),
            format!("apu_pcm_frame_count={}", input.apu_pcm_frame_count),
            format!("apu_pcm_drop_count={}", input.apu_pcm_drop_count),
            format!("apu_pop_risk_count={}", input.apu_pop_risk_count),
            format!("apu_wave_alias_count={}", input.apu_wave_alias_count),
            format!("apu_noise_lock_count={}", input.apu_noise_lock_count),
        ],
        gaps: vec![
            "DAC/pop/HPF/CH3/CH4 behavior is still coarse without an external oracle.".into(),
            "PCM queue activity alone does not prove hardware-equivalent audio output.".into(),
        ],
        related_jobs: vec![
            "samples/jobs/regression/apu_register_probe.json".into(),
            "samples/jobs/regression/apu_channel_mixer_probe.json".into(),
            "samples/jobs/regression/apu_pcm_output_probe.json".into(),
            "samples/jobs/regression/apu_finer_accuracy_probe.json".into(),
        ],
        related_aliases: vec!["apu_trigger".into(), "apu_mix".into(), "apu_pcm".into(), "apu_pop".into()],
        next_steps: vec![
            "Run the APU probes in a Rust-enabled environment and compare against trusted audio references before using [val].".into(),
            "Treat pop/alias/noise-lock counters as quirk surfaces, not proof of exact hardware parity.".into(),
        ],
    });

    TimingPackReport {
        generated: true,
        packs,
        carry_forward_notes: vec![
            "Timing pack levels separate observability / implementation / validation so past [x] markers are not over-read.".into(),
            "In this environment, all packs should still be treated as pre-[val] until Rust toolchain and external reference runs are available.".into(),
        ],
    }
}

// Append independent observation-based rules in fixed order without executing the machine.
// Missing visible changes can be intentional; callers must interpret these hints within the sampled run window.
pub fn analyze_basic(machine: &Machine, input: DiagnosticInput) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    // Report the advertised SGB header capability without implying that SGB behavior was exercised.
    if machine.cartridge.supports_sgb() {
        out.push(Diagnostic {
            code: DiagnosticCode::SgbHeaderObservedOutOfScope,
            severity: Severity::Info,
            message: "ROM header advertises SGB support, but SGB command/border/multiplayer behavior is currently treated as out-of-scope and only DMG-compatible execution is provided.".into(),
        });
    }
    if input.unsupported_opcode_count > 0 {
        let severity = if input.halted_on_unsupported_opcode {
            Severity::Error
        } else {
            Severity::Warning
        };
        let suffix = if input.halted_on_unsupported_opcode {
            " Execution stopped early after the first unsupported opcode hit."
        } else {
            ""
        };
        out.push(Diagnostic {
            code: DiagnosticCode::UnsupportedOpcodeEncountered,
            severity,
            message: format!(
                "Observed {} unsupported opcode hit(s) in the current run window. Inspect report.unsupported_opcodes for the opcode/PC breakdown.{}",
                input.unsupported_opcode_count,
                suffix
            ),
        });
    }
    // This particular rule tests the entire LCDC byte for zero, not just its LCD-enable bit.
    if machine.ppu.lcdc == 0 {
        out.push(Diagnostic {
            code: DiagnosticCode::WhiteScreenLcdcOff,
            severity: Severity::Warning,
            message: "LCDC is off; display may remain blank.".into(),
        });
    }
    // No VBlank may simply reflect a short observation window; report it as information.
    if input.vblank_events == 0 {
        out.push(Diagnostic {
            code: DiagnosticCode::WhiteScreenNoVblank,
            severity: Severity::Info,
            message: "No VBlank observed during the current run window.".into(),
        });
    }
    // Use a two-frame minimum for the static-frame hint; a stable image can still be intentional.
    if input.executed_frames >= 2 && !input.screen_changed && machine.ppu.lcdc & 0x80 != 0 {
        out.push(Diagnostic { code: DiagnosticCode::WhiteScreenFrameStatic, severity: Severity::Warning, message: "Frame hash did not change across the observed run window while LCDC stayed enabled.".into() });
    }
    // Treat a repeated-PC threshold as a possible loop, not proof of deadlock.
    if input.repeated_pc_hits >= 32 {
        out.push(Diagnostic {
            code: DiagnosticCode::PcStuckLoop,
            severity: Severity::Warning,
            message: format!(
                "Program counter repeated in a tight range; peak repeated hit count was {}.",
                input.repeated_pc_hits
            ),
        });
    }
    // Compare current timer enable with observed interrupts, allowing a minimum two-frame window.
    if machine.timer.tac & 0x04 != 0 && input.executed_frames >= 2 && input.timer_interrupts == 0 {
        out.push(Diagnostic {
            code: DiagnosticCode::TimerInactive,
            severity: Severity::Info,
            message:
                "Timer was enabled but no timer interrupt was observed in the current run window."
                    .into(),
        });
    }
    // A pending serial request without a completion interrupt can mean it is still waiting for a peer.
    if machine.serial.transfer_active() && input.serial_interrupts == 0 {
        out.push(Diagnostic { code: DiagnosticCode::SerialTransferNoInterrupt, severity: Severity::Info, message: "Serial transfer remained active, but no serial interrupt edge was observed in the current run window.".into() });
    }
    // This IRQ hint specifically uses recognized input-routine hits; direct P1 reads alone do not trigger it.
    if input.input_active && input.input_path_count > 0 && input.joypad_interrupts == 0 {
        out.push(Diagnostic { code: DiagnosticCode::JoypadEdgeNoInterrupt, severity: Severity::Info, message: "Input path was active, but no joypad interrupt edge was observed in the current run window.".into() });
    }
    // Distinguish observed blocked requests from a window containing at least one serviced interrupt.
    if input.interrupt_pending_blocked_count > 0 && input.interrupt_services == 0 {
        out.push(Diagnostic { code: DiagnosticCode::InterruptPendingButNotServiced, severity: Severity::Info, message: "Interrupts became pending but none were serviced in the current run window; IME gating or wait-loop behavior may be blocking progress.".into() });
    }
    // Report trigger activity that produced no observed host-consumable PCM in the supplied window.
    if input.apu_trigger_count > 0 && input.apu_pcm_frame_count == 0 {
        out.push(Diagnostic { code: DiagnosticCode::ApuActiveNoPcm, severity: Severity::Info, message: "APU channel trigger activity was observed, but no PCM frames were buffered for host-side consumption in the current run window.".into() });
    }
    // Overflow counts diagnose lost buffered frames, not necessarily an emulated sound-register fault.
    if input.apu_pcm_drop_count > 0 {
        out.push(Diagnostic { code: DiagnosticCode::ApuPcmBufferOverflow, severity: Severity::Warning, message: format!("APU PCM buffering dropped {} frame(s); bounded host-audio buffering prevented unbounded growth, but the consumer side is lagging behind.", input.apu_pcm_drop_count) });
    }
    // Possible pop edges are derived from control transitions; audible output is not measured here.
    if input.apu_pop_risk_count > 0 {
        out.push(Diagnostic { code: DiagnosticCode::ApuPopRiskObserved, severity: Severity::Info, message: format!("Observed {} APU pop-risk edge(s) from DAC or mixer control changes; HPF smoothing is modeled coarsely, so audible spikes may still differ from hardware.", input.apu_pop_risk_count) });
    }
    // This uses the aggregate wave-access counter; it does not independently distinguish
    // CGB current-byte aliases from blocked DMG accesses reported by the core.
    if input.apu_wave_alias_count > 0 {
        out.push(Diagnostic { code: DiagnosticCode::ApuWaveRamAliasedWhileCh3Active, severity: Severity::Info, message: format!("Observed {} CH3 wave RAM alias access event(s) while the channel was active; CGB-style current-byte aliasing was exercised.", input.apu_wave_alias_count) });
    }
    if input.apu_noise_lock_count > 0 {
        out.push(Diagnostic { code: DiagnosticCode::ApuNoiseClockFrozen, severity: Severity::Info, message: format!("Observed {} CH4 high-shift noise configuration event(s) that freeze LFSR clocking in the coarse model.", input.apu_noise_lock_count) });
    }
    if machine.interrupt.has_pending() && !machine.cpu.ime {
        out.push(Diagnostic { code: DiagnosticCode::InterruptsPendingButImeOff, severity: Severity::Info, message: "Interrupt sources are pending while IME is disabled; progress may stall in wait loops.".into() });
    }
    // Rely on the caller's bounded return/restore tracking rather than inferring call completion from the current bank alone.
    if input.far_call_count > 0 && !input.bank_restored {
        out.push(Diagnostic { code: DiagnosticCode::FarCallBankNotRestored, severity: Severity::Warning, message: format!("Observed {} far-call entr{} and at least one return remained unresolved beyond the grace period.", input.far_call_count, if input.far_call_count == 1 { "y" } else { "ies" }) });
    }
    // Symbol-derived rendering activity and framebuffer changes are independent signals; compare both.
    if input.intrinsic_count > 0 && !input.screen_changed && machine.ppu.lcdc & 0x80 != 0 {
        out.push(Diagnostic { code: DiagnosticCode::IntrinsicNoVisibleEffect, severity: Severity::Info, message: format!("Observed {} KITAQGB intrinsic-like symbol transition(s), but the framebuffer hash did not change.", input.intrinsic_count) });
    }
    if input.settile_flush_count > 0 && !input.screen_changed && machine.ppu.lcdc & 0x80 != 0 {
        out.push(Diagnostic { code: DiagnosticCode::SetTileFlushNoVisibleEffect, severity: Severity::Warning, message: format!("Observed {} set-tile flush-like intrinsic event(s), but no visible framebuffer change followed in the run window.", input.settile_flush_count) });
    }
    // Flag dense far-call activity only within the short-window threshold; this is a performance/flow hint.
    if input.far_call_count >= 8 && input.executed_frames <= 8 {
        out.push(Diagnostic { code: DiagnosticCode::FarCallStorm, severity: Severity::Info, message: format!("Observed {} far-call-like bank transitions in only {} frame(s); this may indicate unstable bank choreography.", input.far_call_count, input.executed_frames) });
    }
    // Buffered tile writes can remain invisible until a later flush outside this observation window.
    if input.buffered_settile_count > 0 && input.settile_flush_count == 0 && !input.screen_changed {
        out.push(Diagnostic { code: DiagnosticCode::SetTileBufferedWithoutFlush, severity: Severity::Info, message: format!("Observed {} buffered set-tile intrinsic event(s), but no flush-like intrinsic or visible framebuffer change followed in the run window.", input.buffered_settile_count) });
    }
    // Check CGB-oriented library activity against the caller-provided hardware-mode flag.
    if input.dmg_mode && input.cgb_settile_count > 0 {
        out.push(Diagnostic { code: DiagnosticCode::SetTile16CgbUsedInDmgMode, severity: Severity::Warning, message: format!("Observed {} CGB-oriented set-tile intrinsic event(s) while the machine is running in DMG mode.", input.cgb_settile_count) });
    }
    if input.flush_rows_count > 0 && !input.screen_changed {
        out.push(Diagnostic { code: DiagnosticCode::FlushRowsNoVisibleEffect, severity: Severity::Warning, message: format!("Observed {} FlushRows-like intrinsic event(s), but no visible framebuffer change followed in the run window.", input.flush_rows_count) });
    }
    if input.settile_flush_count > 0 && input.vblank_events == 0 {
        out.push(Diagnostic { code: DiagnosticCode::FlushLikeWithoutVblank, severity: Severity::Info, message: format!("Observed {} flush-like intrinsic event(s), but no VBlank was seen during the same run window.", input.settile_flush_count) });
    }
    if input.oam_dma_count > 0 && !input.screen_changed {
        out.push(Diagnostic { code: DiagnosticCode::OamDmaNoVisibleEffect, severity: Severity::Info, message: format!("Observed {} OAM DMA-like intrinsic event(s), but the framebuffer hash did not change in the run window.", input.oam_dma_count) });
    }
    // A start with no OAM delta may copy identical data; this comparison alone cannot establish a failed transfer.
    if input.oam_dma_start_count > 0 && !input.oam_changed {
        out.push(Diagnostic { code: DiagnosticCode::CoreOamDmaNoOamChange, severity: Severity::Info, message: format!("Observed {} core OAM DMA start event(s), but OAM contents did not change in the run window.", input.oam_dma_start_count) });
    }
    if input.wait_vblank_count > 0 && input.vblank_events == 0 {
        out.push(Diagnostic { code: DiagnosticCode::WaitVBlankSeenNoVblank, severity: Severity::Warning, message: format!("Observed {} WaitVBlank-like symbol transition(s), but no actual VBlank event occurred in the run window.", input.wait_vblank_count) });
    }
    if input.dmg_mode && input.cgb_palette_count > 0 {
        out.push(Diagnostic {
            code: DiagnosticCode::CgbPaletteUsedInDmgMode,
            severity: Severity::Info,
            message: format!(
                "Observed {} CGB palette routine(s) while the machine is running in DMG mode.",
                input.cgb_palette_count
            ),
        });
    }
    if input.trap_check_count > 0 && input.repeated_pc_hits >= 16 {
        out.push(Diagnostic { code: DiagnosticCode::TrapCheckHotLoop, severity: Severity::Warning, message: format!("Observed {} trap-check-like symbol hit(s) together with a hot PC loop (peak repeated hit count {}).", input.trap_check_count, input.repeated_pc_hits) });
    }
    if input.bank_guard_count > 0 && input.bank_thrash_score >= 2 {
        out.push(Diagnostic {
            code: DiagnosticCode::BankGuardThrash,
            severity: Severity::Info,
            message: format!(
                "Observed {} bank-guard symbol hit(s) while bank thrash score reached {}.",
                input.bank_guard_count, input.bank_thrash_score
            ),
        });
    }
    if input.oam_transfer_count > 0 && !input.screen_changed {
        out.push(Diagnostic { code: DiagnosticCode::OamTransferNoVisibleEffect, severity: Severity::Info, message: format!("Observed {} OAM transfer routine(s), but no visible framebuffer change followed in the run window.", input.oam_transfer_count) });
    }
    if input.present_path_count > 0 && !input.screen_changed {
        out.push(Diagnostic { code: DiagnosticCode::PresentPathNoVisibleEffect, severity: Severity::Warning, message: format!("Observed {} present/build-screen routine hit(s), but no visible framebuffer change followed in the run window.", input.present_path_count) });
    }
    // Use the caller's observed ordering flag; separate hit counts do not establish which call came first.
    if input.wait_vblank_count > 0 && input.present_path_count > 0 && !input.wait_before_present {
        out.push(Diagnostic { code: DiagnosticCode::WaitVBlankPresentOrderingSuspicious, severity: Severity::Info, message: format!("Present-like path hit {} time(s) and WaitVBlank-like path hit {} time(s), but no WaitVBlank-before-Present ordering was observed in the run window.", input.present_path_count, input.wait_vblank_count) });
    }
    // Recognize either mapped input routines or direct P1 reads when checking input-related visible effects.
    if input.input_active && has_input_sampling(&input) && !input.screen_changed {
        out.push(Diagnostic { code: DiagnosticCode::InputReadNoVisibleEffect, severity: Severity::Info, message: format!("Input mask was active and {}, but no visible framebuffer change followed in the run window.", input_sampling_clause(&input)) });
    }
    if input.input_active && has_input_sampling(&input) && input.repeated_pc_hits >= 16 {
        out.push(Diagnostic { code: DiagnosticCode::InputReadNoProgress, severity: Severity::Warning, message: format!("Input mask was active and {}, but execution still looks stuck (hot-loop score {}).", input_sampling_clause(&input), input.repeated_pc_hits) });
    }
    if input.oam_transfer_count > 0 && input.present_path_count == 0 {
        out.push(Diagnostic { code: DiagnosticCode::TransferOamWithoutPresent, severity: Severity::Info, message: format!("Observed {} OAM transfer routine hit(s), but no present/build-screen path was seen in the same run window.", input.oam_transfer_count) });
    }
    // Separate backing-memory changes from the final visible-frame digest.
    if input.vram_changed && !input.screen_changed {
        out.push(Diagnostic { code: DiagnosticCode::VramChangedButFrameStatic, severity: Severity::Info, message: "VRAM contents changed during the run window, but the final framebuffer hash stayed unchanged.".into() });
    }
    if !input.bg_enabled && (input.present_path_count > 0 || input.settile_flush_count > 0) {
        out.push(Diagnostic {
            code: DiagnosticCode::BgDisabledButRenderPathActive,
            severity: Severity::Info,
            message:
                "Render/present-like routines ran while the BG layer appears disabled in LCDC."
                    .into(),
        });
    }
    if input.oam_transfer_count > 0 && input.oam_changed && input.visible_sprite_count == 0 {
        out.push(Diagnostic { code: DiagnosticCode::TransferOamSpriteStatic, severity: Severity::Info, message: format!("Observed {} OAM transfer routine hit(s) and OAM changed, but no visible sprite candidates remain on screen.", input.oam_transfer_count) });
    }
    // Use nonzero slots plus the visibility estimate to suggest checking offscreen sprite placement.
    if input.oam_changed && input.visible_sprite_count == 0 && input.nonzero_oam_entries > 0 {
        out.push(Diagnostic { code: DiagnosticCode::OamChangedButSpriteStatic, severity: Severity::Info, message: format!("OAM contents changed and {} sprite slot(s) are non-zero, but no visible sprite candidates are currently on screen.", input.nonzero_oam_entries) });
    }
    if input.window_enabled && !input.bg_enabled && input.present_path_count > 0 {
        out.push(Diagnostic { code: DiagnosticCode::WindowEnabledButInvisible, severity: Severity::Info, message: "Window is enabled, but BG is disabled while present-like paths are active; composition may still look blank.".into() });
    }
    if !input.input_active && has_input_sampling(&input) {
        out.push(Diagnostic {
            code: DiagnosticCode::InputReadWithoutJoypadMask,
            severity: Severity::Info,
            message: format!(
                "{}, but no joypad mask was active in this run window.",
                input_sampling_clause(&input)
            ),
        });
    }
    // Avoid the empty-OAM hint until at least two frames of enabled-sprite observation.
    if input.sprite_enabled && input.nonzero_oam_entries == 0 && input.executed_frames >= 2 {
        out.push(Diagnostic {
            code: DiagnosticCode::SpritesEnabledButOamEmpty,
            severity: Severity::Info,
            message: "OBJ rendering is enabled, but OAM stayed empty in the observed run window."
                .into(),
        });
    }
    if input.oam_transfer_count > 0 && !input.sprite_changed {
        out.push(Diagnostic { code: DiagnosticCode::TransferOamNoSpriteChange, severity: Severity::Info, message: format!("Observed {} OAM transfer routine hit(s), but sprite-side state did not change in the run window.", input.oam_transfer_count) });
    }
    // Check the three supplied layer-change flags together rather than equating a present call with a drawn change.
    if input.present_path_count > 0
        && !input.bg_changed
        && !input.window_changed
        && !input.sprite_changed
    {
        out.push(Diagnostic { code: DiagnosticCode::PresentNoLayerChange, severity: Severity::Warning, message: format!("Observed {} present/build-screen routine hit(s), but BG/Window/sprite all remained unchanged in the run window.", input.present_path_count) });
    }
    // Layer-related state hashes and visible-layer changes are intentionally separate observations.
    if input.window_hash_changed && !input.window_changed {
        out.push(Diagnostic { code: DiagnosticCode::WindowMapChangedButWindowStatic, severity: Severity::Info, message: "Window-related state changed, but the visible window layer still looks static in this run window.".into() });
    }
    if input.bg_hash_changed && !input.bg_changed {
        out.push(Diagnostic { code: DiagnosticCode::BgMapChangedButBgStatic, severity: Severity::Info, message: "BG-related state changed, but the visible BG layer still looks static in this run window.".into() });
    }
    if input.oam_transfer_count > 0 && !input.sprite_hash_changed {
        out.push(Diagnostic { code: DiagnosticCode::SpriteHashStaticAfterOamTransfer, severity: Severity::Info, message: "OAM transfer path ran, but the sprite-side hash stayed unchanged in this run window.".into() });
    }
    // Require both enough switches and a high alternation score for the repeated-thrash warning.
    if input.bank_switches >= 6 && input.bank_thrash_score >= 4 {
        out.push(Diagnostic { code: DiagnosticCode::RepeatedBankThrash, severity: Severity::Warning, message: format!("Bank switching alternated aggressively; thrash score reached {} across {} bank switch(es).", input.bank_thrash_score, input.bank_switches) });
    }
    // Report missing scanline observations only after a full frame with LCD currently enabled.
    if input.scanline_event_count == 0 && input.executed_frames >= 1 && machine.ppu.lcdc & 0x80 != 0
    {
        out.push(Diagnostic {
            code: DiagnosticCode::ScanlineProgressSuspicious,
            severity: Severity::Info,
            message: "No scanline-advance events were observed while LCD was enabled.".into(),
        });
    }
    if input.scanline_event_count > 0
        && input.scanline_render_count == 0
        && input.executed_frames >= 1
        && machine.ppu.lcdc & 0x80 != 0
    {
        out.push(Diagnostic {
            code: DiagnosticCode::ScanlineProgressSuspicious,
            severity: Severity::Info,
            message: "Scanlines advanced, but no scanline-render boundary was observed while LCD was enabled.".into(),
        });
    }
    // Compare HDMA/GDMA start activity against VRAM delta without claiming a byte-for-byte transfer check.
    if input.hdma_start_count > 0 && !input.vram_changed {
        out.push(Diagnostic { code: DiagnosticCode::HdmaTransferNoVramEffect, severity: Severity::Info, message: format!("Observed {} HDMA/GDMA start event(s), but VRAM contents did not change in the run window.", input.hdma_start_count) });
    }
    if input.hdma_deferred_count > 0 {
        out.push(Diagnostic { code: DiagnosticCode::HdmaDeferredWhileCpuHalted, severity: Severity::Info, message: format!("Observed {} deferred HBlank HDMA window(s), usually because the CPU was halted when HBlank arrived.", input.hdma_deferred_count) });
    }
    if input.hdma_ignored_write_count > 0 {
        out.push(Diagnostic { code: DiagnosticCode::HdmaControlWriteIgnored, severity: Severity::Info, message: format!("Observed {} FF55 write(s) ignored while an HBlank HDMA transfer was already active.", input.hdma_ignored_write_count) });
    }
    // The remaining LCD/STAT/LYC rules report observed register activity, not automatically faulty writes.
    if input.lcd_toggle_count > 0 {
        out.push(Diagnostic {
            code: DiagnosticCode::LcdToggleDuringRun,
            severity: Severity::Info,
            message: format!(
                "LCDC enable state toggled {} time(s) during the run window.",
                input.lcd_toggle_count
            ),
        });
    }
    if input.stat_write_count > 0 {
        out.push(Diagnostic {
            code: DiagnosticCode::StatRegisterWriteObserved,
            severity: Severity::Info,
            message: format!(
                "STAT control bits were rewritten {} time(s) in the run window.",
                input.stat_write_count
            ),
        });
    }
    if input.lyc_write_count > 0 {
        out.push(Diagnostic {
            code: DiagnosticCode::LycRetargetObserved,
            severity: Severity::Info,
            message: format!(
                "LYC compare target changed {} time(s) in the run window.",
                input.lyc_write_count
            ),
        });
    }
    // A missing coincidence/STAT observation is a timing-investigation hint, not a verified missing interrupt.
    if input.stat_signal_count == 0 && input.executed_frames >= 1 && machine.ppu.lcdc & 0x80 != 0 {
        out.push(Diagnostic {
            code: DiagnosticCode::StatIrqPathSuspicious,
            severity: Severity::Info,
            message: "No STAT/coincidence signal was observed in the run window.".into(),
        });
    }
    out
}

#[cfg(test)]
mod auto_diagnosis_tests {
    use super::*;
    use kokura_core::Machine;

    #[test]
    // Check that a halted unsupported-opcode observation ranks first in the supplied synthetic evidence.
    fn auto_diagnosis_prioritizes_unsupported_opcode() {
        let mut machine = Machine::new();
        machine.ppu.lcdc = 0x91;
        let diagnostics = vec![Diagnostic {
            code: DiagnosticCode::UnsupportedOpcodeEncountered,
            severity: Severity::Error,
            message: "unsupported".to_string(),
        }];
        let report = build_auto_diagnosis(
            &machine,
            DiagnosticInput {
                unsupported_opcode_count: 1,
                halted_on_unsupported_opcode: true,
                ..DiagnosticInput::default()
            },
            &diagnostics,
            None,
        );
        assert!(!report.suspects.is_empty());
        assert_eq!(report.suspects[0].key, "unsupported_opcode_gap");
    }

    #[test]
    // Check that bank-switch/restore evidence creates a banking candidate.
    fn auto_diagnosis_emits_banking_suspect() {
        let machine = Machine::new();
        let diagnostics = vec![Diagnostic {
            code: DiagnosticCode::RepeatedBankThrash,
            severity: Severity::Warning,
            message: "thrash".to_string(),
        }];
        let report = build_auto_diagnosis(
            &machine,
            DiagnosticInput {
                bank_switches: 8,
                bank_thrash_score: 5,
                far_call_count: 3,
                bank_restored: false,
                ..DiagnosticInput::default()
            },
            &diagnostics,
            None,
        );
        assert!(report
            .suspects
            .iter()
            .any(|s| s.key == "bank_choreography_instability"));
    }

    #[test]
    // Check that direct P1 reads qualify as input sampling without a recognized input routine.
    fn analyze_basic_uses_ff00_reads_for_input_visibility_diagnostics() {
        let mut machine = Machine::new();
        machine.ppu.lcdc = 0x91;
        let diagnostics = analyze_basic(
            &machine,
            DiagnosticInput {
                input_active: true,
                joypad_read_count: 8,
                joypad_button_read_count: 4,
                joypad_dpad_read_count: 4,
                ..DiagnosticInput::default()
            },
        );
        assert!(diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::InputReadNoVisibleEffect));
    }

    #[test]
    // Check that the candidate evidence preserves the direct P1 read count.
    fn auto_diagnosis_input_suspect_mentions_ff00_reads() {
        let machine = Machine::new();
        let diagnostics = vec![Diagnostic {
            code: DiagnosticCode::InputReadNoVisibleEffect,
            severity: Severity::Info,
            message: "input".to_string(),
        }];
        let report = build_auto_diagnosis(
            &machine,
            DiagnosticInput {
                input_active: true,
                joypad_read_count: 12,
                joypad_button_read_count: 6,
                joypad_dpad_read_count: 6,
                ..DiagnosticInput::default()
            },
            &diagnostics,
            None,
        );
        let suspect = report
            .suspects
            .iter()
            .find(|s| s.key == "input_path_not_advancing_state")
            .expect("input suspect");
        assert!(suspect
            .evidence
            .iter()
            .any(|line| line.contains("FF00/P1 was read 12 time(s)")));
    }

    #[test]
    // Check the DMA candidate classification from injected start/completion counters and no diagnostics.
    // This test does not execute a DMA transfer or establish hardware parity.
    fn timing_pack_report_marks_dma_candidate_when_edges_and_completion_exist() {
        let report = build_timing_pack_report(
            DiagnosticInput {
                oam_dma_start_count: 1,
                oam_dma_complete_count: 1,
                hdma_start_count: 1,
                hdma_complete_count: 1,
                gdma_stall_cycles_estimate: 32,
                ..DiagnosticInput::default()
            },
            &[],
        );
        let dma = report.packs.iter().find(|p| p.key == "dma_timing").unwrap();
        assert!(matches!(
            dma.verification,
            TimingPackVerification::Candidate
        ));
    }

    #[test]
    // Check that trigger/mixer evidence without PCM cannot promote the APU pack to candidate.
    fn timing_pack_report_keeps_apu_regression_only_when_pcm_missing() {
        let report = build_timing_pack_report(
            DiagnosticInput {
                apu_trigger_count: 1,
                apu_mix_output_count: 2,
                ..DiagnosticInput::default()
            },
            &[],
        );
        let apu = report.packs.iter().find(|p| p.key == "apu_timing").unwrap();
        assert!(matches!(
            apu.verification,
            TimingPackVerification::RegressionOnly
        ));
    }
}
