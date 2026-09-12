use serde::{Deserialize, Serialize};

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MemoryWatchBaselineMode {
    #[default]
    Initial,
    PreviousFrame,
    Named,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryWatchSpec {
    pub name: String,
    pub addr: u16,
    pub size: u16,
}

impl MemoryWatchSpec {
    pub fn validate(self) -> Result<Self, String> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err("memory watch name must not be empty".to_string());
        }
        if self.size == 0 {
            return Err(format!("memory watch '{name}' must have a non-zero size"));
        }
        let end = u32::from(self.addr) + u32::from(self.size);
        if end > 0x1_0000 {
            return Err(format!(
                "memory watch '{name}' exceeds address space: 0x{:04X}+0x{:04X}",
                self.addr, self.size
            ));
        }
        Ok(Self {
            name: name.to_string(),
            addr: self.addr,
            size: self.size,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryWatchDiffByte {
    pub offset: u16,
    pub addr: u16,
    pub before: u8,
    pub after: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryWatchInsight {
    pub kind: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryWatchResult {
    pub name: String,
    pub addr: u16,
    pub size: u16,
    pub hash: u32,
    pub nonzero_bytes: u32,
    pub changed: bool,
    pub changed_bytes: u32,
    pub first_change_addr: Option<u16>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub preview_bytes: Vec<u8>,
    #[serde(skip_serializing_if = "is_false", default)]
    pub preview_truncated: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub baseline_preview_bytes: Vec<u8>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub diff_preview: Vec<MemoryWatchDiffByte>,
    #[serde(skip_serializing_if = "is_false", default)]
    pub diff_preview_truncated: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub watch_insights: Vec<MemoryWatchInsight>,
}

#[cfg(test)]
mod tests {
    use super::{MemoryWatchBaselineMode, MemoryWatchSpec};

    #[test]
    fn memory_watch_validation_rejects_empty_name() {
        let err = MemoryWatchSpec {
            name: "   ".to_string(),
            addr: 0xC000,
            size: 0x20,
        }
        .validate()
        .unwrap_err();
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn memory_watch_validation_rejects_zero_size() {
        let err = MemoryWatchSpec {
            name: "WRAM".to_string(),
            addr: 0xC000,
            size: 0,
        }
        .validate()
        .unwrap_err();
        assert!(err.contains("non-zero size"));
    }

    #[test]
    fn memory_watch_validation_rejects_overflow() {
        let err = MemoryWatchSpec {
            name: "Tail".to_string(),
            addr: 0xFFF0,
            size: 0x20,
        }
        .validate()
        .unwrap_err();
        assert!(err.contains("exceeds address space"));
    }

    #[test]
    fn memory_watch_validation_accepts_valid_window() {
        let spec = MemoryWatchSpec {
            name: " WRAM ".to_string(),
            addr: 0xC000,
            size: 0x40,
        }
        .validate()
        .unwrap();
        assert_eq!(spec.name, "WRAM");
        assert_eq!(spec.addr, 0xC000);
        assert_eq!(spec.size, 0x40);
    }

    #[test]
    fn baseline_mode_defaults_to_initial() {
        assert_eq!(
            MemoryWatchBaselineMode::default(),
            MemoryWatchBaselineMode::Initial
        );
    }
}
