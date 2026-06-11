//! Optional timing collection for FalkorDB edge bulk writes.

use std::collections::HashMap;
use std::time::Duration;

/// Accumulated write timing per Cypher relationship type.
#[derive(Debug, Default, Clone)]
pub struct RelTypeBoltStats {
    pub bolt_executions: u64,
    pub elapsed: Duration,
}

/// Profile data for one edge-flush pass (`CORTEX_INDEX_PROFILE=1`).
#[derive(Debug, Default, Clone)]
pub struct EdgeWriteProfile {
    pub by_rel: HashMap<String, RelTypeBoltStats>,
}

impl EdgeWriteProfile {
    pub fn record(&mut self, rel_type: &str, bolt_executions: u64, elapsed: Duration) {
        let entry = self.by_rel.entry(rel_type.to_string()).or_default();
        entry.bolt_executions += bolt_executions;
        entry.elapsed += elapsed;
    }
}
