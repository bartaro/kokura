use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecuteBreakpointSpec {
    #[serde(default)]
    pub pc: Option<u16>,
    #[serde(default)]
    pub bank: Option<u16>,
    #[serde(default)]
    pub symbol: Option<String>,
}

impl ExecuteBreakpointSpec {
    pub fn label(&self) -> String {
        if let Some(symbol) = &self.symbol {
            if let Some(bank) = self.bank {
                return format!("symbol:{}@bank:{}", symbol, bank);
            }
            return format!("symbol:{symbol}");
        }
        if let Some(pc) = self.pc {
            if let Some(bank) = self.bank {
                return format!("pc:{pc:04X}@bank:{bank}");
            }
            return format!("pc:{pc:04X}");
        }
        "breakpoint:<invalid>".to_string()
    }

    pub fn matches(&self, rom_bank: u16, pc: u16, symbol: Option<&str>) -> bool {
        if let Some(expected_bank) = self.bank {
            if expected_bank != rom_bank {
                return false;
            }
        }
        if let Some(expected_pc) = self.pc {
            if expected_pc != pc {
                return false;
            }
        }
        if let Some(expected_symbol) = &self.symbol {
            return symbol.is_some_and(|current| current == expected_symbol);
        }
        self.pc.is_some()
    }

