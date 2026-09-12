use std::{fs, path::Path};

use crate::symbols::SourceLocationInfo;

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

fn parse_bank_addr(token: &str) -> Option<(u16, u16)> {
    let (bank, addr) = token.split_once(':')?;
    Some((parse_u16_token(bank)?, parse_u16_token(addr)?))
}

fn parse_path_line_col(token: &str) -> Option<(String, u32, Option<u32>)> {
    let (before_col, col) = token.rsplit_once(':')?;
    if let Some((path, line)) = before_col.rsplit_once(':') {
        if let (Some(line), Some(col)) = (parse_u32_token(line), parse_u32_token(col)) {
            return Some((path.to_string(), line, Some(col)));
        }
    }
    parse_u32_token(col).map(|line| (before_col.to_string(), line, None))
}

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
            if !info.path.is_empty() && info.line > 0 {
                out.push(info);
            }
        }
    }
    out.sort_by_key(|s| (s.bank, s.addr));
    out
}

pub fn parse_source_map_file<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<SourceLocationInfo>> {
    let text = fs::read_to_string(path)?;
    Ok(parse_source_map_text(&text))
}

#[cfg(test)]
mod tests {
    use super::parse_source_map_text;

    #[test]
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
