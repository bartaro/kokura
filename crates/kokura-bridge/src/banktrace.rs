use serde::{Deserialize, Serialize};

use crate::symbols::SymbolTable;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankTraceInfo {
    pub from_bank: u16,
    pub to_bank: u16,
    pub from_symbol: Option<String>,
    pub to_symbol: Option<String>,
    pub suspected_far_call: bool,
}

fn far_like_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    (lower
        .find("__kq_thunk_b")
        .is_some_and(|start| !lower[start..].ends_with("_main")))
        || lower.starts_with("__kq_far_")
        || lower.starts_with("__kq_farcall_")
        || lower.contains("farcall")
        || lower.contains("callbank")
}

fn symbol_bank_for_addr(selected_bank: u16, addr: u16) -> u16 {
    if addr < 0x4000 {
        0
    } else {
        selected_bank
    }
}

pub fn describe_bank_switch(
    symbols: Option<&SymbolTable>,
    from_bank: u16,
    to_bank: u16,
    addr: u16,
) -> BankTraceInfo {
    let from_code_bank = symbol_bank_for_addr(from_bank, addr);
    let to_code_bank = symbol_bank_for_addr(to_bank, addr);
    let from_symbol = symbols
        .and_then(|table| {
            table
                .lookup(from_code_bank, addr)
                .or_else(|| table.lookup_nearest_before(from_code_bank, addr))
        })
        .map(|s| s.name.clone());
    let to_symbol = symbols
        .and_then(|table| {
            table
                .lookup(to_code_bank, addr)
                .or_else(|| table.lookup_nearest_before(to_code_bank, addr))
        })
        .map(|s| s.name.clone());

    let suspected_far_call = from_bank != to_bank
        && (from_symbol.as_deref().is_some_and(far_like_name)
            || to_symbol.as_deref().is_some_and(far_like_name));

    BankTraceInfo {
        from_bank,
        to_bank,
        from_symbol,
        to_symbol,
        suspected_far_call,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::{SymbolInfo, SymbolTable};

    fn symbol(bank: u16, start: u16, end: u32, name: &str) -> SymbolInfo {
        SymbolInfo {
            bank,
            start,
            end,
            name: name.to_string(),
            kind: Some("function".to_string()),
            region: Some("rom".to_string()),
            section: None,
        }
    }

    #[test]
    fn fixed_bank_switch_uses_bank_zero_thunk_symbol() {
        let table = SymbolTable {
            symbols: vec![symbol(0, 0x0200, 0x0240, "__kq_thunk_b3_Draw")],
            ..SymbolTable::default()
        };
        let trace = describe_bank_switch(Some(&table), 1, 3, 0x0210);
        assert_eq!(trace.from_symbol.as_deref(), Some("__kq_thunk_b3_Draw"));
        assert_eq!(trace.to_symbol.as_deref(), Some("__kq_thunk_b3_Draw"));
        assert!(trace.suspected_far_call);
    }

    #[test]
    fn ordinary_audio_symbol_is_not_a_far_call() {
        let table = SymbolTable {
            symbols: vec![symbol(1, 0x4000, 0x4100, "Audio_Update")],
            ..SymbolTable::default()
        };
        let trace = describe_bank_switch(Some(&table), 1, 2, 0x4050);
        assert!(!trace.suspected_far_call);
    }

    #[test]
    fn nonreturning_main_thunk_is_not_tracked_as_a_far_call() {
        let table = SymbolTable {
            symbols: vec![symbol(0, 0x0300, 0x0340, "__kq_thunk_b3_main")],
            ..SymbolTable::default()
        };
        let trace = describe_bank_switch(Some(&table), 1, 3, 0x0310);
        assert!(!trace.suspected_far_call);
    }

    #[test]
    fn generated_thunk_internal_label_is_recognized() {
        let table = SymbolTable {
            symbols: vec![symbol(
                0,
                0x0200,
                0x0240,
                "kq_thunk_depth_ok___kq_thunk_b4_MusicPump",
            )],
            ..SymbolTable::default()
        };
        let trace = describe_bank_switch(Some(&table), 3, 4, 0x0210);
        assert!(trace.suspected_far_call);
    }
}
