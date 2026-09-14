use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
// Named bank/address span with an exclusive u32 end, allowing 10000
// to represent the boundary beyond the largest CPU address.
pub struct SymbolInfo {
    pub bank: u16,
    pub start: u16,
    pub end: u32,
    pub name: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub section: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// Source position attached to a bank/address, with optional column,
// symbol and section metadata supplied by sidecar files.
pub struct SourceLocationInfo {
    pub bank: u16,
    pub addr: u16,
    pub path: String,
    pub line: u32,
    #[serde(default)]
    pub column: Option<u32>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub section: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// Compiler-supplied function range and calling-convention metadata;
// byte sizes describe the declared interface rather than an executed call.
pub struct FunctionInfo {
    pub name: String,
    pub bank: u16,
    pub start: u16,
    pub end: u32,
    pub size_bytes: u32,
    #[serde(default)]
    pub section: Option<String>,
    #[serde(default)]
    pub is_stack_call: bool,
    #[serde(default)]
    pub is_fast_call: bool,
    #[serde(default)]
    pub has_fixed_bank: bool,
    #[serde(default)]
    pub has_fixed_order: bool,
    #[serde(default)]
    pub return_size: u8,
    #[serde(default)]
    pub param_sizes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// Named storage location with size, region and optional bank context.
pub struct VariableInfo {
    pub name: String,
    pub address: u16,
    pub size: u16,
    pub region: String,
    #[serde(default)]
    pub bank: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// Static function/call counts and interface metadata from build output,
// kept separate from measured execution statistics.
pub struct StaticEstimateInfo {
    pub name: String,
    pub bank: u16,
    pub size_bytes: u32,
    #[serde(default)]
    pub incoming_call_count: u32,
    #[serde(default)]
    pub outgoing_call_count: u32,
    #[serde(default)]
    pub cross_bank_outgoing_count: u32,
    #[serde(default)]
    pub far_call_count: u32,
    #[serde(default)]
    pub is_stack_call: bool,
    #[serde(default)]
    pub is_fast_call: bool,
    #[serde(default)]
    pub return_size: u8,
    #[serde(default)]
    pub param_sizes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// Caller/callee relationship and provenance fields carried by sidecars.
// Signed bank fields are retained as supplied; merging does not add counts.
pub struct CallEdgeInfo {
    pub caller: String,
    pub callee: String,
    pub caller_bank: i32,
    pub callee_bank: i32,
    pub kind: String,
    #[serde(default)]
    pub via_thunk: bool,
    #[serde(default)]
    pub via_farcall: bool,
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub last_source: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
// Combined symbol, source, function, variable, estimate and call-edge
// collections. Missing optional collections deserialize as empty.
pub struct SymbolTable {
    pub symbols: Vec<SymbolInfo>,
    #[serde(default)]
    pub source_locations: Vec<SourceLocationInfo>,
    #[serde(default)]
    pub functions: Vec<FunctionInfo>,
    #[serde(default)]
    pub variables: Vec<VariableInfo>,
    #[serde(default)]
    pub static_estimates: Vec<StaticEstimateInfo>,
    #[serde(default)]
    pub call_edges: Vec<CallEdgeInfo>,
}

impl SymbolTable {
    // Report empty only when all six metadata collections contain no records.
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
            && self.source_locations.is_empty()
            && self.functions.is_empty()
            && self.variables.is_empty()
            && self.static_estimates.is_empty()
            && self.call_edges.is_empty()
    }

    // Append and sort locations, then remove adjacent records equal in every
    // source field. The sort key omits symbol/section, so this is not a global
    // set-based deduplication of all identical records.
    pub fn merge_sources(&mut self, mut other: Vec<SourceLocationInfo>) {
        self.source_locations.append(&mut other);
        self.source_locations.sort_by_key(|s| {
            (
                s.bank,
                s.addr,
                s.path.clone(),
                s.line,
                s.column.unwrap_or(0),
            )
        });
        self.source_locations.dedup_by(|a, b| {
            a.bank == b.bank
                && a.addr == b.addr
                && a.path == b.path
                && a.line == b.line
                && a.column == b.column
                && a.symbol == b.symbol
                && a.section == b.section
        });
    }

