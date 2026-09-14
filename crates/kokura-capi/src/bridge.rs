use kokura_bridge::{
    map_parser::parse_map_text, source_map::parse_source_map_text, symbols::SymbolTable,
};

// Replace only symbol spans with parsed map entries, retaining other
// metadata collections. Malformed map rows are skipped by the parser.
pub fn apply_symbol_map_text(table: &mut SymbolTable, text: &str) {
    let parsed = parse_map_text(text);
    table.symbols = parsed.symbols;
}

// Replace source locations and normalize their ordering/duplicates
// through merge_sources, retaining symbols and other metadata.
pub fn apply_source_map_text(table: &mut SymbolTable, text: &str) {
    let parsed = parse_source_map_text(text);
    table.source_locations.clear();
    table.merge_sources(parsed);
}

// Decode the complete SymbolTable shape and expose parse failure
// as an English error string; this does not validate every metadata range.
pub fn parse_symbol_table_json(text: &str) -> Result<SymbolTable, String> {
    serde_json::from_str::<SymbolTable>(text)
        .map_err(|err| format!("failed to parse SymbolTable JSON: {err}"))
}