    pub fn validate(self) -> Result<Self, String> {
        if self.pc.is_none()
            && self
                .symbol
                .as_ref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(true)
        {
            return Err("breakpoint must specify either pc or symbol".to_string());
        }
        Ok(Self {
            pc: self.pc,
            bank: self.bank,
            symbol: self
                .symbol
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryWatchpointSpec {
    #[serde(default)]
    pub name: Option<String>,
    pub addr: u16,
    #[serde(default = "default_watch_size")]
    pub size: u16,
}

const fn default_watch_size() -> u16 {
    1
}

impl MemoryWatchpointSpec {
    pub fn label(&self) -> String {
        self.name.clone().unwrap_or_else(|| {
            if self.size <= 1 {
                format!("watch@{:04X}", self.addr)
            } else {
                format!("watch@{:04X}+{:04X}", self.addr, self.size)
            }
        })
    }

    pub fn validate(self) -> Result<Self, String> {
        if self.size == 0 {
            return Err("watchpoint size must be non-zero".to_string());
        }
        let end = u32::from(self.addr) + u32::from(self.size);
        if end > 0x1_0000 {
            return Err(format!(
                "watchpoint exceeds address space: 0x{:04X}+0x{:04X}",
                self.addr, self.size
            ));
        }
        Ok(Self {
            name: self
                .name
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            addr: self.addr,
            size: self.size,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MmioWriteStopSpec {
    pub addr: u16,
    #[serde(default)]
    pub name: Option<String>,
}

impl MmioWriteStopSpec {
    pub fn label(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| format!("mmio@{:04X}", self.addr))
    }

    pub fn validate(self) -> Result<Self, String> {
        if !(0xFF00..=0xFFFF).contains(&self.addr) {
            return Err(format!(
                "MMIO stop address must be in FF00-FFFF, got 0x{:04X}",
                self.addr
            ));
        }
        Ok(Self {
            addr: self.addr,
            name: self
                .name
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InterruptStopPhase {
    Requested,
    Serviced,
    Blocked,
    Any,
}

impl Default for InterruptStopPhase {
    fn default() -> Self {
        Self::Any
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InterruptStopSpec {
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub phase: InterruptStopPhase,
}

impl InterruptStopSpec {
    pub fn label(&self) -> String {
        let phase = match self.phase {
            InterruptStopPhase::Requested => "requested",
            InterruptStopPhase::Serviced => "serviced",
            InterruptStopPhase::Blocked => "blocked",
            InterruptStopPhase::Any => "any",
        };
        if let Some(source) = &self.source {
            format!("irq:{source}:{phase}")
        } else {
            format!("irq:any:{phase}")
        }
    }

    pub fn validate(self) -> Result<Self, String> {
        let source = self
            .source
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty());
        if let Some(source) = &source {
            match source.as_str() {
                "vblank" | "lcd_stat" | "timer" | "serial" | "joypad" | "any" => {}
                other => {
                    return Err(format!(
                        "unsupported interrupt source '{other}' (expected vblank/lcd_stat/timer/serial/joypad/any)"
                    ))
                }
            }
        }
        Ok(Self {
            source,
            phase: self.phase,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DmaStopSpec {
    pub event: String,
}

impl DmaStopSpec {
    pub fn label(&self) -> String {
        format!("dma:{}", self.event)
    }

    pub fn validate(self) -> Result<Self, String> {
        let event = self.event.trim().to_ascii_lowercase();
        match event.as_str() {
            "oam_start"
            | "oam_complete"
            | "hdma_start"
            | "hdma_block"
            | "hdma_complete"
            | "hdma_cancel"
            | "gdma_stall"
            | "hdma_deferred"
            | "hdma_ignored" => Ok(Self { event }),
            _ => Err(format!(
                "unsupported dma stop event '{}' (expected oam_start/oam_complete/hdma_start/hdma_block/hdma_complete/hdma_cancel/gdma_stall/hdma_deferred/hdma_ignored)",
                self.event
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StopConditionSet {
    #[serde(default)]
    pub breakpoints: Vec<ExecuteBreakpointSpec>,
    #[serde(default)]
    pub watchpoints: Vec<MemoryWatchpointSpec>,
    #[serde(default)]
    pub mmio_writes: Vec<MmioWriteStopSpec>,
    #[serde(default)]
    pub interrupts: Vec<InterruptStopSpec>,
    #[serde(default)]
    pub dma_events: Vec<DmaStopSpec>,
}

impl StopConditionSet {
    pub fn validate(mut self) -> Result<Self, String> {
        self.breakpoints = self
            .breakpoints
            .into_iter()
            .map(ExecuteBreakpointSpec::validate)
            .collect::<Result<Vec<_>, _>>()?;
        self.watchpoints = self
            .watchpoints
            .into_iter()
            .map(MemoryWatchpointSpec::validate)
            .collect::<Result<Vec<_>, _>>()?;
        self.mmio_writes = self
            .mmio_writes
            .into_iter()
            .map(MmioWriteStopSpec::validate)
            .collect::<Result<Vec<_>, _>>()?;
        self.interrupts = self
            .interrupts
            .into_iter()
            .map(InterruptStopSpec::validate)
            .collect::<Result<Vec<_>, _>>()?;
        self.dma_events = self
            .dma_events
            .into_iter()
            .map(DmaStopSpec::validate)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self)
    }

    pub fn is_empty(&self) -> bool {
        self.breakpoints.is_empty()
            && self.watchpoints.is_empty()
            && self.mmio_writes.is_empty()
            && self.interrupts.is_empty()
            && self.dma_events.is_empty()
    }

    pub fn merged(&self, overlay: &StopConditionSet) -> StopConditionSet {
        let mut merged = self.clone();
        merged.breakpoints.extend(overlay.breakpoints.clone());
        merged.watchpoints.extend(overlay.watchpoints.clone());
        merged.mmio_writes.extend(overlay.mmio_writes.clone());
        merged.interrupts.extend(overlay.interrupts.clone());
        merged.dma_events.extend(overlay.dma_events.clone());
        merged
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayControlSet {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_replay_checkpoint_interval_frames")]
    pub checkpoint_interval_frames: u64,
    #[serde(default = "default_replay_max_checkpoints")]
    pub max_checkpoints: usize,
    #[serde(default)]
    pub auto_rewind_on_stop_frames: Option<u64>,
    #[serde(default)]
    pub stop_on_divergence: bool,
}

const fn default_replay_checkpoint_interval_frames() -> u64 {
    1
}

const fn default_replay_max_checkpoints() -> usize {
    16
}

impl Default for ReplayControlSet {
    fn default() -> Self {
        Self {
            enabled: false,
            checkpoint_interval_frames: default_replay_checkpoint_interval_frames(),
            max_checkpoints: default_replay_max_checkpoints(),
            auto_rewind_on_stop_frames: None,
            stop_on_divergence: false,
        }
    }
}

impl ReplayControlSet {
    pub fn validate(self) -> Result<Self, String> {
        if self.enabled {
            if self.checkpoint_interval_frames == 0 {
                return Err("replay checkpoint interval must be non-zero".to_string());
            }
            if self.max_checkpoints == 0 {
                return Err("replay max_checkpoints must be non-zero".to_string());
            }
            if self.auto_rewind_on_stop_frames == Some(0) {
                return Err(
                    "replay auto_rewind_on_stop_frames must be non-zero when provided".to_string(),
                );
            }
        }
        Ok(self)
    }

    pub fn merged(&self, overlay: &ReplayControlSet) -> ReplayControlSet {
        let default = ReplayControlSet::default();
        ReplayControlSet {
            enabled: self.enabled || overlay.enabled,
            checkpoint_interval_frames: if overlay.checkpoint_interval_frames
                != default.checkpoint_interval_frames
            {
                overlay.checkpoint_interval_frames
            } else {
                self.checkpoint_interval_frames
            },
            max_checkpoints: if overlay.max_checkpoints != default.max_checkpoints {
                overlay.max_checkpoints
            } else {
                self.max_checkpoints
            },
            auto_rewind_on_stop_frames: overlay
                .auto_rewind_on_stop_frames
                .or(self.auto_rewind_on_stop_frames),
            stop_on_divergence: self.stop_on_divergence || overlay.stop_on_divergence,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceLocationStop {
    pub path: String,
    pub line: u32,
    pub column: Option<u32>,
    pub bank: u16,
    pub addr: u16,
    pub symbol: Option<String>,
    pub section: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionContextFrame {
    pub reason: String,
    pub rom_bank: u16,
    pub pc: u16,
    pub sp: u16,
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub stack_preview: Vec<u8>,
    pub symbol: Option<String>,
    pub source: Option<SourceLocationStop>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StopReason {
    pub kind: String,
    pub label: String,
    pub detail: String,
    pub frame: u64,
    pub cycle: u64,
    pub pc: u16,
    pub rom_bank: u16,
    pub symbol: Option<String>,
    pub source: Option<SourceLocationStop>,
    pub execution_context_trail: Vec<ExecutionContextFrame>,
}

#[cfg(test)]
mod tests {
    use super::{
        DmaStopSpec, ExecuteBreakpointSpec, InterruptStopPhase, InterruptStopSpec,
        MemoryWatchpointSpec, MmioWriteStopSpec, ReplayControlSet, StopConditionSet,
    };

    #[test]
    fn breakpoint_requires_pc_or_symbol() {
        let err = ExecuteBreakpointSpec::default().validate().unwrap_err();
        assert!(err.contains("either pc or symbol"));
    }

    #[test]
    fn watchpoint_rejects_overflow() {
        let err = MemoryWatchpointSpec {
            name: None,
            addr: 0xFFF0,
            size: 0x20,
        }
        .validate()
        .unwrap_err();
        assert!(err.contains("exceeds address space"));
    }

    #[test]
    fn mmio_requires_ff_range() {
        let err = MmioWriteStopSpec {
            addr: 0xC000,
            name: None,
        }
        .validate()
        .unwrap_err();
        assert!(err.contains("FF00-FFFF"));
    }

    #[test]
    fn interrupt_spec_normalizes_source() {
        let spec = InterruptStopSpec {
            source: Some(" Timer ".to_string()),
            phase: InterruptStopPhase::Serviced,
        }
        .validate()
        .unwrap();
        assert_eq!(spec.source.as_deref(), Some("timer"));
    }

    #[test]
    fn dma_event_rejects_unknown_value() {
        let err = DmaStopSpec {
            event: "weird".to_string(),
        }
        .validate()
        .unwrap_err();
        assert!(err.contains("unsupported dma stop event"));
    }

    #[test]
    fn stop_condition_set_merges_lists() {
        let base = StopConditionSet {
            breakpoints: vec![ExecuteBreakpointSpec {
                pc: Some(0x0150),
                bank: None,
                symbol: None,
            }],
            ..StopConditionSet::default()
        };
        let overlay = StopConditionSet {
            mmio_writes: vec![MmioWriteStopSpec {
                addr: 0xFF46,
                name: None,
            }],
            ..StopConditionSet::default()
        };
        let merged = base.merged(&overlay);
        assert_eq!(merged.breakpoints.len(), 1);
        assert_eq!(merged.mmio_writes.len(), 1);
    }

    #[test]
    fn replay_control_rejects_zero_interval_when_enabled() {
        let err = ReplayControlSet {
            enabled: true,
            checkpoint_interval_frames: 0,
            ..ReplayControlSet::default()
        }
        .validate()
        .unwrap_err();
        assert!(err.contains("interval"));
    }

    #[test]
    fn replay_control_merges_overlay_values() {
        let base = ReplayControlSet {
            enabled: true,
            checkpoint_interval_frames: 2,
            max_checkpoints: 8,
            auto_rewind_on_stop_frames: Some(3),
            stop_on_divergence: false,
        };
        let overlay = ReplayControlSet {
            enabled: true,
            checkpoint_interval_frames: 4,
            max_checkpoints: 32,
            auto_rewind_on_stop_frames: Some(1),
            stop_on_divergence: true,
        };
        let merged = base.merged(&overlay);
        assert_eq!(merged.checkpoint_interval_frames, 4);
        assert_eq!(merged.max_checkpoints, 32);
        assert_eq!(merged.auto_rewind_on_stop_frames, Some(1));
        assert!(merged.stop_on_divergence);
    }
}
