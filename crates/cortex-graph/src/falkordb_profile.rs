//! FalkorDB `GRAPH.QUERY` micro-metrics for indexing performance analysis.
//!
//! Enable with `CORTEX_FALKORDB_PROFILE=1` (also on when `CORTEX_INDEX_PROFILE=1`).

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static QUERY_COUNT: AtomicU64 = AtomicU64::new(0);
static QUERY_BYTES_TOTAL: AtomicU64 = AtomicU64::new(0);
static QUERY_BYTES_MAX: AtomicU64 = AtomicU64::new(0);
static LOCK_WAIT_US_TOTAL: AtomicU64 = AtomicU64::new(0);
static QUERY_WALL_US_TOTAL: AtomicU64 = AtomicU64::new(0);
static QUERY_WALL_US_MAX: AtomicU64 = AtomicU64::new(0);

/// Whether FalkorDB query micro-profiling is active.
pub fn falkordb_profile_enabled() -> bool {
    fn truthy(name: &str) -> bool {
        std::env::var(name)
            .ok()
            .is_some_and(|v| matches!(v.as_str(), "1" | "true" | "yes"))
    }
    truthy("CORTEX_FALKORDB_PROFILE") || truthy("CORTEX_INDEX_PROFILE")
}

/// Reset counters (call at start of an index run).
pub fn reset() {
    QUERY_COUNT.store(0, Ordering::Relaxed);
    QUERY_BYTES_TOTAL.store(0, Ordering::Relaxed);
    QUERY_BYTES_MAX.store(0, Ordering::Relaxed);
    LOCK_WAIT_US_TOTAL.store(0, Ordering::Relaxed);
    QUERY_WALL_US_TOTAL.store(0, Ordering::Relaxed);
    QUERY_WALL_US_MAX.store(0, Ordering::Relaxed);
}

/// Record one executed `GRAPH.QUERY`.
pub fn record_query(query_bytes: usize, lock_wait: Duration, wall: Duration) {
    if !falkordb_profile_enabled() {
        return;
    }
    let bytes = query_bytes as u64;
    QUERY_COUNT.fetch_add(1, Ordering::Relaxed);
    QUERY_BYTES_TOTAL.fetch_add(bytes, Ordering::Relaxed);
    update_max(&QUERY_BYTES_MAX, bytes);
    LOCK_WAIT_US_TOTAL.fetch_add(lock_wait.as_micros() as u64, Ordering::Relaxed);
    let wall_us = wall.as_micros() as u64;
    QUERY_WALL_US_TOTAL.fetch_add(wall_us, Ordering::Relaxed);
    update_max(&QUERY_WALL_US_MAX, wall_us);
}

fn update_max(slot: &AtomicU64, value: u64) {
    let mut current = slot.load(Ordering::Relaxed);
    while value > current {
        match slot.compare_exchange_weak(current, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(v) => current = v,
        }
    }
}

/// Snapshot of FalkorDB write-path stats for an index run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FalkorDbProfileSnapshot {
    pub query_count: u64,
    pub query_bytes_total: u64,
    pub query_bytes_max: u64,
    pub query_bytes_avg: u64,
    pub lock_wait_secs: f64,
    pub query_wall_secs: f64,
    pub query_wall_secs_avg: f64,
    pub query_wall_secs_max: f64,
    pub lock_wait_fraction: f64,
}

/// Take a snapshot and optionally reset counters.
pub fn snapshot(reset_after: bool) -> FalkorDbProfileSnapshot {
    let query_count = QUERY_COUNT.load(Ordering::Relaxed);
    let query_bytes_total = QUERY_BYTES_TOTAL.load(Ordering::Relaxed);
    let query_bytes_max = QUERY_BYTES_MAX.load(Ordering::Relaxed);
    let lock_wait_us = LOCK_WAIT_US_TOTAL.load(Ordering::Relaxed);
    let query_wall_us = QUERY_WALL_US_TOTAL.load(Ordering::Relaxed);
    let query_wall_us_max = QUERY_WALL_US_MAX.load(Ordering::Relaxed);

    let lock_wait_secs = lock_wait_us as f64 / 1_000_000.0;
    let query_wall_secs = query_wall_us as f64 / 1_000_000.0;
    let query_wall_secs_max = query_wall_us_max as f64 / 1_000_000.0;
    let denom = query_count.max(1) as f64;
    let snap = FalkorDbProfileSnapshot {
        query_count,
        query_bytes_total,
        query_bytes_max,
        query_bytes_avg: if query_count > 0 {
            query_bytes_total / query_count
        } else {
            0
        },
        lock_wait_secs,
        query_wall_secs,
        query_wall_secs_avg: query_wall_secs / denom,
        query_wall_secs_max,
        lock_wait_fraction: if query_wall_secs > 0.0 {
            lock_wait_secs / query_wall_secs
        } else {
            0.0
        },
    };
    if reset_after {
        reset();
    }
    snap
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_aggregates() {
        reset();
        unsafe {
            std::env::set_var("CORTEX_FALKORDB_PROFILE", "1");
        }
        record_query(100, Duration::from_millis(1), Duration::from_millis(5));
        record_query(200, Duration::from_millis(2), Duration::from_millis(10));
        let s = snapshot(false);
        assert_eq!(s.query_count, 2);
        assert_eq!(s.query_bytes_total, 300);
        assert_eq!(s.query_bytes_max, 200);
        unsafe {
            std::env::remove_var("CORTEX_FALKORDB_PROFILE");
        }
        reset();
    }
}
