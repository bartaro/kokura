use std::{fs, path::Path};

use crate::symbols::{SymbolInfo, SymbolTable};

fn parse_kitaqgb_columnar(parts: &[&str], table: &mut SymbolTable) {
    if parts.len() < 6 {
        return;
    }
    let addr_cpu = parts[0];
    let bank = parts[1];
    let kind = parts[3];
    if !matches!(kind, "S" | "L") {
        return;
    }
    if let (Ok(start), Ok(bank)) = (u16::from_str_radix(addr_cpu, 16), bank.parse::<u16>()) {
        table.symbols.push(SymbolInfo {
            bank,
            start,
            end: u32::from(start).saturating_add(1),
            name: parts[5..].join(" "),
            kind: Some(kind.to_string()),
            region: Some(parts[4].to_string()),
            section: None,
        });
    }
}

fn parse_bank_addr_form(parts: &[&str], table: &mut SymbolTable) {
    if let Some((bank_hex, addr_hex)) = parts[0].split_once(':') {
        if let (Ok(bank), Ok(start)) = (
            u16::from_str_radix(bank_hex, 16),
            u16::from_str_radix(addr_hex, 16),
        ) {
            let mut name_index = 1;
            if parts
                .get(1)
                .is_some_and(|s| s.starts_with("0x") || s.chars().all(|c| c.is_ascii_hexdigit()))
            {
                name_index = 2;
            }
            if parts.len() > name_index {
                table.symbols.push(SymbolInfo {
                    bank,
                    start,
                    end: u32::from(start).saturating_add(1),
                    name: parts[name_index..].join(" "),
                    kind: None,
                    region: None,
                    section: None,
                });
            }
        }
    }
}

pub fn parse_map_text(text: &str) -> SymbolTable {
    let mut table = SymbolTable::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        if parts.len() >= 5
            && parts[0].len() == 4
            && parts[0].chars().all(|c| c.is_ascii_hexdigit())
            && parts[1].chars().all(|c| c.is_ascii_digit())
            && parts[2].len() == 4
            && parts[2].chars().all(|c| c.is_ascii_hexdigit())
        {
            parse_kitaqgb_columnar(&parts, &mut table);
            continue;
        }

        parse_bank_addr_form(&parts, &mut table);
    }

    table.symbols.sort_by_key(|s| (s.bank, s.start));
    for i in 0..table.symbols.len() {
        if i + 1 < table.symbols.len() && table.symbols[i].bank == table.symbols[i + 1].bank {
            let next = table.symbols[i + 1].start;
            table.symbols[i].end =
                u32::from(next).max(u32::from(table.symbols[i].start).saturating_add(1));
        } else {
            table.symbols[i].end = 0x10000;
        }
    }
    table
}

pub fn parse_map_file<P: AsRef<Path>>(path: P) -> std::io::Result<SymbolTable> {
    let text = fs::read_to_string(path)?;
    Ok(parse_map_text(&text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kitaqgb_columnar_map_keeps_region_out_of_symbol_name() {
        let table = parse_map_text(
            "; Addr(CPU) Bank Off Kind Region Name\n\
             06E3 0 06E3 S ROM __kq_thunk_b4_HM_MusicPump\n",
        );
        let symbol = table.lookup(0, 0x06E3).expect("parsed thunk symbol");
        assert_eq!(symbol.name, "__kq_thunk_b4_HM_MusicPump");
        assert_eq!(symbol.kind.as_deref(), Some("S"));
        assert_eq!(symbol.region.as_deref(), Some("ROM"));
    }
}