    // Append and sort each metadata collection, removing adjacent records
    // using collection-specific equality keys. This retains representative
    // records; call counts and static estimates are not summed or recomputed.
    pub fn merge_from(&mut self, mut other: SymbolTable) {
        self.symbols.append(&mut other.symbols);
        self.symbols
            .sort_by_key(|s| (s.bank, s.start, s.name.clone()));
        self.symbols.dedup_by(|a, b| {
            a.bank == b.bank
                && a.start == b.start
                && a.end == b.end
                && a.name == b.name
                && a.kind == b.kind
                && a.region == b.region
                && a.section == b.section
        });

        self.source_locations.append(&mut other.source_locations);
        self.source_locations.sort_by_key(|s| {
            (
                s.bank,
                s.addr,
                s.path.clone(),
                s.line,
                s.column.unwrap_or(0),
            )
        });
        self.source_locations.dedup_by(|a, b| {
            a.bank == b.bank
                && a.addr == b.addr
                && a.path == b.path
                && a.line == b.line
                && a.column == b.column
                && a.symbol == b.symbol
                && a.section == b.section
        });

        self.functions.append(&mut other.functions);
        self.functions
            .sort_by_key(|f| (f.bank, f.start, f.name.clone()));
        // Function equality here ignores size and calling-convention metadata
        // when name, bank and range already agree.
        self.functions.dedup_by(|a, b| {
            a.bank == b.bank && a.start == b.start && a.end == b.end && a.name == b.name
        });

        self.variables.append(&mut other.variables);
        self.variables
            .sort_by_key(|v| (v.address, v.name.clone(), v.bank.unwrap_or(0)));
        // Variable equality uses address, size, name and bank, but not region.
        self.variables.dedup_by(|a, b| {
            a.address == b.address && a.size == b.size && a.name == b.name && a.bank == b.bank
        });

        self.static_estimates.append(&mut other.static_estimates);
        self.static_estimates
            .sort_by_key(|s| (s.bank, s.name.clone()));
        self.static_estimates
            .dedup_by(|a, b| a.bank == b.bank && a.name == b.name);

        self.call_edges.append(&mut other.call_edges);
        self.call_edges.sort_by_key(|c| {
            (
                c.caller.clone(),
                c.callee.clone(),
                c.caller_bank,
                c.callee_bank,
            )
        });
        // Keep different counts/kinds as separate records; last_source is not
        // part of the adjacent-duplicate comparison.
        self.call_edges.dedup_by(|a, b| {
            a.caller == b.caller
                && a.callee == b.callee
                && a.caller_bank == b.caller_bank
                && a.callee_bank == b.callee_bank
                && a.kind == b.kind
                && a.via_thunk == b.via_thunk
                && a.via_farcall == b.via_farcall
                && a.count == b.count
        });
    }

    // Return the first symbol in this bank whose half-open range contains
    // the address; overlapping ranges use collection order.
    pub fn lookup(&self, bank: u16, addr: u16) -> Option<&SymbolInfo> {
        let addr = u32::from(addr);
        self.symbols
            .iter()
            .find(|s| s.bank == bank && addr >= u32::from(s.start) && addr < s.end)
    }

    // Find the greatest symbol start at or below the address in this bank,
    // without requiring the address to remain inside that symbol range.
    pub fn lookup_nearest_before(&self, bank: u16, addr: u16) -> Option<&SymbolInfo> {
        self.symbols
            .iter()
            .filter(|s| s.bank == bank && s.start <= addr)
            .max_by_key(|s| s.start)
    }

    // Find the smallest symbol start strictly above the address in this bank.
    pub fn lookup_next_after(&self, bank: u16, addr: u16) -> Option<&SymbolInfo> {
        self.symbols
            .iter()
            .filter(|s| s.bank == bank && s.start > addr)
            .min_by_key(|s| s.start)
    }

    // Prefer an exact bank/address source position, falling back to the
    // nearest preceding position without a maximum distance limit.
    pub fn lookup_source(&self, bank: u16, addr: u16) -> Option<&SourceLocationInfo> {
        self.source_locations
            .iter()
            .find(|s| s.bank == bank && s.addr == addr)
            .or_else(|| self.lookup_source_nearest_before(bank, addr))
    }

    // Find the greatest source address at or below the requested address
    // within the same bank, irrespective of function boundaries.
    pub fn lookup_source_nearest_before(
        &self,
        bank: u16,
        addr: u16,
    ) -> Option<&SourceLocationInfo> {
        self.source_locations
            .iter()
            .filter(|s| s.bank == bank && s.addr <= addr)
            .max_by_key(|s| s.addr)
    }

    // Find the next strictly greater source address in the same bank.
    pub fn lookup_source_next_after(&self, bank: u16, addr: u16) -> Option<&SourceLocationInfo> {
        self.source_locations
            .iter()
            .filter(|s| s.bank == bank && s.addr > addr)
            .min_by_key(|s| s.addr)
    }

    // Return the first function whose bank and half-open address range match.
    pub fn lookup_function(&self, bank: u16, addr: u16) -> Option<&FunctionInfo> {
        let addr = u32::from(addr);
        self.functions
            .iter()
            .find(|f| f.bank == bank && addr >= u32::from(f.start) && addr < f.end)
    }

    // Resolve a containing function, then match its bank and name to the
    // first static estimate; no runtime measurements are inferred here.
    pub fn lookup_static_estimate(&self, bank: u16, addr: u16) -> Option<&StaticEstimateInfo> {
        let function = self.lookup_function(bank, addr)?;
        self.static_estimates
            .iter()
            .find(|estimate| estimate.bank == function.bank && estimate.name == function.name)
    }

    // Return the first exact name match without disambiguating variable bank
    // or address when multiple entries share that name.
    pub fn lookup_variable_by_name(&self, name: &str) -> Option<&VariableInfo> {
        self.variables.iter().find(|variable| variable.name == name)
    }
}
