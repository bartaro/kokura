use std::{fs, path::Path};

use crate::symbols::SourceLocationInfo;

// Trim and remove an optional hex prefix, then try hexadecimal before
// decimal. Thus unprefixed digit-only bank/address tokens are normally hex.
fn parse_u16_token(token: &str) -> Option<u16> {
    let token = token.trim();
    let token = token
        .strip_prefix("0x")
        .or_else(|| token.strip_prefix("0X"))
        .unwrap_or(token);
    u16::from_str_radix(token, 16)
        .ok()
        .or_else(|| token.parse::<u16>().ok())
}

// Parse line/column numbers as decimal unless an explicit 0x/0X prefix
// selects hexadecimal.
fn parse_u32_token(token: &str) -> Option<u32> {
    let token = token.trim();
    if let Some(hex) = token
        .strip_prefix("0x")
        .or_else(|| token.strip_prefix("0X"))
    {
        return u32::from_str_radix(hex, 16).ok();
    }
    token.parse::<u32>().ok()
}

// Split one bank:address token and decode both halves with the u16 rules.
fn parse_bank_addr(token: &str) -> Option<(u16, u16)> {
    let (bank, addr) = token.split_once(':')?;
    Some((parse_u16_token(bank)?, parse_u16_token(addr)?))
}

// Split from the right to recognize path:line:column or path:line while
// retaining earlier colons in the path, such as a Windows drive prefix.
fn parse_path_line_col(token: &str) -> Option<(String, u32, Option<u32>)> {
    let (before_col, col) = token.rsplit_once(':')?;
    if let Some((path, line)) = before_col.rsplit_once(':') {
        if let (Some(line), Some(col)) = (parse_u32_token(line), parse_u32_token(col)) {
            return Some((path.to_string(), line, Some(col)));
        }
    }
    parse_u32_token(col).map(|line| (before_col.to_string(), line, None))
}

// Parse bank:address or separate bank/address rows with source positions
// and optional symbol=/section= metadata. Skip invalid or zero-line entries
// and sort accepted positions by bank/address; paths are whitespace-delimited.
pub fn parse_source_map_text(text: &str) -> Vec<SourceLocationInfo> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        // Prefer the compact address form and path suffix notation; otherwise
        // accept positional path, line and column fields.
        let parsed = if let Some((bank, addr)) = parse_bank_addr(parts[0]) {
            if let Some((path, line_no, col)) = parse_path_line_col(parts[1]) {
                let symbol = parts
                    .iter()
                    .skip(2)
                    .find_map(|part| part.strip_prefix("symbol="))
                    .map(str::to_string);
                let section = parts
                    .iter()
                    .skip(2)
                    .find_map(|part| part.strip_prefix("section="))
                    .map(str::to_string);
                Some(SourceLocationInfo {
                    bank,
                    addr,
                    path,
                    line: line_no,
                    column: col,
                    symbol,
                    section,
                })
            } else if parts.len() >= 4 {
                let symbol = parts
                    .iter()
                    .skip(4)
                    .find_map(|part| part.strip_prefix("symbol="))
                    .map(str::to_string);
                let section = parts
                    .iter()
                    .skip(4)
                    .find_map(|part| part.strip_prefix("section="))
                    .map(str::to_string);
                Some(SourceLocationInfo {
                    bank,
                    addr,
                    path: parts[1].to_string(),
                    line: parse_u32_token(parts[2]).unwrap_or(0),
                    column: parse_u32_token(parts[3]),
                    symbol,
                    section,
                })
            } else {
                None
            }
        } else if parts.len() >= 4 {
            if let (Some(bank), Some(addr), Some(line_no)) = (
                parse_u16_token(parts[0]),
                parse_u16_token(parts[1]),
                parse_u32_token(parts[3]),
            ) {
                let symbol = parts
                    .iter()
                    .skip(4)
                    .find_map(|part| part.strip_prefix("symbol="))
                    .map(str::to_string);
                let section = parts
                    .iter()
                    .skip(4)
                    .find_map(|part| part.strip_prefix("section="))
                    .map(str::to_string);
                Some(SourceLocationInfo {
                    bank,
                    addr,
                    path: parts[2].to_string(),
                    line: line_no,
                    column: parts.get(4).and_then(|v| parse_u32_token(v)),
                    symbol,
                    section,
                })
            } else {
                None
            }
        } else {
            None
        };

        if let Some(info) = parsed {
            // Discard rows with no path or a missing/invalid line number rather
            // than emitting an unusable source location.
            if !info.path.is_empty() && info.line > 0 {
                out.push(info);
            }
        }
    }
    out.sort_by_key(|s| (s.bank, s.addr));
    out
}

// Read a UTF-8 source-map file and delegate permissive row parsing.
pub fn parse_source_map_file<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<SourceLocationInfo>> {
    let text = fs::read_to_string(path)?;
    Ok(parse_source_map_text(&text))
}

#[cfg(test)]
mod tests {
    use super::parse_source_map_text;

    #[test]
    // Verify one bank:address plus path:line:column row, including optional
    // symbol and section metadata. Other input forms are outside this test.
    fn parses_bank_addr_path_line() {
        let out =
            parse_source_map_text("01:4000 src/main.c:120:7 symbol=MainLoop section=render\n");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].bank, 1);
        assert_eq!(out[0].addr, 0x4000);
        assert_eq!(out[0].path, "src/main.c");
        assert_eq!(out[0].line, 120);
        assert_eq!(out[0].column, Some(7));
        assert_eq!(out[0].symbol.as_deref(), Some("MainLoop"));
        assert_eq!(out[0].section.as_deref(), Some("render"));
    }
}
