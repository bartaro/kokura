use kokura_bridge::{
    map_parser::parse_map_text, source_map::parse_source_map_text, symbols::SymbolTable,
};

pub fn apply_symbol_map_text(table: &mut SymbolTable, text: &str) {
    let parsed = parse_map_text(text);
    table.symbols = parsed.symbols;
}

pub fn apply_source_map_text(table: &mut SymbolTable, text: &str) {
    let parsed = parse_source_map_text(text);
    table.source_locations.clear();
    table.merge_sources(parsed);
}

pub fn parse_symbol_table_json(text: &str) -> Result<SymbolTable, String> {
    serde_json::from_str::<SymbolTable>(text)
        .map_err(|err| format!("failed to parse SymbolTable JSON: {err}"))
}
